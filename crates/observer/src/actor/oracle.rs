use std::collections::HashMap;
use std::ops::{Div, Mul};

use actix::prelude::*;
use actix_broker::{Broker, BrokerSubscribe, SystemBroker};
use rust_decimal::prelude::*;
//use rust_decimal_macros::dec;
use terra_rust_api::core_types::Coin;
use terra_rust_api::messages::oracle::MsgAggregateExchangeRateVote;
use terra_rust_api::Terra;

use crate::messages::{
    MessagePriceAbstain, MessagePriceDrift, MessageTX, MessageValidatorEvent,
    MessageValidatorStakedTotal, ValidatorEventType,
};
use crate::BrokerType;
use constellation_shared::MessageStop;
use std::collections::hash_map::Entry;

pub struct OracleActor {
    pub vote_period: u64,
    pub vote_threshold: Decimal,
    pub reward_band: Decimal,
    pub reward_distribution_window: u64,
    pub slash_fraction: Decimal,
    pub slash_window: u64,
    pub min_valid_per_window: Decimal,
    pub last_avg_at_height: u64,
    pub validator_vote_last_seen: HashMap<String, u64>,
    pub validator_weight: HashMap<String, u64>,
    pub validator_vote_last_hash: HashMap<String, String>,
    pub validator_vote_prices: HashMap<String, Vec<Coin>>,
}
impl OracleActor {
    pub async fn create(lcd: &str, chain: &str) -> anyhow::Result<OracleActor> {
        let terra = Terra::lcd_client_no_tx(lcd, chain).await?;
        let params = terra.oracle().parameters().await?.result;

        Ok(OracleActor {
            vote_period: params.vote_period,
            vote_threshold: Decimal::from_f64(params.vote_threshold).unwrap(),
            reward_band: Decimal::from_f64(params.reward_band).unwrap(),
            reward_distribution_window: params.reward_distribution_window,
            slash_fraction: Decimal::from_f64(params.slash_fraction).unwrap(),
            slash_window: params.slash_window,
            min_valid_per_window: Decimal::from_f64(params.min_valid_per_window).unwrap(),
            validator_vote_last_seen: Default::default(),
            validator_vote_prices: Default::default(),
            validator_vote_last_hash: Default::default(),
            validator_weight: Default::default(),
            last_avg_at_height: 0,
        })
    }

