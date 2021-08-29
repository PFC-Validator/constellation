use futures::{SinkExt, StreamExt};

use crate::types::NewBlock;
use crate::MessageTX;
use actix_broker::{Broker, SystemBroker};
use constellation_shared::AppState;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::http::Request;
use tokio_tungstenite::tungstenite::Message;

/// VERSION number of package
pub const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
/// NAME of package
pub const NAME: Option<&'static str> = option_env!("CARGO_PKG_NAME");

pub async fn run(
    state: AppState,
    //  tx: mpsc::Sender<()>,
    connect_addr: String,
) -> anyhow::Result<()> {
    //   let oracle_actor = crate::actor::OracleActor.start();
    //let url = url::Url::parse(&connect_addr).unwrap();
    let ws_request = Request::builder()
        .header(
            "User-Agent",
            format!(
                "{}/{}",
                NAME.unwrap_or("Constellation"),
                VERSION.unwrap_or("dev")
            ),
        )
        .uri(&connect_addr)
        .body(())?;

    let (mut ws_stream, _) = connect_async(ws_request).await.expect("Failed to connect");
    log::info!("Connected");
    //  let (mut write, read) = ws_stream.split();
    let msg = Message::text("{\"subscribe\":\"new_block\",\"chain_id\":\"columbus-4\"}");
    let _sndmsg = ws_stream.send(msg).await?;

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
                    Message::Text(text) => match serde_json::from_str::<NewBlock>(&text) {
                        Ok(new_block) => {
                            log::info!(
                                "Block:{} {}",
                                new_block.chain_id,
                                new_block.data.block.header.height
                            );
                            if let Err(e) = process_block_emit(&state, &new_block) {
                                log::error!("Error pushing block to actors: {}", e);
                            }
                            //  process_block(&state, &new_block)?;
                        }
                        Err(e) => {
                            log::error!("Error parsing block: {}", e);
                            log::error!("{}", text);
                        }
                    },
                    Message::Binary(_) => {}
                    Message::Ping(p) => {
                        let pong = Message::Pong(p);
                        ws_stream.send(pong).await?;
                    }
                    Message::Pong(_) => {}
                    Message::Close(_) => {}
                }
            }
            Err(e) => {
                log::error!("{:#?}", e)
            }
        }
    }

    Ok(())
}
fn process_block_emit(_state: &AppState, block: &NewBlock) -> anyhow::Result<()> {
    if let Some(txs) = &block.data.txs {
        txs.iter().for_each(|tx| {
            Broker::<SystemBroker>::issue_async(MessageTX { tx: tx.clone() });
        })
    }
    Ok(())
}
/*
fn _process_block(_state: &AppState, block: &NewBlock) -> anyhow::Result<()> {
    &block
        .data
        .result_begin_block
        .events
        .iter()
        .for_each(|event| {
            let mut validator = None;
            let mut amount: Vec<Coin> = vec![];

            event.attributes.iter().for_each(|attribute| {
                if attribute.key == "validator" {
                    if let Some(v) = &attribute.value {
                        validator = Some(v)
                    } else {
                        validator = None;
                    }
                } else if attribute.key == "amount" {
                    if let Some(v) = &attribute.value {
                        amount = Coin::parse_coins(v).unwrap_or_default()
                    } else if event.s_type != "commission" {
                        log::info!("unable to find coins? {}", event.s_type)
                    }
                } else {
                    // log::info!("{} {}", attribute.key, attribute.value)
                }
            });
            if let Some(v) = validator {
                if v.eq_ignore_ascii_case("terravaloper12g4nkvsjjnl0t7fvq3hdcw7y8dc9fq69nyeu9q") {
                    log::info!("{} {} {:?}", event.s_type, v, amount);
                }
            }
        });
    // let end_block_events = block.data.result_end_block.events;
    if let Some(ebe) = &block.data.result_end_block.events {
        ebe.iter().for_each(|event| {
            let mut validator = None;
            let mut amount: Vec<Coin> = vec![];

            event.attributes.iter().for_each(|attribute| {
                if attribute.key == "validator" {
                    if let Some(v) = &attribute.value {
                        validator = Some(v)
                    } else {
                        validator = None;
                    }
                } else if attribute.key == "amount" {
                    if let Some(v) = &attribute.value {
                        amount = Coin::parse_coins(v).unwrap_or_default()
                    } else if event.s_type != "commission" {
                        log::info!("unable to find coins? {}", event.s_type)
                    }
                } else {
                    // log::info!("{} {}", attribute.key, attribute.value)
                }
            });
            if let Some(v) = validator {
                if v.eq_ignore_ascii_case("terravaloper12g4nkvsjjnl0t7fvq3hdcw7y8dc9fq69nyeu9q") {
                    log::info!("{} {} {:?}", event.s_type, v, amount);
                }
            }
        });
    }
    if let Some(txs) = &block.data.txs {
        let votes = txs
            .iter()
            .flat_map(|tx| {
                tx.tx
                    .value
                    .msg
                    .iter()
                    .filter(|f| f.s_type == "oracle/MsgAggregateExchangeRateVote")
            })
            .collect::<Vec<_>>();
        let x = votes
            .iter()
            .flat_map(|f| {
                if let Some(vv) = f.value.get("validator") {
                    if let Some(validator) = vv.as_str() {
                        if let Some(ee) = f.value.get("exchange_rates") {
                            if let Some(exchange_rates) = ee.as_str() {
                                Some((validator, exchange_rates))
                            } else {
                                log::info!("{} Exchange rates missing?", validator);
                                None
                            }
                        } else {
                            log::info!("{} Exchange rates missing?", validator);
                            None
                        }
                    } else {
                        log::info!("Validator missing?");
                        None
                    }
                } else {
                    log::info!("Validator missing?");
                    None
                }
            })
            .collect::<Vec<_>>();
        if votes.len() != x.len() {
            log::error!("{} {} should be the same?", votes.len(), x.len())
        }
        x.iter().for_each(|x| println!("{} {}", x.0, x.1));
        // log::info!("{:?}", x);
        txs.iter().for_each(|tx| {
            tx.tx.value.msg.iter().for_each(|f| {
                match f.s_type.as_str() {
                    "oracle/MsgAggregateExchangeRateVote" => {
                        // log::info!("{} {}", f.s_type, f.value.to_string())
                    }
                    "oracle/MsgAggregateExchangeRatePrevote" => {
                        //         log::info!("{}", f.s_type)
                        // log::info!("{} {}", f.s_type, f.value.to_string())
                    }
                    _ => {
                        log::info!("{}", f.s_type)
                    }
                };
            })
        })

        /*  */
    }

    Ok(())
}
*/
