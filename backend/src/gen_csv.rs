use crate::{db, db::TableInfo, MainError};
use bitcoin::Network;
use diesel::SqliteConnection;
use log::info;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::sync::{Arc, Mutex};

const METRIC_TABLES: [&str; 6] = [
    "block_stats",
    "tx_stats",
    "script_stats",
    "input_stats",
    "output_stats",
    "feerate_stats",
];
const COLUMN_NAMES_THAT_ARENT_METRICS: [&str; 8] = [
    "height",
    "date",
    "version",
    "nonce",
    "bits",
    "pool_id",
    "hash",
    "stats_version",
];

// An array with pool IDs based on https://github.com/bitcoin-data/mining-pools/blob/generated/pool-list.json
// representing the "AntPool & Friends" proxy pool group.
// This group is based on the observed stratum jobs they sent out.
pub const PROXY_POOL_GROUP_ANTPOOL: [u64; 11] = [
    61,  // AntPool
    111, // Poolin
    72,  // Ultimus Pool
    119, // Braiins
    146, // SecPool
    48,  // SigmaPool
    123, // Binance Pool
    136, // Rawpool
    4,   // Luxor
    43,  // CloverPool (formerly BTC.com)
    152, // Mining Squared
         // When updating this list, make sure to update the following files too:
         // - frontend/content/charts/mining-pools-antpool-and-friends.md
         // - frontend/content/charts/mining-pools-centralization-index-with-proxy-pools.md
];

// Generates a date.csv file with a single column with the date.
// To be used together with other metric CSV files.
pub fn date_csv(csv_path: &str, connection: Arc<Mutex<SqliteConnection>>) -> Result<(), MainError> {
    let connection = Arc::clone(&connection);
    let mut conn = connection.lock().unwrap();
    info!("Generating date.csv file...");
    let date_column = db::date_column(&mut conn)?;
    let mut date_file = std::fs::File::create(format!("{}/date.csv", csv_path))?;
    let date_content: String = date_column
        .iter()
        .map(|row| format!("{}\n", row.date))
        .collect();
    date_file.write_all("date\n".as_bytes())?;
    date_file.write_all(date_content.as_bytes())?;
    Ok(())
}

// Generates multiple metric csv files where each metrics has its own file.
// A metric csv file can be used together with the date.csv file and other metric csv files.
pub fn metrics_csv(
    csv_path: &str,
    connection: Arc<Mutex<SqliteConnection>>,
) -> Result<(), MainError> {
    let connection = Arc::clone(&connection);
    let mut conn = connection.lock().unwrap();

    for table in METRIC_TABLES.iter() {
        let columns = db::list_column_names(&mut conn, table)?;

        // filter out columns that aren't metrics and we don't want to create csv files for
        let columns_filtered: Vec<&TableInfo> = columns
            .iter()
            .filter(|col| !COLUMN_NAMES_THAT_ARENT_METRICS.contains(&&col.name[..]))
            .collect();

        for column in columns_filtered.iter().map(|col| col.name.clone()) {
            info!("Generating metrics for '{}' in table '{}'.", column, table);
            let avg_and_sum = db::column_sum_and_avg_by_date(&mut conn, &column, table)?;

            let mut avg_file = std::fs::File::create(format!("{}/{}_avg.csv", csv_path, column))?;
            let avg_content: String = avg_and_sum
                .iter()
                .map(|aas| format!("{:.4}\n", aas.avg))
                .collect();
            avg_file.write_all(format!("{}_avg\n", column).as_bytes())?;
            avg_file.write_all(avg_content.as_bytes())?;

            let mut sum_file = std::fs::File::create(format!("{}/{}_sum.csv", csv_path, column))?;
            let sum_content: String = avg_and_sum
                .iter()
                .map(|aas| format!("{}\n", aas.sum))
                .collect();
            sum_file.write_all(format!("{}_sum\n", column).as_bytes())?;
            sum_file.write_all(sum_content.as_bytes())?;
        }
    }
    Ok(())
}

