pub mod db;
mod gen_csv;
mod rest;
mod schema;
mod stats;

use clap::Parser;
use diesel::SqliteConnection;
use log::{debug, error, info, warn};
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use stats::Stats;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::{error, fmt, io, thread};

const DATABASE_BATCH_SIZE: usize = 100;

// Don't fetch (and process) the most recent blocks to be safe
// in-case of a reorg.
pub const REORG_SAFETY_MARGIN: u64 = 6;

#[derive(Debug)]
pub enum MainError {
    DB(diesel::result::Error),
    DBConnection(diesel::result::ConnectionError),
    DBMigration(db::MigrationError),
    REST(rest::RestError),
    Stats(stats::StatsError),
    IBDNotDone,
    IOError(io::Error),
}

impl fmt::Display for MainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MainError::DB(e) => write!(f, "Database Error: {:?}", e),
            MainError::DBConnection(e) => write!(f, "Database Connection Error: {}", e),
            MainError::DBMigration(e) => write!(f, "Database Migration Error: {}", e),
            MainError::IBDNotDone => write!(f, "Node is still in IBD"),
            MainError::REST(e) => write!(f, "REST error: {}", e),
            MainError::Stats(e) => write!(f, "Stats generation error: {}", e),
            MainError::IOError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl error::Error for MainError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            MainError::DB(ref e) => Some(e),
            MainError::DBConnection(ref e) => Some(e),
            MainError::DBMigration(ref _e) => None,
            MainError::REST(ref e) => Some(e),
            MainError::Stats(ref e) => Some(e),
            MainError::IBDNotDone => None,
            MainError::IOError(ref e) => Some(e),
        }
    }
}

impl From<diesel::result::Error> for MainError {
    fn from(e: diesel::result::Error) -> Self {
        MainError::DB(e)
    }
}

impl From<diesel::result::ConnectionError> for MainError {
    fn from(e: diesel::result::ConnectionError) -> Self {
        MainError::DBConnection(e)
    }
}

impl From<db::MigrationError> for MainError {
    fn from(e: db::MigrationError) -> Self {
        MainError::DBMigration(e)
    }
}

impl From<rest::RestError> for MainError {
    fn from(e: rest::RestError) -> Self {
        MainError::REST(e)
    }
}

impl From<stats::StatsError> for MainError {
    fn from(e: stats::StatsError) -> Self {
        MainError::Stats(e)
    }
}

impl From<io::Error> for MainError {
    fn from(e: io::Error) -> Self {
        MainError::IOError(e)
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Host part of the Bitcoin Core REST API endpoint
    #[arg(long, default_value = "localhost")]
    pub rest_host: String,

    /// Port part of the Bitcoin Core REST API endpoint
    #[arg(long, default_value_t = 8332)]
    pub rest_port: u16,

    /// Path to the SQLite database file where the stats are stored
    #[arg(long, default_value = "./db.sqlite")]
    pub database_path: String,

    /// Path where the CSV files should be written to
    #[arg(long, default_value = "./csv")]
    pub csv_path: String,

    /// Flag to disable CSV file writing
    #[arg(long, default_value_t = false)]
    pub no_csv: bool,

    /// Flag to disable stat generation
    #[arg(long, default_value_t = false)]
    pub no_stats: bool,

    /// Number of threads to use for parallel block fetching.
    /// As of v29.0, Bitcoin Core starts 16 threads for handling HTTP requests.
    /// By default, we use 14 of these and leave 2 threads to service other requests.
    #[arg(long, default_value_t = 14)]
    pub num_threads: usize,
}

pub fn collect_statistics(
    rest_host: &str,
    rest_port: u16,
    connection: Arc<Mutex<SqliteConnection>>,
    num_threads: usize,
) -> Result<(), MainError> {
    let connection = Arc::clone(&connection);
    let db_height: i64 = {
        let mut conn = connection.lock().unwrap();
        db::get_db_block_height(&mut conn)?.unwrap_or_default()
    };

    let client = rest::RestClient::new(rest_host, rest_port);
    let chain_info = match client.chain_info() {
        Ok(chain_info) => chain_info,
        Err(e) => {
            error!(
                "Could load chain information from Bitcoin Core at {}:{}: {}",
                rest_host, rest_port, e
            );
            return Err(MainError::REST(e));
        }
    };

    if chain_info.initialblockdownload {
        error!("The Bitcoin Core node is in initial block download (progress: {:.2}%). Please try again once the IBD is done.", chain_info.verificationprogress*100.0);
        return Err(MainError::IBDNotDone);
    }
    let rest_height = chain_info.blocks;

    let (block_sender, block_receiver) = mpsc::sync_channel(10);
    let (stat_sender, stat_receiver) = mpsc::sync_channel(100);

    // get-blocks task
    // gets blocks from the Bitcoin Core REST interface and sends them onwards
    // to the `calc-stats` task
    let get_blocks_task = thread::spawn(move || -> Result<(), MainError> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap();
        let mut heights: Vec<i64> = (std::cmp::max(db_height + 1, 0)
            ..std::cmp::max((rest_height - REORG_SAFETY_MARGIN) as i64, 0))
            .collect();
        heights.sort();
        pool.install(|| {
            heights.par_iter()
                .map(|&height| {
                    debug!("get-blocks: getting block at height {}", height);
                    let block = match client.block_at_height(height as u64) {
                        Ok(block) => block,
                        Err(e) => {
                            error!("Could not get block at height {}: {}", height, e);
                            return Err(MainError::REST(e));
                        }
                    };
                    if let Err(_) = block_sender.send((height as i64, block)) {
                        warn!(
                            "during sending block at height {} to stats generator: block receiver dropped",
                            height
                        );
                        // We can return OK here. When the receiver is dropped, there
                        // probably was an error in the calc-stats task.
                        return Ok(());
                    }
                    Ok(())
                })
                .for_each(drop); // Drop the result of the map (since it's already handled)
        });
        Ok(())
    });

