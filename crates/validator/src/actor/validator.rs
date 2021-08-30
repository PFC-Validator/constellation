use std::collections::HashMap;

use actix::prelude::*;
use actix_broker::BrokerSubscribe;
use chrono::{DateTime, Utc};
//use rust_decimal::Decimal;
use terra_rust_api::staking_types::Validator;
use terra_rust_api::Terra;

use constellation_observer::messages::{MessagePriceAbstain, MessagePriceDrift};
use constellation_observer::BrokerType;
use std::collections::hash_map::Entry;

#[derive(Clone, Debug)]
pub struct ValidatorDetails {
    pub validator: Validator,

    pub first_seen_date: DateTime<Utc>,
    pub last_updated_date: DateTime<Utc>,
    pub first_seen_block: u64,
    pub last_updated_block: u64,
    pub abstains: u64,
    pub drifts: u64,
}
pub struct ValidatorActor {
    pub validators: HashMap<String, ValidatorDetails>,
    pub moniker: HashMap<String, String>,
}
impl ValidatorActor {
    pub async fn create(lcd: &str, chain: &str) -> anyhow::Result<ValidatorActor> {
        let terra = Terra::lcd_client_no_tx(lcd, chain).await?;
        let validator_result = terra.staking().validators().await?;

        Ok(ValidatorActor::create_from_validator_list(
            validator_result.height,
            validator_result.result,
        ))
    }
    pub fn create_from_validator_list(
        height: u64,
        validator_list: Vec<Validator>,
    ) -> ValidatorActor {
        let now = Utc::now();
        let mut validators: HashMap<String, ValidatorDetails> = Default::default();
        let mut moniker: HashMap<String, String> = Default::default();
        validator_list.iter().for_each(|v| {
            validators.insert(
                v.operator_address.clone(),
                ValidatorDetails {
                    validator: v.clone(),
                    first_seen_date: now,
                    last_updated_date: now,
                    first_seen_block: height,
                    last_updated_block: height,
                    abstains: 0,
                    drifts: 0,
                },
            );
            moniker.insert(v.description.moniker.clone(), v.operator_address.clone());
        });
        ValidatorActor {
            validators,
            moniker,
        }
    }
}
impl Actor for ValidatorActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.subscribe_sync::<BrokerType, MessagePriceDrift>(ctx);
        self.subscribe_sync::<BrokerType, MessagePriceAbstain>(ctx);
    }
}

impl Handler<MessagePriceAbstain> for ValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: MessagePriceAbstain, _ctx: &mut Self::Context) {
        let height = msg.height;
        let now = Utc::now();
        match self.validators.entry(msg.operator_address.clone()) {
            Entry::Occupied(mut e) => {
                let mut v = e.get_mut();
                v.abstains = v.abstains + 1;
                v.last_updated_block = height;
                v.last_updated_date = now;
                //   e.into_mut();
                log::info!(
                    "{} '{}' {} has abstained from voting for this denomination Abstains:{} Drifts:{} - {}",
                    height,
                    v.validator.description.moniker,
                    msg.denom,
                    v.abstains,
                    v.drifts,
                    msg.txhash,
                );
            }
            Entry::Vacant(_) => {
                log::error!("Validator not found ? {}", msg.operator_address)
            }
        }
        /*
        if let Some(v) = self.validators.get_mut(&msg.operator_address) {
            v.abstains = v.abstains + 1;
            v.last_updated_block = height;
            v.last_updated_date = now;
            self.validators.insert(msg.operator_address.clone(), *v);
            log::info!(
                "{} '{}' {} has abstained from voting for this denomination - {}",
                height,
                v.validator.description.moniker,
                msg.denom,
                msg.txhash
            );
        } else {
            log::error!("Validator not found ? {}", msg.operator_address)
        }*/
    }
}
impl Handler<MessagePriceDrift> for ValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: MessagePriceDrift, _ctx: &mut Self::Context) {
        let now = Utc::now();
        let height = msg.height;
        match self.validators.entry(msg.operator_address.clone()) {
            Entry::Occupied(mut e) => {
                let mut v = e.get_mut();
                v.drifts = v.drifts + 1;
                v.last_updated_block = height;
                v.last_updated_date = now;

                log::info!(
                    "{} '{}' {} price drift submitted {:4} which is too far away from {:4} Abstains:{} Drifts:{}  - {}",
                    height,
                    v.validator.description.moniker,
                    msg.denom,
                    msg.submitted,
                    msg.average,
                    v.abstains,
                    v.drifts,
                    msg.txhash
                );
            }
            Entry::Vacant(_) => {
                log::error!("Validator not found ? {}", msg.operator_address)
            }
        }
        if let Some(v) = self.validators.get(&msg.operator_address) {
            if msg.denom.eq("uusd") || msg.denom.eq("ukrw") {
                log::info!(
                    "{} '{}' {} price drift submitted {} which is too far away from {} - {}",
                    height,
                    v.validator.description.moniker,
                    msg.denom,
                    msg.submitted,
                    msg.average,
                    msg.txhash
                );
            }
        } else {
            log::error!("Validator not found ? {}", msg.operator_address)
        }
    }
}
