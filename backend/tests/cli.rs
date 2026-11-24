//! Tests for the command line binary.

mod common;

use common::TempDir;
use diesel::{sql_query, Connection, RunQueryDsl, SqliteConnection};
use std::process::{Command, Output};

const BINARY: &str = env!("CARGO_BIN_EXE_mainnet-observer-backend");

fn run(args: &[&str]) -> Output {
    Command::new(BINARY)
        .args(args)
        .output()
        .expect("running the backend binary")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// A port nothing listens on.
fn closed_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.local_addr().expect("local addr").port()
}

#[test]
fn creates_and_migrates_the_database() {
    let dir = TempDir::new("cli-db");
    let db_path = dir.child("db.sqlite");

    let output = run(&[
        "--no-stats",
        "--no-csv",
        "--database-path",
        &db_path.to_string_lossy(),
    ]);

    assert!(
        output.status.success(),
        "the binary should exit cleanly: {}",
        stderr(&output)
    );
    assert!(
        db_path.exists(),
        "the database file should have been created"
    );

    // The migrations ran if the tables they create are there.
    let mut conn =
        SqliteConnection::establish(&db_path.to_string_lossy()).expect("open the new database");
    for table in ["block_stats", "tx_stats", "feerate_stats"] {
        sql_query(format!("SELECT count(*) FROM {}", table))
            .execute(&mut conn)
            .unwrap_or_else(|e| panic!("table {} should exist: {}", table, e));
    }
}

#[test]
fn exits_with_an_error_when_the_node_cannot_be_reached() {
    let dir = TempDir::new("cli-unreachable");
    let db_path = dir.child("db.sqlite");

    let output = run(&[
        "--no-csv",
        "--rest-host",
        "127.0.0.1",
        "--rest-port",
        &closed_port().to_string(),
        "--num-threads",
        "1",
        "--database-path",
        &db_path.to_string_lossy(),
    ]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "an unreachable node should exit with 1: {}",
        stderr(&output)
    );
    let stderr = stderr(&output);
    assert!(
        stderr.contains("Could not collect statistics"),
        "the error should say what failed: {}",
        stderr
    );
}

#[test]
fn writes_header_only_csv_files_for_an_empty_database() {
    let dir = TempDir::new("cli-csv");
    let db_path = dir.child("db.sqlite");
    let csv_path = dir.child("csv");

    let output = run(&[
        "--no-stats",
        "--database-path",
        &db_path.to_string_lossy(),
        "--csv-path",
        &csv_path.to_string_lossy(),
    ]);

    assert!(
        output.status.success(),
        "the binary should exit cleanly: {}",
        stderr(&output)
    );
    let dates = std::fs::read_to_string(csv_path.join("date.csv")).expect("date.csv");
    assert_eq!(
        dates, "date\n",
        "an empty database should give a csv file with nothing but the header"
    );
    let files = std::fs::read_dir(&csv_path).expect("read csv dir").count();
    assert!(files > 1, "only {} csv file(s) were written", files);
}

#[test]
fn prints_the_help() {
    let output = run(&["--help"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for flag in [
        "--rest-host",
        "--rest-port",
        "--database-path",
        "--csv-path",
        "--no-csv",
        "--no-stats",
        "--num-threads",
        "--start-height",
    ] {
        assert!(stdout.contains(flag), "{} should be documented", flag);
    }
}
