use bitcoin::{self, absolute::LockTime, block, Amount, BlockHash, ScriptBuf, Sequence, Weight};
use serde::Deserialize;
use std::time::{Duration, Instant};
use std::{error, fmt};

/// Upper bound on the size of a block in its verbose JSON representation. The
/// JSON is several times the size of the block itself, so ureq's 10 MB default
/// isn't enough.
const MAX_BLOCK_JSON_SIZE: u64 = 50 * 1024 * 1024;

/// Where the time went while fetching a single block.
#[derive(Default, Debug)]
pub struct FetchTiming {
    /// Looking up the block hash for a height.
    pub hash_request: Duration,
    /// Between asking for the block and Bitcoin Core starting to answer.
    pub first_byte: Duration,
    /// Receiving the block JSON, which is also Bitcoin Core serializing it.
    pub body: Duration,
    /// Deserializing the block JSON.
    pub parse: Duration,
    /// Size of the block JSON in bytes.
    pub bytes: usize,
}

pub struct RestClient {
    host: String,
    port: u16,
    agent: ureq::Agent,
}

#[derive(Deserialize)]
pub struct ChainInfo {
    pub initialblockdownload: bool,
    pub verificationprogress: f32,
    pub blocks: u64,
}

/// Decodes a hex string into `T` without allocating an intermediate `String`.
pub mod serde_hex {
    use bitcoin::hex::FromHex;
    use serde::de::{Error, Visitor};
    use serde::Deserializer;
    use std::fmt;
    use std::marker::PhantomData;

    struct HexVisitor<T>(PhantomData<T>);

    impl<'de, T: FromHex> Visitor<'de> for HexVisitor<T> {
        type Value = T;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a hex string")
        }

        fn visit_str<E: Error>(self, v: &str) -> Result<T, E> {
            T::from_hex(v).map_err(E::custom)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>, T: FromHex>(d: D) -> Result<T, D::Error> {
        d.deserialize_str(HexVisitor(PhantomData))
    }

    pub mod opt {
        use super::HexVisitor;
        use bitcoin::hex::FromHex;
        use serde::de::{Error, Visitor};
        use serde::Deserializer;
        use std::fmt;
        use std::marker::PhantomData;

        struct OptHexVisitor<T>(PhantomData<T>);

        impl<'de, T: FromHex> Visitor<'de> for OptHexVisitor<T> {
            type Value = Option<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an optional hex string")
            }

            fn visit_none<E: Error>(self) -> Result<Option<T>, E> {
                Ok(None)
            }

            fn visit_unit<E: Error>(self) -> Result<Option<T>, E> {
                Ok(None)
            }

            fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Option<T>, D::Error> {
                d.deserialize_str(HexVisitor(PhantomData)).map(Some)
            }
        }

        pub fn deserialize<'de, D: Deserializer<'de>, T: FromHex>(
            d: D,
        ) -> Result<Option<T>, D::Error> {
            d.deserialize_option(OptHexVisitor(PhantomData))
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScriptPubkeyType {
    Nonstandard,
    Pubkey,
    PubkeyHash,
    ScriptHash,
    MultiSig,
    NullData,
    Witness_v0_KeyHash,
    Witness_v0_ScriptHash,
    Witness_v1_Taproot,
    Witness_Unknown,
    Anchor,
}

#[derive(Deserialize)]
pub struct ScriptPubKey {
    #[serde(rename = "hex")]
    pub script: ScriptBuf,
    #[serde(rename = "type")]
    pub type_: ScriptPubkeyType,
}

/// The script_pub_key of a prevout. Unlike an output's script_pub_key, we only
/// need the type here, so the script itself isn't hex-decoded.
#[derive(Deserialize)]
pub struct PrevoutScriptPubKey {
    #[serde(rename = "type")]
    pub type_: ScriptPubkeyType,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prevout {
    pub height: i64,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub value: Amount,
    pub script_pub_key: PrevoutScriptPubKey,
}

/// An input of a transaction. A coinbase input has `coinbase` set, every other
/// input has `txid` and `prevout` set.
///
/// These are deliberately kept as plain `Option` fields rather than as an enum
/// behind `#[serde(flatten)]`: flattening and untagged enums force serde to
/// buffer every field of every input into an intermediate representation before
/// it can pick a variant, which costs more than the fields themselves.
#[derive(Deserialize)]
pub struct Input {
    pub sequence: Sequence,
    #[serde(default, with = "serde_hex::opt")]
    pub coinbase: Option<Vec<u8>>,
    #[serde(default)]
    pub txid: Option<bitcoin::Txid>,
    #[serde(default)]
    pub vout: Option<u32>,
    #[serde(default)]
    pub prevout: Option<Prevout>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub value: Amount,
    pub n: u32,
    pub script_pub_key: ScriptPubKey,
}

/// A transaction as returned by the REST interface.
///
/// Only fields that aren't already contained in `raw` are deserialized here.
/// Everything else is read from the transaction parsed out of `raw`, which is
/// far cheaper than parsing it out of the JSON a second time. The notable
/// exceptions are `fee` and the `prevout`s, which the raw transaction doesn't
/// carry.
#[derive(Deserialize)]
pub struct Transaction {
    #[serde(rename = "hex", with = "serde_hex")]
    pub raw: Vec<u8>,
    pub txid: bitcoin::Txid,
    pub size: u32,
    pub vsize: u32,
    pub version: u32,
    #[serde(default, with = "bitcoin::amount::serde::as_btc::opt")]
    pub fee: Option<Amount>,
    #[serde(rename = "locktime")]
    pub lock_time: LockTime,
    #[serde(rename = "vin")]
    pub input: Vec<Input>,
    #[serde(rename = "vout")]
    pub output: Vec<Output>,
}

impl Transaction {
    pub fn is_lock_time_enabled(&self) -> bool {
        self.input.iter().any(|i| i.sequence != Sequence::MAX)
    }
}

#[derive(Deserialize)]
pub struct Block {
    pub hash: BlockHash,
    pub size: i64,
    #[serde(rename = "strippedsize")]
    pub stripped_size: i64,
    pub weight: Weight,
    pub height: i64,
    pub version: block::Version,
    #[serde(rename = "tx")]
    pub txdata: Vec<Transaction>,
    pub time: u32,
    pub nonce: u32,
    pub bits: String,
}

#[derive(Debug)]
pub enum RestError {
    Ureq(Box<ureq::Error>),
    IoError(std::io::Error),
    BitcoinDecode(bitcoin::consensus::encode::Error),
    /// The node answered, but with something we could not make sense of.
    Response(String),
}

impl fmt::Display for RestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RestError::Ureq(e) => write!(f, "HTTP request error: {}", e),
            RestError::IoError(e) => write!(f, "IO error: {}", e),
            RestError::BitcoinDecode(e) => write!(f, "Bitcoin decode error: {:?}", e),
            RestError::Response(msg) => write!(f, "Unexpected REST response: {}", msg),
        }
    }
}

impl error::Error for RestError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            RestError::Ureq(ref e) => Some(e),
            RestError::IoError(ref e) => Some(e),
            RestError::BitcoinDecode(ref e) => Some(e),
            RestError::Response(_) => None,
        }
    }
}

