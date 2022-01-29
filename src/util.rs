use anyhow::Context;
use chrono::{Date, DateTime, FixedOffset, Local, NaiveDate, TimeZone, Utc};
use home::home_dir;
use lazy_static::lazy_static;
use log::debug;
use minijinja::State;
use regex::Regex;
use std::collections::HashMap;
use std::env::temp_dir;
use std::path::PathBuf;

const OS_TYPE: &str = std::env::consts::OS;

lazy_static! {
    pub static ref DEFAULT_DB_FILE: String = default_location("onehistory.db");
    pub static ref DEFAULT_CSV_FILE: String = default_location(&format!("onehistory-{}.csv", unixepoch_as_ymd(tomorrow_midnight()-1)));
    static ref DEFAULT_PROFILES: HashMap<&'static str, String> = {
        let mut m = HashMap::new();
        if let Some(home) = home_dir() {
            m.insert(
                "chrome-linux",
                join_path(home.clone(), ".config/google-chrome/Default/History"),
            );
            m.insert(
                "chrome-macos",
                join_path(
                    home.clone(),
                    "Library/Application Support/Google/Chrome/Default/History",
                ),
            );
            m.insert(
                "chrome-windows",
                join_path(
                    home.clone(),
                    "AppData/Local/Google/Chrome/User Data/Default/History",
                ),
            );
            // Firefox
            m.insert(
                "firefox-linux",
                join_path(home.clone(), ".mozilla/firefox/*.default/places.sqlite"),
            );
            m.insert("firefox-macos", join_path(home.clone(), "Library/Application Support/Firefox/Profiles/*/places.sqlite"));
            m.insert(
                "firefox-windows",
                join_path(
                    home.clone(),
                    "AppData/Roaming/Mozilla/Firefox/Profiles/*/places.sqlite",
                ),
            );
            // Safari
            m.insert("safari-macos", join_path(home, "Library/Safari/History.db"));
        }
        m
    };
}

pub fn detect_history_files() -> Vec<String> {
    let mut files = Vec::new();
    let browsers = vec!["safari", "chrome", "firefox"];
    for b in browsers {
        let k = format!("{b}-{OS_TYPE}");
        debug!("detect {k}...");
        if let Some(pattern) = DEFAULT_PROFILES.get(k.as_str()) {
            debug!("find {pattern} in {k}...");
            if let Ok(entries) = glob::glob(pattern) {
                for e in entries {
                    match e {
                        Ok(file) => files.push(file.into_os_string().into_string().unwrap()),
                        Err(e) => debug!("glob err:{:?}", e),
                    }
                }
            }
        }
    }

    files
}

fn join_path(mut base: PathBuf, rest: &str) -> String {
    base.push(rest);
    base.into_os_string().into_string().unwrap()
}

fn default_location(filename: &str) -> String {
    let base = home_dir().unwrap_or_else(temp_dir);
    join_path(base, filename)
}

pub fn tomorrow_midnight() -> i64 {
    let now = Local::today();
    let dt: DateTime<Local> = now.and_hms(0, 0, 0);
    dt.timestamp_millis() + 24 * 3_600_000
}

pub fn ymd_midnight(ymd: &str) -> anyhow::Result<i64> {
    let nd = NaiveDate::parse_from_str(ymd, "%Y-%m-%d").context("not ymd date")?;

    lazy_static! {
        static ref LOCAL_OFFSET: FixedOffset = *Local::now().offset();
    }

    let dc: Date<Local> = Date::from_utc(nd, *LOCAL_OFFSET);
    Ok(dc.and_hms(0, 0, 0).timestamp_millis())
}

pub fn unixepoch_as_ymd(ts: i64) -> String {
    let utc = Utc.timestamp(ts / 1000, 0);
    let dt: DateTime<Local> = DateTime::from(utc);
    dt.format("%Y-%m-%d").to_string()
}

pub fn unixepoch_as_hms(ts: i64) -> String {
    let utc = Utc.timestamp(ts / 1000, 0);
    let dt: DateTime<Local> = DateTime::from(utc);
    dt.format("%H:%M:%S").to_string()
}

pub fn unixepoch_as_ymdhms(ts: i64) -> String {
    let utc = Utc.timestamp(ts / 1000, 0);
    let dt: DateTime<Local> = DateTime::from(utc);
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn minijinja_format_as_ymd(_state: &State, ts: i64) -> Result<String, minijinja::Error> {
    Ok(unixepoch_as_ymd(ts))
}

pub fn minijinja_format_as_hms(_state: &State, ts: i64) -> Result<String, minijinja::Error> {
    Ok(unixepoch_as_hms(ts))
}

pub fn minijinja_format_title(
    _state: &State,
    title: String,
    url: String,
) -> Result<String, minijinja::Error> {
    if title.is_empty() {
        Ok(url)
    } else {
        Ok(title)
    }
}

pub fn domain_from(url: String) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new("://(.+?)/").unwrap();
    }

    if let Some(cap) = RE.captures_iter(&url).next() {
        return cap[1].to_string();
    }

    url
}

pub fn full_timerange() -> (i64, i64) {
    let start = 0;
    let end = tomorrow_midnight();
    (start, end) // Use this timerange to represent ALL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_path() {
        let mut base = PathBuf::new();
        base.push("/tmp");

        assert_eq!("/tmp/abc.txt", join_path(base.clone(), "abc.txt"));
        assert_eq!("/tmp/history.txt", join_path(base.clone(), "history.txt"));
    }

    #[test]
    fn test_demain_from() {
        let cases = vec![
            ("https://emacs-china.org/", "emacs-china.org"),
            ("https://github.com/notifications", "github.com"),
            ("data:text/html", "data:text/html"),
        ];

        for (url, expected) in cases {
            assert_eq!(domain_from(url.to_string()), expected);
        }
    }

    #[test]
    fn test_ymd_midnight() {
        let cases = vec![("2022-01-15", 1642176000000)];
        for (ymd, expected) in cases {
            assert_eq!(ymd_midnight(ymd).unwrap(), expected);
        }
    }
}
