//! Tests for the public query and insert functions in `db`.
//!
//! Every test starts from a fresh, fully migrated in-memory SQLite database and
//! fills it with rows derived from the `361582` test block.

use diesel::sql_query;
use diesel::sqlite::SqliteConnection;
use diesel::{QueryableByName, RunQueryDsl};
use mainnet_observer_backend::db;
use mainnet_observer_backend::rest::Block;
use mainnet_observer_backend::stats::Stats;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::sync::OnceLock;

/// The height of the block in `testdata/361582.json`. Tests must not use this
/// height for their own rows, see `insert_rows()`.
const FIXTURE_HEIGHT: i64 = 361_582;

/// The six tables `insert_stats()` writes to.
const ALL_TABLES: [&str; 6] = [
    "block_stats",
    "tx_stats",
    "input_stats",
    "output_stats",
    "script_stats",
    "feerate_stats",
];

/// The four tables whose height and date fields are private to `stats.rs` and
/// which therefore need the fixup in `insert_rows()`.
const TABLES_WITH_PRIVATE_HEIGHT: [&str; 4] = [
    "input_stats",
    "output_stats",
    "script_stats",
    "feerate_stats",
];

fn fixture_stats() -> &'static Stats {
    static FIXTURE: OnceLock<Stats> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let buffer = BufReader::new(File::open("./testdata/361582.json").unwrap());
        let mut de = serde_json::Deserializer::from_reader(buffer);
        let block = Block::deserialize(&mut de).expect("test block json to be valid");
        Stats::from_block(block).expect("test block should produce stats")
    })
}

fn fresh_db() -> SqliteConnection {
    db::open_db_and_run_migrations(":memory:").expect("in-memory database should open")
}

/// Builds a `Stats` for the given height, date and pool id, based on the test
/// block. All other values are the ones of the test block.
fn stats_row(height: i64, date: &str, pool_id: i32) -> Stats {
    let mut stats = fixture_stats().clone();
    stats.block.height = height;
    stats.block.date = date.to_string();
    stats.block.pool_id = pool_id;
    stats.tx.height = height;
    stats.tx.date = date.to_string();
    stats
}

/// Inserts rows one by one and makes sure all six tables end up with the same
/// height and date.
///
/// `insert_stats()` writes six tables, but only `block_stats` and `tx_stats`
/// have public height and date fields. The other four keep the test block's
/// height, so they are rewritten here.
fn insert_rows(conn: &mut SqliteConnection, rows: &[Stats]) {
    for row in rows {
        let height = row.block.height;
        assert_ne!(
            height, FIXTURE_HEIGHT,
            "tests must not reuse the height of the test block"
        );
        let date = row.block.date.clone();
        db::insert_stats(conn, std::slice::from_ref(row)).unwrap();
        for table in TABLES_WITH_PRIVATE_HEIGHT.iter() {
            sql_query(format!(
                "UPDATE OR REPLACE {} SET height = {}, date = '{}' WHERE height = {}",
                table, height, date, FIXTURE_HEIGHT
            ))
            .execute(conn)
            .unwrap();
        }
    }
}

/// Inserts many rows in batches. Only `block_stats` ends up with one row per
/// passed `Stats`; the other five tables are left with a single row. Used by
/// the tests that need thousands of blocks and only look at `block_stats`.
fn insert_block_rows(conn: &mut SqliteConnection, rows: &[Stats]) {
    for chunk in rows.chunks(100) {
        db::insert_stats(conn, chunk).unwrap();
    }
}

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

fn row_count(conn: &mut SqliteConnection, table: &str) -> i64 {
    let rows: Vec<Count> = sql_query(format!("SELECT COUNT(*) AS count FROM {}", table))
        .get_results(conn)
        .unwrap();
    rows[0].count
}

fn update_column(conn: &mut SqliteConnection, table: &str, column: &str, value: i64, height: i64) {
    sql_query(format!(
        "UPDATE {} SET {} = {} WHERE height = {}",
        table, column, value, height
    ))
    .execute(conn)
    .unwrap();
}

