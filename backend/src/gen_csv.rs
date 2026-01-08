use crate::{db, db::TableInfo, MainError};
use bitcoin::Network;
use diesel::SqliteConnection;
use log::info;
use std::collections::BTreeSet;
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
const COLUMN_NAMES_THAT_ARENT_METRICS: [&str; 6] =
    ["height", "date", "version", "nonce", "bits", "pool_id"];

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
    let date_column = db::date_column(&mut conn);
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
            let avg_and_sum = db::column_sum_and_avg_by_date(&mut conn, &column, table);

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
    let mut pool_names: [&str; 5] = ["", "", "", "", ""];
    for (i, top_pool) in top_pools.iter().enumerate() {
        if i >= pool_ids.len() {
            break;
        }
        pool_ids[i] = vec![top_pool.pool_id];
        for pool in pool_data.iter().rev() {
            if top_pool.pool_id == pool.id as i32 {
                pool_names[i] = &pool.name;
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
    let mut pool_names: [&str; 5] = ["AntPool & friends", "", "", "", ""];
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
            for pool in pool_data.iter().rev() {
                if top_pool.pool_id == pool.id as i32 {
                    pool_names[pools_added] = &pool.name;
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
