use futures::{SinkExt, StreamExt};

use crate::messages::{
    MessageBlockEventCommission, MessageBlockEventExchangeRate, MessageBlockEventLiveness,
    MessageBlockEventReward, MessageTX,
};
use crate::types::{NewBlock, NewBlockEvent};
use actix_broker::{Broker, SystemBroker};
use constellation_shared::AppState;
use rust_decimal::prelude::FromStr;
use rust_decimal::Decimal;
use std::collections::HashMap;
use terra_rust_api::core_types::Coin;
use tokio::time::Duration;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::http::Request;
use tokio_tungstenite::tungstenite::Message;

/// VERSION number of package
pub const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
/// NAME of package
pub const NAME: Option<&'static str> = option_env!("CARGO_PKG_NAME");
// TODO add proposing validator to messages.
pub async fn run(_state: AppState, connect_addr: String) {
    loop {
        match Request::builder()
            .header(
                "User-Agent",
                format!(
                    "{}/{}",
                    NAME.unwrap_or("Constellation"),
                    VERSION.unwrap_or("dev")
                ),
            )
            .uri(&connect_addr)
            .body(())
        {
            Ok(ws_request) => {
                match connect_async(ws_request).await {
                    Ok((mut ws_stream, _)) => {
                        log::info!("Connected");
                        //  let (mut write, read) = ws_stream.split();
                        let msg = Message::text(
                            "{\"subscribe\":\"new_block\",\"chain_id\":\"columbus-5\"}",
                        );
                        match ws_stream.send(msg).await {
                            Ok(_) => {
                                while let Some(message) = ws_stream.next().await {
                                    // read.for_each(|message| async {
                                    match message {
                                        Ok(msg) => {
                                            /*
                                            log::info!(
                                                "empty {} ping {} pong {} bin {} text {} close {} len {}",
                                                msg.is_empty(),
                                                msg.is_ping(),
                                                msg.is_pong(),
                                                msg.is_binary(),
                                                msg.is_text(),
                                                msg.is_close(),
                                                msg.len()
                                            );

                                             */
                                            match msg {
                                                Message::Text(text) => {
                                                    match serde_json::from_str::<NewBlock>(&text) {
                                                        Ok(new_block) => {
                                                            log::info!(
                                                                "Block:{} {}",
                                                                new_block.chain_id,
                                                                new_block.data.block.header.height
                                                            );
                                                            if let Err(e) =
                                                                process_block_emit(&new_block)
                                                            {
                                                                log::error!(
                                                            "Error pushing block to actors: {}",
                                                            e
                                                        );
                                                            }
                                                        }
                                                        Err(e) => {
                                                            log::error!(
                                                                "Error parsing block: {}",
                                                                e
                                                            );
                                                            log::error!("{}", text);
                                                        }
                                                    }
                                                }
                                                Message::Binary(_) => {}
                                                Message::Ping(p) => {
                                                    let pong = Message::Pong(p);
                                                    if let Err(e) = ws_stream.send(pong).await {
                                                        log::error!(
                                                            "Unable to respond pong {:#?}",
                                                            e
                                                        )
                                                    }
                                                }
                                                Message::Pong(_) => {}
                                                Message::Close(_) => {
                                                    log::warn!("Socket Closing..TBD do something")
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            log::error!("{:#?}", e)
                                        }
                                    }
                                }
                            }
                            Err(e) => log::error!("Unable to send message {:#?}", e),
                        }
                    }

                    Err(e) => log::error!("Failed to Connect to observer:{:?}", e),
                }
            }
            Err(e) => log::error!("Unable to initiate observer:{}", e),
        }
        log::warn!("Observer exited..retrying in 2s");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
fn process_block_emit(block: &NewBlock) -> anyhow::Result<()> {
    let height = block.data.block.header.height;
    if let Some(txs) = &block.data.txs {
        txs.iter().for_each(|tx| {
            Broker::<SystemBroker>::issue_async(MessageTX { tx: tx.clone() });
        })
    }
    block
        .data
        .result_begin_block
        .events
        .iter()
        .for_each(|event| process_event(height, true, event));
    match &block.data.result_end_block.events {
        None => {}
        Some(end_block_events) => {
            end_block_events
                .iter()
                .for_each(|event| process_event(height, false, event));
        }
    }
    let v = &block.data.result_end_block.validator_updates;
    v.iter().for_each(|f| {
        log::info!(
            "Validator update: {} pub key:{} power:{}",
            height,
            f.pub_key,
            f.power
        )
    });

    Ok(())
}
fn get_required_kv(hash_map: &HashMap<String, Option<String>>, key: &str) -> Option<String> {
    if let Some(val) = hash_map.get(key) {
        val.as_ref().map(|value| value.into())
    } else {
        None
    }
}
fn process_event(height: u64, is_begin: bool, event: &NewBlockEvent) {
    let attributes = event.attribute_map();

    match event.s_type.as_str() {
        "rewards" => match get_required_kv(&attributes, "validator") {
            Some(validator_value) => match get_required_kv(&attributes, "amount") {
                Some(amount_str) => match Coin::parse_coins(&amount_str) {
                    Ok(coins) => {
                        Broker::<SystemBroker>::issue_async(MessageBlockEventReward {
                            height,
                            is_begin,
                            is_proposer: false,
                            validator: validator_value,
                            amount: coins,
                        });
                    }
                    Err(e) => {
                        log::error!(
                            "Bad Coin String: {} {} {} {}",
                            height,
                            validator_value,
                            amount_str,
                            e
                        )
                    }
                },
                None => {
                    log::debug!("Rewards Zero? {} {}", height, validator_value);
                    Broker::<SystemBroker>::issue_async(MessageBlockEventReward {
                        height,
                        is_begin,
                        is_proposer: false,
                        validator: validator_value,
                        amount: vec![],
                    });
                }
            },
            None => log::warn!(
                "Expecting validator key for rewards event {} {:#?}",
                height,
                attributes
            ),
        },
        "proposer_reward" => match get_required_kv(&attributes, "validator") {
            Some(validator_value) => match get_required_kv(&attributes, "amount") {
                Some(amount_str) => match Coin::parse_coins(&amount_str) {
                    Ok(coins) => {
                        Broker::<SystemBroker>::issue_async(MessageBlockEventReward {
                            height,
                            is_begin,
                            is_proposer: true,
                            validator: validator_value,
                            amount: coins,
                        });
                    }
                    Err(e) => {
                        log::error!(
                            "Bad Coin String: {} {} {} {}",
                            height,
                            validator_value,
                            amount_str,
                            e
                        )
                    }
                },
                None => {
                    log::debug!("Proposer Rewards Zero? {} {}", height, validator_value);
                    Broker::<SystemBroker>::issue_async(MessageBlockEventReward {
                        height,
                        is_begin,
                        is_proposer: true,
                        validator: validator_value,
                        amount: vec![],
                    });
                }
            },
            None => log::warn!(
                "Expecting validator key for rewards event {} {:#?}",
                height,
                attributes
            ),
        },
        "commission" => match get_required_kv(&attributes, "validator") {
            Some(validator_value) => match get_required_kv(&attributes, "amount") {
                Some(amount_str) => match Coin::parse_coins(&amount_str) {
                    Ok(coins) => {
                        Broker::<SystemBroker>::issue_async(MessageBlockEventCommission {
                            height,
                            is_begin,
                            validator: validator_value,
                            amount: coins,
                        });
                    }
                    Err(e) => {
                        log::error!("Bad Coin String: {} {} {}", height, amount_str, e)
                    }
                },
                None => {
                    Broker::<SystemBroker>::issue_async(MessageBlockEventCommission {
                        height,
                        is_begin,
                        validator: validator_value,
                        amount: vec![],
                    });
                }
            },
            None => log::warn!(
                "Expecting validator key for commission event {} {:#?}",
                height,
                attributes
            ),
        },
        "liveness" => {
            let missed_blocks_o = get_required_kv(&attributes, "missed_blocks");
            let address_o = get_required_kv(&attributes, "address");
            let height_o = get_required_kv(&attributes, "height");
            if let Some(tm_address) = address_o {
                let mb_str = missed_blocks_o.unwrap_or_else(|| "0".into());
                let mb: usize = mb_str.parse().unwrap_or(0);
                Broker::<SystemBroker>::issue_async(MessageBlockEventLiveness {
                    height,
                    is_begin,
                    tendermint_address: tm_address,
                    missed: mb,
                });
            } else {
                log::warn!(
                    "bad message ? Liveness:{}/{} Addr:{} Missed:{}",
                    height,
                    height_o.unwrap_or_default(),
                    address_o.unwrap_or_default(),
                    missed_blocks_o.unwrap_or_default()
                )
            }
        }
        "transfer" => {
            let sender_o = get_required_kv(&attributes, "sender");
            let recipient_o = get_required_kv(&attributes, "recipient");
            let amount_o = get_required_kv(&attributes, "amount");
            log::debug!(
                "Transfer:{} {} {} {}",
                height,
                sender_o.unwrap_or_default(),
                recipient_o.unwrap_or_default(),
                amount_o.unwrap_or_default()
            )
        }
        "message" => {
            let sender_o = get_required_kv(&attributes, "sender");

            log::debug!("Message: {} {}", height, sender_o.unwrap_or_default(),)
        }
        "exchange_rate_update" => {
            let exchange_rate_o = get_required_kv(&attributes, "exchange_rate");
            let denom_o = get_required_kv(&attributes, "denom");

            if let Some(denom) = denom_o {
                if let Some(exchange_rate) = exchange_rate_o {
                    if denom == "uusd" {
                        log::info!(
                            "exchange_rate_update: {} {} {}",
                            height,
                            denom,
                            exchange_rate
                        )
                    } else {
                        log::debug!(
                            "exchange_rate_update: {} {} {}",
                            height,
                            denom,
                            exchange_rate
                        )
                    }

                    if let Ok(ex_rate) = Decimal::from_str(&exchange_rate) {
                        Broker::<SystemBroker>::issue_async(MessageBlockEventExchangeRate {
                            height,
                            denom,
                            exchange_rate: ex_rate,
                        });
                    }
                } else {
                    log::warn!(
                        "exchange_rate_update: {} missing rate for denom: {}",
                        height,
                        denom
                    )
                }
            } else {
                log::warn!(
                    "exchange_rate_update: {} missing denom {}",
                    height,
                    exchange_rate_o.unwrap_or_default()
                )
            }
        }
        "complete_unbonding" => {
            let validator_o = get_required_kv(&attributes, "validator");
            let delegator_o = get_required_kv(&attributes, "delegator");
            let amount_o = get_required_kv(&attributes, "amount");
            log::info!(
                "complete_unbonding: {} {} {} {}",
                height,
                validator_o.unwrap_or_default(),
                delegator_o.unwrap_or_default(),
                amount_o.unwrap_or_default()
            )
        }
        "complete_redelegation" => {
            let validator_o = get_required_kv(&attributes, "validator");
            let delegator_o = get_required_kv(&attributes, "delegator");
            let amount_o = get_required_kv(&attributes, "amount");

            log::info!(
                "complete_redelegation: {} {} {} {}",
                height,
                validator_o.unwrap_or_default(),
                delegator_o.unwrap_or_default(),
                amount_o.unwrap_or_default()
            )
        }
        "mint" => {
            let bonded_ratio_o = get_required_kv(&attributes, "bonded_ratio");
            let amount_o = get_required_kv(&attributes, "amount");
            let inflation_o = get_required_kv(&attributes, "inflation");
            let annual_provisions_o = get_required_kv(&attributes, "annual_provisions");

            log::debug!(
                "mint:{} Bonded:{} amount:{} inflation:{} annual provisions:{}",
                height,
                bonded_ratio_o.unwrap_or_default(),
                amount_o.unwrap_or_default(),
                inflation_o.unwrap_or_default(),
                annual_provisions_o.unwrap_or_default()
            )
        }
        "coin_spent" => {
            // amount
            // spender
        }
        "coin_received" => {
            // receiver
            // amount
        }
        //   "transfer" => {}
        unknown => {
            log::info!(
                "Unrecognized Event: {} {} {:#?}",
                height,
                unknown,
                attributes
            )
        }
    }
}
