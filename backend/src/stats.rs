use bitcoin::{
    error::UnprefixedHexError, script::Instruction, Amount, CompactTarget, Network, Target,
    Transaction, Txid,
};
use bitcoin_pool_identification::{default_data, Pool, PoolIdentification};
use chrono::DateTime;
use diesel::prelude::*;
use log::{debug, error};
use rawtx_rs::{
    input::InputType, output::OpReturnFlavor, output::OutputType, script::DEREncoding,
    script::SignatureType, tx::TxInfo,
};
use statrs::statistics::Data;
use statrs::statistics::OrderStatistics;
use std::{collections::HashSet, error, fmt, num::ParseIntError};

use crate::rest::{Block, InputData, ScriptPubkeyType};

const UNKNOWN_POOL_ID: i32 = 0;
const P2A_DUST_THRESHOLD: u64 = 240;

// The version we want the stats in the database to be and, at
// the same time also the stats_version we set when generating
// and writing stats to the database.
pub const STATS_VERSION: i32 = 1;

#[derive(Debug)]
pub enum StatsError {
    TxInfoError(rawtx_rs::tx::TxInfoError),
    BitcoinEncodeError(bitcoin::consensus::encode::Error),
    ParseIntError(ParseIntError),
    UnprefixedHexError(UnprefixedHexError),
}

impl fmt::Display for StatsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StatsError::TxInfoError(e) => write!(f, "Bitcoin Script Error: {:?}", e),
            StatsError::BitcoinEncodeError(e) => write!(f, "Bitcoin Encode Error: {:?}", e),
            StatsError::ParseIntError(e) => write!(f, "Parse Int Error: {:?}", e),
            StatsError::UnprefixedHexError(e) => write!(f, "Unprefixed Hex Error: {:?}", e),
        }
    }
}

impl error::Error for StatsError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            StatsError::TxInfoError(ref e) => Some(e),
            StatsError::BitcoinEncodeError(ref e) => Some(e),
            StatsError::ParseIntError(ref e) => Some(e),
            StatsError::UnprefixedHexError(ref e) => Some(e),
        }
    }
}

impl From<rawtx_rs::tx::TxInfoError> for StatsError {
    fn from(e: rawtx_rs::tx::TxInfoError) -> Self {
        StatsError::TxInfoError(e)
    }
}

impl From<bitcoin::consensus::encode::Error> for StatsError {
    fn from(e: bitcoin::consensus::encode::Error) -> Self {
        StatsError::BitcoinEncodeError(e)
    }
}

impl From<ParseIntError> for StatsError {
    fn from(e: ParseIntError) -> Self {
        StatsError::ParseIntError(e)
    }
}

impl From<UnprefixedHexError> for StatsError {
    fn from(e: UnprefixedHexError) -> Self {
        StatsError::UnprefixedHexError(e)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub block: BlockStats,
    pub tx: TxStats,
    pub input: InputStats,
    pub output: OutputStats,
    pub feerate: FeerateStats,
    pub script: ScriptStats,
}

impl Stats {
    pub fn from_block(block: Block) -> Result<Stats, StatsError> {
        let timestamp =
            DateTime::from_timestamp(block.time as i64, 0).expect("invalid block header timestamp");
        let date = timestamp.format("%Y-%m-%d").to_string();
        let mut tx_infos: Vec<TxInfo> = Vec::with_capacity(block.txdata.len());
        for tx in block.txdata.iter() {
            let tx: Transaction = bitcoin::consensus::deserialize(&tx.raw)?;
            match TxInfo::new(&tx) {
                Ok(txinfo) => tx_infos.push(txinfo),
                Err(e) => {
                    error!(
                        "Could not create TxInfo for {} in block {}: {}",
                        tx.compute_txid(),
                        block.height,
                        e
                    );
                    return Err(StatsError::TxInfoError(e));
                }
            }
        }

        // TODO: if we ever wanted to generate stats on a network other than
        // mainnet and do pool identification, we'd need to be able to change
        // the network here.
        let pools = default_data(Network::Bitcoin);

        Ok(Stats {
            block: BlockStats::from_block(&block, date.clone(), &tx_infos, &pools)?,
            tx: TxStats::from_block(&block, date.clone(), &tx_infos),
            input: InputStats::from_block(&block, date.clone(), &tx_infos),
            output: OutputStats::from_block(&block, date.clone(), &tx_infos),
            script: ScriptStats::from_block(&block, date.clone(), &tx_infos),
            feerate: FeerateStats::from_block(&block, date.clone(), &tx_infos),
        })
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, Clone, Debug, PartialEq)]
#[diesel(table_name = crate::schema::block_stats)]
#[diesel(primary_key(height))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct BlockStats {
    /// The version of stats we have for this block. If the stats version is too
    /// old, we need to update the stats.
    pub stats_version: i32,

    pub height: i64,
    pub date: String,

    pub version: i32,
    pub nonce: i32,
    pub bits: i32,
    /// Low-presision block difficulty. Stored as i64 as SQLite doesn't support
    /// f64 nor u128.
    pub difficulty: i64,
    /// Low-presision log2(work) for this block. Not to be confused with Bitcoin Core's cumulative log2_work
    /// for a block at a given height. This one is not cumulative.
    pub log2_work: f32,

    /// the size of the block in bytes
    pub size: i64,
    /// the size of the block excluding the witness data.
    pub stripped_size: i64,
    /// the virtual size of the block in bytes (ceil(weight / 4.0))
    pub vsize: i64,
    /// the size of the block in bytes
    pub weight: i64,
    /// the block is empty (no tx besides the coinbase tx)
    pub empty: bool,

    /// Coinbase output amounts (sum)
    pub coinbase_output_amount: i64,
    /// Coinbase transactoin weight
    pub coinbase_weight: i64,

    /// number of transactions in the block
    pub transactions: i32,
    /// number of payments in the block
    pub payments: i32,
    /// count of payments made by SegWit spending transactions
    pub payments_segwit_spending_tx: i32,
    /// count of payments made by Taproot spending transactions
    pub payments_taproot_spending_tx: i32,
    /// count of payments where the transaction signals RBF
    pub payments_signaling_explicit_rbf: i32,

    /// number of inputs spent in this block
    pub inputs: i32,
    /// number of outputs created in this block
    pub outputs: i32,
    /// the pool id, if the pool could be identified. If the pool is unknown,
    /// the id will be 0. See the IDs in https://github.com/bitcoin-data/mining-pools/blob/generated/pool-list.json
    pub pool_id: i32,
}

impl BlockStats {
    pub fn from_block(
        block: &Block,
        date: String,
        tx_infos: &Vec<TxInfo>,
        pools: &[Pool],
    ) -> Result<BlockStats, StatsError> {
        let height = block.height;
        let coinbase_tx: Transaction = bitcoin::consensus::deserialize(
            &block
                .txdata
                .first()
                .expect("block should have a coinbase tx")
                .raw,
        )?;
        let pool_id: i32 = match coinbase_tx.identify_pool(Network::Bitcoin, &pools) {
            Some(result) => {
                debug!(
                    "Identified pool '{}' at height {} with method '{:?}'",
                    result.pool.name, height, result.identification_method
                );
                result.pool.id as i32
            }
            None => {
                debug!("Could not identify pool at height {}", height);
                UNKNOWN_POOL_ID
            }
        };

        let target = Target::from_compact(CompactTarget::from_unprefixed_hex(&block.bits)?);

        Ok(BlockStats {
            stats_version: STATS_VERSION,
            height: height,
            date: date.to_string(),
            version: block.version.to_consensus(),
            nonce: block.nonce as i32,
            bits: i32::from_str_radix(&block.bits, 16)?,
            difficulty: target.difficulty_float() as i64,
            log2_work: target.to_work().log2() as f32,
            pool_id,

            size: block.size,
            stripped_size: block.stripped_size,
            vsize: block.txdata.iter().map(|x| x.vsize).sum::<u32>() as i64,
            weight: block.weight.to_wu() as i64,
            empty: block.txdata.len() == 1,

            coinbase_output_amount: block
                .txdata
                .first()
                .expect("block should have a coinbase tx")
                .output
                .iter()
                .map(|o| o.value.to_sat())
                .sum::<u64>() as i64,
            coinbase_weight: block
                .txdata
                .first()
                .expect("block should have a coinbase tx")
                .weight
                .to_wu() as i64,

            transactions: block.txdata.len() as i32,
            payments: tx_infos.iter().map(|ti| ti.payments()).sum::<u32>() as i32,
            payments_segwit_spending_tx: tx_infos
                .iter()
                .filter(|ti| ti.is_spending_segwit())
                .map(|ti| ti.payments())
                .sum::<u32>() as i32,
            payments_taproot_spending_tx: tx_infos
                .iter()
                .filter(|ti| ti.is_spending_taproot())
                .map(|ti| ti.payments())
                .sum::<u32>() as i32,
            payments_signaling_explicit_rbf: tx_infos
                .iter()
                .filter(|ti| ti.is_signaling_explicit_rbf_replicability())
                .map(|ti| ti.payments())
                .sum::<u32>() as i32,

            inputs: block.txdata.iter().map(|tx| tx.input.len()).sum::<usize>() as i32,
            outputs: block.txdata.iter().map(|tx| tx.output.len()).sum::<usize>() as i32,
        })
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, Clone, Default, Debug, PartialEq)]
#[diesel(table_name = crate::schema::tx_stats)]
#[diesel(primary_key(height))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TxStats {
    pub height: i64,
    pub date: String,

    // number of version 1 transactions
    pub tx_version_1: i32,
    // number of version 2 transactions
    pub tx_version_2: i32,
    // number of version 3 transactions
    pub tx_version_3: i32,
    // number of transactions with an unknown version (might change once there are proposals to use e.g. version=4)
    pub tx_version_unknown: i32,

    pub tx_output_amount: i64,

    pub tx_spending_segwit: i32,
    pub tx_spending_only_segwit: i32,
    pub tx_spending_only_legacy: i32,
    pub tx_spending_only_taproot: i32,
    pub tx_spending_segwit_and_legacy: i32,
    pub tx_spending_nested_segwit: i32,
    pub tx_spending_native_segwit: i32,
    pub tx_spending_taproot: i32,

    pub tx_bip69_compliant: i32,
    pub tx_signaling_explicit_rbf: i32,

    pub tx_1_input: i32,
    pub tx_1_output: i32,
    pub tx_1_input_1_output: i32,
    pub tx_1_input_2_output: i32,
    pub tx_spending_newly_created_utxos: i32,
    pub tx_spending_ephemeral_dust: i32,

    pub tx_timelock_height: i32,
    pub tx_timelock_timestamp: i32,
    pub tx_timelock_not_enforced: i32,
    pub tx_timelock_too_high: i32,
}

