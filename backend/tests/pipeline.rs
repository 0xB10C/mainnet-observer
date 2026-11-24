//! Tests for the fetch -> calc-stats -> batch-insert pipeline.
//!
//! These drive the public `collect_statistics` and `write_csv_files` against
//! the in-process mock of Bitcoin Core's REST interface from
//! `tests/common/mod.rs`, so they need neither a node nor a network.

mod common;

use common::{Endpoint, Fault, MockCore, TempDir};
use mainnet_observer_backend::stats::STATS_VERSION;
use mainnet_observer_backend::REORG_SAFETY_MARGIN;
use mainnet_observer_backend::{collect_statistics, db, write_csv_files, MainError};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// `collect_statistics` fetches `0..(chaininfo.blocks - REORG_SAFETY_MARGIN)`,
/// so this is the tip a node needs to report for `height` to be the last block
/// that gets fetched.
fn tip_for_last_fetched_height(height: u64) -> u64 {
    height + REORG_SAFETY_MARGIN + 1
}

#[test]
fn syncs_a_few_blocks_and_writes_the_csv_files() {
    common::init_logger();
    let mock = MockCore::builder()
        .blocks(tip_for_last_fetched_height(361582))
        .block_at(361580, "215049")
        .block_at(361581, "227154")
        .block_at(361582, "361582")
        .start();
    let conn = common::memory_db();

    collect_statistics(mock.host(), mock.port(), Arc::clone(&conn), 3, Some(361580))
        .expect("the sync should succeed");

    {
        let mut conn = conn.lock().unwrap();
        assert_eq!(db::get_db_block_height(&mut conn).unwrap(), Some(361582));
        assert_eq!(common::row_count(&mut conn, "block_stats"), 3);
        for height in 361580..=361582 {
            assert_eq!(
                common::block_stats_column(&mut conn, "stats_version", height),
                Some(STATS_VERSION as i64),
                "block {} should have been written with the current stats version",
                height
            );
        }
    }

    // Each block is one hash request plus one block request.
    assert_eq!(mock.requests(Endpoint::ChainInfo), 1);
    assert_eq!(mock.requests(Endpoint::BlockHash), 3);
    assert_eq!(mock.requests(Endpoint::Block), 3);

    let csv_dir = TempDir::new("csv");
    write_csv_files(&csv_dir.str_path(), Arc::clone(&conn)).expect("writing the csv files");

    let files: Vec<_> = std::fs::read_dir(csv_dir.path())
        .expect("read csv dir")
        .map(|e| e.expect("dir entry").file_name())
        .collect();
    assert!(!files.is_empty(), "no csv files were written");
    let dates = std::fs::read_to_string(csv_dir.child("date.csv")).expect("date.csv");
    assert!(
        dates.starts_with("date\n") && dates.lines().count() > 1,
        "date.csv should have a header and at least one date: {:?}",
        dates
    );
}

#[test]
fn retries_a_block_that_fails_to_fetch_a_few_times() {
    common::init_logger();
    let mock = MockCore::builder()
        .blocks(tip_for_last_fetched_height(361582))
        .block_at(361582, "361582")
        // The node answers the first two block requests with a 500.
        .fault(Endpoint::Block, Fault::status(500, 2))
        .start();
    let conn = common::memory_db();

    let started = Instant::now();
    collect_statistics(mock.host(), mock.port(), Arc::clone(&conn), 1, Some(361582))
        .expect("the sync should survive a transient failure");
    let elapsed = started.elapsed();

    {
        let mut conn = conn.lock().unwrap();
        assert_eq!(db::get_db_block_height(&mut conn).unwrap(), Some(361582));
    }
    // Two failed attempts plus the one that worked.
    assert_eq!(mock.requests(Endpoint::Block), 3);
    assert_eq!(mock.requests(Endpoint::BlockHash), 3);
    // The waits double: 500 ms before the second attempt, 1000 ms before the
    // third one. Hammering a node that just failed is what we want to avoid.
    assert!(
        elapsed >= Duration::from_millis(1400),
        "the two retries waited only {:?}",
        elapsed
    );
}