// ---------------------------------------------------------------------------
// block height and stats version
// ---------------------------------------------------------------------------

#[test]
fn empty_database_has_no_block_height() {
    let mut conn = fresh_db();
    assert_eq!(db::get_db_block_height(&mut conn).unwrap(), None);
}

#[test]
fn block_height_is_the_highest_inserted_height() {
    let mut conn = fresh_db();
    insert_rows(
        &mut conn,
        &[
            stats_row(100, "2024-01-01", 1),
            stats_row(300, "2024-01-01", 1),
            stats_row(200, "2024-01-01", 1),
        ],
    );
    assert_eq!(db::get_db_block_height(&mut conn).unwrap(), Some(300));
}

#[test]
fn block_heights_greater_equals_version_only_returns_new_enough_rows() {
    let mut conn = fresh_db();
    let mut old = stats_row(100, "2024-01-01", 1);
    old.block.stats_version = 3;
    let mut current = stats_row(200, "2024-01-01", 1);
    current.block.stats_version = 5;
    let mut newer = stats_row(300, "2024-01-01", 1);
    newer.block.stats_version = 9;
    insert_rows(&mut conn, &[old, current, newer]);

    let mut heights = db::block_heights_greater_equals_version(&mut conn, 5).unwrap();
    heights.sort();
    assert_eq!(heights, vec![200, 300]);

    assert!(db::block_heights_greater_equals_version(&mut conn, 10)
        .unwrap()
        .is_empty());
    assert_eq!(
        db::block_heights_greater_equals_version(&mut conn, 0)
            .unwrap()
            .len(),
        3
    );
}

// ---------------------------------------------------------------------------
// schema helpers
// ---------------------------------------------------------------------------

#[test]
fn list_column_names_returns_the_block_stats_columns() {
    let mut conn = fresh_db();
    let names: Vec<String> = db::list_column_names(&mut conn, "block_stats")
        .unwrap()
        .into_iter()
        .map(|c| c.name)
        .collect();
    for expected in [
        "height",
        "date",
        "version",
        "pool_id",
        "hash",
        "sigops",
        "stats_version",
        "block_count",
        "coinbase_unclaimed_sat",
    ] {
        assert!(
            names.contains(&expected.to_string()),
            "block_stats should have a '{}' column, got {:?}",
            expected,
            names
        );
    }
}

#[test]
fn list_column_names_of_an_unknown_table_is_empty() {
    let mut conn = fresh_db();
    assert!(db::list_column_names(&mut conn, "no_such_table")
        .unwrap()
        .is_empty());
}

// ---------------------------------------------------------------------------
// per date metrics
// ---------------------------------------------------------------------------

#[test]
fn date_column_lists_every_date_once_and_sorted() {
    let mut conn = fresh_db();
    insert_rows(
        &mut conn,
        &[
            stats_row(100, "2024-01-03", 1),
            stats_row(101, "2024-01-01", 1),
            stats_row(102, "2024-01-03", 1),
            stats_row(103, "2024-01-02", 1),
        ],
    );
    let dates: Vec<String> = db::date_column(&mut conn)
        .into_iter()
        .map(|d| d.date)
        .collect();
    assert_eq!(dates, vec!["2024-01-01", "2024-01-02", "2024-01-03"]);
}

#[test]
fn column_sum_and_avg_by_date_aggregates_per_date() {
    let mut conn = fresh_db();
    let mut first = stats_row(100, "2024-01-01", 1);
    first.block.transactions = 10;
    let mut second = stats_row(101, "2024-01-01", 1);
    second.block.transactions = 20;
    let mut third = stats_row(102, "2024-01-02", 1);
    third.block.transactions = 7;
    insert_rows(&mut conn, &[first, second, third]);

    let rows = db::column_sum_and_avg_by_date(&mut conn, "transactions", "block_stats");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].sum, 30);
    assert_eq!(rows[0].avg, 15.0);
    assert_eq!(rows[1].sum, 7);
    assert_eq!(rows[1].avg, 7.0);
}

