use std::collections::HashMap;

use actix::prelude::*;
use actix_broker::{Broker, BrokerSubscribe, SystemBroker};
use chrono::{DateTime, Utc};
//use rust_decimal::Decimal;
use terra_rust_api::staking_types;
use terra_rust_api::tendermint_types;
use terra_rust_api::Terra;

use constellation_observer::messages::{
    MessageBlockEventLiveness, MessagePriceAbstain, MessagePriceDrift, MessageValidator,
    MessageValidatorEvent, MessageValidatorStakedTotal, ValidatorEventType,
};
use constellation_observer::BrokerType;
use constellation_shared::MessageStop;
use std::collections::hash_map::Entry;

#[derive(Clone, Debug)]
pub struct ValidatorDetails {
    pub validator: staking_types::Validator,
    pub tendermint_account: Option<String>,

    pub first_seen_date: DateTime<Utc>,
    pub last_updated_date: DateTime<Utc>,
    pub first_seen_block: u64,
    pub last_updated_block: u64,
    pub abstains: u64,
    pub drifts: u64,
}
struct MergedValidatorLists {
    pub validator_details: HashMap<String, ValidatorDetails>,
    pub monikers: HashMap<String, String>,
    pub cons_pub: HashMap<String, String>,
    pub cons: HashMap<String, String>,
}
pub struct ValidatorActor {
    pub validators: HashMap<String, ValidatorDetails>,
    pub moniker: HashMap<String, String>,
    pub cons_pub: HashMap<String, String>, // tendermint public key -> validator oper
    pub cons: HashMap<String, String>,     // tendermint account -> validator oper
    pub lcd: String,
    pub chain: String,
}
impl ValidatorActor {
    pub async fn create(lcd: &str, chain: &str) -> anyhow::Result<ValidatorActor> {
        log::info!("Validator Actor starting up");
        match Terra::lcd_client_no_tx(lcd, chain).await {
            Ok(terra) => match terra.staking().validators().await {
                Ok(validator_result) => match terra.tendermint().validatorsets(0, 999).await {
                    Ok(tendermint_result) => {
                        log::info!(
                            "Have validator/tendermint list kickstart v:{} t:{}",
                            validator_result.result.len(),
                            tendermint_result.result.validators.len()
                        );
                        let merged_validator_lists = ValidatorActor::from_validator_list(
                            validator_result.height,
                            validator_result.result,
                            tendermint_result.result.validators,
                        );
                        Ok(ValidatorActor {
                            validators: merged_validator_lists.validator_details,
                            moniker: merged_validator_lists.monikers,
                            cons_pub: merged_validator_lists.cons_pub,
                            cons: merged_validator_lists.cons,
                            lcd: lcd.into(),
                            chain: chain.into(),
                        })
                    }
                    Err(e) => {
                        log::error!("validator sets {}", e);
                        Err(e)
                    }
                },
                Err(e) => {
                    log::error!("staking validators {}", e);
                    Err(e)
                }
            },
            Err(e) => {
                log::error!("LCD Client {}", e);
                Err(e)
            }
        }
    }
    fn from_validator_list(
        height: u64,
        validator_list: Vec<staking_types::Validator>,
        tendermint_list: Vec<tendermint_types::Validator>,
    ) -> MergedValidatorLists {
        let now = Utc::now();
        let mut validators: HashMap<String, ValidatorDetails> = Default::default();
        let mut moniker: HashMap<String, String> = Default::default();
        let mut cons_pub: HashMap<String, String> = Default::default();
        let mut cons: HashMap<String, String> = Default::default();
        for v in &validator_list {
            cons_pub.insert(v.consensus_pubkey.clone(), v.operator_address.clone());
        }
        for tm in tendermint_list {
            if let Some(v_account) = cons_pub.get(&tm.pub_key) {
                cons.insert(tm.address.clone(), v_account.into());
            }
        }

        validator_list.iter().for_each(|v| {
            validators.insert(
                v.operator_address.clone(),
                ValidatorDetails {
                    validator: v.clone(),
                    tendermint_account: cons_pub.get(&v.consensus_pubkey).map(|f| f.into()),
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
        MergedValidatorLists {
            validator_details: validators,
            monikers: moniker,
            cons_pub,
            cons,
        }
    }
}
impl Actor for ValidatorActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.subscribe_sync::<BrokerType, MessagePriceDrift>(ctx);
        self.subscribe_sync::<BrokerType, MessagePriceAbstain>(ctx);
        self.subscribe_sync::<BrokerType, MessageValidator>(ctx);
        self.subscribe_sync::<BrokerType, MessageBlockEventLiveness>(ctx);

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
        log::debug!(
            "Validator Updated MSG {}/{}",
            msg.validator.description.moniker,
            msg.tendermint.is_some()
        );
        let height = msg.height;
        let now = Utc::now();
        match self.validators.entry(msg.operator_address.clone()) {
            Entry::Occupied(mut e) => {
                let mut v = e.get_mut();

                v.last_updated_block = height;
                v.last_updated_date = now;
                v.validator = msg.validator.clone();
                v.tendermint_account = msg.tendermint.map(|v| v.address)
            }
            Entry::Vacant(e) => {
                let v = ValidatorDetails {
                    validator: msg.validator.clone(),
                    tendermint_account: msg.tendermint.map(|v| v.address),
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
                                      msg.denom,
                                      msg.submitted,
                                      msg.average,
                                      msg.weighted_average,
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

impl Handler<MessageBlockEventLiveness> for ValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: MessageBlockEventLiveness, _ctx: &mut Self::Context) {
        log::info!("Liveness {}", msg.tendermint_address);
        let height = msg.height;
        if let Some(validator_address) = self.cons.get(&msg.tendermint_address) {
            match self.validators.get(validator_address) {
                Some(v) => {
                    let message = format!("Liveness check failed missed: {}", msg.missed);

                    Broker::<SystemBroker>::issue_async(MessageValidatorEvent {
                        height,
                        operator_address: validator_address.clone(),
                        moniker: Some(v.validator.description.moniker.clone()),
                        event_type: ValidatorEventType::WARN,
                        message,
                        hash: None,
                    });
                }
                None => {
                    log::warn!("Validator not found ? {}", msg.tendermint_address)
                }
            }
        }
    }
}
