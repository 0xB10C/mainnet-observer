//! Checks that the REST client keeps connections alive between requests.
//!
//! A full sync makes two REST requests per block, so opening a fresh
//! connection for each of them is a lot of wasted handshakes.

mod common;

use common::MockCore;
use mainnet_observer_backend::rest::RestClient;
use std::sync::Arc;
use std::thread;

#[test]
fn keeps_connections_alive_between_requests() {
    const THREADS: usize = 8;
    const BLOCKS_PER_THREAD: u64 = 10;
    const BLOCKS: u64 = THREADS as u64 * BLOCKS_PER_THREAD;

    let mock = MockCore::builder().blocks_at(0..BLOCKS, "361582").start();

    let client = Arc::new(RestClient::new(mock.host(), mock.port(), THREADS));
    let mut handles = Vec::new();
    for t in 0..THREADS {
        let client = Arc::clone(&client);
        handles.push(thread::spawn(move || {
            for i in 0..BLOCKS_PER_THREAD {
                client
                    .block_at_height(t as u64 * BLOCKS_PER_THREAD + i)
                    .expect("block");
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }

    // Two requests per block, but no more than one connection per thread.
    let opened = mock.connections();
    assert!(
        opened <= THREADS,
        "{} requests opened {} connections, expected at most {}",
        BLOCKS * 2,
        opened,
        THREADS
    );
}