#[test]
fn column_sum_and_avg_by_date_counts_blocks_per_day() {
    let mut conn = fresh_db();
    insert_rows(
        &mut conn,
        &[
            stats_row(100, "2024-01-01", 1),
            stats_row(101, "2024-01-01", 1),
            stats_row(102, "2024-01-02", 1),
        ],
    );
    // block_count defaults to 1 for every row, so its sum is the number of
    // blocks of that day.
    let rows = db::column_sum_and_avg_by_date(&mut conn, "block_count", "block_stats");
    assert_eq!(rows.iter().map(|r| r.sum).collect::<Vec<i64>>(), vec![2, 1]);
}

#[test]
fn coinbase_subsidy_halves_at_the_halving_height() {
    let mut conn = fresh_db();
    // The last block before the first halving and the first block after it.
    let mut before = stats_row(209_999, "2024-01-01", 1);
    before.block.coinbase_output_amount = 5_000_000_000 + 12_345;
    let mut after = stats_row(210_000, "2024-01-02", 1);
    after.block.coinbase_output_amount = 2_500_000_000 + 500;
    // The second halving.
    let mut second_halving = stats_row(420_000, "2024-01-03", 1);
    second_halving.block.coinbase_output_amount = 1_250_000_000;
    insert_rows(&mut conn, &[before, after, second_halving]);

    let rows = db::coinbase_subsidy_and_fees_avg_by_date(&mut conn);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].subsidy_avg, 5_000_000_000.0);
    assert_eq!(rows[0].fees_avg, 12_345.0);
    assert_eq!(rows[1].subsidy_avg, 2_500_000_000.0);
    assert_eq!(rows[1].fees_avg, 500.0);
    assert_eq!(rows[2].subsidy_avg, 1_250_000_000.0);
    assert_eq!(rows[2].fees_avg, 0.0);
}

// ---------------------------------------------------------------------------
// mining pools
// ---------------------------------------------------------------------------

#[test]
fn current_top_mining_pools_ignores_blocks_outside_the_recent_window() {
    let mut conn = fresh_db();
    // The query looks at the 2016*2 = 4032 most recent blocks. Pool 3 only
    // mined the oldest block and must not show up.
    let mut rows = vec![stats_row(1, "2024-01-01", 3)];
    for height in 2..=3000 {
        rows.push(stats_row(height, "2024-01-01", 1));
    }
    for height in 3001..=4033 {
        rows.push(stats_row(height, "2024-01-01", 2));
    }
    insert_block_rows(&mut conn, &rows);

    let top = db::current_top_mining_pools(&mut conn).unwrap();
    let ids: Vec<i32> = top.iter().map(|p| p.pool_id).collect();
    assert_eq!(ids, vec![1, 2], "pools must be ordered by block count");
    assert_eq!(top[0].count, 2999);
    assert_eq!(top[1].count, 1033);
}

#[test]
fn blocks_per_day_top5_pool_groups_sums_a_group_into_one_column() {
    let mut conn = fresh_db();
    insert_rows(
        &mut conn,
        &[
            stats_row(100, "2024-01-01", 1),
            stats_row(101, "2024-01-01", 2),
            stats_row(102, "2024-01-01", 1),
            stats_row(103, "2024-01-01", 3),
            // A pool that is in none of the groups. It is not counted in any
            // of the top columns but it is part of the daily total.
            stats_row(104, "2024-01-01", 9),
        ],
    );

    let groups = [vec![1, 2], vec![3], vec![], vec![], vec![]];
    let rows = db::blocks_per_day_top5_pool_groups(&mut conn, &groups).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].date, "2024-01-01");
    assert_eq!(rows[0].top1_blocks, 3);
    assert_eq!(rows[0].top2_blocks, 1);
    assert_eq!(rows[0].top3_blocks, 0);
    assert_eq!(rows[0].top4_blocks, 0);
    assert_eq!(rows[0].top5_blocks, 0);
    assert_eq!(rows[0].total, 5);
}

