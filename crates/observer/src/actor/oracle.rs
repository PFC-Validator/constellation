use crate::{BrokerType, MessageTX};
use actix::prelude::*;
use actix_broker::BrokerSubscribe;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::ops::Div;
use terra_rust_api::core_types::Coin;
use terra_rust_api::messages::oracle::MsgAggregateExchangeRateVote;
use terra_rust_api::Terra;

pub struct OracleActor {
    pub vote_period: u64,
    pub vote_threshold: f64,
    pub reward_band: f64,
    pub reward_distribution_window: u64,
    pub slash_fraction: f64,
    pub slash_window: u64,
    pub min_valid_per_window: f64,
    pub last_avg_at_height: u64,
    pub validator_vote_last_seen: HashMap<String, u64>,
    pub validator_vote_prices: HashMap<String, Vec<Coin>>,
}
impl OracleActor {
    pub async fn create(lcd: &str, chain: &str) -> anyhow::Result<OracleActor> {
        let terra = Terra::lcd_client_no_tx(lcd, chain).await?;
        let params = terra.oracle().parameters().await?.result;

        Ok(OracleActor {
            vote_period: params.vote_period,
            vote_threshold: params.vote_threshold,
            reward_band: params.reward_band,
            reward_distribution_window: params.reward_distribution_window,
            slash_fraction: params.slash_fraction,
            slash_window: params.slash_window,
            min_valid_per_window: params.min_valid_per_window,
            validator_vote_last_seen: Default::default(),
            validator_vote_prices: Default::default(),
            last_avg_at_height: 0,
        })
    }

    pub fn do_price_averages(&mut self) {
        if !self.validator_vote_prices.is_empty() {
            let mut agg_price: HashMap<String, (usize, Decimal)> = Default::default();
            self.validator_vote_prices.iter().for_each(|v_vote| {
                v_vote.1.iter().for_each(|coin| {
                    let updated = match agg_price.get(&coin.denom) {
                        None => (1, coin.amount),
                        Some(i) => (1 + i.0, i.1 + coin.amount),
                    };
                    agg_price.insert(coin.denom.clone(), updated);
                })
            });
            let averages: HashMap<String, Decimal> = agg_price
                .iter()
                .map(|f| {
                    let sum = f.1 .0;
                    let avg_price = f.1 .1.div(Decimal::from(sum));
                    (f.0.clone(), avg_price)
                })
                .collect();
            averages.iter().for_each(|f| log::info!("{} {}", f.0, f.1));
            let uusd = averages.get("uusd").unwrap();
            self.validator_vote_prices.iter().for_each(|f| {
                if let Some(uusd_coin) =
                    f.1.iter()
                        .filter(|c| c.denom == "uusd")
                        .collect::<Vec<_>>()
                        .first()
                {
                    let uusd_price = uusd_coin.amount;
                    println!(
                        "Validator:{}\tsupplied:{}\tAverage:{}\tdrift:{}",
                        f.0,
                        uusd_price,
                        uusd,
                        (uusd_price - uusd)
                    );
                } else {
                    log::warn!("Validator: {} missing uusd", f.0);
                }
            })
        }
    }
}
impl Actor for OracleActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.subscribe_sync::<BrokerType, MessageTX>(ctx);
    }
}

/*
{"exchange_rates":"34.750000000000000000uusd,40830.000000000000000000ukrw,24.449822000000001054usdr,99068.814719250003690831umnt,29.460632999999997850ueur,25.245180000000001286ugbp,224.895188999999987800ucny,3816.505763999999999214ujpy,2554.013938999999936641uinr,43.867878750000002697ucad,31.678551750000000453uchf,270.621011249999980919uhkd,47.524621250000002703uaud,46.773673749999993277usgd,1128.037263999999822772uthb,300.404020000000002710usek,219.053713999999985163udkk,497729.462499999965075403uidr,1733.343031249999967258uphp","feeder":"terra1ml8x4n3yhq4jq6kfd4rr97jc058jyrexxqs84z","salt":"521c","validator":"terravaloper162892yn0tf8dxl8ghgneqykyr8ufrwmcs4q5m8"}
*/
impl Handler<MessageTX> for OracleActor {
    type Result = ();

    fn handle(&mut self, msg: MessageTX, _ctx: &mut Self::Context) {
        let height = msg.tx.height;
        if msg.tx.tx.s_type == "core/StdTx" {
            let messages = msg.tx.tx.value;
            messages.msg.iter().for_each(|message| {
                if message.s_type == "oracle/MsgAggregateExchangeRateVote" {
                    let v = serde_json::from_value::<MsgAggregateExchangeRateVote>(
                        message.value.clone(),
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
                        Err(e) => log::error!("Expected vote: {} - {}", e, message.value),
                    }
                }
            })
        } else {
            log::info!("Height: {} Type {}", msg.tx.height, msg.tx.tx.s_type);
        }
        if height >= self.last_avg_at_height + self.vote_period {
            self.do_price_averages();
            let laggy = self
                .validator_vote_last_seen
                .iter()
                .filter(|f| f.1 < &self.last_avg_at_height)
                .collect::<Vec<_>>();
            log::info!("Seen {} price votes", self.validator_vote_prices.len());
            if !laggy.is_empty() {
                laggy
                    .iter()
                    .for_each(|f| log::info!("laggy: {} Last Seen:{}", f.0, f.1))
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
            self.last_avg_at_height = height;
        }
    }
}
