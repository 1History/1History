use std::fs;
use std::io::Write;

use crate::progress::TUICollector;
use crate::source::Source;
use crate::{database::Database, util::full_timerange};
use anyhow::{Context, Error, Result};
use log::{debug, error, info, warn};

pub fn backup(history_files: Vec<String>, db_file: String, dry_run: bool) -> Result<()> {
    let (start, end) = full_timerange();
    debug!("files:{:?}, start:{}, end:{}", history_files, start, end);

    let db = Database::open(db_file).context("open 1History DB")?;

    let mut found = 0;
    let mut total_affected = 0;
    let mut total_duplicated = 0;
    let mut persist = |history_file: &str| {
        let s = Source::open(history_file).context("open")?;
        let rows = s.select(start, end).context("select")?.collect::<Vec<_>>();
        debug!("{:?} select {} histories", s.name(), rows.len());
        found += rows.len();

        info!("Begin backup {}...", &history_file);
        let collector = TUICollector::new(rows.len() as u64);
        if !dry_run {
            let (affected, duplicated) =
                db.persist(s.path(), rows, collector).context("persist")?;
            debug!(
                "{:?} affected:{}, duplicated:{}",
                s.name(),
                affected,
                duplicated
            );
            total_affected += affected;
            total_duplicated += duplicated;
        };
        info!("Finish backup {}", &history_file);

        Ok::<_, Error>(())
    };
    for his_file in &history_files {
        if let Err(e) = persist(his_file) {
            let msg = format!("{e:?}");
            if msg.contains("The database file is locked") {
                debug!("Open database directly failed, copy to temp and backup again");
                let mut tmp = match tempfile::NamedTempFile::new() {
                    Ok(f) => f,
                    Err(_) => continue,
                };
                let body = match fs::read(his_file) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if let Err(e) = tmp.write_all(&body) {
                    debug!("Copy to backup file failed, msg:{e}");
                    continue;
                }
                let path = tmp.into_temp_path();
                let path = path.to_string_lossy();
                if let Err(e) = persist(&path) {
                    debug!("{} persist failed, backup:{}, err: {:?}", his_file, path, e);
                }
            }
        }
    }

    info!("Summary\nFound:{found}, Imported:{total_affected}, Duplicated: {total_duplicated}");
    Ok(())
}