#[test]
fn blocks_per_day_top5_pool_groups_accepts_empty_groups() {
    // An empty group produces an `IN ()` in the SQL, which SQLite accepts.
    let mut conn = fresh_db();
    insert_rows(&mut conn, &[stats_row(100, "2024-01-01", 1)]);
    let groups = [vec![], vec![], vec![], vec![], vec![]];
    let rows = db::blocks_per_day_top5_pool_groups(&mut conn, &groups).unwrap();
    assert!(
        rows.is_empty(),
        "without any pool id nothing matches the WHERE clause"
    );
}

#[test]
fn blocks_per_day_top5_pool_groups_is_sorted_by_date() {
    let mut conn = fresh_db();
    insert_rows(
        &mut conn,
        &[
            stats_row(100, "2024-03-01", 1),
            stats_row(101, "2024-01-01", 1),
            stats_row(102, "2024-02-01", 1),
        ],
    );
    let groups = [vec![1], vec![-1], vec![-1], vec![-1], vec![-1]];
    let rows = db::blocks_per_day_top5_pool_groups(&mut conn, &groups).unwrap();
    let dates: Vec<&str> = rows.iter().map(|r| r.date.as_str()).collect();
    assert_eq!(dates, vec!["2024-01-01", "2024-02-01", "2024-03-01"]);
}

#[test]
fn blocks_per_day_top5_pool_groups_drops_days_without_a_listed_pool() {
    // Days on which none of the listed pools mined a block are missing from
    // the result entirely, which leaves gaps in the CSV file.
    let mut conn = fresh_db();
    insert_rows(
        &mut conn,
        &[
            stats_row(100, "2024-01-01", 1),
            stats_row(101, "2024-01-02", 9),
            stats_row(102, "2024-01-03", 1),
        ],
    );
    let groups = [vec![1], vec![-1], vec![-1], vec![-1], vec![-1]];
    let rows = db::blocks_per_day_top5_pool_groups(&mut conn, &groups).unwrap();
    let dates: Vec<&str> = rows.iter().map(|r| r.date.as_str()).collect();
    assert_eq!(dates, vec!["2024-01-01", "2024-01-03"]);
}

#[test]
fn mining_centralization_index_ranks_the_pools_of_a_day() {
    let mut conn = fresh_db();
    let mut rows = Vec::new();
    let mut height = 100;
    for (pool, blocks) in [(1, 3), (2, 2), (3, 1)] {
        for _ in 0..blocks {
            rows.push(stats_row(height, "2024-01-01", pool));
            height += 1;
        }
    }
    insert_rows(&mut conn, &rows);

    let index = db::mining_centralization_index(&mut conn).unwrap();
    assert_eq!(index.len(), 1);
    assert_eq!(index[0].date, "2024-01-01");
    assert_eq!(index[0].top1_count, 3);
    assert_eq!(index[0].top2_count, 2);
    assert_eq!(index[0].top3_count, 1);
    assert_eq!(index[0].top4_count, 0);
    assert_eq!(index[0].top5_count, 0);
    assert_eq!(index[0].top6_count, 0);
    assert_eq!(index[0].total_blocks, 6);
}

#[test]
fn mining_centralization_index_with_proxy_pools_collapses_the_antpool_group() {
    let mut conn = fresh_db();
    let mut rows = Vec::new();
    let mut height = 100;
    // 61 (AntPool) and 111 (Poolin) are both in the AntPool proxy group.
    for (pool, blocks) in [(61, 2), (111, 2), (7, 3)] {
        for _ in 0..blocks {
            rows.push(stats_row(height, "2024-01-01", pool));
            height += 1;
        }
    }
    insert_rows(&mut conn, &rows);

    let plain = db::mining_centralization_index(&mut conn).unwrap();
    assert_eq!(plain[0].top1_count, 3, "pool 7 is the biggest single pool");

    let grouped = db::mining_centralization_index_with_proxy_pools(&mut conn).unwrap();
    assert_eq!(grouped.len(), 1);
    assert_eq!(
        grouped[0].top1_count, 4,
        "the two AntPool group pools count as one"
    );
    assert_eq!(grouped[0].top2_count, 3);
    assert_eq!(grouped[0].top3_count, 0);
    assert_eq!(grouped[0].total_blocks, 7);
    assert_eq!(
        grouped[0].top1_count + grouped[0].top2_count,
        grouped[0].total_blocks,
        "the ranks must add up to all blocks of the day"
    );
}