impl TxStats {
    pub fn from_block(block: &Block, date: String, tx_infos: &Vec<TxInfo>) -> TxStats {
        let height = block.height;
        let mut s = TxStats::default();

        let mut txids_in_this_block: HashSet<&Txid> = HashSet::with_capacity(block.txdata.len());
        let mut ephemeral_dust_outpoints_in_this_block: HashSet<(&Txid, u32)> = HashSet::new();

        s.height = height;
        s.date = date;

        for (tx, tx_info) in block.txdata.iter().zip(tx_infos.iter()) {
            match tx.version {
                1 => s.tx_version_1 += 1,
                2 => s.tx_version_2 += 1,
                3 => s.tx_version_3 += 1,
                _ => s.tx_version_unknown += 1,
            }

            s.tx_output_amount += tx_info.output_value_sum().to_sat() as i64;

            if tx_info.is_spending_segwit() {
                s.tx_spending_segwit += 1;
                if tx_info.is_spending_native_segwit() {
                    s.tx_spending_native_segwit += 1;
                }
                if tx_info.is_spending_nested_segwit() {
                    s.tx_spending_nested_segwit += 1;
                }
                if tx_info.is_spending_taproot() {
                    s.tx_spending_taproot += 1;
                }
            }

            if tx_info.is_spending_segwit_and_legacy() {
                s.tx_spending_segwit_and_legacy += 1;
            }

            if tx_info.is_only_spending_legacy() {
                s.tx_spending_only_legacy += 1;
            } else if tx_info.is_only_spending_segwit() {
                s.tx_spending_only_segwit += 1;
                if tx_info.is_only_spending_taproot() {
                    s.tx_spending_only_taproot += 1;
                }
            }

            if tx_info.is_bip69_compliant() {
                s.tx_bip69_compliant += 1;
            }

            if tx_info.is_signaling_explicit_rbf_replicability() {
                s.tx_signaling_explicit_rbf += 1;
            }

            if tx.input.len() == 1 {
                s.tx_1_input += 1;
                match tx.output.len() {
                    1 => s.tx_1_input_1_output += 1,
                    2 => s.tx_1_input_2_output += 1,
                    _ => (),
                }
            }
            if tx.output.len() == 1 {
                s.tx_1_output += 1;
            }

            let mut tx_spending_newly_created_utxos = false;
            let mut tx_spending_ephemeral_dust = false;
            for (txid, vout) in tx.input.iter().filter_map(|i| {
                if let InputData::NonCoinbase { txid, vout, .. } = &i.data {
                    Some((txid, vout))
                } else {
                    None
                }
            }) {
                tx_spending_newly_created_utxos |= txids_in_this_block.contains(txid);
                tx_spending_ephemeral_dust |= ephemeral_dust_outpoints_in_this_block
                    .take(&(txid, *vout))
                    .is_some()
                    && tx.version == 3
                    // unwrap safety: we filter for non-coinbase inputs above, hence this transaction is non-coinbase
                    && tx.fee.unwrap() != Amount::ZERO
                    && tx.vsize <= 1_000;

                if (tx_spending_ephemeral_dust || ephemeral_dust_outpoints_in_this_block.is_empty())
                    && tx_spending_newly_created_utxos
                {
                    break;
                }
            }
            s.tx_spending_newly_created_utxos += i32::from(tx_spending_newly_created_utxos);
            s.tx_spending_ephemeral_dust += i32::from(tx_spending_ephemeral_dust);

            // A parent is always ordered before the child in the transaction list of a block, so we can insert the
            // parent here, and detect any children of this parent in subsequent iterations of the loop
            txids_in_this_block.insert(&tx.txid);

            // We do not include any cases of ephemeral dust on coinbase transactions (these transactions have their
            // `tx.fee` set to `None`); we are only interested in cases of ephemeral dust that could have been submitted
            // via the p2p network.
            if tx.version == 3 && tx.fee == Some(Amount::ZERO) && tx.vsize <= 10_000 {
                let staged_ephemeral_dust_outpoints: Vec<_> = tx
                    .output
                    .iter()
                    .filter_map(|output| {
                        (output.value < output.script_pub_key.script.minimal_non_dust())
                            .then_some((&tx.txid, output.n))
                    })
                    .collect();

                // A transaction with more than 1 dust output was likely submitted out-of-band, so don't count them in
                // the `tx_spending_ephemeral_dust` tally
                if staged_ephemeral_dust_outpoints.len() == 1 {
                    ephemeral_dust_outpoints_in_this_block
                        .extend(staged_ephemeral_dust_outpoints.into_iter());
                }
            }

            if tx.lock_time.is_block_height() && tx.lock_time.to_consensus_u32() > 0 {
                s.tx_timelock_height += 1;
            } else if tx.lock_time.is_block_time() {
                s.tx_timelock_timestamp += 1;
            }

            if tx.lock_time.to_consensus_u32() > 0 && !tx.is_lock_time_enabled() {
                s.tx_timelock_not_enforced += 1;
            }

            if tx.lock_time.is_block_height() && tx.lock_time.to_consensus_u32() > height as u32 {
                s.tx_timelock_too_high += 1;
            }
        }

        return s;
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, Clone, Default, Debug, PartialEq)]
#[diesel(table_name = crate::schema::script_stats)]
#[diesel(primary_key(height))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ScriptStats {
    height: i64,
    date: String,

    pubkeys: i32,
    pubkeys_compressed: i32,
    pubkeys_uncompressed: i32,
    pubkeys_compressed_inputs: i32,
    pubkeys_uncompressed_inputs: i32,
    pubkeys_compressed_outputs: i32,
    pubkeys_uncompressed_outputs: i32,

    sigs_schnorr: i32,
    sigs_ecdsa: i32,
    sigs_ecdsa_not_strict_der: i32,
    sigs_ecdsa_strict_der: i32,

    sigs_ecdsa_length_less_70byte: i32,
    sigs_ecdsa_length_70byte: i32,
    sigs_ecdsa_length_71byte: i32,
    sigs_ecdsa_length_72byte: i32,
    sigs_ecdsa_length_73byte: i32,
    sigs_ecdsa_length_74byte: i32,
    sigs_ecdsa_length_75byte_or_more: i32,

    sigs_ecdsa_low_r: i32,
    sigs_ecdsa_high_r: i32,
    sigs_ecdsa_low_s: i32,
    sigs_ecdsa_high_s: i32,
    sigs_ecdsa_high_rs: i32,
    sigs_ecdsa_low_rs: i32,
    sigs_ecdsa_low_r_high_s: i32,
    sigs_ecdsa_high_r_low_s: i32,

    sigs_sighashes: i32,
    sigs_sighash_all: i32,
    sigs_sighash_none: i32,
    sigs_sighash_single: i32,
    sigs_sighash_all_acp: i32,
    sigs_sighash_none_acp: i32,
    sigs_sighash_single_acp: i32,
}