pub fn coinbase_subsidy_and_fees_csv(
    csv_path: &str,
    connection: Arc<Mutex<SqliteConnection>>,
) -> Result<(), MainError> {
    let connection = Arc::clone(&connection);
    let mut conn = connection.lock().unwrap();
    info!("Generating coinbase subsidy and fees CSV files...");
    let rows = db::coinbase_subsidy_and_fees_avg_by_date(&mut conn)?;

    let mut subsidy_file = std::fs::File::create(format!("{}/coinbase_subsidy_avg.csv", csv_path))?;
    subsidy_file.write_all("coinbase_subsidy_avg\n".as_bytes())?;
    let subsidy_content: String = rows
        .iter()
        .map(|r| format!("{:.4}\n", r.subsidy_avg))
        .collect();
    subsidy_file.write_all(subsidy_content.as_bytes())?;

    let mut fees_file = std::fs::File::create(format!("{}/coinbase_fees_avg.csv", csv_path))?;
    fees_file.write_all("coinbase_fees_avg\n".as_bytes())?;
    let fees_content: String = rows
        .iter()
        .map(|r| format!("{:.4}\n", r.fees_avg))
        .collect();
    fees_file.write_all(fees_content.as_bytes())?;

    Ok(())
}

// Generates a top5_miningpools.csv file with the current top5 pools and their blocks
// per day along with the total daily blocks.
pub fn top5_miningpools_csv(
    csv_path: &str,
    connection: Arc<Mutex<SqliteConnection>>,
) -> Result<(), MainError> {
    const FILENAME: &str = "top5pools";

    let connection = Arc::clone(&connection);
    let mut conn = connection.lock().unwrap();
    info!("Generating {} file...", FILENAME);

    let pool_data = bitcoin_pool_identification::default_data(Network::Bitcoin);

    let top_pools = db::current_top_mining_pools(&mut conn)?;
    let mut pool_ids: [Vec<i32>; 5] = [vec![-1], vec![-1], vec![-1], vec![-1], vec![-1]];
    let mut pool_names: [String; 5] = Default::default();
    for (i, top_pool) in top_pools.iter().enumerate() {
        if i >= pool_ids.len() {
            break;
        }
        pool_ids[i] = vec![top_pool.pool_id];
        // Pools we can't identify are named Unknown like in the other CSV
        // files, plus their id so that two of them don't share a column name.
        // The columns of the remaining array entries keep their empty name,
        // as they have no pool at all.
        pool_names[i] = format!("Unknown ({})", top_pool.pool_id);
        for pool in pool_data.iter().rev() {
            if top_pool.pool_id == pool.id as i32 {
                pool_names[i] = pool.name.clone();
                break;
            }
        }
    }

    let mut file = std::fs::File::create(format!("{}/{}.csv", csv_path, FILENAME))?;
    file.write_all(
        format!(
            "{},{},{},{},{},{},{}\n",
            "date",
            pool_names[0],
            pool_names[1],
            pool_names[2],
            pool_names[3],
            pool_names[4],
            "total"
        )
        .as_bytes(),
    )?;
    let rows = db::blocks_per_day_top5_pool_groups(&mut conn, &pool_ids)?;
    let content: String = rows
        .iter()
        .map(|row| {
            format!(
                "{},{},{},{},{},{},{}\n",
                row.date,
                row.top1_blocks,
                row.top2_blocks,
                row.top3_blocks,
                row.top4_blocks,
                row.top5_blocks,
                row.total
            )
        })
        .collect();
    file.write_all(content.as_bytes())?;
    Ok(())
}