#[test]
fn gives_up_and_stops_the_other_threads_when_the_retries_run_out() {
    common::init_logger();
    const THREADS: usize = 2;
    const FIRST_HEIGHT: u64 = 361580;
    const HEIGHTS: u64 = 20;
    // Five attempts per block, as `MAX_RETRY_ATTEMPTS` says.
    const MAX_ATTEMPTS_PER_BLOCK: usize = 5;

    let mock = MockCore::builder()
        .blocks(tip_for_last_fetched_height(FIRST_HEIGHT + HEIGHTS - 1))
        .blocks_at(FIRST_HEIGHT..FIRST_HEIGHT + HEIGHTS, "361582")
        .fault(Endpoint::BlockHash, Fault::always_status(500))
        .start();
    let conn = common::memory_db();

    let started = Instant::now();
    let result = collect_statistics(
        mock.host(),
        mock.port(),
        Arc::clone(&conn),
        THREADS,
        Some(FIRST_HEIGHT),
    );
    let elapsed = started.elapsed();

    // Four waits of 500, 1000, 2000 and 4000 ms before the five attempts are
    // used up, 7.5 s in total. There is no point in waiting again after the
    // last one, which would add 8 s; the bound sits between the two with room
    // for a slow machine.
    assert!(
        elapsed < Duration::from_secs(14),
        "giving up took {:?}, the last attempt waited before failing",
        elapsed
    );

    match result {
        Err(MainError::REST(e)) => assert!(
            e.to_string().contains("http status: 500"),
            "expected the injected HTTP 500 to be reported, got: {}",
            e
        ),
        other => panic!("expected a REST error, got {:?}", other),
    }

    // Every thread gives up on its own block, but no thread starts working
    // through the remaining 18 blocks after the first failure.
    let requests = mock.requests(Endpoint::BlockHash);
    assert!(
        requests <= MAX_ATTEMPTS_PER_BLOCK * (THREADS + 1),
        "{} hash requests for {} blocks: the other threads did not stop promptly",
        requests,
        HEIGHTS
    );
    let mut conn = conn.lock().unwrap();
    assert_eq!(common::row_count(&mut conn, "block_stats"), 0);
}

#[test]
fn refuses_to_run_against_a_node_in_initial_block_download() {
    common::init_logger();
    let mock = MockCore::builder()
        .blocks(361588)
        .ibd(true, 0.42)
        .block_at(361582, "361582")
        .start();
    let conn = common::memory_db();

    let result = collect_statistics(mock.host(), mock.port(), Arc::clone(&conn), 1, None);

    assert!(
        matches!(result, Err(MainError::IBDNotDone)),
        "expected an IBD error, got {:?}",
        result
    );
    assert_eq!(
        mock.requests(Endpoint::BlockHash),
        0,
        "no block should be fetched from a node in IBD"
    );
}

#[test]
fn handles_a_node_with_fewer_blocks_than_the_reorg_margin() {
    common::init_logger();
    // A freshly started regtest node, or a node that hasn't synced past the
    // reorg margin yet. There is nothing to fetch, which is not an error.
    let mock = MockCore::builder().blocks(3).start();
    let conn = common::memory_db();

    collect_statistics(mock.host(), mock.port(), Arc::clone(&conn), 2, None)
        .expect("a node with fewer blocks than the reorg margin is not an error");

    assert_eq!(mock.requests(Endpoint::BlockHash), 0);
    let mut conn = conn.lock().unwrap();
    assert_eq!(common::row_count(&mut conn, "block_stats"), 0);
}

#[test]
fn skips_blocks_whose_stats_are_current_and_redoes_the_outdated_ones() {
    common::init_logger();
    const MARKER_NONCE: i32 = 424242;
    let mock = MockCore::builder()
        .blocks(tip_for_last_fetched_height(361582))
        .block_at(361581, "361582")
        .block_at(361582, "361582")
        .start();
    let conn = common::memory_db();

    {
        // One row written by an older version of the stats code and one that
        // is already up to date. Both carry a marker so we can tell whether
        // they were written again.
        let mut outdated = common::fixture_stats("361582", 361581, STATS_VERSION - 1);
        outdated.block.nonce = MARKER_NONCE;
        let mut current = common::fixture_stats("361582", 361582, STATS_VERSION);
        current.block.nonce = MARKER_NONCE;
        let mut conn = conn.lock().unwrap();
        db::insert_stats(&mut conn, &[outdated, current]).expect("pre-filling the database");
    }

    collect_statistics(mock.host(), mock.port(), Arc::clone(&conn), 1, Some(361581))
        .expect("the sync should succeed");

    assert_eq!(
        mock.requested_heights_sorted(),
        vec![361581],
        "only the block with outdated stats should be fetched again"
    );
    let mut conn = conn.lock().unwrap();
    assert_eq!(
        common::block_stats_column(&mut conn, "stats_version", 361581),
        Some(STATS_VERSION as i64)
    );
    assert_ne!(
        common::block_stats_column(&mut conn, "nonce", 361581),
        Some(MARKER_NONCE as i64),
        "the outdated row should have been overwritten"
    );
    assert_eq!(
        common::block_stats_column(&mut conn, "nonce", 361582),
        Some(MARKER_NONCE as i64),
        "the up-to-date row should have been left alone"
    );
}

