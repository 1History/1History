mod backup;
mod database;
mod export;
mod progress;
mod source;
mod types;
mod util;
mod web;

use crate::util::{DEFAULT_CSV_FILE, DEFAULT_DB_FILE};
use anyhow::Result;
use clap::{Parser, Subcommand};
use export::export_csv;
use log::{debug, error, info, LevelFilter};
use std::io::Write;
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
    env_logger::Builder::new()
        .filter_level(level)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {} [{}:{}] {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S.%3f"),
                buf.default_styled_level(record.level()),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();

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
            backup::backup(fs, cli.db_file, dry_run)
        }
    }
}