// Generates a miningpools-antpool-and-friends.csv file with the current top5
// pool groups and including "AntPool and Friends".
pub fn antpool_and_friends_csv(
    csv_path: &str,
    connection: Arc<Mutex<SqliteConnection>>,
) -> Result<(), MainError> {
    const FILENAME: &str = "miningpools-antpool-and-friends";

    let connection = Arc::clone(&connection);
    let mut conn = connection.lock().unwrap();
    info!("Generating {} file...", FILENAME);

    let pool_data = bitcoin_pool_identification::default_data(Network::Bitcoin);

    let top_pools = db::current_top_mining_pools(&mut conn)?;
    let mut pool_ids: [Vec<i32>; 5] = [
        PROXY_POOL_GROUP_ANTPOOL.iter().map(|i| *i as i32).collect(),
        vec![-1],
        vec![-1],
        vec![-1],
        vec![-1],
    ];
    let mut pool_names: [String; 5] = Default::default();
    pool_names[0] = "AntPool & friends".to_string();
    let mut pools_added = 1;

    for top_pool in top_pools.iter() {
        if pools_added >= pool_ids.len() {
            break;
        }
        if PROXY_POOL_GROUP_ANTPOOL.contains(&(top_pool.pool_id as u64)) {
            // We already added the "antpool and friends" group,
            // don't add pools of this group again. Skip them.
            continue;
        } else {
            pool_ids[pools_added] = vec![top_pool.pool_id];
            // Pools we can't identify are named Unknown like in the other CSV
            // files, plus their id so that two of them don't share a column
            // name.
            pool_names[pools_added] = format!("Unknown ({})", top_pool.pool_id);
            for pool in pool_data.iter().rev() {
                if top_pool.pool_id == pool.id as i32 {
                    pool_names[pools_added] = pool.name.clone();
                    break;
                }
            }
            pools_added += 1;
        }
    }

    let mut file = std::fs::File::create(format!("{}/{}.csv", csv_path, FILENAME))?;
    file.write_all(
        format!(
            "{},{},{},{},{},{},{}\n",
            "date",
            pool_names[0],
            pool_names[1],
            pool_names[2],
            pool_names[3],
            pool_names[4],
            "total"
        )
        .as_bytes(),
    )?;
    let rows = db::blocks_per_day_top5_pool_groups(&mut conn, &pool_ids)?;
    let content: String = rows
        .iter()
        .map(|row| {
            format!(
                "{},{},{},{},{},{},{}\n",
                row.date,
                row.top1_blocks,
                row.top2_blocks,
                row.top3_blocks,
                row.top4_blocks,
                row.top5_blocks,
                row.total
            )
        })
        .collect();
    file.write_all(content.as_bytes())?;
    Ok(())
}

// Generates a miningpools-centralization-index.csv file.
pub fn mining_centralization_index_csv(
    csv_path: &str,
    connection: Arc<Mutex<SqliteConnection>>,
) -> Result<(), MainError> {
    const FILENAME: &str = "miningpools-centralization-index";

    let connection = Arc::clone(&connection);
    let mut conn = connection.lock().unwrap();
    info!("Generating {} file...", FILENAME);

    let mut file = std::fs::File::create(format!("{}/{}.csv", csv_path, FILENAME))?;
    file.write_all(
        "date,top1,top2,top3,top4,top5,top6,total\n"
            .to_string()
            .as_bytes(),
    )?;
    let rows = db::mining_centralization_index(&mut conn)?;
    let content: String = rows
        .iter()
        .map(|row| {
            format!(
                "{},{},{},{},{},{},{},{}\n",
                row.date,
                row.top1_count,
                row.top2_count,
                row.top3_count,
                row.top4_count,
                row.top5_count,
                row.top6_count,
                row.total_blocks
            )
        })
        .collect();
    file.write_all(content.as_bytes())?;
    Ok(())
}

// Generates a pools-mining-ephemeral-dust.csv file.
pub fn pools_mining_ephemeral_dust_csv(
    csv_path: &str,
    connection: Arc<Mutex<SqliteConnection>>,
) -> Result<(), MainError> {
    const FILENAME: &str = "miningpools-mining-ephemeral-dust";

    let connection = Arc::clone(&connection);
    let mut conn = connection.lock().unwrap();
    info!("Generating {} file...", FILENAME);

    let mut file = std::fs::File::create(format!("{}/{}.csv", csv_path, FILENAME))?;
    file.write_all("pool,height,date,total\n".to_string().as_bytes())?;

    let pool_data = bitcoin_pool_identification::default_data(Network::Bitcoin);
    let pool_names: BTreeMap<u64, String> =
        pool_data.iter().map(|p| (p.id, p.name.clone())).collect();

    let rows = db::get_pools_mining_ephemeral_dust(&mut conn)?;
    let content: String = rows
        .iter()
        .map(|row| {
            format!(
                "{},{},{},{}\n",
                pool_names
                    .get(&(row.pool_id as u64))
                    .unwrap_or(&"Unknown".to_string()),
                row.first_ephemeral_dust_height,
                row.first_ephemeral_dust_date,
                row.count,
            )
        })
        .collect();
    file.write_all(content.as_bytes())?;
    Ok(())
}