#[test]
fn get_blocks_per_day_per_pool_returns_the_pool_and_the_daily_total() {
    let mut conn = fresh_db();
    insert_rows(
        &mut conn,
        &[
            stats_row(100, "2024-01-01", 5),
            stats_row(101, "2024-01-01", 5),
            stats_row(102, "2024-01-01", 6),
            stats_row(103, "2024-01-02", 5),
        ],
    );
    let rows = db::get_blocks_per_day_per_pool(&mut conn, 5).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].date, "2024-01-01");
    assert_eq!(rows[0].count, 2);
    assert_eq!(rows[0].total, 3);
    assert_eq!(rows[1].date, "2024-01-02");
    assert_eq!(rows[1].count, 1);
    assert_eq!(rows[1].total, 1);

    assert!(db::get_blocks_per_day_per_pool(&mut conn, 999)
        .unwrap()
        .is_empty());
}

// ---------------------------------------------------------------------------
// pool feature tables
// ---------------------------------------------------------------------------

#[test]
fn get_pools_mining_ephemeral_dust_counts_blocks_not_transactions() {
    let mut conn = fresh_db();
    let mut first = stats_row(100, "2024-01-01", 1);
    first.tx.tx_spending_ephemeral_dust = 2;
    let mut second = stats_row(101, "2024-01-02", 1);
    second.tx.tx_spending_ephemeral_dust = 1;
    let mut other_pool = stats_row(102, "2024-01-03", 2);
    other_pool.tx.tx_spending_ephemeral_dust = 5;
    let mut without_dust = stats_row(103, "2024-01-01", 3);
    without_dust.tx.tx_spending_ephemeral_dust = 0;
    insert_rows(&mut conn, &[first, second, other_pool, without_dust]);

    let rows = db::get_pools_mining_ephemeral_dust(&mut conn).unwrap();
    assert_eq!(rows.len(), 2, "a pool without dust blocks is not listed");
    assert_eq!(rows[0].pool_id, 1);
    // Pool 1 mined three dust spending transactions in two blocks. The count
    // is the number of blocks, matching the "first block" columns next to it.
    assert_eq!(rows[0].count, 2);
    assert_eq!(rows[0].first_ephemeral_dust_height, 100);
    assert_eq!(rows[0].first_ephemeral_dust_date, "2024-01-01");
    assert_eq!(rows[1].pool_id, 2);
    assert_eq!(rows[1].count, 1);
}

#[test]
fn get_pools_mining_bip54_coinbase_lists_the_first_block_per_pool() {
    let mut conn = fresh_db();
    let mut first = stats_row(100, "2024-01-02", 1);
    first.block.coinbase_locktime_set_bip54 = true;
    let mut second = stats_row(101, "2024-01-03", 1);
    second.block.coinbase_locktime_set_bip54 = true;
    let mut other_pool = stats_row(102, "2024-01-01", 2);
    other_pool.block.coinbase_locktime_set_bip54 = true;
    let mut without = stats_row(103, "2024-01-01", 3);
    without.block.coinbase_locktime_set_bip54 = false;
    insert_rows(&mut conn, &[first, second, other_pool, without]);

    let rows = db::get_pools_mining_bip54_coinbase(&mut conn).unwrap();
    assert_eq!(rows.len(), 2);
    // Ordered by the date of the first such block.
    assert_eq!(rows[0].pool_id, 2);
    assert_eq!(rows[0].count, 1);
    assert_eq!(rows[0].first_bip54_coinbase_height, 102);
    assert_eq!(rows[0].first_bip54_coinbase_date, "2024-01-01");
    assert_eq!(rows[1].pool_id, 1);
    assert_eq!(rows[1].count, 2);
    assert_eq!(rows[1].first_bip54_coinbase_height, 100);
}