#[test]
fn start_height_skips_the_blocks_below_it() {
    common::init_logger();
    let mock = MockCore::builder()
        .blocks(tip_for_last_fetched_height(361583))
        .blocks_at(361580..=361583, "361582")
        .start();
    let conn = common::memory_db();

    collect_statistics(mock.host(), mock.port(), Arc::clone(&conn), 2, Some(361582))
        .expect("the sync should succeed");

    assert_eq!(mock.requested_heights_sorted(), vec![361582, 361583]);
    let mut conn = conn.lock().unwrap();
    assert_eq!(common::row_count(&mut conn, "block_stats"), 2);
}

/// The last block that gets fetched is `blocks - REORG_SAFETY_MARGIN - 1`, so
/// the pipeline leaves the tip and `REORG_SAFETY_MARGIN` blocks below it
/// alone, one more than the name of the constant suggests. This test pins
/// that down; it is the behaviour the sync has always had.
#[test]
fn leaves_the_reorg_margin_and_one_more_block_alone() {
    common::init_logger();
    const TIP: u64 = 361588;
    let mock = MockCore::builder()
        .blocks(TIP)
        .blocks_at(361580..=361582, "361582")
        .start();
    let conn = common::memory_db();

    collect_statistics(mock.host(), mock.port(), Arc::clone(&conn), 2, Some(361580))
        .expect("the sync should succeed");

    let heights = mock.requested_heights_sorted();
    assert_eq!(heights, vec![361580, 361581]);
    assert_eq!(
        *heights.last().unwrap(),
        TIP - REORG_SAFETY_MARGIN - 1,
        "the highest fetched height is one below the reorg margin"
    );
}

/// The stats are inserted in batches of `DATABASE_BATCH_SIZE` (100), with
/// whatever is left over written as a final, smaller batch.
#[test]
fn writes_the_final_partial_batch() {
    common::init_logger();
    const FIRST_HEIGHT: u64 = 361500;
    const BLOCKS: u64 = 106; // a full batch of 100 plus a partial one
    const LAST_HEIGHT: u64 = FIRST_HEIGHT + BLOCKS - 1;

    let mock = MockCore::builder()
        .blocks(tip_for_last_fetched_height(LAST_HEIGHT))
        // The same (small) fixture at every height. The mock rewrites the
        // height in the JSON, so every block ends up as its own row.
        .blocks_at(FIRST_HEIGHT..=LAST_HEIGHT, "361582")
        .start();
    let conn = common::memory_db();

    collect_statistics(
        mock.host(),
        mock.port(),
        Arc::clone(&conn),
        4,
        Some(FIRST_HEIGHT),
    )
    .expect("the sync should succeed");

    let mut conn = conn.lock().unwrap();
    assert_eq!(common::row_count(&mut conn, "block_stats"), BLOCKS as i64);
    assert_eq!(
        db::get_db_block_height(&mut conn).unwrap(),
        Some(LAST_HEIGHT as i64)
    );
}

#[test]
fn fails_when_the_stats_of_a_block_cannot_be_generated() {
    common::init_logger();
    let mock = MockCore::builder()
        .blocks(tip_for_last_fetched_height(361581))
        .block_at(361580, "361582")
        // A block whose first transaction doesn't decode. Bitcoin Core would
        // never serve this, but a bug in our parsing or in rawtx-rs looks
        // exactly like it.
        .raw_block_at(361581, block_with_undecodable_transaction("361582"))
        .start();
    let conn = common::memory_db();

    let result = collect_statistics(mock.host(), mock.port(), Arc::clone(&conn), 1, Some(361580));

    assert!(
        matches!(result, Err(MainError::Stats(_))),
        "a block whose stats can't be generated has to fail the run, got {:?}",
        result
    );
}

