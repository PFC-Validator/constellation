use actix::prelude::*;

use crate::types::TXandResult;
use rust_decimal::Decimal;
use terra_rust_api::core_types::Coin;
use terra_rust_api::staking_types::Validator;

#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessageTX {
    pub tx: TXandResult,
}
#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessageBlockEventReward {
    pub height: u64,
    pub is_begin: bool,
    pub is_proposer: bool,
    pub validator: String,
    pub amount: Vec<Coin>,
}
#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessageBlockEventCommission {
    pub height: u64,
    pub is_begin: bool,
    pub validator: String,
    pub amount: Vec<Coin>,
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessagePriceDrift {
    pub height: u64,
    pub operator_address: String,
    pub denom: String,
    pub average: Decimal,
    pub weighted_average: Decimal,
    pub submitted: Decimal,
    pub txhash: String,
}
#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessagePriceAbstain {
    pub height: u64,
    pub operator_address: String,
    pub denoms: Vec<String>,
    pub txhash: String,
}
#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessageValidatorStakedTotal {
    pub height: u64,
    pub operator_address: String,
    pub tokens: u64,
}
#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessageValidatorStakedDelta {
    pub height: u64,
    pub operator_address: String,
    pub token_delta: Decimal,
}

/// Sent when system refreshes validator record via LCD
#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessageValidator {
    pub height: u64,
    pub operator_address: String,
    pub validator: Validator,
}

/// Sent when system wants to notify places that an event on a validator occurred
#[derive(Clone, Debug)]
pub enum ValidatorEventType {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR,
    CRITICAL,
}
#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessageValidatorEvent {
    pub height: u64,
    pub operator_address: String,
    pub moniker: Option<String>,
    pub event_type: ValidatorEventType,
    pub message: String,
    pub hash: Option<String>,
}
