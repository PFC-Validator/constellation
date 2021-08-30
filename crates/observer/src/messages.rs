use actix::prelude::*;

use crate::types::TXandResult;
use rust_decimal::Decimal;

#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessageTX {
    pub tx: TXandResult,
}
#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessagePriceDrift {
    pub height: u64,
    pub operator_address: String,
    pub denom: String,
    pub average: Decimal,
    pub submitted: Decimal,
    pub txhash: String,
}
#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessagePriceAbstain {
    pub height: u64,
    pub operator_address: String,
    pub denom: String,
    pub txhash: String,
}
#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessageValidatorStakedTotal {
    pub height: u64,
    pub operator_address: String,
    pub tokens: Decimal,
}
#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessageValidatorStakedDelta {
    pub height: u64,
    pub operator_address: String,
    pub token_delta: Decimal,
}
