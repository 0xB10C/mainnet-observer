use clap::Parser;
use env_logger::Env;
use log::{error, info};
use mainnet_observer_backend::{collect_statistics, db, write_csv_files, Args};
use std::process::exit;
use std::sync::{Arc, Mutex};

const DEFAULT_LOG_LEVEL: &str = "info";

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or(DEFAULT_LOG_LEVEL)).init();

    let args = Args::parse();

    let conn = match db::open_db_and_run_migrations(&args.database_path) {
        Ok(conn) => conn,
        Err(e) => {
            error!("Could not open database: {}", e);
            exit(1);
        }
    };
    let conn = Arc::new(Mutex::new(conn));

    info!(
        "Using {} threads for block fetching & processing",
        args.num_threads
    );

    if !args.no_stats {
        if let Err(e) = collect_statistics(
            &args.rest_host,
            args.rest_port,
            Arc::clone(&conn),
            args.num_threads,
        ) {
            error!("Could not collect statistics: {}", e);
            exit(1);
        };
    }

    if !args.no_csv {
        if let Err(e) = write_csv_files(&args.csv_path, conn) {
            error!("Could not write CSV files to disk: {}", e);
            exit(1);
        };
    }
}