impl ScriptStats {
    pub fn from_block(block: &Block, date: String, tx_infos: &Vec<TxInfo>) -> ScriptStats {
        let height = block.height;
        let mut s = ScriptStats::default();

        s.height = height;
        s.date = date;

        for (_, tx_info) in block.txdata.iter().zip(tx_infos.iter()) {
            for input in tx_info.input_infos.iter() {
                // pubkey stats
                for pubkey in input.pubkey_stats.iter() {
                    s.pubkeys += 1;
                    if pubkey.compressed {
                        s.pubkeys_compressed += 1;
                        s.pubkeys_compressed_inputs += 1;
                    } else {
                        s.pubkeys_uncompressed += 1;
                        s.pubkeys_uncompressed_inputs += 1;
                    }
                }

                // signature stats
                for sig in input.signature_info.iter() {
                    if matches!(sig.signature, SignatureType::Schnorr(_)) {
                        s.sigs_schnorr += 1;
                    } else if matches!(sig.signature, SignatureType::Ecdsa(_)) {
                        s.sigs_ecdsa += 1;
                        if sig.der_encoded == DEREncoding::Valid {
                            s.sigs_ecdsa_strict_der += 1;
                        } else {
                            s.sigs_ecdsa_not_strict_der += 1;
                        }
                        match sig.length {
                            8..=69 => s.sigs_ecdsa_length_less_70byte += 1,
                            70 => s.sigs_ecdsa_length_70byte += 1,
                            71 => s.sigs_ecdsa_length_71byte += 1,
                            72 => s.sigs_ecdsa_length_72byte += 1,
                            73 => s.sigs_ecdsa_length_73byte += 1,
                            74 => s.sigs_ecdsa_length_74byte += 1,
                            75.. => s.sigs_ecdsa_length_75byte_or_more += 1,
                            _ => panic!("ECDSA signature with {} bytes..?", sig.length),
                        }

                        let is_r_low = sig.low_r();
                        let is_s_low = sig.low_s();

                        if is_r_low {
                            s.sigs_ecdsa_low_r += 1;
                        } else {
                            s.sigs_ecdsa_high_r += 1;
                        }

                        if is_s_low {
                            s.sigs_ecdsa_low_s += 1;
                        } else {
                            s.sigs_ecdsa_high_s += 1;
                        }

                        if is_r_low && is_s_low {
                            s.sigs_ecdsa_low_rs += 1;
                        } else if !is_r_low && !is_s_low {
                            s.sigs_ecdsa_high_rs += 1;
                        } else if is_r_low && !is_s_low {
                            s.sigs_ecdsa_low_r_high_s += 1;
                        } else if !is_r_low && is_s_low {
                            s.sigs_ecdsa_high_r_low_s += 1;
                        }

                        s.sigs_sighashes += 1;
                        match sig.sig_hash {
                            0x01 => s.sigs_sighash_all += 1,
                            0x02 => s.sigs_sighash_none += 1,
                            0x03 => s.sigs_sighash_single += 1,
                            0x81 => s.sigs_sighash_all_acp += 1,
                            0x82 => s.sigs_sighash_none_acp += 1,
                            0x83 => s.sigs_sighash_single_acp += 1,
                            _ => (),
                        }
                    }
                }
            }

            for output in tx_info.output_infos.iter() {
                // pubkey stats
                for pubkey in output.pubkey_stats.iter() {
                    s.pubkeys += 1;
                    if pubkey.compressed {
                        s.pubkeys_compressed += 1;
                        s.pubkeys_compressed_outputs += 1;
                    } else {
                        s.pubkeys_uncompressed += 1;
                        s.pubkeys_uncompressed_outputs += 1;
                    }
                }
            }
        }
        s
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, Clone, Default, Debug, PartialEq)]
#[diesel(table_name = crate::schema::input_stats)]
#[diesel(primary_key(height))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct InputStats {
    height: i64,
    date: String,

    inputs_spending_legacy: i32,
    inputs_spending_segwit: i32,
    inputs_spending_taproot: i32,
    inputs_spending_nested_segwit: i32,
    inputs_spending_native_segwit: i32,
    inputs_spending_multisig: i32,
    inputs_spending_p2ms_multisig: i32,
    inputs_spending_p2sh_multisig: i32,
    inputs_spending_nested_p2wsh_multisig: i32,
    inputs_spending_p2wsh_multisig: i32,

    inputs_p2pk: i32,
    inputs_p2pkh: i32,
    inputs_nested_p2wpkh: i32,
    inputs_p2wpkh: i32,
    inputs_p2ms: i32,
    inputs_p2sh: i32,
    inputs_nested_p2wsh: i32,
    inputs_p2wsh: i32,
    inputs_coinbase: i32,
    inputs_witness_coinbase: i32,
    inputs_p2tr_keypath: i32,
    inputs_p2tr_scriptpath: i32,
    inputs_p2a: i32,
    inputs_p2a_dust: i32,
    inputs_unknown: i32,

    inputs_spend_in_same_block: i32,
}

impl InputStats {
    pub fn from_block(block: &Block, date: String, tx_infos: &Vec<TxInfo>) -> InputStats {
        let height = block.height;
        let txids_in_this_block: HashSet<Txid> = block.txdata.iter().map(|tx| tx.txid).collect();

        let mut s = InputStats::default();
        s.height = height;
        s.date = date;

        for (tx, tx_info) in block.txdata.iter().zip(tx_infos.iter()) {
            for input in tx_info.input_infos.iter() {
                if input.is_spending_legacy() {
                    s.inputs_spending_legacy += 1;
                }
                if input.is_spending_segwit() {
                    s.inputs_spending_segwit += 1;
                }
                if input.is_spending_taproot() {
                    s.inputs_spending_taproot += 1;
                }
                if input.is_spending_nested_segwit() {
                    s.inputs_spending_nested_segwit += 1;
                }
                if input.is_spending_native_segwit() {
                    s.inputs_spending_native_segwit += 1;
                }
                if input.is_spending_multisig() {
                    s.inputs_spending_multisig += 1;
                    match input.in_type {
                        InputType::P2ms => s.inputs_spending_p2ms_multisig += 1,
                        InputType::P2shP2wsh => s.inputs_spending_nested_p2wsh_multisig += 1,
                        InputType::P2wsh => s.inputs_spending_p2wsh_multisig += 1,
                        InputType::P2sh => s.inputs_spending_p2sh_multisig += 1,
                        _ => (),
                    }
                }

                match input.in_type {
                    InputType::P2pk | InputType::P2pkLaxDer => s.inputs_p2pk += 1,
                    InputType::P2pkh | InputType::P2pkhLaxDer => s.inputs_p2pkh += 1,
                    InputType::P2shP2wpkh => s.inputs_nested_p2wpkh += 1,
                    InputType::P2wpkh => s.inputs_p2wpkh += 1,
                    InputType::P2ms | InputType::P2msLaxDer => s.inputs_p2ms += 1,
                    InputType::P2sh => s.inputs_p2sh += 1,
                    InputType::P2shP2wsh => s.inputs_nested_p2wsh += 1,
                    InputType::P2wsh => s.inputs_p2wsh += 1,
                    InputType::Coinbase => s.inputs_coinbase += 1,
                    InputType::CoinbaseWitness => s.inputs_witness_coinbase += 1,
                    InputType::P2trkp => s.inputs_p2tr_keypath += 1,
                    InputType::P2trsp => s.inputs_p2tr_scriptpath += 1,
                    InputType::Unknown | InputType::P2a => s.inputs_unknown += 1,
                }
            }
            for input in tx.input.iter() {
                let InputData::NonCoinbase { txid, prevout, .. } = &input.data else {
                    continue;
                };
                if txids_in_this_block.contains(txid) {
                    s.inputs_spend_in_same_block += 1;
                }

                if matches!(prevout.script_pub_key.type_, ScriptPubkeyType::Anchor) {
                    s.inputs_p2a += 1;
                    s.inputs_unknown -= 1;

                    if prevout.value < bitcoin::Amount::from_sat(P2A_DUST_THRESHOLD) {
                        s.inputs_p2a_dust += 1;
                    }
                }
            }
        }
        s
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, Clone, Default, Debug, PartialEq)]
#[diesel(table_name = crate::schema::output_stats)]
#[diesel(primary_key(height))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct OutputStats {
    height: i64,
    date: String,

    outputs_p2pk: i32,
    outputs_p2pkh: i32,
    outputs_p2wpkh: i32,
    outputs_p2ms: i32,
    outputs_p2sh: i32,
    outputs_p2wsh: i32,
    outputs_opreturn: i32,
    outputs_p2tr: i32,
    outputs_p2a: i32,
    outputs_p2a_dust: i32,
    outputs_unknown: i32,

    outputs_p2pk_amount: i64,
    outputs_p2pkh_amount: i64,
    outputs_p2wpkh_amount: i64,
    outputs_p2ms_amount: i64,
    outputs_p2sh_amount: i64,
    outputs_p2wsh_amount: i64,
    outputs_p2tr_amount: i64,
    outputs_p2a_amount: i64,
    outputs_opreturn_amount: i64,
    outputs_unknown_amount: i64,

    outputs_opreturn_omnilayer: i32,
    outputs_opreturn_stacks_block_commit: i32,
    outputs_opreturn_bip47_payment_code: i32,
    outputs_opreturn_coinbase_rsk: i32,
    outputs_opreturn_coinbase_coredao: i32,
    outputs_opreturn_coinbase_exsat: i32,
    outputs_opreturn_coinbase_hathor: i32,
    outputs_opreturn_coinbase_witness_commitment: i32,
    outputs_opreturn_runestone: i32,
    outputs_opreturn_bytes: i64,
}

/// Returns the total size of data pushed in an OP_RETURN script.
/// Only counts the actual payload bytes (PushBytes), excluding opcodes.
fn calculate_opreturn_data_size(script: &bitcoin::ScriptBuf) -> usize {
    if !script.is_op_return() {
        return 0;
    }

    let mut total = 0;
    for inst in script.instructions().flatten() {
        if let Instruction::PushBytes(bytes) = inst {
            total += bytes.len();
        }
    }
    total
}

