use crate::b64::{b64_format, b64_o_format};
use actix::prelude::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use terra_rust_api::client::tendermint_types::Block;
use terra_rust_api::client::tx_types::TxResultBlockMsg;
use terra_rust_api::core_types::Coin;
use terra_rust_api::{terra_datetime_format, terra_u64_format};

/// new_block type from observer
#[derive(Deserialize, Serialize, Debug)]
pub struct NewBlock {
    /// The chain
    pub chain_id: String,
    /// The type of message
    #[serde(rename = "type")]
    pub s_type: String,
    pub data: NewBlockData,
}

/// new_block 'data'
#[derive(Deserialize, Serialize, Debug)]
pub struct NewBlockData {
    /// The chain
    pub block: Block,
    pub result_begin_block: NewBlockBeginBlock,
    pub result_end_block: NewBlockEndBlock,
    pub txs: Option<Vec<TXandResult>>,
    pub supply: Vec<Coin>,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TXandResult {
    #[serde(with = "terra_u64_format")]
    pub height: u64,
    pub txhash: String,
    pub raw_log: String,
    pub logs: Option<Vec<TxResultBlockMsg>>,
    #[serde(with = "terra_u64_format")]
    pub gas_wanted: u64,
    #[serde(with = "terra_u64_format")]
    pub gas_used: u64,
    pub tx: TxOuter,
    #[serde(with = "terra_datetime_format")]
    pub timestamp: DateTime<Utc>,
}
#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessageTX {
    pub tx: TXandResult,
}

#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct TxOuter {
    #[serde(rename = "type")]
    pub s_type: String,
    pub value: BlockTransaction,
}
#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct BlockTransaction {
    pub msg: Vec<BlockMsg>,
    pub fee: Fee,
    pub signatures: Vec<TransactionSignature>,
    pub memo: String,
}
#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct BlockMsg {
    #[serde(rename = "type")]
    pub s_type: String,
    pub value: serde_json::Value,
}
#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct Fee {
    pub amount: Vec<Coin>,
    #[serde(with = "terra_u64_format")]
    pub gas: u64,
}

#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct TransactionSignature {
    pub pub_key: TransactionSignaturePubKey,
    pub signature: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct NewBlockBeginBlock {
    pub events: Vec<NewBlockEvent>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct NewBlockEvent {
    #[serde(rename = "type")]
    pub s_type: String,
    pub attributes: Vec<NewBlockAttributes>,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct NewBlockAttributes {
    #[serde(with = "b64_format")]
    pub key: String,
    #[serde(with = "b64_o_format")]
    pub value: Option<String>,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct NewBlockEndBlock {
    pub validator_updates: Vec<NewBlockValidatorUpdate>,
    pub events: Option<Vec<NewBlockEvent>>,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct PubKey {
    #[serde(rename = "type")]
    pub s_type: String,
    pub data: String,
}
#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct TransactionSignaturePubKey {
    #[serde(rename = "type")]
    pub s_type: String,
    pub value: String,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct NewBlockValidatorUpdate {
    pub pub_key: PubKey,
    #[serde(with = "terra_u64_format")]
    pub power: u64,
}