impl From<ureq::Error> for RestError {
    fn from(e: ureq::Error) -> Self {
        RestError::Ureq(Box::new(e))
    }
}

impl From<std::io::Error> for RestError {
    fn from(e: std::io::Error) -> Self {
        RestError::IoError(e)
    }
}

impl From<bitcoin::consensus::encode::Error> for RestError {
    fn from(e: bitcoin::consensus::encode::Error) -> Self {
        RestError::BitcoinDecode(e)
    }
}

impl RestClient {
    /// Creates a new client for the Bitcoin Core REST interface on
    /// `host`:`port`.
    ///
    /// `num_threads` is the number of threads sharing this client. The idle
    /// connection pool is sized to match, so each thread can keep its
    /// connection to Bitcoin Core alive between requests. With ureq's defaults
    /// (ten idle connections, three per host) most connections would be torn
    /// down right after use and reopened for the next request, which adds up:
    /// a full sync makes two requests per block.
    pub fn new(host: &str, port: u16, num_threads: usize) -> RestClient {
        let max_idle = num_threads.max(1);
        let config = ureq::Agent::config_builder()
            .max_idle_connections(max_idle)
            .max_idle_connections_per_host(max_idle)
            .build();
        RestClient {
            host: host.to_string(),
            port,
            agent: config.into(),
        }
    }

    pub fn chain_info(&self) -> Result<ChainInfo, RestError> {
        let url = format!("http://{}:{}/rest/chaininfo.json", self.host, self.port);
        let mut resp = self.agent.get(&url).call()?;
        Ok(resp.body_mut().read_json::<ChainInfo>()?)
    }

    /// Gets the block at `height`, along with a breakdown of where the time
    /// went.
    ///
    /// The body is read into memory before it's deserialized, rather than
    /// deserialized straight off the socket, so that waiting for Bitcoin Core
    /// can be told apart from our own parsing.
    pub fn block_at_height(&self, height: u64) -> Result<(Block, FetchTiming), RestError> {
        let mut timing = FetchTiming::default();

        let start = Instant::now();
        let url = format!(
            "http://{}:{}/rest/blockhashbyheight/{}.hex",
            self.host, self.port, height
        );
        let mut resp = self.agent.get(&url).call()?;
        let hash_str = resp.body_mut().read_to_string()?;
        let hash = hash_str.trim();
        timing.hash_request = start.elapsed();

        let start = Instant::now();
        let url = format!(
            "http://{}:{}/rest/block/{}.json",
            self.host, self.port, hash
        );
        // `call()` returns once the response headers are in, so this is how
        // long Bitcoin Core took before it started answering.
        let mut resp = self.agent.get(&url).call()?;
        timing.first_byte = start.elapsed();

        let start = Instant::now();
        let body = resp
            .body_mut()
            .with_config()
            .limit(MAX_BLOCK_JSON_SIZE)
            .read_to_vec()?;
        timing.body = start.elapsed();
        timing.bytes = body.len();

        let start = Instant::now();
        let block = serde_json::from_slice(&body).map_err(|e| {
            RestError::Response(format!("block {} at height {}: {}", hash, height, e))
        })?;
        timing.parse = start.elapsed();

        Ok((block, timing))
    }
}