impl OutputStats {
    pub fn from_block(block: &Block, date: String, tx_infos: &Vec<TxInfo>) -> OutputStats {
        let height = block.height;
        let mut s = OutputStats::default();

        s.height = height;
        s.date = date;

        let mut is_coinbase = true;
        for (tx, tx_info) in block.txdata.iter().zip(tx_infos.iter()) {
            for (output_index, output) in tx_info.output_infos.iter().enumerate() {
                match output.out_type {
                    OutputType::P2pk => {
                        s.outputs_p2pk += 1;
                        s.outputs_p2pk_amount += output.value.to_sat() as i64;
                    }
                    OutputType::P2pkh => {
                        s.outputs_p2pkh += 1;
                        s.outputs_p2pkh_amount += output.value.to_sat() as i64;
                    }
                    OutputType::P2wpkhV0 => {
                        s.outputs_p2wpkh += 1;
                        s.outputs_p2wpkh_amount += output.value.to_sat() as i64;
                    }
                    OutputType::P2ms => {
                        s.outputs_p2ms += 1;
                        s.outputs_p2ms_amount += output.value.to_sat() as i64;
                    }
                    OutputType::P2sh => {
                        s.outputs_p2sh += 1;
                        s.outputs_p2sh_amount += output.value.to_sat() as i64;
                    }
                    OutputType::P2wshV0 => {
                        s.outputs_p2wsh += 1;
                        s.outputs_p2wsh_amount += output.value.to_sat() as i64;
                    }
                    OutputType::P2tr => {
                        s.outputs_p2tr += 1;
                        s.outputs_p2tr_amount += output.value.to_sat() as i64;
                    }
                    OutputType::P2a => {
                        s.outputs_p2a += 1;
                        s.outputs_p2a_amount += output.value.to_sat() as i64;

                        if output.value < bitcoin::Amount::from_sat(P2A_DUST_THRESHOLD) {
                            s.outputs_p2a_dust += 1;
                        }
                    }
                    OutputType::OpReturn(flavor) => {
                        s.outputs_opreturn += 1;
                        s.outputs_opreturn_amount += output.value.to_sat() as i64;

                        // Calculate OP_RETURN payload size (only counts PushBytes data)
                        let script = &tx.output[output_index].script_pub_key.script;
                        let data_size = calculate_opreturn_data_size(script);
                        s.outputs_opreturn_bytes += data_size as i64;

                        match flavor {
                            OpReturnFlavor::Runestone => s.outputs_opreturn_runestone += 1,
                            OpReturnFlavor::Omni => s.outputs_opreturn_omnilayer += 1,
                            OpReturnFlavor::StacksBlockCommit => {
                                s.outputs_opreturn_stacks_block_commit += 1
                            }
                            OpReturnFlavor::Bip47PaymentCode => {
                                s.outputs_opreturn_bip47_payment_code += 1
                            }
                            OpReturnFlavor::RSKBlock => {
                                s.outputs_opreturn_coinbase_rsk += if is_coinbase { 1 } else { 0 }
                            }
                            OpReturnFlavor::CoreDao => {
                                s.outputs_opreturn_coinbase_coredao +=
                                    if is_coinbase { 1 } else { 0 }
                            }
                            OpReturnFlavor::ExSat => {
                                s.outputs_opreturn_coinbase_exsat += if is_coinbase { 1 } else { 0 }
                            }
                            OpReturnFlavor::HathorNetwork => {
                                s.outputs_opreturn_coinbase_hathor +=
                                    if is_coinbase { 1 } else { 0 }
                            }
                            OpReturnFlavor::WitnessCommitment => {
                                s.outputs_opreturn_coinbase_witness_commitment +=
                                    if is_coinbase { 1 } else { 0 }
                            }
                            OpReturnFlavor::Len1Byte => (), // TODO: not implemented yet
                            OpReturnFlavor::Len20Byte => (), // TODO: not implemented yet
                            OpReturnFlavor::Len80Byte => (), // TODO: not implemented yet
                            OpReturnFlavor::Unspecified => (), // we don't know
                        }
                    }
                    OutputType::Unknown => {
                        s.outputs_unknown += 1;
                        s.outputs_unknown_amount += output.value.to_sat() as i64;
                    }
                }
            }
            is_coinbase = false;
        }
        s
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, Clone, Debug, PartialEq, Default)]
#[diesel(table_name = crate::schema::feerate_stats)]
#[diesel(primary_key(height))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct FeerateStats {
    height: i64,
    date: String,

    fee_min: i64,
    fee_5th_percentile: i64,
    fee_10th_percentile: i64,
    fee_25th_percentile: i64,
    fee_35th_percentile: i64,
    fee_50th_percentile: i64,
    fee_65th_percentile: i64,
    fee_75th_percentile: i64,
    fee_90th_percentile: i64,
    fee_95th_percentile: i64,
    fee_max: i64,
    fee_sum: i64,
    fee_avg: f32,
    size_min: i32,
    size_5th_percentile: i32,
    size_10th_percentile: i32,
    size_25th_percentile: i32,
    size_35th_percentile: i32,
    size_50th_percentile: i32,
    size_65th_percentile: i32,
    size_75th_percentile: i32,
    size_90th_percentile: i32,
    size_95th_percentile: i32,
    size_max: i32,
    size_avg: f32,
    size_sum: i64,
    feerate_min: f32,
    feerate_5th_percentile: f32,
    feerate_10th_percentile: f32,
    feerate_25th_percentile: f32,
    feerate_35th_percentile: f32,
    feerate_50th_percentile: f32,
    feerate_65th_percentile: f32,
    feerate_75th_percentile: f32,
    feerate_90th_percentile: f32,
    feerate_95th_percentile: f32,
    feerate_max: f32,
    feerate_avg: f32,
    feerate_package_min: f32,
    feerate_package_5th_percentile: f32,
    feerate_package_10th_percentile: f32,
    feerate_package_25th_percentile: f32,
    feerate_package_35th_percentile: f32,
    feerate_package_50th_percentile: f32,
    feerate_package_65th_percentile: f32,
    feerate_package_75th_percentile: f32,
    feerate_package_90th_percentile: f32,
    feerate_package_95th_percentile: f32,
    feerate_package_max: f32,
    feerate_package_avg: f32,
    // Added 2025-08-01:
    zero_fee_tx: i32,
    below_1_sat_vbyte: i32,
    // Fee band counts, added 2025-08-11
    feerate_1_2_sat_vbyte: i32,
    feerate_2_5_sat_vbyte: i32,
    feerate_5_10_sat_vbyte: i32,
    feerate_10_25_sat_vbyte: i32,
    feerate_25_50_sat_vbyte: i32,
    feerate_50_100_sat_vbyte: i32,
    feerate_100_250_sat_vbyte: i32,
    feerate_250_500_sat_vbyte: i32,
    feerate_500_1000_sat_vbyte: i32,
    feerate_1000_plus_sat_vbyte: i32,
}

/// helper function to treat f64::NAN values as 0. If we try to insert NANs into the database,
/// these will be treated as NULL, which collides with the NOT NULL constraints we have on some
/// tables
fn f64_nan_as_0(a: f64) -> f64 {
    if a.is_nan() {
        return 0f64;
    }
    a
}