// Generates a pools-mining-bip54-coinbase.csv file.
pub fn pools_mining_bip54_coinbase_csv(
    csv_path: &str,
    connection: Arc<Mutex<SqliteConnection>>,
) -> Result<(), MainError> {
    const FILENAME: &str = "miningpools-mining-bip54-coinbase";

    let connection = Arc::clone(&connection);
    let mut conn = connection.lock().unwrap();
    info!("Generating {} file...", FILENAME);

    let mut file = std::fs::File::create(format!("{}/{}.csv", csv_path, FILENAME))?;
    file.write_all("pool,height,date,total\n".to_string().as_bytes())?;

    let pool_data = bitcoin_pool_identification::default_data(Network::Bitcoin);
    let pool_names: BTreeMap<u64, String> =
        pool_data.iter().map(|p| (p.id, p.name.clone())).collect();

    let rows = db::get_pools_mining_bip54_coinbase(&mut conn)?;
    let content: String = rows
        .iter()
        .map(|row| {
            format!(
                "{},{},{},{}\n",
                pool_names
                    .get(&(row.pool_id as u64))
                    .unwrap_or(&"Unknown".to_string()),
                row.first_bip54_coinbase_height,
                row.first_bip54_coinbase_date,
                row.count,
            )
        })
        .collect();
    file.write_all(content.as_bytes())?;
    Ok(())
}

// Generates a pools-mining-p2a.csv file.
pub fn pools_mining_p2a_csv(
    csv_path: &str,
    connection: Arc<Mutex<SqliteConnection>>,
) -> Result<(), MainError> {
    const FILENAME: &str = "miningpools-mining-p2a";

    let connection = Arc::clone(&connection);
    let mut conn = connection.lock().unwrap();
    info!("Generating {} file...", FILENAME);

    let mut file = std::fs::File::create(format!("{}/{}.csv", csv_path, FILENAME))?;
    file.write_all(
        "pool,first spend,first creation,total inputs, total outputs\n"
            .to_string()
            .as_bytes(),
    )?;

    let pool_data = bitcoin_pool_identification::default_data(Network::Bitcoin);
    let pool_names: BTreeMap<u64, String> =
        pool_data.iter().map(|p| (p.id, p.name.clone())).collect();

    let rows = db::get_pools_mining_p2a(&mut conn)?;
    let content: String = rows
        .iter()
        .map(|row| {
            let input_date_string = match row.first_p2a_input_height {
                Some(h) => format!(
                    "{} ({})",
                    h,
                    row.first_p2a_input_date.clone().unwrap_or_default()
                ),
                None => "-".to_string(),
            };

            let output_date_string = match row.first_p2a_output_height {
                Some(h) => format!(
                    "{} ({})",
                    h,
                    row.first_p2a_output_date.clone().unwrap_or_default()
                ),
                None => "-".to_string(),
            };

            format!(
                "{},{},{},{},{}\n",
                pool_names
                    .get(&(row.pool_id as u64))
                    .unwrap_or(&"Unknown".to_string()),
                input_date_string,
                output_date_string,
                row.total_inputs,
                row.total_outputs,
            )
        })
        .collect();
    file.write_all(content.as_bytes())?;
    Ok(())
}

// Generates a miningpools-centralization-index-with-proxy-pools.csv file.
pub fn mining_centralization_index_with_proxy_pools_csv(
    csv_path: &str,
    connection: Arc<Mutex<SqliteConnection>>,
) -> Result<(), MainError> {
    const FILENAME: &str = "miningpools-centralization-index-with-proxy-pools";

    let connection = Arc::clone(&connection);
    let mut conn = connection.lock().unwrap();
    info!("Generating {} file...", FILENAME);

    let mut file = std::fs::File::create(format!("{}/{}.csv", csv_path, FILENAME))?;
    file.write_all(
        "date,top1,top2,top3,top4,top5,top6,total\n"
            .to_string()
            .as_bytes(),
    )?;
    let rows = db::mining_centralization_index_with_proxy_pools(&mut conn)?;
    let content: String = rows
        .iter()
        .map(|row| {
            format!(
                "{},{},{},{},{},{},{},{}\n",
                row.date,
                row.top1_count,
                row.top2_count,
                row.top3_count,
                row.top4_count,
                row.top5_count,
                row.top6_count,
                row.total_blocks
            )
        })
        .collect();
    file.write_all(content.as_bytes())?;
    Ok(())
}

