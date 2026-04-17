use bitcoin::{
    self, absolute::LockTime, address::NetworkUnchecked, block, Address, Amount, BlockHash,
    ScriptBuf, Sequence, TxMerkleNode, Weight, Witness,
};
use serde::Deserialize;
use std::{error, fmt};

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

pub mod serde_hex {
    use bitcoin::hex::FromHex;
    use serde::{de::Error, Deserialize, Deserializer};

    pub fn deserialize<'de, D: Deserializer<'de>, T: FromHex>(d: D) -> Result<T, D::Error> {
        let hex_str: String = Deserialize::deserialize(d)?;
        T::from_hex(&hex_str).map_err(D::Error::custom)
    }
}

#[derive(Deserialize)]
pub struct ScriptSig {
    #[serde(rename = "hex")]
    pub script: ScriptBuf,
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
    #[serde(rename = "desc")]
    pub descriptor: Option<String>,
    #[serde(rename = "type")]
    pub type_: ScriptPubkeyType,
    pub address: Option<Address<NetworkUnchecked>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prevout {
    pub generated: bool,
    pub height: i64,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub value: Amount,
    pub script_pub_key: ScriptPubKey,
}

#[derive(Deserialize)]
pub enum InputData {
    #[serde(rename = "coinbase", with = "serde_hex")]
    Coinbase(Vec<u8>),
    #[serde(untagged, rename_all = "camelCase")]
    NonCoinbase {
        txid: bitcoin::Txid,
        vout: u32,
        script_sig: ScriptSig,
        prevout: Prevout,
    },
}

#[derive(Deserialize)]
pub struct Input {
    pub sequence: Sequence,
    #[serde(rename = "txinwitness")]
    pub witness: Option<Witness>,
    #[serde(flatten)]
    pub data: InputData,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub value: Amount,
    pub n: u32,
    pub script_pub_key: ScriptPubKey,
}

#[derive(Deserialize)]
pub struct Transaction {
    #[serde(rename = "hex", with = "serde_hex")]
    pub raw: Vec<u8>,
    pub txid: bitcoin::Txid,
    pub hash: bitcoin::Wtxid,
    pub size: u32,
    pub vsize: u32,
    pub weight: Weight,
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
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub hash: BlockHash,
    pub confirmations: i64,
    pub size: i64,
    #[serde(rename = "strippedsize")]
    pub stripped_size: i64,
    pub weight: Weight,
    pub height: i64,
    pub version: block::Version,
    #[serde(rename = "merkleroot")]
    pub merkle_root: TxMerkleNode,
    #[serde(rename = "tx")]
    pub txdata: Vec<Transaction>,
    pub time: u32,
    #[serde(rename = "mediantime")]
    pub median_time: u32,
    pub nonce: u32,
    pub bits: String,
    pub difficulty: f64,
    #[serde(rename = "chainwork", with = "serde_hex")]
    pub chain_work: Vec<u8>,
    pub n_tx: u32,
    #[serde(rename = "previousblockhash")]
    pub previous_block_hash: Option<BlockHash>,
    #[serde(rename = "nextblockhash")]
    pub next_block_hash: Option<BlockHash>,
}

#[derive(Debug)]
pub enum RestError {
    Ureq(Box<ureq::Error>),
    IoError(std::io::Error),
    BitcoinDecode(bitcoin::consensus::encode::Error),
}

impl fmt::Display for RestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RestError::Ureq(e) => write!(f, "HTTP request error: {}", e),
            RestError::IoError(e) => write!(f, "IO error: {}", e),
            RestError::BitcoinDecode(e) => write!(f, "Bitcoin decode error: {:?}", e),
        }
    }
}

impl error::Error for RestError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            RestError::Ureq(ref e) => Some(e),
            RestError::IoError(ref e) => Some(e),
            RestError::BitcoinDecode(ref e) => Some(e),
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
    pub fn new(host: &str, port: u16) -> RestClient {
        RestClient {
            host: host.to_string(),
            port,
            agent: ureq::agent(),
        }
    }

    pub fn chain_info(&self) -> Result<ChainInfo, RestError> {
        let url = format!("http://{}:{}/rest/chaininfo.json", self.host, self.port);
        let mut resp = self.agent.get(&url).call()?;
        Ok(resp.body_mut().read_json::<ChainInfo>()?)
    }

    pub fn block_at_height(&self, height: u64) -> Result<Block, RestError> {
        let url = format!(
            "http://{}:{}/rest/blockhashbyheight/{}.hex",
            self.host, self.port, height
        );
        let mut resp = self.agent.get(&url).call()?;
        let hash_str = resp.body_mut().read_to_string()?;
        let hash = hash_str.trim();

        let url = format!(
            "http://{}:{}/rest/block/{}.json",
            self.host, self.port, hash
        );
        let mut resp = self.agent.get(&url).call()?;
        Ok(resp
            .body_mut()
            .with_config()
            .limit(50 * 1024 * 1024)
            .read_json()?)
    }
}