#[test]
fn get_pools_mining_p2a_handles_pools_with_only_inputs_or_only_outputs() {
    let mut conn = fresh_db();
    insert_rows(
        &mut conn,
        &[
            stats_row(100, "2024-01-01", 1),
            stats_row(101, "2024-01-02", 2),
            stats_row(102, "2024-01-03", 3),
        ],
    );
    // Make sure no other row has P2A activity.
    for height in [100, 101, 102] {
        update_column(&mut conn, "input_stats", "inputs_p2a", 0, height);
        update_column(&mut conn, "output_stats", "outputs_p2a", 0, height);
    }
    // Pool 1 only spends P2A outputs, pool 2 only creates them.
    update_column(&mut conn, "input_stats", "inputs_p2a", 3, 100);
    update_column(&mut conn, "output_stats", "outputs_p2a", 7, 101);

    let rows = db::get_pools_mining_p2a(&mut conn).unwrap();
    assert_eq!(rows.len(), 2, "pool 3 has no P2A activity at all");

    let only_inputs = rows.iter().find(|r| r.pool_id == 1).unwrap();
    assert_eq!(only_inputs.first_p2a_input_height, Some(100));
    assert_eq!(
        only_inputs.first_p2a_input_date,
        Some("2024-01-01".to_string())
    );
    assert_eq!(only_inputs.first_p2a_output_height, None);
    assert_eq!(only_inputs.first_p2a_output_date, None);
    assert_eq!(only_inputs.total_inputs, 3);
    assert_eq!(only_inputs.total_outputs, 0);

    let only_outputs = rows.iter().find(|r| r.pool_id == 2).unwrap();
    assert_eq!(only_outputs.first_p2a_input_height, None);
    assert_eq!(only_outputs.first_p2a_input_date, None);
    assert_eq!(only_outputs.first_p2a_output_height, Some(101));
    assert_eq!(only_outputs.total_inputs, 0);
    assert_eq!(only_outputs.total_outputs, 7);

    // Pools without a first P2A input are sorted last.
    assert_eq!(rows[0].pool_id, 1);
    assert_eq!(rows[1].pool_id, 2);
}

// ---------------------------------------------------------------------------
// version bit signaling
// ---------------------------------------------------------------------------

/// A block version signaling bit 4 (and the usual BIP9 top bits).
const VERSION_WITH_BIT_4: i32 = 0x2000_0010;
/// A block version without bit 4 set.
const VERSION_WITHOUT_BIT_4: i32 = 0x2000_0000;

#[test]
fn get_pools_mining_version_bit_includes_both_range_ends() {
    let mut conn = fresh_db();
    let mut rows = Vec::new();
    for (height, date, pool) in [
        (100, "2024-01-01", 1),
        (200, "2024-01-02", 1),
        (300, "2024-01-03", 2),
    ] {
        let mut row = stats_row(height, date, pool);
        row.block.version = VERSION_WITH_BIT_4;
        rows.push(row);
    }
    insert_rows(&mut conn, &rows);

    let all = db::get_pools_mining_version_bit(&mut conn, 4, 100, 300).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0], (1, 2, 100, "2024-01-01".to_string()));
    assert_eq!(all[1], (2, 1, 300, "2024-01-03".to_string()));

    // One height narrower on each side drops the first and the last block.
    let narrowed = db::get_pools_mining_version_bit(&mut conn, 4, 101, 299).unwrap();
    assert_eq!(narrowed.len(), 1);
    assert_eq!(narrowed[0], (1, 1, 200, "2024-01-02".to_string()));
}