impl FeerateStats {
    pub fn from_block(block: &Block, date: String, tx_infos: &Vec<TxInfo>) -> FeerateStats {
        let num_tx_without_coinbase = block.txdata.len() - 1;

        let mut fees_sat = Vec::with_capacity(num_tx_without_coinbase);
        let mut sizes: Vec<u64> = Vec::with_capacity(num_tx_without_coinbase);
        let mut feerates = Vec::with_capacity(num_tx_without_coinbase);
        let mut zero_fee_tx = 0;
        let mut below_1_sat_vbyte = 0;
        // Fee band counters
        let mut feerate_1_2_sat_vbyte = 0;
        let mut feerate_2_5_sat_vbyte = 0;
        let mut feerate_5_10_sat_vbyte = 0;
        let mut feerate_10_25_sat_vbyte = 0;
        let mut feerate_25_50_sat_vbyte = 0;
        let mut feerate_50_100_sat_vbyte = 0;
        let mut feerate_100_250_sat_vbyte = 0;
        let mut feerate_250_500_sat_vbyte = 0;
        let mut feerate_500_1000_sat_vbyte = 0;
        let mut feerate_1000_plus_sat_vbyte = 0;

        let mut is_coinbase = true;
        for (tx, _) in block.txdata.iter().zip(tx_infos.iter()) {
            if is_coinbase {
                // We don't consider the coinbase in the feerate stats. It has a fee of 0.
                is_coinbase = false;
                continue;
            }
            let fee = tx.fee.unwrap_or_default();
            let feerate: f64 = fee.to_sat() as f64 / tx.vsize as f64;

            // Count transactions in fee rate bands
            match feerate {
                x if x < 1.0 => below_1_sat_vbyte += 1,
                1.0..2.0 => feerate_1_2_sat_vbyte += 1,
                2.0..5.0 => feerate_2_5_sat_vbyte += 1,
                5.0..10.0 => feerate_5_10_sat_vbyte += 1,
                10.0..25.0 => feerate_10_25_sat_vbyte += 1,
                25.0..50.0 => feerate_25_50_sat_vbyte += 1,
                50.0..100.0 => feerate_50_100_sat_vbyte += 1,
                100.0..250.0 => feerate_100_250_sat_vbyte += 1,
                250.0..500.0 => feerate_250_500_sat_vbyte += 1,
                500.0..1000.0 => feerate_500_1000_sat_vbyte += 1,
                _ => feerate_1000_plus_sat_vbyte += 1, // 1000 or more
            }

            if let Some(fee) = tx.fee {
                if fee.to_sat() == 0 {
                    zero_fee_tx += 1;
                }
            }

            fees_sat.push(fee.to_sat());
            sizes.push(tx.size as u64);
            feerates.push(feerate);
        }

        let mut fees_data: Data<Vec<f64>> = Data::new(fees_sat.iter().map(|f| *f as f64).collect());
        let mut sizes_data: Data<Vec<f64>> = Data::new(sizes.iter().map(|f| *f as f64).collect());
        let mut feerates_data: Data<Vec<f64>> = Data::new(feerates.clone());

        let fee_sum: u64 = fees_sat.iter().sum();
        let fee_avg = match num_tx_without_coinbase {
            0 => 0.0f32,
            _ => fee_sum as f32 / num_tx_without_coinbase as f32,
        };

        let size_sum: u64 = sizes.iter().sum();
        let size_avg = match num_tx_without_coinbase {
            0 => 0.0f32,
            _ => size_sum as f32 / num_tx_without_coinbase as f32,
        };

        let feerate_sum: f64 = feerates.iter().sum();
        let feerate_avg = match num_tx_without_coinbase {
            0 => 0.0f32,
            _ => feerate_sum as f32 / num_tx_without_coinbase as f32,
        };

        FeerateStats {
            height: block.height,
            date: date,
            fee_min: *(fees_sat.iter().min().unwrap_or(&0)) as i64,
            fee_5th_percentile: fees_data.percentile(5) as i64,
            fee_10th_percentile: fees_data.percentile(10) as i64,
            fee_25th_percentile: fees_data.percentile(25) as i64,
            fee_35th_percentile: fees_data.percentile(35) as i64,
            fee_50th_percentile: fees_data.percentile(50) as i64,
            fee_65th_percentile: fees_data.percentile(65) as i64,
            fee_75th_percentile: fees_data.percentile(75) as i64,
            fee_90th_percentile: fees_data.percentile(90) as i64,
            fee_95th_percentile: fees_data.percentile(95) as i64,
            fee_max: *(fees_sat.iter().max().unwrap_or(&0)) as i64,
            fee_sum: fee_sum as i64,
            fee_avg,
            size_min: *(sizes.iter().min().unwrap_or(&0)) as i32,
            size_5th_percentile: sizes_data.percentile(5) as i32,
            size_10th_percentile: sizes_data.percentile(10) as i32,
            size_25th_percentile: sizes_data.percentile(25) as i32,
            size_35th_percentile: sizes_data.percentile(35) as i32,
            size_50th_percentile: sizes_data.percentile(50) as i32,
            size_65th_percentile: sizes_data.percentile(65) as i32,
            size_75th_percentile: sizes_data.percentile(75) as i32,
            size_90th_percentile: sizes_data.percentile(90) as i32,
            size_95th_percentile: sizes_data.percentile(95) as i32,
            size_max: *(sizes.iter().max().unwrap_or(&0)) as i32,
            size_avg,
            size_sum: size_sum as i64,
            feerate_min: *(feerates
                .iter()
                .filter(|x| !x.is_nan())
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&0.0)) as f32,
            feerate_5th_percentile: f64_nan_as_0(feerates_data.percentile(5)) as f32,
            feerate_10th_percentile: f64_nan_as_0(feerates_data.percentile(10)) as f32,
            feerate_25th_percentile: f64_nan_as_0(feerates_data.percentile(25)) as f32,
            feerate_35th_percentile: f64_nan_as_0(feerates_data.percentile(35)) as f32,
            feerate_50th_percentile: f64_nan_as_0(feerates_data.percentile(50)) as f32,
            feerate_65th_percentile: f64_nan_as_0(feerates_data.percentile(65)) as f32,
            feerate_75th_percentile: f64_nan_as_0(feerates_data.percentile(75)) as f32,
            feerate_90th_percentile: f64_nan_as_0(feerates_data.percentile(90)) as f32,
            feerate_95th_percentile: f64_nan_as_0(feerates_data.percentile(95)) as f32,
            feerate_max: *(feerates
                .iter()
                .filter(|x| !x.is_nan())
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&0.0)) as f32,
            feerate_avg,
            // TODO: Transaction package feerate stats are not yet implemented.
            feerate_package_min: 0.0f32,
            feerate_package_5th_percentile: 0.0f32,
            feerate_package_10th_percentile: 0.0f32,
            feerate_package_25th_percentile: 0.0f32,
            feerate_package_35th_percentile: 0.0f32,
            feerate_package_50th_percentile: 0.0f32,
            feerate_package_65th_percentile: 0.0f32,
            feerate_package_75th_percentile: 0.0f32,
            feerate_package_90th_percentile: 0.0f32,
            feerate_package_95th_percentile: 0.0f32,
            feerate_package_max: 0.0f32,
            feerate_package_avg: 0.0f32,
            zero_fee_tx,
            below_1_sat_vbyte,
            feerate_1_2_sat_vbyte,
            feerate_2_5_sat_vbyte,
            feerate_5_10_sat_vbyte,
            feerate_10_25_sat_vbyte,
            feerate_25_50_sat_vbyte,
            feerate_50_100_sat_vbyte,
            feerate_100_250_sat_vbyte,
            feerate_250_500_sat_vbyte,
            feerate_500_1000_sat_vbyte,
            feerate_1000_plus_sat_vbyte,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rest::Block;
    use crate::stats::{
        BlockStats, FeerateStats, InputStats, OutputStats, ScriptStats, TxStats, STATS_VERSION,
    };
    use crate::Stats;
    use serde::Deserialize;
    use std::fs::File;
    use std::io::BufReader;

    // helper to make diffs in large Stats structs better visible
    fn diff_stats(got: &Stats, expected: &Stats) {
        let got_str = format!("{:#?}", got);
        let expected_str = format!("{:#?}", expected);
        assert_eq!(got_str.lines().count(), expected_str.lines().count());
        for (got_line, expected_line) in got_str.lines().zip(expected_str.lines()) {
            if got_line != expected_line {
                println!("Mismatch ⚠️");
                println!("Got:      {}", got_line);
                println!("Expected: {}", expected_line);
            }
        }
    }

    #[test]
    fn test_ephemeral_dust() {
        // potential false positives

        let stats_215049 = {
            let buffer = BufReader::new(File::open("./testdata/215049.json").unwrap());
            let mut de = serde_json::Deserializer::from_reader(buffer);
            let block = Block::deserialize(&mut de).expect("test block json to be valid");
            Stats::from_block(block).expect("testdata blocks should not error")
        };
        // https://mempool.space/tx/f415cbeb5abfd19758a79e984de8e9a1a15ec5cb3bb07f6c816edac13dfcd908#vout=0
        assert_eq!(stats_215049.tx.tx_spending_ephemeral_dust, 0);

        let stats_227154 = {
            let buffer = BufReader::new(File::open("./testdata/227154.json").unwrap());
            let mut de = serde_json::Deserializer::from_reader(buffer);
            let block = Block::deserialize(&mut de).expect("test block json to be valid");
            Stats::from_block(block).expect("testdata blocks should not error")
        };
        // https://mempool.space/tx/e36a2ff16b6b45e3cc873815f899a98b0e923d4d48901f4737c133dc5a740551#vout=1
        assert_eq!(stats_227154.tx.tx_spending_ephemeral_dust, 0);

        let stats_367843 = {
            let buffer = BufReader::new(File::open("./testdata/367843.json").unwrap());
            let mut de = serde_json::Deserializer::from_reader(buffer);
            let block = Block::deserialize(&mut de).expect("test block json to be valid");
            Stats::from_block(block).expect("testdata blocks should not error")
        };
        // https://mempool.space/tx/a842b87403e6d6ca1a9ea39b16d496ebeb6ab15b83acb619cc10daba08114029#vout=0
        assert_eq!(stats_367843.tx.tx_spending_ephemeral_dust, 0);

        // true positives

        let stats_920533 = {
            let buffer = BufReader::new(File::open("./testdata/920533.json").unwrap());
            let mut de = serde_json::Deserializer::from_reader(buffer);
            let block = Block::deserialize(&mut de).expect("test block json to be valid");
            Stats::from_block(block).expect("testdata blocks should not error")
        };
        // https://mempool.space/tx/c660274eea2851d78fc8beffc0a2ff5420599d371560fb6a46a8d0254fa8840d#vin=0
        assert_eq!(stats_920533.tx.tx_spending_ephemeral_dust, 1);

        let stats_913612 = {
            let buffer = BufReader::new(File::open("./testdata/913612.json").unwrap());
            let mut de = serde_json::Deserializer::from_reader(buffer);
            let block = Block::deserialize(&mut de).expect("test block json to be valid");
            Stats::from_block(block).expect("testdata blocks should not error")
        };
        // https://mempool.space/tx/f71f1aeee07e564195fbe643d77951a184747efa9e9d95ce56115478c1ed0323#vout=1
        // https://mempool.space/tx/6bc6b8bfe565cf40b6d2b9c50b31f5824d6e915eb9af1c533344aaeafe558e63#vout=1
        assert_eq!(stats_913612.tx.tx_spending_ephemeral_dust, 2);

        let stats_925262 = {
            let buffer = BufReader::new(File::open("./testdata/925262.json").unwrap());
            let mut de = serde_json::Deserializer::from_reader(buffer);
            let block = Block::deserialize(&mut de).expect("test block json to be valid");
            Stats::from_block(block).expect("testdata blocks should not error")
        };
        // https://mempool.space/tx/aef997c8b3b32d9244805ee99ba4dd6eb4808fe424c2c6da601934909bebda97#vout=1
        // https://mempool.space/tx/af99d2ff40bfef5fabeea2b64d3c84fe5e2ae2a13229fdd904f7520daaa9e92f#vout=1
        // https://mempool.space/tx/885234259164df14a1a110418a2246c80b6528af42779d3fe80da56dd7cd0a58#vout=1
        // https://mempool.space/tx/3aa339e620e0255f761216fdf207d8d18fd5d67696d6d87d25e377f32ac4fce9#vout=1
        // https://mempool.space/tx/35212b331df085522eceec0a5212d92db5f29f8e31db8ea393ace655d90e5edf#vout=1
        // https://mempool.space/tx/8de58c517db1542c5553c96fe1bdab8c08ceb76998cdfb388a4c61ccd1838122#vout=1
        assert_eq!(stats_925262.tx.tx_spending_ephemeral_dust, 6);
    }