    // calc-stats task
    // calculates the per block stats and sends them onwards to the batch-insert
    // task
    let calc_stats_task = thread::spawn(move || -> Result<(), MainError> {
        while let Ok((height, block)) = block_receiver.recv() {
            debug!("calc-stats: processing block at height {}..", height);
            let stat_sender_clone = stat_sender.clone();
            rayon::spawn(move || {
                let stats_result = Stats::from_block(block);
                if let Err(e) = stats_result {
                    error!(
                        "Could not calculate stats for block at height {}: {}",
                        height, e
                    );
                    // We can't continue here and probably need to fix something
                    // in rawtx_rs..
                    panic!(
                        "Could not process block {}: {}",
                        height,
                        MainError::Stats(e)
                    );
                };
                if let Err(e) = stat_sender_clone.send(stats_result) {
                    // We can't continue here..
                    panic!(
                        "during sending stats at height {} to db writer: stats receiver dropped: {}",
                        height, e
                    );
                } else {
                    debug!("calc-stats: processed block at height {}", height);
                }
            });
        }
        // Reaching this point doesn't mean we're done processing all block just yet
        // We might still be processing some..
        debug!("calc-stats: received all blocks and started processing them..");
        Ok(())
    });

    // batch-insert task
    // inserts the block stats in batches
    let batch_insert_task = thread::spawn(move || -> Result<(), MainError> {
        let connection = Arc::clone(&connection);
        let mut conn = connection.lock().unwrap();
        db::performance_tune(&mut conn)?;
        let mut stat_buffer = Vec::with_capacity(DATABASE_BATCH_SIZE);

        loop {
            let stat_recv_result = stat_receiver.recv();
            let stat = match stat_recv_result {
                Ok(stat_result) => match stat_result {
                    Ok(stat) => stat,
                    Err(e) => {
                        error!("Could write stat: {}", e);
                        return Err(MainError::Stats(e));
                    }
                },
                Err(e) => {
                    info!("batch-insert: the calc-stats task finished ({})", e);
                    break;
                }
            };

            stat_buffer.push(stat);
            if stat_buffer.len() >= DATABASE_BATCH_SIZE {
                info!(
                    "writing a batch of {} block-stats to database (max height: {})",
                    stat_buffer.len(),
                    stat_buffer
                        .iter()
                        .map(|s| s.block.height)
                        .max()
                        .unwrap_or_default()
                );
                db::insert_stats(&mut conn, &stat_buffer)?;
                stat_buffer.clear();
            }
        }

        if stat_buffer.len() > 0 {
            // once the stat_receiver is closed, insert the remaining buffer
            // contents into the database
            info!(
                "collect-statistics: writing the final batch of {} block-stats to database",
                stat_buffer.len()
            );
            db::insert_stats(&mut conn, &stat_buffer)?;
        } else {
            info!("collect-statistics: no new blocks to insert.");
        }
        let db_height = db::get_db_block_height(&mut conn)?.unwrap_or_default();
        info!(
            "collect-statistics: database is at height {} with a reorg-safety margin of {}",
            db_height, REORG_SAFETY_MARGIN,
        );
        Ok(())
    });

    get_blocks_task
        .join()
        .expect("The get-blocks task thread panicked")?;
    calc_stats_task
        .join()
        .expect("The calc-stats task thread panicked")?;
    batch_insert_task
        .join()
        .expect("The batch-insert task thread panicked")?;

    Ok(())
}

pub fn write_csv_files(
    csv_path: &str,
    connection: Arc<Mutex<SqliteConnection>>,
) -> Result<(), MainError> {
    gen_csv::date_csv(csv_path, connection.clone())?;
    gen_csv::metrics_csv(csv_path, connection.clone())?;
    gen_csv::top5_miningpools_csv(csv_path, connection.clone())?;
    gen_csv::antpool_and_friends_csv(csv_path, connection.clone())?;
    gen_csv::mining_centralization_index_csv(csv_path, connection.clone())?;
    gen_csv::mining_centralization_index_with_proxy_pools_csv(csv_path, connection.clone())?;
    gen_csv::mining_pool_blocks_per_day_csv(csv_path, connection.clone())?;
    Ok(())
}