    pub fn do_price_averages(&mut self, height: u64) {
        if !self.validator_vote_prices.is_empty() {
            let mut agg_price: HashMap<String, (usize, u64, Decimal, Decimal)> = Default::default();

            self.validator_vote_prices.iter().for_each(|v_vote| {
                let mut denom_abstain: Vec<String> = vec![];
                let operator_address = v_vote.0;
                let txhash = self
                    .validator_vote_last_hash
                    .get(operator_address)
                    .map(String::from);

                let validator_prices = v_vote.1;
                if let Some(weight) = self.validator_weight.get(operator_address) {
                    validator_prices.iter().for_each(|coin| {
                        if coin.amount > Decimal::from_f64(0.001f64).unwrap() {
                            let updated = match agg_price.get(&coin.denom) {
                                None => (
                                    1,
                                    *weight,
                                    coin.amount,
                                    coin.amount.mul(Decimal::from(*weight)),
                                ),
                                Some(i) => (
                                    1 + i.0,
                                    weight + i.1,
                                    coin.amount + i.2,
                                    coin.amount.mul(Decimal::from(*weight)) + i.3,
                                ),
                            };
                            agg_price.insert(coin.denom.clone(), updated);
                        } else {
                            denom_abstain.push(coin.denom.clone());
                        }
                    })
                } else {
                    log::info!("Validator {} has no weight, skipping", operator_address);
                }
                if !denom_abstain.is_empty() {
                    Broker::<SystemBroker>::issue_async(MessagePriceAbstain {
                        height,
                        operator_address: operator_address.clone(),
                        denoms: denom_abstain,
                        txhash: txhash.unwrap_or_else(|| "-missing hash-".into()),
                    });
                }
            });

            let averages: HashMap<String, (Decimal, Decimal)> = agg_price
                .iter()
                .map(|f| {
                    let sum = f.1 .0;
                    let weighted_sum = f.1 .1;
                    let avg_price = f.1 .2.div(Decimal::from(sum));
                    let avg_weighted_price = f.1 .3.div(Decimal::from(weighted_sum));
                    (f.0.clone(), (avg_price, avg_weighted_price))
                })
                .collect();
            let drift_max_percentage: Decimal = self.reward_band;
            averages.iter().for_each(|f| {
                if f.0 == "uusd" {
                    log::info!("{} AVG:{:.4}\t Weighted:{:.4}", f.0, f.1 .0, f.1 .1)
                } else {
                    log::debug!("{} AVG:{:.4}\t Weighted:{:.4}", f.0, f.1 .0, f.1 .1)
                }
            });
            averages.iter().for_each(|f| {
                let denom = f.0;
                let average_price = f.1 .0;
                let average_weighted_price = f.1 .1;
                let drift_max = average_price.mul(drift_max_percentage).abs();
                let _drift_weighted_max = average_weighted_price.mul(drift_max_percentage).abs();

                self.validator_vote_prices
                    .iter()
                    .for_each(|validator_price| {
                        let operator_address = String::from(validator_price.0);
                        if let Some(price_submitted_coin) = validator_price
                            .1
                            .iter()
                            .filter(|c| c.denom.eq(denom))
                            .collect::<Vec<_>>()
                            .first()
                        {
                            let submitted_price = price_submitted_coin.amount;
                            if submitted_price > Decimal::from_f64(0.001f64).unwrap() {
                                let drift = submitted_price - average_price;
                                if drift.abs() > drift_max {
                                    let txhash = self
                                        .validator_vote_last_hash
                                        .get(&operator_address)
                                        .map(String::from)
                                        .unwrap_or_else(|| "-missing hash-".into());

                                    log::debug!(
                                        "Drift detected {} {} {} {:4}/{:4} {}/{} - {}",
                                        operator_address,
                                        denom,
                                        submitted_price,
                                        average_price,
                                        average_weighted_price,
                                        drift,
                                        drift_max,
                                        txhash
                                    );

                                    Broker::<SystemBroker>::issue_async(MessagePriceDrift {
                                        height,
                                        operator_address,
                                        denom: denom.into(),
                                        average: average_price,
                                        weighted_average: average_weighted_price,
                                        submitted: submitted_price,
                                        txhash,
                                    });
                                }
                            }
                        } else {
                            log::warn!("Validator: {} missing denom {}", operator_address, denom);
                        }
                    })
            });
        }
    }
}
impl Actor for OracleActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.subscribe_sync::<BrokerType, MessageTX>(ctx);
        self.subscribe_sync::<BrokerType, MessageValidatorStakedTotal>(ctx);
        self.subscribe_sync::<BrokerType, MessageStop>(ctx);
    }
}

impl Handler<MessageStop> for OracleActor {
    type Result = ();

    fn handle(&mut self, _msg: MessageStop, ctx: &mut Self::Context) {
        log::info!("Oracle Actor Stopping");
        ctx.stop()
    }
}

impl Handler<MessageValidatorStakedTotal> for OracleActor {
    type Result = ();

    fn handle(&mut self, msg: MessageValidatorStakedTotal, _ctx: &mut Self::Context) {
        match self.validator_weight.entry(msg.operator_address.clone()) {
            Entry::Occupied(mut e) => {
                let v = e.get_mut();
                *v = msg.tokens;
                //   e.into();
            }
            Entry::Vacant(v) => {
                v.insert(msg.tokens);
            }
        }
    }
}
/*
{"exchange_rates":"34.750000000000000000uusd,40830.000000000000000000ukrw,24.449822000000001054usdr,99068.814719250003690831umnt,29.460632999999997850ueur,25.245180000000001286ugbp,224.895188999999987800ucny,3816.505763999999999214ujpy,2554.013938999999936641uinr,43.867878750000002697ucad,31.678551750000000453uchf,270.621011249999980919uhkd,47.524621250000002703uaud,46.773673749999993277usgd,1128.037263999999822772uthb,300.404020000000002710usek,219.053713999999985163udkk,497729.462499999965075403uidr,1733.343031249999967258uphp","feeder":"terra1ml8x4n3yhq4jq6kfd4rr97jc058jyrexxqs84z","salt":"521c","validator":"terravaloper162892yn0tf8dxl8ghgneqykyr8ufrwmcs4q5m8"}
*/
impl Handler<MessageTX> for OracleActor {
    type Result = ();

