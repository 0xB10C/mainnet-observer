use bitcoin::{
    self, absolute::LockTime, address::NetworkUnchecked, block, Address, Amount, BlockHash,
    ScriptBuf, Sequence, TxMerkleNode, Weight, Witness,
};
use serde::Deserialize;
use std::{error, fmt};

pub struct RestClient {
    host: String,
    port: u16,
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
    MinReq(minreq::Error),
    BitcoinDecode(bitcoin::consensus::encode::Error),
    Http(i32, String),
}

impl fmt::Display for RestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RestError::MinReq(e) => write!(f, "MinReq HTTP GET request error: {:?}", e),
            RestError::BitcoinDecode(e) => write!(f, "Bitcoin decode error: {:?}", e),
            RestError::Http(code, msg) => write!(f, "HTTP error: {} {}", code, msg),
        }
    }
}

impl error::Error for RestError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            RestError::MinReq(ref e) => Some(e),
            RestError::BitcoinDecode(ref e) => Some(e),
            RestError::Http(_, _) => None,
        }
    }
}

impl From<minreq::Error> for RestError {
    fn from(e: minreq::Error) -> Self {
        RestError::MinReq(e)
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
        }
    }

    pub fn chain_info(&self) -> Result<ChainInfo, RestError> {
        let url = format!("http://{}:{}/rest/chaininfo.json", self.host, self.port);
        let response = minreq::get(url).send()?;
        if !(response.status_code == 200 && response.reason_phrase == "OK") {
            return Err(RestError::Http(
                response.status_code,
                response.reason_phrase,
            ));
        }

        Ok(response.json::<ChainInfo>()?)
    }

    pub fn block_at_height(&self, height: u64) -> Result<Block, RestError> {
        let url = format!(
            "http://{}:{}/rest/blockhashbyheight/{}.hex",
            self.host, self.port, height
        );
        let response_hash = minreq::get(url).send()?;
        if !(response_hash.status_code == 200 && response_hash.reason_phrase == "OK") {
            return Err(RestError::Http(
                response_hash.status_code,
                response_hash.reason_phrase,
            ));
        }

        let hash = response_hash.as_str()?.trim();

        let url = format!(
            "http://{}:{}/rest/block/{}.json",
            self.host, self.port, hash
        );
        let response_block = minreq::get(url).send()?;
        if !(response_block.status_code == 200 && response_block.reason_phrase == "OK") {
            return Err(RestError::Http(
                response_block.status_code,
                response_block.reason_phrase,
            ));
        }

        Ok(response_block.json()?)
    }
}
