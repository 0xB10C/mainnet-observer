//! Tests for the REST client and for the serde structs it parses Bitcoin
//! Core's block JSON into.

mod common;

use common::{Endpoint, Fault, MockCore};
use mainnet_observer_backend::rest::{
    Block, Input, RestClient, RestError, ScriptPubkeyType, Transaction as RestTransaction,
};
use serde::Deserialize;

// --- serde_hex -------------------------------------------------------------

#[derive(Deserialize)]
struct Hex {
    #[serde(with = "mainnet_observer_backend::rest::serde_hex")]
    value: Vec<u8>,
}

#[derive(Deserialize)]
struct OptHex {
    #[serde(default, with = "mainnet_observer_backend::rest::serde_hex::opt")]
    value: Option<Vec<u8>>,
}

#[test]
fn serde_hex_decodes_a_hex_string() {
    let hex: Hex = serde_json::from_str(r#"{"value":"deadbeef"}"#).expect("valid hex");
    assert_eq!(hex.value, vec![0xde, 0xad, 0xbe, 0xef]);

    let empty: Hex = serde_json::from_str(r#"{"value":""}"#).expect("empty hex");
    assert!(empty.value.is_empty());
}

#[test]
fn serde_hex_rejects_anything_that_isnt_hex() {
    for json in [
        r#"{"value":"abc"}"#,        // odd number of characters
        r#"{"value":"zz"}"#,         // not hex digits
        r#"{"value":"de ad"}"#,      // not hex digits either
        r#"{"value":1234}"#,         // not a string
        r#"{"value":null}"#,         // not a string
        r#"{"value":["deadbeef"]}"#, // not a string
        r#"{}"#,                     // no value at all
    ] {
        assert!(
            serde_json::from_str::<Hex>(json).is_err(),
            "{} should not deserialize",
            json
        );
    }
}

#[test]
fn serde_hex_opt_handles_a_missing_value() {
    let some: OptHex = serde_json::from_str(r#"{"value":"0011"}"#).expect("valid hex");
    assert_eq!(some.value, Some(vec![0x00, 0x11]));

    let null: OptHex = serde_json::from_str(r#"{"value":null}"#).expect("null");
    assert_eq!(null.value, None);

    let missing: OptHex = serde_json::from_str(r#"{}"#).expect("missing field");
    assert_eq!(missing.value, None);

    assert!(
        serde_json::from_str::<OptHex>(r#"{"value":"xyz"}"#).is_err(),
        "a present but broken hex string is an error"
    );
}

// --- ScriptPubkeyType ------------------------------------------------------

fn script_type(name: &str) -> ScriptPubkeyType {
    serde_json::from_str(&format!("\"{}\"", name)).expect("script type")
}

#[test]
fn script_pubkey_type_knows_every_type_core_emits() {
    // The list Bitcoin Core's GetTxnOutputType() can return, as of v29.
    assert_eq!(script_type("nonstandard"), ScriptPubkeyType::Nonstandard);
    assert_eq!(script_type("pubkey"), ScriptPubkeyType::Pubkey);
    assert_eq!(script_type("pubkeyhash"), ScriptPubkeyType::PubkeyHash);
    assert_eq!(script_type("scripthash"), ScriptPubkeyType::ScriptHash);
    assert_eq!(script_type("multisig"), ScriptPubkeyType::MultiSig);
    assert_eq!(script_type("nulldata"), ScriptPubkeyType::NullData);
    assert_eq!(
        script_type("witness_v0_keyhash"),
        ScriptPubkeyType::Witness_v0_KeyHash
    );
    assert_eq!(
        script_type("witness_v0_scripthash"),
        ScriptPubkeyType::Witness_v0_ScriptHash
    );
    assert_eq!(
        script_type("witness_v1_taproot"),
        ScriptPubkeyType::Witness_v1_Taproot
    );
    assert_eq!(
        script_type("witness_unknown"),
        ScriptPubkeyType::Witness_Unknown
    );
    assert_eq!(script_type("anchor"), ScriptPubkeyType::Anchor);
}

/// A script type a future Bitcoin Core might emit must not fail the whole
/// block: it becomes `Unknown`.
#[test]
fn script_pubkey_type_falls_back_to_unknown() {
    assert_eq!(
        script_type("witness_v2_something"),
        ScriptPubkeyType::Unknown
    );
    assert_eq!(script_type(""), ScriptPubkeyType::Unknown);
    assert_eq!(script_type("ANCHOR"), ScriptPubkeyType::Unknown);

    let block: Block = serde_json::from_str(&block_json_with_script_type("witness_v2_something"))
        .expect("a block with an unknown script type should still parse");
    assert_eq!(
        block.txdata[0].output[0].script_pub_key.type_,
        ScriptPubkeyType::Unknown
    );
}

// --- Input, Transaction, Block ---------------------------------------------

#[test]
fn input_of_a_coinbase_transaction() {
    let input: Input = serde_json::from_str(
        r#"{"coinbase":"03ae84050101","sequence":4294967295,"txinwitness":["00"]}"#,
    )
    .expect("coinbase input");

    assert_eq!(
        input.coinbase,
        Some(vec![0x03, 0xae, 0x84, 0x05, 0x01, 0x01])
    );
    assert_eq!(input.txid, None);
    assert_eq!(input.vout, None);
    assert!(input.prevout.is_none());
    assert_eq!(input.sequence, bitcoin::Sequence::MAX);
}

#[test]
fn input_spending_a_previous_output() {
    let input: Input = serde_json::from_str(
        r#"{
            "txid":"4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
            "vout":2,
            "sequence":4294967293,
            "prevout":{
                "generated":false,
                "height":700000,
                "value":1.5,
                "scriptPubKey":{
                    "asm":"",
                    "hex":"0014751e76e8199196d454941c45d1b3a323f1433bd6",
                    "type":"witness_v0_keyhash"
                }
            }
        }"#,
    )
    .expect("input with a prevout");

    assert_eq!(input.coinbase, None);
    assert_eq!(
        input.txid.map(|t| t.to_string()),
        Some("4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b".to_string())
    );
    assert_eq!(input.vout, Some(2));
    let prevout = input.prevout.expect("prevout");
    assert_eq!(prevout.height, 700000);
    assert_eq!(prevout.value.to_sat(), 150_000_000);
    assert_eq!(
        prevout.script_pub_key.type_,
        ScriptPubkeyType::Witness_v0_KeyHash
    );
}

#[test]
fn lock_time_is_only_enabled_when_an_input_says_so() {
    let disabled: RestTransaction =
        serde_json::from_str(&transaction_json(&[4294967295, 4294967295])).expect("transaction");
    assert!(!disabled.is_lock_time_enabled());

    let enabled: RestTransaction =
        serde_json::from_str(&transaction_json(&[4294967295, 4294967294])).expect("transaction");
    assert!(enabled.is_lock_time_enabled());

    let single: RestTransaction =
        serde_json::from_str(&transaction_json(&[0])).expect("transaction");
    assert!(single.is_lock_time_enabled());
}

#[test]
fn block_ignores_fields_it_doesnt_need() {
    let block: Block = serde_json::from_str(&block_json_with_script_type("pubkeyhash"))
        .expect("block with extra fields");

    assert_eq!(
        block.hash.to_string(),
        "0000000000000000000000000000000000000000000000000000000000000abc"
    );
    assert_eq!(block.height, 800000);
    assert_eq!(block.size, 285);
    assert_eq!(block.stripped_size, 249);
    assert_eq!(block.weight.to_wu(), 1032);
    assert_eq!(block.time, 1690000000);
    assert_eq!(block.nonce, 42);
    assert_eq!(block.bits, "170594db");
    assert_eq!(block.txdata.len(), 1);
    assert_eq!(block.txdata[0].size, 285);
    assert_eq!(block.txdata[0].vsize, 258);
    assert_eq!(block.txdata[0].fee, None);
}

#[test]
fn block_fixtures_parse() {
    let block = common::fixture_block("361582");
    assert_eq!(block.height, 361582);
    assert_eq!(
        block.hash.to_string(),
        "0000000000000000070a2bb3b92c20d5c2c971e6e1a7abe55cdbbe6a2dd9a5ad"
    );
    assert!(!block.txdata.is_empty());
    assert_eq!(block.txdata[0].input.len(), 1);
    assert!(
        block.txdata[0].input[0].coinbase.is_some(),
        "the first transaction of a block is the coinbase"
    );
}

// --- RestClient ------------------------------------------------------------

#[test]
fn chain_info_of_a_healthy_node() {
    let mock = MockCore::builder().blocks(912345).start();
    let client = RestClient::new(mock.host(), mock.port(), 1);

    let info = client.chain_info().expect("chain info");

    assert_eq!(info.blocks, 912345);
    assert!(!info.initialblockdownload);
    assert_eq!(info.verificationprogress, 1.0);
}

#[test]
fn chain_info_reports_an_http_error() {
    let mock = MockCore::builder()
        .blocks(1)
        .fault(Endpoint::ChainInfo, Fault::always_status(500))
        .start();
    let client = RestClient::new(mock.host(), mock.port(), 1);

    let error = expect_error(client.chain_info());

    assert!(
        error.to_string().contains("500"),
        "the status should be in the error: {}",
        error
    );
}

#[test]
fn chain_info_reports_a_body_that_isnt_json() {
    let mock = MockCore::builder()
        .blocks(1)
        .fault(Endpoint::ChainInfo, Fault::body("<html>not json</html>"))
        .start();
    let client = RestClient::new(mock.host(), mock.port(), 1);

    assert!(
        client.chain_info().is_err(),
        "a body that isn't chain info is an error"
    );
}

#[test]
fn block_at_height_reports_an_unknown_height() {
    // The mock has no block at any height, so the hash lookup 404s.
    let mock = MockCore::builder().blocks(100).start();
    let client = RestClient::new(mock.host(), mock.port(), 1);

    let error = expect_error(client.block_at_height(50));

    assert!(
        error.to_string().contains("404"),
        "the status should be in the error: {}",
        error
    );
    assert_eq!(mock.requests(Endpoint::BlockHash), 1);
    assert_eq!(
        mock.requests(Endpoint::Block),
        0,
        "without a hash there is nothing to ask for"
    );
}

#[test]
fn block_at_height_reports_an_http_error_on_the_block() {
    let mock = MockCore::builder()
        .blocks(100)
        .block_at(50, "361582")
        .fault(Endpoint::Block, Fault::always_status(503))
        .start();
    let client = RestClient::new(mock.host(), mock.port(), 1);

    let error = expect_error(client.block_at_height(50));

    assert!(
        error.to_string().contains("503"),
        "the status should be in the error: {}",
        error
    );
}

#[test]
fn block_at_height_reports_a_block_that_isnt_json() {
    let mock = MockCore::builder()
        .blocks(100)
        .block_at(50, "361582")
        .fault(Endpoint::Block, Fault::body("}{ not json"))
        .start();
    let client = RestClient::new(mock.host(), mock.port(), 1);

    assert!(
        client.block_at_height(50).is_err(),
        "a block body that isn't JSON is an error"
    );
}

#[test]
fn block_at_height_returns_the_block() {
    let mock = MockCore::builder()
        .blocks(100)
        .block_at(50, "361582")
        .start();
    let client = RestClient::new(mock.host(), mock.port(), 1);

    let block = client.block_at_height(50).expect("block");

    assert_eq!(block.height, 50, "the mock serves the fixture at height 50");
    assert_eq!(block.hash.to_string(), common::hash_at_height(50));
}

/// The client caps the block body at 50 MB. A node that answers with more
/// than that has to end in an error rather than in an endless read.
#[test]
fn block_at_height_gives_up_on_a_body_over_the_size_limit() {
    const LIMIT: usize = 50 * 1024 * 1024;
    let mock = MockCore::builder()
        .blocks(100)
        .block_at(50, "361582")
        .fault(Endpoint::Block, Fault::oversized(LIMIT + 1024 * 1024))
        .start();
    let client = RestClient::new(mock.host(), mock.port(), 1);

    let error = expect_error(client.block_at_height(50));

    // ureq parses the body while reading it, so the limit surfaces as a json
    // error that carries the body limit message.
    let message = error.to_string();
    assert!(
        matches!(&error, RestError::Ureq(e) if matches!(**e, ureq::Error::Json(_)))
            && message.contains("larger than request limit"),
        "expected the body size limit error, got: {}",
        message
    );
}

// --- helpers ---------------------------------------------------------------

/// Unwraps the error of a REST call. Neither `ChainInfo` nor `Block` implement
/// `Debug`, so `expect_err` is not available.
fn expect_error<T>(result: Result<T, RestError>) -> RestError {
    match result {
        Ok(_) => panic!("expected the call to fail"),
        Err(e) => e,
    }
}

/// A block with one transaction whose single output has the given script
/// type, along with a good number of fields the `Block` struct doesn't parse.
fn block_json_with_script_type(script_type: &str) -> String {
    format!(
        r#"{{
            "hash":"0000000000000000000000000000000000000000000000000000000000000abc",
            "confirmations":17,
            "height":800000,
            "version":536870912,
            "versionHex":"20000000",
            "merkleroot":"4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
            "time":1690000000,
            "mediantime":1689999000,
            "nonce":42,
            "bits":"170594db",
            "target":"0000000000000000000594db0000000000000000000000000000000000000000",
            "difficulty":52278304845.59,
            "chainwork":"00000000000000000000000000000000000000004f0d1f2b3c4d5e6f70819200",
            "nTx":1,
            "previousblockhash":"0000000000000000000000000000000000000000000000000000000000000abb",
            "strippedsize":249,
            "size":285,
            "weight":1032,
            "tx":[{}],
            "somethingfromthefuture":{{"nested":[1,2,3]}}
        }}"#,
        transaction_json_with_script_type(&[4294967295], script_type)
    )
}

fn transaction_json(sequences: &[u32]) -> String {
    transaction_json_with_script_type(sequences, "pubkeyhash")
}

fn transaction_json_with_script_type(sequences: &[u32], script_type: &str) -> String {
    let inputs: Vec<String> = sequences
        .iter()
        .map(|sequence| {
            format!(
                r#"{{"coinbase":"03ae840500","sequence":{},"txinwitness":["0000000000000000000000000000000000000000000000000000000000000000"]}}"#,
                sequence
            )
        })
        .collect();
    format!(
        r#"{{
            "txid":"4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
            "hash":"4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
            "version":2,
            "size":285,
            "vsize":258,
            "weight":1032,
            "locktime":0,
            "vin":[{}],
            "vout":[{{
                "value":6.25005000,
                "n":0,
                "scriptPubKey":{{
                    "asm":"OP_DUP OP_HASH160",
                    "desc":"addr(bc1q)#checksum",
                    "hex":"76a914751e76e8199196d454941c45d1b3a323f1433bd688ac",
                    "address":"1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH",
                    "type":"{}"
                }}
            }}],
            "hex":"02000000010000"
        }}"#,
        inputs.join(","),
        script_type
    )
}
