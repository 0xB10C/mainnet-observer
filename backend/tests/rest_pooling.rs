//! Checks that the REST client keeps connections alive between requests.
//!
//! A full sync makes two REST requests per block, so opening a fresh
//! connection for each of them is a lot of wasted handshakes.
use mainnet_observer_backend::rest::RestClient;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

/// A minimal HTTP/1.1 server that keeps the connection open and answers the
/// two endpoints `block_at_height()` uses.
fn serve(mut stream: TcpStream, block_json: Arc<String>) {
    let peer = stream.try_clone().unwrap();
    let mut reader = BufReader::new(peer);
    loop {
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
            return; // client hung up
        }
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header).unwrap_or(0) == 0 {
                return;
            }
            if header == "\r\n" || header == "\n" {
                break;
            }
        }

        let body: &str = if request_line.contains("/rest/blockhashbyheight/") {
            "00000000000000000001b2a3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f7\n"
        } else {
            block_json.as_str()
        };

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        if stream.write_all(response.as_bytes()).is_err() {
            return;
        }
        let _ = stream.flush();
    }
}

#[test]
fn keeps_connections_alive_between_requests() {
    let block_json = Arc::new(
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/361582.json"))
            .expect("block fixture"),
    );

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let connections = Arc::new(AtomicUsize::new(0));

    let accepted = Arc::clone(&connections);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            accepted.fetch_add(1, Ordering::SeqCst);
            let json = Arc::clone(&block_json);
            thread::spawn(move || serve(stream, json));
        }
    });

    const THREADS: usize = 8;
    const BLOCKS_PER_THREAD: u64 = 10;

    let client = Arc::new(RestClient::new("127.0.0.1", port, THREADS));
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
    let opened = connections.load(Ordering::SeqCst);
    assert!(
        opened <= THREADS,
        "{} requests opened {} connections, expected at most {}",
        THREADS as u64 * BLOCKS_PER_THREAD * 2,
        opened,
        THREADS
    );
}
