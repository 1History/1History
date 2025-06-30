use anyhow::{Context, Result};
use log::{debug, info};
use std::{fs::OpenOptions, io::BufWriter};

use crate::{
    database::Database,
    util::{full_timerange, unixepoch_as_ymdhms},
};

pub fn export_csv(csv_file: String, db_file: String) -> Result<()> {
    let (start, end) = full_timerange();
    debug!("start:{start}, end:{end}");

    let db = Database::open(db_file).context("open 1History DB")?;
    let f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&csv_file)
        .context(csv_file.clone())?;
    let mut csv_writer = csv::Writer::from_writer(BufWriter::new(f));

    csv_writer.write_record(["time", "title", "url", "visit_type"])?;
    let visits = db.select_visits(start, end, None)?;
    let len = visits.len();
    for visit in visits {
        csv_writer.write_record(vec![
            unixepoch_as_ymdhms(visit.visit_time),
            visit.title,
            visit.url,
            visit.visit_type.to_string(),
        ])?;
    }
    info!("Export {len} histories in {csv_file}.");

    Ok(())
}
