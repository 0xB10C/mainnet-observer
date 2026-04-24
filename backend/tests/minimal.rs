use corepc_node as bitcoind;
use diesel::SqliteConnection;
use log::{error, info};
use mainnet_observer_backend::{collect_statistics, db, write_csv_files, REORG_SAFETY_MARGIN};
use rand::distr::{Alphanumeric, SampleString};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

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

fn setup_db() -> Arc<Mutex<SqliteConnection>> {
    let conn = match db::open_db_and_run_migrations(":memory:") {
        Ok(conn) => conn,
        Err(e) => {
            panic!("Could not open database: {}", e);
        }
    };
    Arc::new(Mutex::new(conn))
}

struct MockRestServer {
    host: String,
    port: u16,
    keep_running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl MockRestServer {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock rest listener");
        listener
            .set_nonblocking(true)
            .expect("set listener nonblocking");
        let port = listener.local_addr().expect("read local addr").port();
        let keep_running = Arc::new(AtomicBool::new(true));
        let keep_running_thread = Arc::clone(&keep_running);

        let handle = thread::spawn(move || {
            while keep_running_thread.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _addr)) => handle_connection(&mut stream),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_e) => break,
                }
            }
        });

        Self {
            host: "127.0.0.1".to_string(),
            port,
            keep_running,
            handle: Some(handle),
        }
    }
}

impl Drop for MockRestServer {
    fn drop(&mut self) {
        self.keep_running.store(false, Ordering::Relaxed);
        let _ = TcpStream::connect((self.host.as_str(), self.port));
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn handle_connection(stream: &mut TcpStream) {
    let mut buffer = [0u8; 4096];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(n) => n,
        Err(_) => return,
    };
    if bytes_read == 0 {
        return;
    }
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    let (status, body, content_type) = if path == "/rest/chaininfo.json" {
        (
            "200 OK",
            r#"{"initialblockdownload":false,"verificationprogress":1.0,"blocks":8}"#,
            "application/json",
        )
    } else if path.starts_with("/rest/blockhashbyheight/") {
        (
            "500 Internal Server Error",
            r#"{"error":"forced failure"}"#,
            "application/json",
        )
    } else {
        (
            "404 Not Found",
            r#"{"error":"not found"}"#,
            "application/json",
        )
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

#[test]
fn test_integration_minimal() {
    const BLOCKS_TO_MINE: i64 = 100;
    init_logger();

    let conn = setup_db();
    let node = setup_node();

    setup_chain(&node, BLOCKS_TO_MINE as usize);

    let (rest_host, rest_port) = rest_host_and_port(&node);
    if let Err(e) = collect_statistics(
        &rest_host,
        rest_port,
        Arc::clone(&conn),
        10, // Bitcoin Core v29 has 16, in the test use just use 10 of them.
        None,
        1000,
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

    let mut dir = env::temp_dir();
    dir.push(format!(
        "mainnet-observer-integration-tests-{}",
        Alphanumeric.sample_string(&mut rand::rng(), 16)
    ));
    fs::create_dir_all(&dir).unwrap();
    info!("Using temp directory {} for csv files", dir.display());

    let mut failed = false;
    if let Err(e) = write_csv_files(&dir.to_string_lossy(), Arc::clone(&conn)) {
        failed = true;
        error!("Failed to write csv files: {:?}", e);
    }

    // cleanup
    fs::remove_dir_all(&dir).unwrap();
    assert!(!failed);
}

#[test]
fn test_collect_statistics_fails_on_block_fetch_retry_exhaustion() {
    init_logger();
    let conn = setup_db();
    let mock = MockRestServer::start();

    let result = collect_statistics(&mock.host, mock.port, Arc::clone(&conn), 2, None, 1000);

    match result {
        Err(mainnet_observer_backend::MainError::REST(e)) => {
            assert!(
                e.to_string().contains("HTTP error: 500"),
                "expected HTTP 500 from mock REST server, got: {e}"
            );
        }
        other => panic!("expected HTTP REST error after retry exhaustion, got: {other:?}"),
    }
}