    fn handle(&mut self, msg: MessageTX, _ctx: &mut Self::Context) {
        let height = msg.tx.height;
        if msg.tx.tx.s_type == "/cosmos.tx.v1beta1.Tx" {
            // if msg.tx.tx.s_type == "core/StdTx" {
            let messages = msg.tx.tx.body;
            let txhash = msg.tx.txhash;
            for m in &messages.messages {
                if let Some(message_type) = m.get("@type") {
                    if message_type == "/terra.oracle.v1beta1.MsgAggregateExchangeRateVote" {
                        //        log::info!("{} -- {} {} ", height, message_type, m.to_string());
                        let v = serde_json::from_value::<MsgAggregateExchangeRateVote>(m.clone());
                        match v {
                            Ok(vote) => {
                                //  log::info!("Vote {} {}", vote.validator, vote.feeder);
                                match Coin::parse_coins(&vote.exchange_rates) {
                                    Ok(rates) => {
                                        self.validator_vote_last_seen
                                            .insert(vote.validator.clone(), height);
                                        self.validator_vote_prices
                                            .insert(vote.validator.clone(), rates);
                                        self.validator_vote_last_hash
                                            .insert(vote.validator.clone(), txhash.clone());
                                    }
                                    Err(e) => {
                                        log::error!(
                                            "Bad Rates: {} {} {}",
                                            vote.validator,
                                            vote.exchange_rates,
                                            e
                                        )
                                    }
                                }
                            }
                            Err(e) => log::error!("Expected vote: {} - {}", e, m.to_string()),
                        }
                    } else {
                        log::debug!("{} -- {} ", height, message_type);
                    }
                }
            }
            /*
            messages.messages.iter().for_each(|message| {
                if message.s_type == "oracle/MsgAggregateExchangeRateVote" {
                    if let Some(execute_msg) = &message.execute_msg {
                        let v = serde_json::from_value::<MsgAggregateExchangeRateVote>(
                            execute_msg.clone(),
                        );
                        match v {
                            Ok(vote) => {
                                //  log::info!("Vote {} {}", vote.validator, vote.feeder);
                                match Coin::parse_coins(&vote.exchange_rates) {
                                    Ok(rates) => {
                                        self.validator_vote_last_seen
                                            .insert(vote.validator.clone(), height);
                                        self.validator_vote_prices
                                            .insert(vote.validator.clone(), rates);
                                        self.validator_vote_last_hash
                                            .insert(vote.validator.clone(), txhash.clone());
                                    }
                                    Err(e) => {
                                        log::error!(
                                            "Bad Rates: {} {} {}",
                                            vote.validator,
                                            vote.exchange_rates,
                                            e
                                        )
                                    }
                                }
                            }
                            Err(e) => log::error!("Expected vote: {} - {}", e, execute_msg),
                        }
                    }
                }
            })

             */
        } else {
            log::info!(
                "Height: {} Type {} Value {:?}",
                msg.tx.height,
                msg.tx.tx.s_type,
                msg.tx.tx.body
            );
        }
        // make 'laggy' be 2 vote periods
        if height >= self.last_avg_at_height + self.vote_period {
            self.do_price_averages(height);
            let laggy_height = if self.vote_period < self.last_avg_at_height {
                self.last_avg_at_height - self.vote_period
            } else {
                0
            };
            let laggy = self
                .validator_vote_last_seen
                .iter()
                .filter(|f| f.1 < &laggy_height)
                .collect::<Vec<_>>();
            log::info!("Seen {} price votes", self.validator_vote_prices.len());
            if !laggy.is_empty() {
                laggy.iter().for_each(|f| {
                    let message = format!("Operator missed a vote? Last Seen:{}", f.1);
                    log::info!("laggy: {} Last Seen:{}", f.0, f.1);
                    Broker::<SystemBroker>::issue_async(MessageValidatorEvent {
                        height,
                        operator_address: f.0.clone(),
                        moniker: None,
                        event_type: ValidatorEventType::WARN,
                        message,
                        hash: None,
                    });
                })
            }
            self.validator_vote_last_seen = self
                .validator_vote_last_seen
                .iter()
                .flat_map(|f| {
                    if f.1 < &(height - 100) {
                        log::info!("Validator is too old: {}", f.0);
                        None
                    } else {
                        Some((f.0.clone(), *f.1))
                    }
                })
                .collect();

            self.validator_vote_prices = Default::default();
            self.validator_vote_last_hash = Default::default();
            self.last_avg_at_height = height;
        }
    }
}
