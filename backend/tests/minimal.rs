//! The slow integration test: it runs the whole sync against a real regtest
//! bitcoind.

mod common;

use common::TempDir;
use corepc_node as bitcoind;
use log::{error, info};
use mainnet_observer_backend::{collect_statistics, db, write_csv_files, REORG_SAFETY_MARGIN};
use std::sync::Arc;

fn init_logger() {
    let _ = env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();
}

fn setup_node() -> corepc_node::Node {
    let mut conf = bitcoind::Conf::default();
    conf.args = vec!["-regtest", "-fallbackfee=0.0001", "-rest"];

    info!("env BITCOIND_EXE={:?}", std::env::var("BITCOIND_EXE"));
    info!("exe_path={:?}", corepc_node::exe_path());

    if let Ok(exe_path) = corepc_node::exe_path() {
        info!("Using bitcoind at '{}'", exe_path);
        return corepc_node::Node::with_conf(exe_path, &conf).unwrap();
    }

    info!("Trying to download a bitcoind..");
    corepc_node::Node::from_downloaded_with_conf(&conf).unwrap()
}

fn setup_chain(node: &corepc_node::Node, blocks: usize) {
    let address = node
        .client
        .new_address()
        .expect("failed to get new address");
    let json = node
        .client
        .generate_to_address(blocks, &address)
        .expect("generatetoaddress");
    json.into_model().unwrap();
    assert_eq!(
        blocks,
        node.client.get_blockchain_info().unwrap().blocks as usize
    );
}

fn rest_host_and_port(node: &corepc_node::Node) -> (String, u16) {
    let rpc_url = node.rpc_url();
    let rpc_host_port = rpc_url.replace("http://", "");
    // TODO: this only works for IPv4..
    let rest_host = rpc_host_port
        .split(":")
        .next()
        .expect("should be able to extract a rpc_host from the rpc_url")
        .to_string();
    let rest_port = rpc_host_port
        .split(":")
        .last()
        .expect("should be able to extract a rpc_port from the rpc_url")
        .parse::<u16>()
        .expect("port part should be an u16");
    (rest_host, rest_port)
}

#[test]
fn test_integration_minimal() {
    const BLOCKS_TO_MINE: i64 = 100;
    init_logger();

    let conn = common::memory_db();
    let node = setup_node();

    setup_chain(&node, BLOCKS_TO_MINE as usize);

    let (rest_host, rest_port) = rest_host_and_port(&node);
    if let Err(e) = collect_statistics(
        &rest_host,
        rest_port,
        Arc::clone(&conn),
        10, // Bitcoin Core v29 has 16, in the test use just use 10 of them.
        None,
    ) {
        panic!("Failed to collect statistics: {:?}", e);
    }

    {
        let mut conn = conn.lock().unwrap();
        // The regtest network starts out with 0 blocks. When we mine 100 blocks,
        // we end up at height 99.
        const OFFSET: i64 = 1;
        assert_eq!(
            BLOCKS_TO_MINE - OFFSET - REORG_SAFETY_MARGIN as i64,
            db::get_db_block_height(&mut conn).unwrap().unwrap()
        );
    }

    let dir = TempDir::new("integration-tests");
    info!(
        "Using temp directory {} for csv files",
        dir.path().display()
    );

    let mut failed = false;
    if let Err(e) = write_csv_files(&dir.str_path(), Arc::clone(&conn)) {
        failed = true;
        error!("Failed to write csv files: {:?}", e);
    }

    assert!(!failed);
}
