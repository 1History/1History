use std::{collections::HashMap, fmt::Display};

use crate::types::{SourceName, VisitDetail};
use anyhow::{bail, Context, Result};
use log::info;
use rusqlite::{named_params, Connection, OpenFlags, ToSql};

pub struct Source {
    path: String,
    name: SourceName,
    conn: Connection,
}

impl Source {
    pub fn open(path: String) -> Result<Source> {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY;
        let conn = Connection::open_with_flags(&path, flags).context(path.clone())?;
        let name = Self::detect_name(&conn).context(format!("detect {path}"))?;
        Ok(Source { path, name, conn })
    }

    // For Safari, seconds since 00:00:00 UTC on 1 January 2001
    // https://stackoverflow.com/a/34546556/2163429
    fn unixepoch_ms_to_nsdate(ts: i64) -> f64 {
        ts as f64 / 1000.0 - 978307200.0
    }

    // For Firefox, 64-bit integer counting number of microseconds
    // https://www.systoolsgroup.com/forensics/sqlite/places.html
    fn unixepoch_ms_to_prtime(ts: i64) -> i64 {
        ts * 1_000
    }

    // For Chrome, microseconds since January 1, 1601 UTC
    // https://www.systoolsgroup.com/forensics/sqlite/places.html
    fn unixepoch_ms_to_webkit(ts: i64) -> i64 {
        ts * 1_000 + 11644473600 * 1_000_000
    }

    fn detect_name(conn: &Connection) -> Result<SourceName> {
        let mut detect_sqls = HashMap::new();
        detect_sqls.insert(
            "select 1 from moz_historyvisits limit 1",
            SourceName::Firefox,
        );
        detect_sqls.insert("select 1 from history_items limit 1", SourceName::Safari);
        detect_sqls.insert("select 1 from visits limit 1", SourceName::Chrome);

        // Error code 14: Unable to open the database file
        // https://github.com/groue/GRDB.swift/issues/415#issuecomment-485220857
        conn.pragma_update(None, "journal_mode", "DELETE")?;
        for (sql, name) in detect_sqls {
            match conn.query_row(sql, [], |row| {
                let r: i64 = row.get(0)?;
                Ok(r)
            }) {
                Ok(_) => return Ok(name),
                Err(e) if e.to_string().contains("no such table") => {
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }

        bail!("No known browser, Only support Safari/Firefox/Chrome");
    }

    pub fn name(&self) -> SourceName {
        self.name
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn select(
        &self,
        inclusive_start: i64,
        exclusive_end: i64,
    ) -> Result<Box<dyn Iterator<Item = VisitDetail>>> {
        match self.name {
            SourceName::Firefox => self.select_firefox(inclusive_start, exclusive_end),
            SourceName::Safari => self.select_safari(inclusive_start, exclusive_end),
            SourceName::Chrome => self.select_chrome(inclusive_start, exclusive_end),
        }
    }

    fn select_safari(&self, start: i64, end: i64) -> Result<Box<dyn Iterator<Item = VisitDetail>>> {
        let sql = r#"
SELECT
    url,
    title,
    CAST((visit_time + 978307200.0) * 1000000 AS integer) as visit_time,     -- convert to PRTime
    -1
FROM
    history_items AS hi,
    history_visits AS hv ON hi.id = hv.history_item
WHERE
    visit_time >= :start
    AND visit_time <= :end
ORDER BY
    visit_time
"#;
        self.select_inner(
            sql,
            Self::unixepoch_ms_to_nsdate(start),
            Self::unixepoch_ms_to_nsdate(end),
        )
    }

    fn select_firefox(
        &self,
        start: i64,
        end: i64,
    ) -> Result<Box<dyn Iterator<Item = VisitDetail>>> {
        let sql = r#"
SELECT
    p.url,
    p.title,
    h.visit_date,
    h.visit_type
FROM
    moz_historyvisits h,
    moz_places p ON h.place_id = p.id
WHERE
    h.visit_date >= :start
    AND h.visit_date <= :end
ORDER BY
    visit_date
"#;

        self.select_inner(
            sql,
            Self::unixepoch_ms_to_prtime(start),
            Self::unixepoch_ms_to_prtime(end),
        )
    }

    fn select_chrome(&self, start: i64, end: i64) -> Result<Box<dyn Iterator<Item = VisitDetail>>> {
        let sql = r#"
SELECT
    u.url,
    u.title,
    v.visit_time - 11644473600*1000000,
    v.transition & 0xFF
FROM
    visits v,
    urls u ON v.url = u.id
WHERE
    v.visit_time >= :start
    AND v.visit_time <= :end
ORDER BY
    visit_time
"#;

        self.select_inner(
            sql,
            Self::unixepoch_ms_to_webkit(start),
            Self::unixepoch_ms_to_webkit(end),
        )
    }

    fn select_inner<T>(
        &self,
        sql_tmpl: &str,
        start: T,
        end: T,
    ) -> Result<Box<dyn Iterator<Item = VisitDetail>>>
    where
        T: PartialOrd + ToSql + Display,
    {
        let name = format!("{:?}", self.name());
        info!("select from {name}, start:{start}, end:{end}");
        let mut stat = self.conn.prepare(sql_tmpl)?;
        let rows = stat.query_map(
            named_params! {
                ":start": start,
                ":end": end,
            },
            |row| {
                let detail = VisitDetail {
                    url: row.get(0)?,
                    title: row.get(1).unwrap_or_else(|_| "".to_string()),
                    visit_time: row.get(2)?,
                    visit_type: row.get(3)?,
                };
                Ok(detail)
            },
        )?;

        let mut res: Vec<VisitDetail> = Vec::new();
        for r in rows {
            res.push(r?);
        }

        Ok(Box::new(res.into_iter()) as Box<dyn Iterator<Item = _>>)
    }
}
