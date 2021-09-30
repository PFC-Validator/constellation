use std::collections::HashMap;

use actix::prelude::*;
use actix_broker::{Broker, BrokerSubscribe, SystemBroker};
use chrono::{DateTime, Timelike, Utc};
//use rust_decimal::Decimal;
use terra_rust_api::staking_types;
use terra_rust_api::tendermint_types;
use terra_rust_api::Terra;

use constellation_observer::messages::{
    MessageBlockEventExchangeRate, MessageBlockEventLiveness, MessageBlockEventReward,
    MessagePriceAbstain, MessagePriceDrift, MessageSendMessageEvent, MessageValidator,
    MessageValidatorEvent, MessageValidatorStakedTotal, ValidatorEventType,
};
use constellation_observer::BrokerType;
use constellation_shared::{MessageStop, MessageTick};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::collections::hash_map::Entry;
use std::ops::{Div, Mul};

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
    pub last_height: u64,
    pub last_tick: Option<DateTime<Utc>>,
    pub validators: HashMap<String, ValidatorDetails>,
    pub moniker: HashMap<String, String>,
    pub cons_pub: HashMap<String, String>, // tendermint public key -> validator oper
    pub cons: HashMap<String, String>,     // tendermint account -> validator oper
    pub rewards: HashMap<String, Decimal>,
    pub rewards_cumulative: HashMap<String, Decimal>,
    pub rates: HashMap<String, Decimal>,
    pub lcd: String,
    pub chain: String,
}
impl ValidatorActor {
    pub async fn create(lcd: &str, chain: &str) -> anyhow::Result<ValidatorActor> {
        log::info!("Validator Actor starting up");
        match Terra::lcd_client_no_tx(lcd, chain).await {
            Ok(terra) => match terra.staking().validators().await {
                Ok(validator_result) => match terra.tendermint().validatorsets_full().await {
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
                            last_height: 0,
                            last_tick: None,
                            validators: merged_validator_lists.validator_details,
                            moniker: merged_validator_lists.monikers,
                            cons_pub: merged_validator_lists.cons_pub,
                            cons: merged_validator_lists.cons,
                            rewards: Default::default(),
                            rewards_cumulative: Default::default(),
                            rates: Default::default(),
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
        self.subscribe_sync::<BrokerType, MessageBlockEventReward>(ctx);
        self.subscribe_sync::<BrokerType, MessageBlockEventExchangeRate>(ctx);

        self.subscribe_sync::<BrokerType, MessageTick>(ctx);
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
        self.last_height = height;
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
        self.last_height = height;
        let now = Utc::now();
        match self.validators.entry(msg.operator_address.clone()) {
            Entry::Occupied(mut e) => {
                let mut v = e.get_mut();
                v.abstains += 1;
                v.last_updated_block = height;
                v.last_updated_date = now;
                //   e.into_mut();
                let message = format!(
                    "abstained from voting for denominations:{} Abstains:{}",
                    //height,
                    //   v.validator.description.moniker,
                    msg.denoms.join(","),
                    v.abstains,
                    //  msg.txhash,
                );
                if v.abstains % 10 == 0 {
                    Broker::<SystemBroker>::issue_async(MessageValidatorEvent {
                        height,
                        operator_address: msg.operator_address.clone(),
                        moniker: Some(v.validator.description.moniker.clone()),
                        event_type: ValidatorEventType::INFO,
                        message,
                        hash: Some(msg.txhash),
                    });
                }
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
        self.last_height = height;
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
        log::debug!("Liveness {}", msg.tendermint_address);
        let height = msg.height;
        self.last_height = height;
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

impl Handler<MessageBlockEventReward> for ValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: MessageBlockEventReward, _ctx: &mut Self::Context) {
        let height = msg.height;
        self.last_height = height;
        let mut block_reward: Decimal = Decimal::from(0);

        for coin_amount in &msg.amount {
            if coin_amount.denom == "uluna" {
                block_reward += coin_amount.amount;
            } else if let Some(rate) = self.rates.get(&coin_amount.denom) {
                if rate > &Decimal::ZERO {
                    block_reward += coin_amount.amount / rate;
                } else {
                    log::warn!(
                        "Reward: {} Exchange rate Zero for {}",
                        height,
                        coin_amount.denom
                    );
                }
            } else if !self.rates.is_empty() {
                log::warn!(
                    "Reward: {} Missing Exchange rate for {}.",
                    height,
                    coin_amount.denom
                );
            }
        }
        self.rewards
            .entry(msg.validator)
            .and_modify(|e| *e += block_reward)
            .or_insert(block_reward);
    }
}
impl Handler<MessageBlockEventExchangeRate> for ValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: MessageBlockEventExchangeRate, _ctx: &mut Self::Context) {
        self.last_height = msg.height;
        self.rates.insert(msg.denom, msg.exchange_rate);
    }
}
impl Handler<MessageTick> for ValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: MessageTick, _ctx: &mut Self::Context) {
        if let Some(validator) = self
            .validators
            .get("terravaloper12g4nkvsjjnl0t7fvq3hdcw7y8dc9fq69nyeu9q")
        {
            let tokens = validator.validator.tokens;
            let rewards = self
                .rewards
                .get("terravaloper12g4nkvsjjnl0t7fvq3hdcw7y8dc9fq69nyeu9q")
                .unwrap_or(&Decimal::ZERO);
            let rate: Decimal = if tokens > 0 {
                rewards / Decimal::from(tokens)
            } else {
                Decimal::ZERO
            };

            let message = format!(
                "{} - Generated Rewards of {:0.0} uluna over {} tokens - 5m Rate {:0.8}",
                validator.validator.description.moniker,
                rewards,
                tokens.div(1_000_000),
                rate
            );
            log::info!("{}", message);
        } else {
            log::warn!("Debug validator not found???");
        }
        let mut validator_rate: Vec<(String, Decimal)> = Default::default();
        let mut validator_msg: HashMap<String, String> = Default::default();
        if let Some(last_date) = self.last_tick {
            if msg.now.hour() != last_date.hour() {
                for validator in &self.rewards_cumulative {
                    if let Some(validator_deets) = self.validators.get(validator.0) {
                        let tokens = validator_deets.validator.tokens;
                        let rewards = validator.1;
                        let mut rate: Decimal = if tokens > 0 {
                            rewards / Decimal::from(tokens)
                        } else {
                            Decimal::ZERO
                        };
                        rate *= Decimal::from(24); // daily
                                                   // todo rate should be aggregated as tokens change
                        validator_rate.push((validator.0.clone(), rate));
                        let message = format!(
                            "{} - Generated Rewards of {:0.2} luna over ~{:.2} tokens - Daily Rate {:0.8}",
                            validator_deets.validator.description.moniker, rewards.div(Decimal::from(1_000_000)), tokens.div(1_000_000), rate
                        );
                        validator_msg.insert(validator.0.clone(), message.clone());
                        Broker::<SystemBroker>::issue_async(MessageValidatorEvent {
                            height: self.last_height,
                            operator_address: validator.0.clone(),
                            moniker: Some(validator_deets.validator.description.moniker.clone()),
                            event_type: ValidatorEventType::INFO,
                            message,
                            hash: None,
                        });
                    } else {
                        log::warn!(
                            "Missing details for validator {} - rewards {}",
                            validator.0,
                            validator.1
                        )
                    }
                }
                self.rewards_cumulative = self.rewards.clone();
            } else {
                for validator in &self.rewards {
                    self.rewards_cumulative
                        .entry(validator.0.clone())
                        .and_modify(|e| *e += validator.1)
                        .or_insert(*validator.1);
                }
            }
        } else {
            self.rewards_cumulative = self.rewards.clone();
        }

        if !validator_rate.is_empty() {
            let average_rate: Decimal = validator_rate.iter().map(|f| f.1).sum::<Decimal>()
                / Decimal::from(validator_rate.len());
            let slip = average_rate.mul(Decimal::from_f64(0.05).unwrap());
            let mut top = validator_rate
                .iter()
                .filter(|r| r.1 > (average_rate + slip))
                .collect::<Vec<_>>();
            top.sort_by(|a, b| b.1.cmp(&a.1));

            let mut bottom = validator_rate
                .iter()
                .filter(|r| r.1 < (average_rate - slip))
                .collect::<Vec<_>>();
            bottom.sort_by(|a, b| a.1.cmp(&b.1));

            Broker::<SystemBroker>::issue_async(MessageSendMessageEvent {
                height: self.last_height,
                event_type: ValidatorEventType::ANNOUNCE,
                message: format!(
                    "Average Rate = {:0.8} #over allowance of 5% = {} #below allowance of 5% {}",
                    average_rate,
                    top.len(),
                    bottom.len()
                ),
                hash: None,
            });
            let top_len = top.len();
            if top_len > 0 {
                let top_msg = top
                    .iter()
                    .take(5)
                    .map(|f| validator_msg.get(&f.0).unwrap_or(&f.0))
                    .fold(String::new(), |a, b| a + b + "\n");
                Broker::<SystemBroker>::issue_async(MessageSendMessageEvent {
                    height: self.last_height,
                    event_type: ValidatorEventType::PRIVATE,
                    message: format!(
                        "Top {} - Average: {:0.8}\n{} ",
                        top_len, average_rate, top_msg
                    ),
                    hash: None,
                });
            }

            let bot_len = bottom.len();
            if bot_len > 0 {
                let bot_msg = bottom
                    .iter()
                    .take(10)
                    .map(|f| validator_msg.get(&f.0).unwrap_or(&f.0))
                    .fold(String::new(), |a, b| a + b + "\n");
                Broker::<SystemBroker>::issue_async(MessageSendMessageEvent {
                    height: self.last_height,
                    event_type: ValidatorEventType::PRIVATE,
                    message: format!(
                        "Bottom {} - Average: {:0.8}\n{} ",
                        bot_len, average_rate, bot_msg
                    ),
                    hash: None,
                });
            }
        }
        self.last_tick = Some(msg.now);

        self.rewards = Default::default();
    }
}