    #[test]
    fn test_block_888395() {
        let buffer = BufReader::new(File::open("./testdata/888395.json").unwrap());
        let mut de = serde_json::Deserializer::from_reader(buffer);
        let block = Block::deserialize(&mut de).expect("test block json to be valid");
        let stats = Stats::from_block(block).expect("testdata blocks should not error");

        let expected_stats = Stats {
            block: BlockStats {
                stats_version: STATS_VERSION,
                height: 888395,
                date: "2025-03-18".to_string(),
                version: 0x24cda000,
                nonce: 0x03a672d8,
                bits: 0x17028281,
                difficulty: 112149504190349,
                log2_work: 78.67244,
                size: 1858801,
                stripped_size: 711367,
                vsize: 998170,
                weight: 3992902,
                empty: false,
                coinbase_output_amount: 313534642,
                coinbase_weight: 784,
                transactions: 74,
                payments: 74,
                payments_segwit_spending_tx: 65,
                payments_taproot_spending_tx: 51,
                payments_signaling_explicit_rbf: 65,
                inputs: 17210,
                outputs: 114,
                // This block was mined by MaraPool which has the ID 140
                // https://github.com/bitcoin-data/mining-pools/blob/7eb988330043456189ba6d01fd32811a1f234f2a/pool-list.json#L1518
                pool_id: 140,
            },
            tx: TxStats {
                height: 888395,
                date: "2025-03-18".to_string(),
                tx_version_1: 7,
                tx_version_2: 67,
                tx_version_3: 0,
                tx_version_unknown: 0,
                tx_output_amount: 654788354,
                tx_spending_segwit: 65,
                tx_spending_only_segwit: 65,
                tx_spending_only_legacy: 8,
                tx_spending_only_taproot: 51,
                tx_spending_segwit_and_legacy: 0,
                tx_spending_nested_segwit: 0,
                tx_spending_native_segwit: 65,
                tx_spending_taproot: 51,
                tx_bip69_compliant: 35,
                tx_signaling_explicit_rbf: 66,
                tx_1_input: 51,
                tx_1_output: 48,
                tx_1_input_1_output: 29,
                tx_1_input_2_output: 8,
                tx_spending_newly_created_utxos: 9,
                tx_spending_ephemeral_dust: 0,
                tx_timelock_height: 6,
                tx_timelock_timestamp: 1,
                tx_timelock_not_enforced: 1,
                tx_timelock_too_high: 0,
            },
            input: InputStats {
                height: 888395,
                date: "2025-03-18".to_string(),
                inputs_spending_legacy: 8,
                inputs_spending_segwit: 17201,
                inputs_spending_taproot: 17034,
                inputs_spending_nested_segwit: 0,
                inputs_spending_native_segwit: 17201,
                inputs_spending_multisig: 0,
                inputs_spending_p2ms_multisig: 0,
                inputs_spending_p2sh_multisig: 0,
                inputs_spending_nested_p2wsh_multisig: 0,
                inputs_spending_p2wsh_multisig: 0,
                inputs_p2pk: 0,
                inputs_p2pkh: 8,
                inputs_nested_p2wpkh: 0,
                inputs_p2wpkh: 166,
                inputs_p2ms: 0,
                inputs_p2sh: 0,
                inputs_nested_p2wsh: 0,
                inputs_p2wsh: 0,
                inputs_coinbase: 0,
                inputs_witness_coinbase: 1,
                inputs_p2tr_keypath: 17000,
                inputs_p2tr_scriptpath: 34,
                inputs_p2a: 1,
                inputs_p2a_dust: 0,
                inputs_unknown: 0,
                inputs_spend_in_same_block: 9,
            },
            output: OutputStats {
                height: 888395,
                date: "2025-03-18".to_string(),
                outputs_p2pk: 0,
                outputs_p2pkh: 3,
                outputs_p2wpkh: 38,
                outputs_p2ms: 0,
                outputs_p2sh: 3,
                outputs_p2wsh: 0,
                outputs_opreturn: 15,
                outputs_p2tr: 54,
                outputs_p2a: 1,
                outputs_p2a_dust: 0,
                outputs_unknown: 0,
                outputs_p2pk_amount: 0,
                outputs_p2pkh_amount: 317791242,
                outputs_p2wpkh_amount: 326633717,
                outputs_p2ms_amount: 0,
                outputs_p2sh_amount: 12155,
                outputs_p2wsh_amount: 0,
                outputs_p2tr_amount: 10350490,
                outputs_p2a_amount: 750,
                outputs_opreturn_amount: 0,
                outputs_unknown_amount: 0,
                outputs_opreturn_bip47_payment_code: 0,
                outputs_opreturn_coinbase_coredao: 0,
                outputs_opreturn_coinbase_exsat: 0,
                outputs_opreturn_coinbase_hathor: 0,
                outputs_opreturn_coinbase_rsk: 0,
                outputs_opreturn_coinbase_witness_commitment: 1,
                outputs_opreturn_omnilayer: 0,
                outputs_opreturn_runestone: 13,
                outputs_opreturn_stacks_block_commit: 0,
                outputs_opreturn_bytes: 103,
            },
            script: ScriptStats {
                height: 888395,
                date: "2025-03-18".to_string(),
                pubkeys: 228,
                pubkeys_compressed: 228,
                pubkeys_uncompressed: 0,
                pubkeys_compressed_inputs: 174,
                pubkeys_uncompressed_inputs: 0,
                pubkeys_compressed_outputs: 54,
                pubkeys_uncompressed_outputs: 0,
                sigs_schnorr: 17034,
                sigs_ecdsa: 174,
                sigs_ecdsa_not_strict_der: 0,
                sigs_ecdsa_strict_der: 174,
                sigs_ecdsa_length_less_70byte: 0,
                sigs_ecdsa_length_70byte: 0,
                sigs_ecdsa_length_71byte: 93,
                sigs_ecdsa_length_72byte: 81,
                sigs_ecdsa_length_73byte: 0,
                sigs_ecdsa_length_74byte: 0,
                sigs_ecdsa_length_75byte_or_more: 0,
                sigs_ecdsa_low_r: 92,
                sigs_ecdsa_high_r: 82,
                sigs_ecdsa_low_s: 174,
                sigs_ecdsa_high_s: 0,
                sigs_ecdsa_high_rs: 0,
                sigs_ecdsa_low_rs: 92,
                sigs_ecdsa_low_r_high_s: 0,
                sigs_ecdsa_high_r_low_s: 82,
                sigs_sighashes: 174,
                sigs_sighash_all: 174,
                sigs_sighash_none: 0,
                sigs_sighash_single: 0,
                sigs_sighash_all_acp: 0,
                sigs_sighash_none_acp: 0,
                sigs_sighash_single_acp: 0,
            },
            feerate: FeerateStats {
                height: 888395,
                date: "2025-03-18".to_string(),
                fee_min: 142,
                fee_5th_percentile: 166,
                fee_10th_percentile: 166,
                fee_25th_percentile: 166,
                fee_35th_percentile: 191,
                fee_50th_percentile: 202,
                fee_65th_percentile: 361,
                fee_75th_percentile: 6197,
                fee_90th_percentile: 59271,
                fee_95th_percentile: 59271,
                fee_max: 59271,
                fee_sum: 1034642,
                fee_avg: 14173.178f32,
                size_min: 77,
                size_5th_percentile: 189,
                size_10th_percentile: 189,
                size_25th_percentile: 320,
                size_35th_percentile: 320,
                size_50th_percentile: 320,
                size_65th_percentile: 353,
                size_75th_percentile: 7801,
                size_90th_percentile: 107057,
                size_95th_percentile: 107057,
                size_max: 107057,
                size_avg: 25458.863,
                size_sum: 1858497,
                feerate_min: 1.0,
                feerate_5th_percentile: 1.010582f32,
                feerate_10th_percentile: 1.010582f32,
                feerate_25th_percentile: 1.0297971f32,
                feerate_35th_percentile: 1.0297971f32,
                feerate_50th_percentile: 1.0412371f32,
                feerate_65th_percentile: 1.0993377f32,
                feerate_75th_percentile: 1.0993377f32,
                feerate_90th_percentile: 2.0282176f32,
                feerate_95th_percentile: 3.5460992f32,
                feerate_max: 31.0f32,
                feerate_avg: 1.7306631f32,
                // TODO: Transaction package feerate stats are not yet implemented.
                feerate_package_min: 0.0f32,
                feerate_package_5th_percentile: 0.0f32,
                feerate_package_10th_percentile: 0.0f32,
                feerate_package_25th_percentile: 0.0f32,
                feerate_package_35th_percentile: 0.0f32,
                feerate_package_50th_percentile: 0.0f32,
                feerate_package_65th_percentile: 0.0f32,
                feerate_package_75th_percentile: 0.0f32,
                feerate_package_90th_percentile: 0.0f32,
                feerate_package_95th_percentile: 0.0f32,
                feerate_package_max: 0.0f32,
                feerate_package_avg: 0.0f32,

                below_1_sat_vbyte: 0,
                zero_fee_tx: 0,
                feerate_1_2_sat_vbyte: 66,
                feerate_2_5_sat_vbyte: 5,
                feerate_5_10_sat_vbyte: 1,
                feerate_10_25_sat_vbyte: 0,
                feerate_25_50_sat_vbyte: 1,
                feerate_50_100_sat_vbyte: 0,
                feerate_100_250_sat_vbyte: 0,
                feerate_250_500_sat_vbyte: 0,
                feerate_500_1000_sat_vbyte: 0,
                feerate_1000_plus_sat_vbyte: 0,
            },
        };

        diff_stats(&stats, &expected_stats);
        assert_eq!(stats, expected_stats, "see diff above");
    }