/// A broken block has to end the run quickly. Fetching the rest of the chain
/// first would mean hours of work for a result that is thrown away.
#[test]
fn stops_fetching_once_the_stats_of_a_block_failed() {
    common::init_logger();
    const FIRST_HEIGHT: u64 = 361500;
    // The fetcher can be about a dozen blocks ahead before the failure
    // reaches it: the block channel buffers ten, plus the ones in flight.
    // Far more blocks than that, so the bound below is nowhere near that
    // floor on a slow machine.
    const BLOCKS: u64 = 200;

    let mock = MockCore::builder()
        .blocks(tip_for_last_fetched_height(FIRST_HEIGHT + BLOCKS - 1))
        .blocks_at(FIRST_HEIGHT..FIRST_HEIGHT + BLOCKS, "361582")
        // The first block of the run is the broken one.
        .raw_block_at(FIRST_HEIGHT, block_with_undecodable_transaction("361582"))
        .start();
    let conn = common::memory_db();

    let result = collect_statistics(
        mock.host(),
        mock.port(),
        Arc::clone(&conn),
        1,
        Some(FIRST_HEIGHT),
    );

    assert!(
        matches!(result, Err(MainError::Stats(_))),
        "expected a stats error, got {:?}",
        result
    );
    let fetched = mock.requested_heights().len() as u64;
    assert!(
        fetched < BLOCKS / 2,
        "{} of {} blocks were fetched after the first one already failed",
        fetched,
        BLOCKS
    );
}

/// A panic in the stats code has to end the run with an error. Without a
/// safety net it aborts the whole process: rayon has no panic handler
/// installed, so a panic on one of its workers is fatal.
#[test]
fn reports_a_block_whose_stats_panic_instead_of_aborting() {
    common::init_logger();
    let mock = MockCore::builder()
        .blocks(tip_for_last_fetched_height(361582))
        // A block without any transaction, not even a coinbase. Bitcoin Core
        // never serves one, but the stats code expects a coinbase and panics.
        .raw_block_at(361582, block_without_transactions("361582"))
        .start();
    let conn = common::memory_db();

    let result = collect_statistics(mock.host(), mock.port(), Arc::clone(&conn), 1, Some(361582));

    match result {
        Err(MainError::Stats(e)) => assert!(
            e.to_string().contains("coinbase"),
            "the error should carry the panic message, got: {}",
            e
        ),
        other => panic!("expected a stats error, got {:?}", other),
    }
}

/// Takes a block fixture and removes all of its transactions.
fn block_without_transactions(fixture: &str) -> String {
    let mut block: serde_json::Value =
        serde_json::from_str(&common::fixture(fixture)).expect("fixture json");
    block["tx"] = serde_json::Value::Array(Vec::new());
    serde_json::to_string(&block).expect("serialize block json")
}

/// Takes a block fixture and replaces the raw hex of its first transaction
/// with something that isn't a transaction.
fn block_with_undecodable_transaction(fixture: &str) -> String {
    let mut block: serde_json::Value =
        serde_json::from_str(&common::fixture(fixture)).expect("fixture json");
    block["tx"][0]["hex"] = serde_json::Value::String("00".to_string());
    serde_json::to_string(&block).expect("serialize block json")
}

/// The mock accepts the connection and closes it without answering, like a
/// node that goes away mid-request.
#[test]
fn reports_a_node_that_hangs_up() {
    common::init_logger();
    let mock = MockCore::builder()
        .blocks(361588)
        .fault(Endpoint::ChainInfo, Fault::drop_connection(usize::MAX))
        .start();
    let conn = common::memory_db();

    let result = collect_statistics(mock.host(), mock.port(), Arc::clone(&conn), 1, None);

    assert!(
        matches!(result, Err(MainError::REST(_))),
        "expected a REST error, got {:?}",
        result
    );
}

#[test]
fn survives_a_slow_node() {
    common::init_logger();
    let mock = MockCore::builder()
        .blocks(tip_for_last_fetched_height(361582))
        .block_at(361582, "361582")
        .fault(
            Endpoint::BlockHash,
            Fault::delay(Duration::from_millis(300), 1),
        )
        .start();
    let conn = common::memory_db();

    collect_statistics(mock.host(), mock.port(), Arc::clone(&conn), 1, Some(361582))
        .expect("a slow answer is not a failure");

    assert_eq!(
        mock.requests(Endpoint::BlockHash),
        1,
        "a slow answer must be waited for, not retried"
    );
    let mut conn = conn.lock().unwrap();
    assert_eq!(common::row_count(&mut conn, "block_stats"), 1);
}
