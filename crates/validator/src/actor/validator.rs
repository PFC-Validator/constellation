use std::collections::HashMap;

use actix::prelude::*;
use actix_broker::{Broker, BrokerSubscribe, SystemBroker};
use chrono::{DateTime, Utc};
//use rust_decimal::Decimal;
use terra_rust_api::staking_types::Validator;
use terra_rust_api::Terra;

use constellation_observer::messages::{
    MessagePriceAbstain, MessagePriceDrift, MessageValidator, MessageValidatorEvent,
    MessageValidatorStakedTotal, ValidatorEventType,
};
use constellation_observer::BrokerType;
use constellation_shared::MessageStop;
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
    pub lcd: String,
    pub chain: String,
}
impl ValidatorActor {
    pub async fn create(lcd: &str, chain: &str) -> anyhow::Result<ValidatorActor> {
        let terra = Terra::lcd_client_no_tx(lcd, chain).await?;
        let validator_result = terra.staking().validators().await?;

        let (validators, monikers) =
            ValidatorActor::from_validator_list(validator_result.height, validator_result.result);
        Ok(ValidatorActor {
            validators,
            moniker: monikers,
            lcd: lcd.into(),
            chain: chain.into(),
        })
    }
    fn from_validator_list(
        height: u64,
        validator_list: Vec<Validator>,
    ) -> (HashMap<String, ValidatorDetails>, HashMap<String, String>) {
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
            Broker::<SystemBroker>::issue_async(MessageValidatorStakedTotal {
                height,
                operator_address: v.operator_address.clone(),
                tokens: v.tokens,
            });
            moniker.insert(v.description.moniker.clone(), v.operator_address.clone());
        });
        (validators, moniker)
    }
}
impl Actor for ValidatorActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.subscribe_sync::<BrokerType, MessagePriceDrift>(ctx);
        self.subscribe_sync::<BrokerType, MessagePriceAbstain>(ctx);
        self.subscribe_sync::<BrokerType, MessageValidator>(ctx);
        //self.subscribe_sync::<BrokerType, MessageBeginBlock>(ctx);
        //self.subscribe_sync::<BrokerType, MessageEndBlock>(ctx);
        self.subscribe_sync::<BrokerType, MessageStop>(ctx);
    }
}
impl Handler<MessageStop> for ValidatorActor {
    type Result = ();

    fn handle(&mut self, _msg: MessageStop, ctx: &mut Self::Context) {
        log::info!("Validator Actor Stopping");
        ctx.stop()
    }
}

impl Handler<MessageValidator> for ValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: MessageValidator, _ctx: &mut Self::Context) {
        let height = msg.height;
        let now = Utc::now();
        match self.validators.entry(msg.operator_address.clone()) {
            Entry::Occupied(mut e) => {
                let mut v = e.get_mut();

                v.last_updated_block = height;
                v.last_updated_date = now;
                v.validator = msg.validator.clone();
            }
            Entry::Vacant(e) => {
                let v = ValidatorDetails {
                    validator: msg.validator.clone(),
                    first_seen_date: now,
                    last_updated_date: now,
                    first_seen_block: height,
                    last_updated_block: height,
                    abstains: 0,
                    drifts: 0,
                };
                e.insert(v);
            }
        }
        Broker::<SystemBroker>::issue_async(MessageValidatorStakedTotal {
            height,
            operator_address: msg.operator_address.clone(),
            tokens: msg.validator.tokens,
        });
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
                v.abstains += 1;
                v.last_updated_block = height;
                v.last_updated_date = now;
                //   e.into_mut();
                let message = format!(
                    "abstained from voting for denominations:{} Abstains:{} Drifts:{}",
                    //height,
                    //   v.validator.description.moniker,
                    msg.denoms.join(","),
                    v.abstains,
                    v.drifts,
                    //  msg.txhash,
                );
                Broker::<SystemBroker>::issue_async(MessageValidatorEvent {
                    height,
                    operator_address: msg.operator_address.clone(),
                    moniker: Some(v.validator.description.moniker.clone()),
                    event_type: ValidatorEventType::INFO,
                    message,
                    hash: Some(msg.txhash),
                });
            }
            Entry::Vacant(_) => {
                log::error!("Validator not found ? {}", msg.operator_address)
            }
        }
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
                v.drifts += 1;
                v.last_updated_block = height;
                v.last_updated_date = now;
                let message = format!("{} price drift submitted {:.4} too far away from Average:{:.4}/ Weighted:{:.4} Drifts:{}",    
                                      //height,
                                      //v.validator.description.moniker,
                                      msg.denom,
                                      msg.submitted,
                                      msg.average,
                                      msg.weighted_average,
//                                      v.abstains,
                                      v.drifts
                                      )
                    ;
                if msg.denom == "uusd" {
                    log::info!("{}", message);
                }
                Broker::<SystemBroker>::issue_async(MessageValidatorEvent {
                    height,
                    operator_address: msg.operator_address.clone(),
                    moniker: Some(v.validator.description.moniker.clone()),
                    event_type: ValidatorEventType::WARN,
                    message,
                    hash: Some(msg.txhash),
                });
            }
            Entry::Vacant(_) => {
                log::error!("Validator not found ? {}", msg.operator_address)
            }
        }
    }
}
