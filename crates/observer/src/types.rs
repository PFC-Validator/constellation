use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use terra_rust_api::client::tendermint_types::Block;
use terra_rust_api::client::tx_types::TxResultBlockMsg;
use terra_rust_api::core_types::Coin;
use terra_rust_api::{terra_datetime_format, terra_u64_format};

use crate::b64::{b64_format, b64_o_format};
use serde_json::Value;
use std::collections::HashMap;

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
    // body
    // signatures
    // auth_info
}

#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct TxOuter {
    #[serde(rename = "@type")]
    pub s_type: String,
    pub body: BlockTransaction,
}
#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct BlockTransaction {
    pub messages: Vec<Value>,
    //pub fee: Fee,
    //pub signatures: Vec<String>,
    pub memo: String,
}
#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct BlockMsg {
    #[serde(rename = "@type")]
    pub s_type: String,
    pub execute_msg: Option<serde_json::Value>,
    // sender
    // contract
    // coins
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

#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct NewBlockBeginBlock {
    pub events: Vec<NewBlockEvent>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct NewBlockEvent {
    #[serde(rename = "type")]
    pub s_type: String,
    pub attributes: Vec<NewBlockAttributes>,
}
impl NewBlockEvent {
    pub fn attribute_map(&self) -> HashMap<String, Option<String>> {
        self.attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect::<HashMap<String, Option<String>>>()
    }
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct NewBlockAttributes {
    #[serde(with = "b64_format")]
    pub key: String,
    #[serde(with = "b64_o_format")]
    pub value: Option<String>,
    pub index: bool,
}
#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct NewBlockEndBlock {
    pub validator_updates: Vec<NewBlockValidatorUpdate>,
    pub events: Option<Vec<NewBlockEvent>>,
}
#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct PubKey {
    #[serde(rename = "@type")]
    pub s_type: String,
    pub data: String,
}
#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct TransactionSignaturePubKey {
    // #[serde(rename = "type")]
    // pub s_type: String,
    pub value: Value, // this can either be a string, or a array if type == tendermint/PubKeyMultisigThreshold
}
#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct NewBlockValidatorUpdate {
    // todo fix it l8r
    pub pub_key: Value,
    #[serde(with = "terra_u64_format")]
    pub power: u64,
}