// Generates a CSV file with per-pool stats for blocks signaling a given version bit:
// pool name, first block height and date, and total count.
// Only blocks within [start_height, end_height] are considered.
pub fn pools_mining_version_bit_csv(
    csv_path: &str,
    connection: Arc<Mutex<SqliteConnection>>,
    filename: &str,
    bit: u8,
    start_height: i64,
    end_height: i64,
) -> Result<(), MainError> {
    let connection = Arc::clone(&connection);
    let mut conn = connection.lock().unwrap();
    info!("Generating {}.csv file...", filename);

    let mut file = std::fs::File::create(format!("{}/{}.csv", csv_path, filename))?;
    file.write_all("pool,height,date,total\n".as_bytes())?;

    let pool_data = bitcoin_pool_identification::default_data(Network::Bitcoin);
    let pool_names: BTreeMap<u64, String> =
        pool_data.iter().map(|p| (p.id, p.name.clone())).collect();

    let rows = db::get_pools_mining_version_bit(&mut conn, bit, start_height, end_height)?;
    let content: String = rows
        .iter()
        .map(|(pool_id, count, first_height, first_date)| {
            format!(
                "{},{},{},{}\n",
                pool_names
                    .get(&(*pool_id as u64))
                    .unwrap_or(&"Unknown".to_string()),
                first_height,
                first_date,
                count,
            )
        })
        .collect();
    file.write_all(content.as_bytes())?;
    Ok(())
}

// Generates a CSV file with the number of blocks per day signaling a given BIP9 version bit.
pub fn version_bit_signaling_csv(
    csv_path: &str,
    connection: Arc<Mutex<SqliteConnection>>,
    filename: &str,
    column_name: &str,
    bit: u8,
) -> Result<(), MainError> {
    let connection = Arc::clone(&connection);
    let mut conn = connection.lock().unwrap();
    info!("Generating {}.csv file...", filename);

    let mut file = std::fs::File::create(format!("{}/{}.csv", csv_path, filename))?;
    file.write_all(format!("{}\n", column_name).as_bytes())?;

    let rows = db::blocks_signaling_version_bit_per_day(&mut conn, bit)?;
    let content: String = rows.iter().map(|count| format!("{}\n", count)).collect();
    file.write_all(content.as_bytes())?;
    Ok(())
}

// Generates a unclaimed-coinbase-blocks.csv file listing all blocks where the miner
// did not claim the full coinbase reward (subsidy + fees).
pub fn unclaimed_coinbase_blocks_csv(
    csv_path: &str,
    connection: Arc<Mutex<SqliteConnection>>,
) -> Result<(), MainError> {
    const FILENAME: &str = "unclaimed-coinbase-blocks";

    let connection = Arc::clone(&connection);
    let mut conn = connection.lock().unwrap();
    info!("Generating {} file...", FILENAME);

    let pool_data = bitcoin_pool_identification::default_data(Network::Bitcoin);
    let pool_names: BTreeMap<u64, String> =
        pool_data.iter().map(|p| (p.id, p.name.clone())).collect();

    let mut file = std::fs::File::create(format!("{}/{}.csv", csv_path, FILENAME))?;
    file.write_all("height,date,unclaimed_sat,pool\n".as_bytes())?;

    let rows = db::get_blocks_with_unclaimed_coinbase(&mut conn)?;
    let content: String = rows
        .iter()
        .map(|row| {
            let pool_name = pool_names
                .get(&(row.pool_id as u64))
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());
            format!(
                "{},{},{},{}\n",
                row.height, row.date, row.coinbase_unclaimed_sat, pool_name
            )
        })
        .collect();
    file.write_all(content.as_bytes())?;
    Ok(())
}