    #[test]
    fn test_block_739990() {
        let buffer = BufReader::new(File::open("./testdata/739990.json").unwrap());
        let mut de = serde_json::Deserializer::from_reader(buffer);
        let block = Block::deserialize(&mut de).expect("test block json to be valid");
        let stats = Stats::from_block(block).expect("testdata blocks should not error");

        let expected_stats = Stats {
            block: BlockStats {
                stats_version: STATS_VERSION,
                height: 739990,
                date: "2022-06-09".to_string(),
                version: 0x20000000,
                nonce: 0x33ca7510,
                bits: 0x17094b6a,
                difficulty: 30283293547736,
                log2_work: 76.78361,
                size: 536844,
                stripped_size: 225535,
                vsize: 303595,
                weight: 1213449,
                empty: false,
                coinbase_output_amount: 626983001,
                coinbase_weight: 1272,
                transactions: 645,
                payments: 1406,
                payments_segwit_spending_tx: 1307,
                payments_taproot_spending_tx: 1,
                payments_signaling_explicit_rbf: 280,
                inputs: 2170,
                outputs: 1882,
                // This block was mined by Binance Pool which has the ID 123
                // https://github.com/bitcoin-data/mining-pools/blob/7eb988330043456189ba6d01fd32811a1f234f2a/pool-list.json#L1330C11-L1330C14
                pool_id: 123,
            },
            tx: TxStats {
                height: 739990,
                date: "2022-06-09".to_string(),
                tx_version_1: 271,
                tx_version_2: 374,
                tx_version_3: 0,
                tx_version_unknown: 0,
                tx_output_amount: 125054585129,
                tx_spending_segwit: 562,
                tx_spending_only_segwit: 553,
                tx_spending_only_legacy: 82,
                tx_spending_only_taproot: 1,
                tx_spending_segwit_and_legacy: 9,
                tx_spending_nested_segwit: 126,
                tx_spending_native_segwit: 443,
                tx_spending_taproot: 1,
                tx_bip69_compliant: 391,
                tx_signaling_explicit_rbf: 210,
                tx_1_input: 499,
                tx_1_output: 177,
                tx_1_input_1_output: 112,
                tx_1_input_2_output: 339,
                tx_spending_newly_created_utxos: 110,
                tx_spending_ephemeral_dust: 0,
                tx_timelock_height: 209,
                tx_timelock_timestamp: 0,
                tx_timelock_not_enforced: 22,
                tx_timelock_too_high: 0,
            },
            input: InputStats {
                height: 739990,
                date: "2022-06-09".to_string(),
                inputs_spending_legacy: 239,
                inputs_spending_segwit: 1930,
                inputs_spending_taproot: 1,
                inputs_spending_nested_segwit: 1327,
                inputs_spending_native_segwit: 603,
                inputs_spending_multisig: 738,
                inputs_spending_p2ms_multisig: 0,
                inputs_spending_p2sh_multisig: 28,
                inputs_spending_nested_p2wsh_multisig: 672,
                inputs_spending_p2wsh_multisig: 38,
                inputs_p2pk: 0,
                inputs_p2pkh: 211,
                inputs_nested_p2wpkh: 654,
                inputs_p2wpkh: 557,
                inputs_p2ms: 0,
                inputs_p2sh: 28,
                inputs_nested_p2wsh: 673,
                inputs_p2wsh: 45,
                inputs_coinbase: 0,
                inputs_witness_coinbase: 1,
                inputs_p2tr_keypath: 1,
                inputs_p2tr_scriptpath: 0,
                inputs_p2a: 0,
                inputs_p2a_dust: 0,
                inputs_unknown: 0,
                inputs_spend_in_same_block: 110,
            },
            output: OutputStats {
                height: 739990,
                date: "2022-06-09".to_string(),
                outputs_p2pk: 0,
                outputs_p2pkh: 332,
                outputs_p2wpkh: 652,
                outputs_p2ms: 0,
                outputs_p2sh: 802,
                outputs_p2wsh: 76,
                outputs_opreturn: 13,
                outputs_p2tr: 7,
                outputs_p2a: 0,
                outputs_p2a_dust: 0,
                outputs_unknown: 0,
                outputs_p2pk_amount: 0,
                outputs_p2pkh_amount: 33803517254,
                outputs_p2wpkh_amount: 58286402491,
                outputs_p2ms_amount: 0,
                outputs_p2sh_amount: 21310299474,
                outputs_p2wsh_amount: 11638052422,
                outputs_p2tr_amount: 16313488,
                outputs_p2a_amount: 0,
                outputs_opreturn_amount: 0,
                outputs_unknown_amount: 0,
                outputs_opreturn_bip47_payment_code: 0,
                outputs_opreturn_coinbase_coredao: 0,
                outputs_opreturn_coinbase_exsat: 0,
                outputs_opreturn_coinbase_hathor: 0,
                outputs_opreturn_coinbase_rsk: 1,
                outputs_opreturn_coinbase_witness_commitment: 1,
                outputs_opreturn_omnilayer: 0,
                outputs_opreturn_runestone: 0,
                outputs_opreturn_stacks_block_commit: 6,
                outputs_opreturn_bytes: 799,
            },
            script: ScriptStats {
                height: 739990,
                date: "2022-06-09".to_string(),
                pubkeys: 3621,
                pubkeys_compressed: 3618,
                pubkeys_uncompressed: 3,
                pubkeys_compressed_inputs: 3611,
                pubkeys_uncompressed_inputs: 3,
                pubkeys_compressed_outputs: 7,
                pubkeys_uncompressed_outputs: 0,
                sigs_schnorr: 1,
                sigs_ecdsa: 2912,
                sigs_ecdsa_not_strict_der: 0,
                sigs_ecdsa_strict_der: 2912,
                sigs_ecdsa_length_less_70byte: 0,
                sigs_ecdsa_length_70byte: 7,
                sigs_ecdsa_length_71byte: 2060,
                sigs_ecdsa_length_72byte: 845,
                sigs_ecdsa_length_73byte: 0,
                sigs_ecdsa_length_74byte: 0,
                sigs_ecdsa_length_75byte_or_more: 0,
                sigs_ecdsa_low_r: 2066,
                sigs_ecdsa_high_r: 846,
                sigs_ecdsa_low_s: 2912,
                sigs_ecdsa_high_s: 0,
                sigs_ecdsa_high_rs: 0,
                sigs_ecdsa_low_rs: 2066,
                sigs_ecdsa_low_r_high_s: 0,
                sigs_ecdsa_high_r_low_s: 846,
                sigs_sighashes: 2912,
                sigs_sighash_all: 2910,
                sigs_sighash_none: 0,
                sigs_sighash_single: 0,
                sigs_sighash_all_acp: 2,
                sigs_sighash_none_acp: 0,
                sigs_sighash_single_acp: 0,
            },
            feerate: FeerateStats {
                height: 739990,
                date: "2022-06-09".to_string(),
                fee_min: 122,
                fee_5th_percentile: 250,
                fee_10th_percentile: 285,
                fee_25th_percentile: 380,
                fee_35th_percentile: 617,
                fee_50th_percentile: 1017,
                fee_65th_percentile: 1425,
                fee_75th_percentile: 2158,
                fee_90th_percentile: 5225,
                fee_95th_percentile: 10710,
                fee_max: 283020,
                fee_sum: 1983001,
                fee_avg: 3079.194f32,
                size_min: 188,
                size_5th_percentile: 192,
                size_10th_percentile: 194,
                size_25th_percentile: 223,
                size_35th_percentile: 223,
                size_50th_percentile: 225,
                size_65th_percentile: 340,
                size_75th_percentile: 372,
                size_90th_percentile: 631,
                size_95th_percentile: 1105,
                size_max: 65782,
                size_avg: 832.9441,
                size_sum: 536416,
                feerate_min: 1.0f32,
                feerate_5th_percentile: 1.2539445f32,
                feerate_10th_percentile: 2.006713f32,
                feerate_25th_percentile: 2.0300858f32,
                feerate_35th_percentile: 3.0f32,
                feerate_50th_percentile: 5.9503546f32,
                feerate_65th_percentile: 8.430503f32,
                feerate_75th_percentile: 9.073471f32,
                feerate_90th_percentile: 18.744535f32,
                feerate_95th_percentile: 26.2769f32,
                feerate_max: 233.64487f32,
                feerate_avg: 10.158637f32,
                // TODO: Transaction package feerate stats are not yet implemented.
                feerate_package_min: 0.0f32,
                feerate_package_5th_percentile: 0.0f32,
                feerate_package_10th_percentile: 0.0f32,
                feerate_package_25th_percentile: 0.0f32,
                feerate_package_35th_percentile: 0.0f32,
                feerate_package_50th_percentile: 0.0f32,
                feerate_package_65th_percentile: 0.0f32,
                feerate_package_75th_percentile: 0.0f32,
                feerate_package_90th_percentile: 0.0f32,
                feerate_package_95th_percentile: 0.0f32,
                feerate_package_max: 0.0f32,
                feerate_package_avg: 0.0f32,

                below_1_sat_vbyte: 0,
                zero_fee_tx: 0,
                feerate_1_2_sat_vbyte: 48,
                feerate_2_5_sat_vbyte: 247,
                feerate_5_10_sat_vbyte: 202,
                feerate_10_25_sat_vbyte: 110,
                feerate_25_50_sat_vbyte: 21,
                feerate_50_100_sat_vbyte: 6,
                feerate_100_250_sat_vbyte: 10,
                feerate_250_500_sat_vbyte: 0,
                feerate_500_1000_sat_vbyte: 0,
                feerate_1000_plus_sat_vbyte: 0,
            },
        };

        diff_stats(&stats, &expected_stats);
        assert_eq!(stats, expected_stats, "see diff above");
    }