#[test]
fn get_pools_mining_version_bit_only_matches_the_requested_bit() {
    let mut conn = fresh_db();
    let mut signaling = stats_row(100, "2024-01-01", 1);
    signaling.block.version = VERSION_WITH_BIT_4;
    let mut not_signaling = stats_row(101, "2024-01-01", 2);
    not_signaling.block.version = VERSION_WITHOUT_BIT_4;
    insert_rows(&mut conn, &[signaling, not_signaling]);

    assert_eq!(
        db::get_pools_mining_version_bit(&mut conn, 4, 0, 1000)
            .unwrap()
            .len(),
        1
    );
    assert!(db::get_pools_mining_version_bit(&mut conn, 3, 0, 1000)
        .unwrap()
        .is_empty());
}

#[test]
fn blocks_signaling_version_bit_per_day_counts_per_date() {
    let mut conn = fresh_db();
    let mut first = stats_row(100, "2024-01-01", 1);
    first.block.version = VERSION_WITH_BIT_4;
    let mut second = stats_row(101, "2024-01-01", 1);
    second.block.version = VERSION_WITH_BIT_4;
    let mut third = stats_row(102, "2024-01-01", 1);
    third.block.version = VERSION_WITHOUT_BIT_4;
    let mut fourth = stats_row(103, "2024-01-02", 1);
    fourth.block.version = VERSION_WITHOUT_BIT_4;
    insert_rows(&mut conn, &[first, second, third, fourth]);

    assert_eq!(
        db::blocks_signaling_version_bit_per_day(&mut conn, 4).unwrap(),
        vec![2, 0]
    );
    assert_eq!(
        db::blocks_signaling_version_bit_per_day(&mut conn, 1).unwrap(),
        vec![0, 0]
    );
}

// ---------------------------------------------------------------------------
// unclaimed coinbase
// ---------------------------------------------------------------------------

#[test]
fn get_blocks_with_unclaimed_coinbase_lists_only_blocks_with_a_shortfall() {
    let mut conn = fresh_db();
    let mut second = stats_row(200, "2024-01-02", 7);
    second.block.coinbase_unclaimed_sat = 50;
    let mut first = stats_row(100, "2024-01-01", 8);
    first.block.coinbase_unclaimed_sat = 1234;
    let mut full = stats_row(150, "2024-01-01", 9);
    full.block.coinbase_unclaimed_sat = 0;
    insert_rows(&mut conn, &[second, first, full]);

    let rows = db::get_blocks_with_unclaimed_coinbase(&mut conn).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].height, 100);
    assert_eq!(rows[0].date, "2024-01-01");
    assert_eq!(rows[0].coinbase_unclaimed_sat, 1234);
    assert_eq!(rows[0].pool_id, 8);
    assert_eq!(rows[1].height, 200);
    assert_eq!(rows[1].coinbase_unclaimed_sat, 50);
}

// ---------------------------------------------------------------------------
// inserting
// ---------------------------------------------------------------------------

#[test]
fn insert_stats_replaces_a_row_with_the_same_height() {
    let mut conn = fresh_db();
    let mut first = stats_row(100, "2024-01-01", 1);
    first.block.transactions = 5;
    insert_rows(&mut conn, &[first]);

    let mut again = stats_row(100, "2024-01-02", 2);
    again.block.transactions = 7;
    insert_rows(&mut conn, &[again]);

    for table in ALL_TABLES.iter() {
        assert_eq!(
            row_count(&mut conn, table),
            1,
            "{} should still have a single row",
            table
        );
    }
    let rows = db::column_sum_and_avg_by_date(&mut conn, "transactions", "block_stats");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].sum, 7);
    assert_eq!(db::date_column(&mut conn)[0].date, "2024-01-02");
}

#[test]
fn insert_stats_writes_a_large_batch() {
    let mut conn = fresh_db();
    let rows: Vec<Stats> = (0..120)
        .map(|i| stats_row(1000 + i, "2024-01-01", 1))
        .collect();
    db::insert_stats(&mut conn, &rows).unwrap();

    assert_eq!(row_count(&mut conn, "block_stats"), 120);
    assert_eq!(row_count(&mut conn, "tx_stats"), 120);
    assert_eq!(db::get_db_block_height(&mut conn).unwrap(), Some(1119));
}