// Generates miningpools-poolid-*.csv files with the number of blocks for this pool id per day.
pub fn mining_pool_blocks_per_day_csv(
    csv_path: &str,
    connection: Arc<Mutex<SqliteConnection>>,
) -> Result<(), MainError> {
    let connection = Arc::clone(&connection);
    let mut conn = connection.lock().unwrap();

    // A set of interesting pool IDs based on https://github.com/bitcoin-data/mining-pools/blob/generated/pool-list.json
    let mut pool_ids = BTreeSet::new();
    // All "AntPool & friends" pools
    for &item in PROXY_POOL_GROUP_ANTPOOL.iter() {
        pool_ids.insert(item as i32);
    }
    pool_ids.insert(0); // Unknown
    pool_ids.insert(88); // Foundry USA
    pool_ids.insert(110); // ViaBTC
    pool_ids.insert(22); // F2Pool
    pool_ids.insert(140); // MaraPool
    pool_ids.insert(145); // Ocean

    for id in pool_ids.iter() {
        let filename = format!("miningpools-poolid-{}", id);
        info!("Generating {} file...", filename);
        let mut file = std::fs::File::create(format!("{}/{}.csv", csv_path, filename))?;

        file.write_all("date,count,total\n".to_string().as_bytes())?;
        let rows = db::get_blocks_per_day_per_pool(&mut conn, *id)?;
        let content: String = rows
            .iter()
            .map(|row| format!("{},{},{}\n", row.date, row.count, row.total))
            .collect();
        file.write_all(content.as_bytes())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Block;
    use crate::stats::Stats;
    use crate::write_csv_files;
    use diesel::sql_query;
    use diesel::RunQueryDsl;
    use rand::distr::{Alphanumeric, SampleString};
    use serde::Deserialize;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::{Path, PathBuf};

    /// The height of the block in `testdata/361582.json`.
    const FIXTURE_HEIGHT: i64 = 361_582;

    /// A temporary directory that removes itself again.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> TempDir {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "mainnet-observer-gen-csv-tests-{}",
                Alphanumeric.sample_string(&mut rand::rng(), 16)
            ));
            std::fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }

        fn path(&self) -> &str {
            self.0.to_str().unwrap()
        }

        fn read(&self, name: &str) -> String {
            std::fs::read_to_string(self.0.join(name))
                .unwrap_or_else(|e| panic!("could not read {}: {}", name, e))
        }

        fn exists(&self, name: &str) -> bool {
            Path::new(&self.0).join(name).exists()
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn fixture_stats() -> &'static Stats {
        static FIXTURE: std::sync::OnceLock<Stats> = std::sync::OnceLock::new();
        FIXTURE.get_or_init(|| {
            let buffer = BufReader::new(File::open("./testdata/361582.json").unwrap());
            let mut de = serde_json::Deserializer::from_reader(buffer);
            let block = Block::deserialize(&mut de).expect("test block json to be valid");
            Stats::from_block(block).expect("test block should produce stats")
        })
    }

    fn stats_row(height: i64, date: &str, pool_id: i32) -> Stats {
        let mut stats = fixture_stats().clone();
        stats.block.height = height;
        stats.block.date = date.to_string();
        stats.block.pool_id = pool_id;
        stats.tx.height = height;
        stats.tx.date = date.to_string();
        stats
    }

    /// Inserts rows and makes sure all six tables use the same height and
    /// date. Only `block_stats` and `tx_stats` have public height and date
    /// fields, the other four keep the test block's height.
    fn insert_rows(conn: &mut SqliteConnection, rows: &[Stats]) {
        for row in rows {
            let height = row.block.height;
            assert_ne!(height, FIXTURE_HEIGHT);
            let date = row.block.date.clone();
            db::insert_stats(conn, std::slice::from_ref(row)).unwrap();
            for table in [
                "input_stats",
                "output_stats",
                "script_stats",
                "feerate_stats",
            ] {
                sql_query(format!(
                    "UPDATE OR REPLACE {} SET height = {}, date = '{}' WHERE height = {}",
                    table, height, date, FIXTURE_HEIGHT
                ))
                .execute(conn)
                .unwrap();
            }
        }
    }

    /// A database with six blocks on three days, mined by three pools.
    /// Pool 88 is Foundry USA, pool 61 is AntPool (part of the AntPool proxy
    /// pool group) and pool 9999 does not exist in the pool list. The three
    /// pools have different block counts so that their ranking is unambiguous.
    fn filled_db() -> Arc<Mutex<SqliteConnection>> {
        let mut conn = db::open_db_and_run_migrations(":memory:").unwrap();
        let mut rows = vec![
            stats_row(800_000, "2024-01-01", 88),
            stats_row(800_001, "2024-01-01", 88),
            stats_row(800_002, "2024-01-01", 88),
            stats_row(800_003, "2024-01-02", 61),
            stats_row(800_004, "2024-01-02", 61),
            stats_row(800_005, "2024-01-03", 9999),
        ];
        for (row, transactions) in rows.iter_mut().zip([10, 20, 30, 7, 3, 3]) {
            row.block.transactions = transactions;
            row.block.version = 0x2000_0000;
            row.tx.tx_spending_ephemeral_dust = 0;
        }
        // Only the two first blocks signal version bit 4.
        rows[0].block.version = 0x2000_0010;
        rows[1].block.version = 0x2000_0010;
        rows[5].tx.tx_spending_ephemeral_dust = 1;
        insert_rows(&mut conn, &rows);
        Arc::new(Mutex::new(conn))
    }

    fn pool_name(id: u64) -> String {
        bitcoin_pool_identification::default_data(Network::Bitcoin)
            .iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("pool {} should be in the pool list", id))
            .name
            .clone()
    }

    fn rows_of(csv: &str) -> Vec<String> {
        csv.trim_end_matches('\n')
            .split('\n')
            .map(|l| l.to_string())
            .collect()
    }

    #[test]
    fn write_csv_files_creates_all_expected_files() {
        let dir = TempDir::new();
        write_csv_files(dir.path(), filled_db()).unwrap();

        for name in [
            "date.csv",
            "transactions_avg.csv",
            "transactions_sum.csv",
            "block_count_sum.csv",
            "coinbase_subsidy_avg.csv",
            "coinbase_fees_avg.csv",
            "top5pools.csv",
            "miningpools-antpool-and-friends.csv",
            "miningpools-centralization-index.csv",
            "miningpools-centralization-index-with-proxy-pools.csv",
            "miningpools-mining-ephemeral-dust.csv",
            "miningpools-mining-bip54-coinbase.csv",
            "miningpools-mining-p2a.csv",
            "miningpools-mining-bip110.csv",
            "bip110-signaling.csv",
            "unclaimed-coinbase-blocks.csv",
            "miningpools-poolid-0.csv",
            "miningpools-poolid-61.csv",
            "miningpools-poolid-88.csv",
        ] {
            assert!(dir.exists(name), "{} should have been written", name);
        }
    }

    #[test]
    fn every_metric_file_has_as_many_rows_as_date_csv() {
        let dir = TempDir::new();
        write_csv_files(dir.path(), filled_db()).unwrap();

        let dates = rows_of(&dir.read("date.csv"));
        assert_eq!(
            dates,
            vec!["date", "2024-01-01", "2024-01-02", "2024-01-03"]
        );

        // The frontend joins these files with date.csv by row position.
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir.0).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().to_string();
            if !name.ends_with("_avg.csv") && !name.ends_with("_sum.csv") {
                continue;
            }
            assert_eq!(
                rows_of(&dir.read(&name)).len(),
                dates.len(),
                "{} does not line up with date.csv",
                name
            );
            checked += 1;
        }
        assert!(checked > 50, "only checked {} metric files", checked);
    }

    #[test]
    fn metric_files_have_a_header_and_four_decimal_averages() {
        let dir = TempDir::new();
        write_csv_files(dir.path(), filled_db()).unwrap();

        assert_eq!(
            dir.read("transactions_avg.csv"),
            "transactions_avg\n20.0000\n5.0000\n3.0000\n"
        );
        assert_eq!(
            dir.read("transactions_sum.csv"),
            "transactions_sum\n60\n10\n3\n"
        );
        // block_count is 1 for every block, so its sum is the blocks per day.
        assert_eq!(
            dir.read("block_count_sum.csv"),
            "block_count_sum\n3\n2\n1\n"
        );
    }

    #[test]
    fn no_metric_files_are_written_for_the_stats_version() {
        let dir = TempDir::new();
        write_csv_files(dir.path(), filled_db()).unwrap();

        // The stats version says how old the stats of a block are. It is
        // bookkeeping, not something worth charting.
        assert!(!dir.exists("stats_version_avg.csv"));
        assert!(!dir.exists("stats_version_sum.csv"));
        // The other non metric columns are already left out.
        for name in ["height_avg.csv", "nonce_sum.csv", "pool_id_avg.csv"] {
            assert!(!dir.exists(name), "{} should not be written", name);
        }
    }

    #[test]
    fn coinbase_subsidy_and_fees_use_four_decimals() {
        let dir = TempDir::new();
        write_csv_files(dir.path(), filled_db()).unwrap();

        let subsidy = rows_of(&dir.read("coinbase_subsidy_avg.csv"));
        assert_eq!(subsidy[0], "coinbase_subsidy_avg");
        // Height 800000 is in the fourth subsidy era: 6.25 BTC.
        assert_eq!(subsidy[1], "625000000.0000");
        assert_eq!(subsidy.len(), 4, "one header plus one row per date");
        assert_eq!(
            rows_of(&dir.read("coinbase_fees_avg.csv"))[0],
            "coinbase_fees_avg"
        );
    }

    #[test]
    fn top5_pools_csv_names_the_pools_it_found() {
        let dir = TempDir::new();
        write_csv_files(dir.path(), filled_db()).unwrap();

        let rows = rows_of(&dir.read("top5pools.csv"));
        let header: Vec<&str> = rows[0].split(',').collect();
        assert_eq!(header[0], "date");
        assert_eq!(header[1], pool_name(88));
        assert_eq!(header[2], pool_name(61));
        // Only three pools mined. The two remaining columns are placeholders
        // without a name. This cannot happen with real data, where there are
        // always more than five pools.
        assert_eq!(header[4], "");
        assert_eq!(header[5], "");
        assert_eq!(header[6], "total");

        assert_eq!(rows[1], "2024-01-01,3,0,0,0,0,3");
        assert_eq!(rows[2], "2024-01-02,0,2,0,0,0,2");
    }

    #[test]
    fn antpool_and_friends_csv_lists_a_group_pool_only_once() {
        let dir = TempDir::new();
        write_csv_files(dir.path(), filled_db()).unwrap();

        let rows = rows_of(&dir.read("miningpools-antpool-and-friends.csv"));
        let header: Vec<&str> = rows[0].split(',').collect();
        assert_eq!(header[0], "date");
        assert_eq!(header[1], "AntPool & friends");
        // AntPool (61) is part of the group, so it must not get its own column.
        assert!(
            !header[2..].contains(&pool_name(61).as_str()),
            "a pool of the AntPool group is listed twice: {:?}",
            header
        );
        assert_eq!(header[2], pool_name(88));
        assert_eq!(*header.last().unwrap(), "total");
    }

    #[test]
    fn top_pool_csv_headers_name_an_unknown_pool_unknown_with_its_id() {
        let dir = TempDir::new();
        write_csv_files(dir.path(), filled_db()).unwrap();

        // Pool 9999 is not in the pool list. Every other csv file calls such
        // a pool "Unknown" instead of leaving the name empty, and an empty
        // name showed up as an unnamed series in the chart.
        let rows = rows_of(&dir.read("top5pools.csv"));
        let header: Vec<&str> = rows[0].split(',').collect();
        assert_eq!(header[3], "Unknown (9999)");

        let rows = rows_of(&dir.read("miningpools-antpool-and-friends.csv"));
        let header: Vec<&str> = rows[0].split(',').collect();
        assert_eq!(header[3], "Unknown (9999)");
    }

    #[test]
    fn two_unidentified_top_pools_get_different_headers() {
        let mut conn = db::open_db_and_run_migrations(":memory:").unwrap();
        insert_rows(
            &mut conn,
            &[
                stats_row(800_000, "2024-01-01", 9999),
                stats_row(800_001, "2024-01-01", 9999),
                stats_row(800_002, "2024-01-02", 9998),
            ],
        );
        let dir = TempDir::new();
        write_csv_files(dir.path(), Arc::new(Mutex::new(conn))).unwrap();

        // The frontend keys the columns by their header, so two unidentified
        // pools with the same header would collapse into one series.
        let rows = rows_of(&dir.read("top5pools.csv"));
        let header: Vec<&str> = rows[0].split(',').collect();
        assert_eq!(header[1], "Unknown (9999)");
        assert_eq!(header[2], "Unknown (9998)");
    }

    #[test]
    fn unknown_pool_ids_are_written_as_unknown() {
        let dir = TempDir::new();
        write_csv_files(dir.path(), filled_db()).unwrap();

        // Pool 9999 mined the only block with an ephemeral dust spend.
        let rows = rows_of(&dir.read("miningpools-mining-ephemeral-dust.csv"));
        assert_eq!(rows[0], "pool,height,date,total");
        assert_eq!(rows[1], "Unknown,800005,2024-01-03,1");
    }

    #[test]
    fn version_bit_files_hold_the_signaling_blocks() {
        let dir = TempDir::new();
        let conn = filled_db();
        write_csv_files(dir.path(), Arc::clone(&conn)).unwrap();

        // bip110-signaling.csv is joined with date.csv by row position.
        assert_eq!(
            dir.read("bip110-signaling.csv"),
            "bip110_signaling_count\n2\n0\n0\n"
        );

        // The default BIP110 height range does not contain our blocks.
        assert_eq!(
            dir.read("miningpools-mining-bip110.csv"),
            "pool,height,date,total\n"
        );

        // With a range that contains them, the signaling pool shows up.
        pools_mining_version_bit_csv(dir.path(), conn, "bit4", 4, 0, 900_000).unwrap();
        let rows = rows_of(&dir.read("bit4.csv"));
        assert_eq!(rows[0], "pool,height,date,total");
        assert_eq!(rows[1], format!("{},800000,2024-01-01,2", pool_name(88)));
        assert_eq!(rows.len(), 2, "only one pool signaled bit 4");
    }
}