    #[test]
    fn test_block_361582() {
        let buffer = BufReader::new(File::open("./testdata/361582.json").unwrap());
        let mut de = serde_json::Deserializer::from_reader(buffer);
        let block = Block::deserialize(&mut de).expect("test block json to be valid");
        let stats = Stats::from_block(block).expect("testdata blocks should not error");

        let expected_stats = Stats {
            block: BlockStats {
                stats_version: STATS_VERSION,
                height: 361582,
                date: "2015-06-19".to_string(),
                version: 2,
                nonce: 0x444386f8,
                bits: 0x18162043,
                difficulty: 49692386354,
                log2_work: 67.532326,
                size: 163491,
                stripped_size: 163491,
                vsize: 163408,
                weight: 653964,
                empty: false,
                coinbase_output_amount: 2503687509,
                coinbase_weight: 408,
                transactions: 277,
                payments: 345,
                payments_segwit_spending_tx: 0,
                payments_taproot_spending_tx: 0,
                payments_signaling_explicit_rbf: 0,
                inputs: 919,
                outputs: 591,
                // This block was mined by MegaBigPower which has the ID 39
                // https://github.com/bitcoin-data/mining-pools/blob/7eb988330043456189ba6d01fd32811a1f234f2a/pool-list.json#L388-L401
                pool_id: 39,
            },
            tx: TxStats {
                height: 361582,
                date: "2015-06-19".to_string(),
                tx_version_1: 277,
                tx_version_2: 0,
                tx_version_3: 0,
                tx_version_unknown: 0,
                tx_output_amount: 305829530827,
                tx_spending_segwit: 0,
                tx_spending_only_segwit: 0,
                tx_spending_only_legacy: 276,
                tx_spending_only_taproot: 0,
                tx_spending_segwit_and_legacy: 0,
                tx_spending_nested_segwit: 0,
                tx_spending_native_segwit: 0,
                tx_spending_taproot: 0,
                tx_bip69_compliant: 116,
                tx_signaling_explicit_rbf: 0,
                tx_1_input: 146,
                tx_1_output: 31,
                tx_1_input_1_output: 16,
                tx_1_input_2_output: 125,
                tx_spending_newly_created_utxos: 45,
                tx_spending_ephemeral_dust: 0,
                tx_timelock_height: 1,
                tx_timelock_timestamp: 0,
                tx_timelock_not_enforced: 0,
                tx_timelock_too_high: 0,
            },
            input: InputStats {
                height: 361582,
                date: "2015-06-19".to_string(),
                inputs_spending_legacy: 918,
                inputs_spending_segwit: 0,
                inputs_spending_taproot: 0,
                inputs_spending_nested_segwit: 0,
                inputs_spending_native_segwit: 0,
                inputs_spending_multisig: 19,
                inputs_spending_p2ms_multisig: 0,
                inputs_spending_p2sh_multisig: 19,
                inputs_spending_nested_p2wsh_multisig: 0,
                inputs_spending_p2wsh_multisig: 0,
                inputs_p2pk: 0,
                inputs_p2pkh: 898,
                inputs_nested_p2wpkh: 0,
                inputs_p2wpkh: 0,
                inputs_p2ms: 0,
                inputs_p2sh: 20,
                inputs_nested_p2wsh: 0,
                inputs_p2wsh: 0,
                inputs_coinbase: 1,
                inputs_witness_coinbase: 0,
                inputs_p2tr_keypath: 0,
                inputs_p2tr_scriptpath: 0,
                inputs_p2a: 0,
                inputs_p2a_dust: 0,
                inputs_unknown: 0,
                inputs_spend_in_same_block: 52,
            },
            output: OutputStats {
                height: 361582,
                date: "2015-06-19".to_string(),
                outputs_p2pk: 0,
                outputs_p2pkh: 568,
                outputs_p2wpkh: 0,
                outputs_p2ms: 0,
                outputs_p2sh: 23,
                outputs_p2wsh: 0,
                outputs_opreturn: 0,
                outputs_p2tr: 0,
                outputs_p2a: 0,
                outputs_p2a_dust: 0,
                outputs_unknown: 0,
                outputs_p2pk_amount: 0,
                outputs_p2pkh_amount: 240283730043,
                outputs_p2wpkh_amount: 0,
                outputs_p2ms_amount: 0,
                outputs_p2sh_amount: 65545800784,
                outputs_p2wsh_amount: 0,
                outputs_p2tr_amount: 0,
                outputs_p2a_amount: 0,
                outputs_opreturn_amount: 0,
                outputs_unknown_amount: 0,
                outputs_opreturn_bip47_payment_code: 0,
                outputs_opreturn_coinbase_coredao: 0,
                outputs_opreturn_coinbase_exsat: 0,
                outputs_opreturn_coinbase_hathor: 0,
                outputs_opreturn_coinbase_rsk: 0,
                outputs_opreturn_coinbase_witness_commitment: 0,
                outputs_opreturn_omnilayer: 0,
                outputs_opreturn_runestone: 0,
                outputs_opreturn_stacks_block_commit: 0,
                outputs_opreturn_bytes: 0,
            },
            script: ScriptStats {
                height: 361582,
                date: "2015-06-19".to_string(),
                pubkeys: 946,
                pubkeys_compressed: 860,
                pubkeys_uncompressed: 86,
                pubkeys_compressed_inputs: 860,
                pubkeys_uncompressed_inputs: 86,
                pubkeys_compressed_outputs: 0,
                pubkeys_uncompressed_outputs: 0,
                sigs_schnorr: 0,
                sigs_ecdsa: 935,
                sigs_ecdsa_not_strict_der: 0,
                sigs_ecdsa_strict_der: 935,
                sigs_ecdsa_length_less_70byte: 0,
                sigs_ecdsa_length_70byte: 3,
                sigs_ecdsa_length_71byte: 438,
                sigs_ecdsa_length_72byte: 451,
                sigs_ecdsa_length_73byte: 43,
                sigs_ecdsa_length_74byte: 0,
                sigs_ecdsa_length_75byte_or_more: 0,
                sigs_ecdsa_low_r: 470,
                sigs_ecdsa_high_r: 465,
                sigs_ecdsa_low_s: 862,
                sigs_ecdsa_high_s: 73,
                sigs_ecdsa_high_rs: 43,
                sigs_ecdsa_low_rs: 440,
                sigs_ecdsa_low_r_high_s: 30,
                sigs_ecdsa_high_r_low_s: 422,
                sigs_sighashes: 935,
                sigs_sighash_all: 935,
                sigs_sighash_none: 0,
                sigs_sighash_single: 0,
                sigs_sighash_all_acp: 0,
                sigs_sighash_none_acp: 0,
                sigs_sighash_single_acp: 0,
            },
            feerate: FeerateStats {
                height: 361582,
                date: "2015-06-19".to_string(),
                fee_min: 242,
                fee_5th_percentile: 10000,
                fee_10th_percentile: 10000,
                fee_25th_percentile: 10000,
                fee_35th_percentile: 10000,
                fee_50th_percentile: 10000,
                fee_65th_percentile: 10000,
                fee_75th_percentile: 10000,
                fee_90th_percentile: 12723,
                fee_95th_percentile: 29414,
                fee_max: 180221,
                fee_sum: 3687509,
                fee_avg: 13360.54f32,
                size_min: 87,
                size_5th_percentile: 223,
                size_10th_percentile: 225,
                size_25th_percentile: 226,
                size_35th_percentile: 226,
                size_50th_percentile: 337,
                size_65th_percentile: 374,
                size_75th_percentile: 409,
                size_90th_percentile: 737,
                size_95th_percentile: 943,
                size_max: 52545,
                size_avg: 591.6884,
                size_sum: 163306,
                feerate_min: 1.00492,
                feerate_5th_percentile: 10.435551f32,
                feerate_10th_percentile: 13.185673f32,
                feerate_25th_percentile: 22.883295f32,
                feerate_35th_percentile: 26.741552f32,
                feerate_50th_percentile: 29.674635f32,
                feerate_65th_percentile: 44.247787f32,
                feerate_75th_percentile: 44.444443f32,
                feerate_90th_percentile: 44.873833f32,
                feerate_95th_percentile: 64.15262f32,
                feerate_max: 444.44446f32,
                feerate_avg: 40.540836f32,
                // TODO: Transaction package feerate stats are not yet implemented.
                feerate_package_min: 0.0f32,
                feerate_package_5th_percentile: 0.0f32,
                feerate_package_10th_percentile: 0.0f32,
                feerate_package_25th_percentile: 0.0f32,
                feerate_package_35th_percentile: 0.0f32,
                feerate_package_50th_percentile: 0.0f32,
                feerate_package_65th_percentile: 0.0f32,
                feerate_package_75th_percentile: 0.0f32,
                feerate_package_90th_percentile: 0.0f32,
                feerate_package_95th_percentile: 0.0f32,
                feerate_package_max: 0.0f32,
                feerate_package_avg: 0.0f32,

                below_1_sat_vbyte: 0,
                zero_fee_tx: 0,
                feerate_1_2_sat_vbyte: 5,
                feerate_2_5_sat_vbyte: 5,
                feerate_5_10_sat_vbyte: 3,
                feerate_10_25_sat_vbyte: 69,
                feerate_25_50_sat_vbyte: 170,
                feerate_50_100_sat_vbyte: 15,
                feerate_100_250_sat_vbyte: 4,
                feerate_250_500_sat_vbyte: 5,
                feerate_500_1000_sat_vbyte: 0,
                feerate_1000_plus_sat_vbyte: 0,
            },
        };

        diff_stats(&stats, &expected_stats);
        assert_eq!(stats, expected_stats, "see diff above");
    }
}
