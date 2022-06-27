mod database;
mod export;
mod progress;
mod source;
mod types;
mod util;
mod web;

use crate::database::Database;
use crate::progress::TUICollector;
use crate::source::Source;
use crate::util::{full_timerange, DEFAULT_CSV_FILE, DEFAULT_DB_FILE};
use anyhow::{Context, Error, Result};
use clap::{Parser, Subcommand};
use export::export_csv;
use log::{debug, error, info, LevelFilter};
use util::detect_history_files;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Database path
    #[clap(short, long, env("OH_DB_FILE"), default_value(&DEFAULT_DB_FILE))]
    db_file: String,

    #[clap(short, long)]
    verbose: bool,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Backup browser history to 1History
    Backup(Backup),
    /// Start HTTP server to visualize history
    Serve(Serve),
    /// Show default history files on your computer
    Show,
    Export(Export),
}

#[derive(Parser, Debug)]
struct Backup {
    /// SQLite file path of different browsers(History.db/places.sqlite...)
    #[clap(short('f'), long, required(false))]
    history_files: Vec<String>,
    /// Disable auto detect history files
    #[clap(short('d'), long)]
    disable_detect: bool,
    #[clap(short('D'), long)]
    dry_run: bool,
}

#[derive(Parser, Debug)]
struct Serve {
    /// Listening address
    #[clap(short, long, default_value("127.0.0.1:9960"))]
    addr: String,
}

#[derive(Parser, Debug)]
struct Export {
    /// Output cse file
    #[clap(short, long, env("OH_EXPORT_CSV_FILE"), default_value(&DEFAULT_CSV_FILE))]
    csv_file: String,
}

fn main() {
    let cli = Cli::parse();
    let level = if cli.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    env_logger::Builder::new().filter_level(level).init();

    if let Err(e) = run(cli) {
        error!("Run failed, err:{:?}", e);
    }
}

fn show(db_file: String) -> Result<()> {
    info!("Local database:{}", db_file);
    let mut cnt = 0;
    for f in detect_history_files() {
        cnt += 1;
        info!("found:{}", f);
    }
    info!("Total:{cnt}");
    Ok(())
}

fn backup(history_files: Vec<String>, db_file: String, dry_run: bool) -> Result<()> {
    let (start, end) = full_timerange();
    debug!("start:{}, end:{}", start, end);

    let db = Database::open(db_file).context("open 1History DB")?;

    let mut found = 0;
    let mut total_affected = 0;
    let mut total_duplicated = 0;
    let mut persist = |history_file: String| {
        let s = Source::open(&history_file).context("open")?;
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
    for his_file in history_files {
        if let Err(e) = persist(his_file.clone()) {
            error!("{} persist failed, err: {:?}", his_file, e);
        }
    }

    info!("Summary\nFound:{found}, Imported:{total_affected}, Duplicated: {total_duplicated}");
    Ok(())
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Show => show(cli.db_file),
        Command::Export(Export { csv_file }) => export_csv(csv_file, cli.db_file),
        Command::Serve(Serve { addr }) => web::serve(addr, cli.db_file),
        Command::Backup(Backup {
            history_files,
            disable_detect,
            dry_run,
        }) => {
            let mut fs = if disable_detect {
                Vec::new()
            } else {
                detect_history_files()
            };
            fs.extend(history_files);
            backup(fs, cli.db_file, dry_run)
        }
    }
}
