use anyhow::{Context, Result};
use log::{debug, info};
use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
};

use crate::{
    database::Database,
    util::{full_timerange, unixepoch_as_ymdhms},
};

pub fn export_csv(csv_file: String, db_file: String) -> Result<()> {
    let (start, end) = full_timerange();
    debug!("start:{}, end:{}", start, end);

    let db = Database::open(db_file).context("open 1History DB")?;
    let f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&csv_file)
        .context(csv_file.clone())?;
    let mut buf_writer = BufWriter::new(f);

    buf_writer.write_all(b"time,title,url,visit_type\n")?;
    let visits = db.select_visits(start, end, None)?;
    let len = visits.len();
    for visit in visits {
        buf_writer.write_all(
            format!(
                "{},{},{},{}\n",
                unixepoch_as_ymdhms(visit.visit_time),
                visit.title.replace(',', ""),
                visit.url,
                visit.visit_type
            )
            .as_bytes(),
        )?;
    }
    info!("Export {len} histories in {csv_file}.");

    Ok(())
}
