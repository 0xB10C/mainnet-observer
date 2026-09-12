//! Instrumentation for the three stage block processing pipeline.
//!
//! Each stage records how long it spent doing work and how long it spent
//! blocked on the channel feeding the next stage. The stage that is never
//! blocked is the one holding up the pipeline.
//!
//! All durations are summed across the threads of a stage, so they are compared
//! against that stage's capacity: its thread count times the wall clock.

use log::info;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Adds `duration` to a counter.
pub fn record(counter: &AtomicU64, duration: Duration) {
    counter.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
}

#[derive(Default, Debug)]
pub struct Timings {
    // get-blocks stage, summed over all fetch threads
    /// Waiting for the block hash of a height.
    pub hash_request: AtomicU64,
    /// Between asking for a block and Bitcoin Core starting to answer.
    pub block_first_byte: AtomicU64,
    /// Receiving the block JSON, which is also Bitcoin Core serializing it.
    pub block_body: AtomicU64,
    /// Deserializing the block JSON.
    pub block_parse: AtomicU64,
    /// Waiting for the calc-stats stage to take a block off our hands.
    pub block_send_blocked: AtomicU64,
    pub blocks: AtomicU64,
    pub block_bytes: AtomicU64,
    pub get_blocks_wall: AtomicU64,

    // calc-stats stage
    /// The dispatch thread waiting for a block to arrive.
    pub block_recv_blocked: AtomicU64,
    /// Computing the stats, summed over the rayon workers.
    pub stats_compute: AtomicU64,
    /// Waiting for the batch-insert stage to take the stats.
    pub stats_send_blocked: AtomicU64,
    pub calc_stats_wall: AtomicU64,

    // batch-insert stage
    /// Waiting for stats to arrive.
    pub stats_recv_blocked: AtomicU64,
    /// Inside `db::insert_stats`.
    pub db_insert: AtomicU64,
    pub batches: AtomicU64,
    pub batch_insert_wall: AtomicU64,
}

fn secs(counter: &AtomicU64) -> f64 {
    counter.load(Ordering::Relaxed) as f64 / 1_000_000_000.0
}

fn count(counter: &AtomicU64) -> u64 {
    counter.load(Ordering::Relaxed)
}

/// Formats a number of seconds, keeping it readable from milliseconds up to
/// hours.
fn duration(s: f64) -> String {
    if s >= 120.0 {
        format!("{:.0}s ({:.1}m)", s, s / 60.0)
    } else if s >= 1.0 {
        format!("{:.2}s", s)
    } else {
        format!("{:.0}ms", s * 1000.0)
    }
}

impl Timings {
    /// Logs what each stage spent its time on.
    ///
    /// `wall` is how long the whole pipeline took. `fetch_threads` and
    /// `stats_threads` are the thread counts of the get-blocks and calc-stats
    /// stages, used to work out how much of their capacity they used.
    pub fn report(&self, wall: Duration, fetch_threads: usize, stats_threads: usize) {
        let wall = wall.as_secs_f64();
        let blocks = count(&self.blocks);
        if blocks == 0 {
            info!("timings: no blocks were fetched, nothing to report");
            return;
        }

        // Every recorded duration is summed across the threads of its stage, so
        // it's shown as a share of that stage's capacity: how long the stage
        // ran, times its thread count.
        let line = |label: &str, value: f64, capacity: f64, per_block: bool| {
            let per_block = if per_block {
                format!("{:>8.0}ms/block", value * 1000.0 / blocks as f64)
            } else {
                String::new()
            };
            info!(
                "  {:<24}{:>16}{:>7.1}%{}",
                label,
                duration(value),
                value / capacity * 100.0,
                per_block
            );
        };

        info!("=== pipeline timings, wall {} ===", duration(wall));
        info!(
            "{} blocks, {:.1} GB of block JSON, {:.1} kB per block",
            blocks,
            count(&self.block_bytes) as f64 / 1e9,
            count(&self.block_bytes) as f64 / blocks as f64 / 1e3,
        );

        // get-blocks. Measured against its own runtime rather than the wall,
        // because it finishes before the other stages have drained.
        let fetch_wall = secs(&self.get_blocks_wall);
        let fetch_capacity = fetch_wall * fetch_threads as f64;
        info!(
            "get-blocks: {} threads, ran {} ({:.0}% of wall), capacity {}",
            fetch_threads,
            duration(fetch_wall),
            fetch_wall / wall * 100.0,
            duration(fetch_capacity),
        );
        let hash = secs(&self.hash_request);
        let first_byte = secs(&self.block_first_byte);
        let body = secs(&self.block_body);
        let parse = secs(&self.block_parse);
        let send_blocked = secs(&self.block_send_blocked);
        line("hash request", hash, fetch_capacity, true);
        line("block, to first byte", first_byte, fetch_capacity, true);
        line("block, receiving body", body, fetch_capacity, true);
        line("block, deserializing", parse, fetch_capacity, true);
        line("blocked on send", send_blocked, fetch_capacity, true);
        line(
            "unaccounted",
            fetch_capacity - hash - first_byte - body - parse - send_blocked,
            fetch_capacity,
            false,
        );

        // calc-stats. The dispatch thread is measured against the wall, the
        // stats themselves against the rayon pool's capacity. The pool keeps
        // working after the dispatch thread has seen the last block, so its
        // capacity is the whole wall.
        info!(
            "calc-stats: 1 dispatch thread, {} rayon threads, dispatch ran {}",
            stats_threads,
            duration(secs(&self.calc_stats_wall)),
        );
        line(
            "blocked on recv",
            secs(&self.block_recv_blocked),
            wall,
            false,
        );
        line(
            "computing stats",
            secs(&self.stats_compute),
            wall * stats_threads as f64,
            true,
        );
        line(
            "blocked on send",
            secs(&self.stats_send_blocked),
            wall * stats_threads as f64,
            true,
        );

        // batch-insert, a single thread that runs until the last stats arrive.
        let insert_wall = secs(&self.batch_insert_wall);
        info!(
            "batch-insert: 1 thread, ran {} ({:.0}% of wall), {} batches of up to {} blocks",
            duration(insert_wall),
            insert_wall / wall * 100.0,
            count(&self.batches),
            crate::DATABASE_BATCH_SIZE,
        );
        let recv_blocked = secs(&self.stats_recv_blocked);
        let insert = secs(&self.db_insert);
        line("blocked on recv", recv_blocked, insert_wall, false);
        line("inserting into sqlite", insert, insert_wall, false);
        line(
            "unaccounted",
            insert_wall - recv_blocked - insert,
            insert_wall,
            false,
        );
    }
}
