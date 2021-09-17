use actix_broker::{Broker, SystemBroker};
use constellation_observer::messages::MessageValidator;
use constellation_shared::state::AppState;
use std::collections::HashMap;
use std::time::Duration;
use terra_rust_api::client::tendermint_types;
use terra_rust_api::Terra;
use tokio::time;

pub async fn run(_state: AppState, period: Duration, chain_id: String, lcd_endpoint: String) {
    log::info!("Validator task starting");
    let mut interval = time::interval(period);
    loop {
        let mut tendermint: HashMap<String, tendermint_types::Validator> = Default::default();
        log::info!("Attempting validator refresh");
        match Terra::lcd_client_no_tx(&lcd_endpoint, &chain_id).await {
            Ok(terra) => {
                let tendermint_validator_set = terra.tendermint().validatorsets_full().await;
                match tendermint_validator_set {
                    Ok(tendermint_list) => {
                        tendermint_list.result.validators.iter().for_each(|v| {
                            tendermint.insert(v.pub_key.clone(), v.clone());
                        });
                    }
                    Err(e) => log::error!("Can't obtain tendermint validator set {}", e),
                }

                match terra.staking().validators().await {
                    Ok(validator_result) => {
                        log::info!("sending {} update messages", validator_result.result.len());
                        validator_result.result.iter().for_each(|v| {
                            Broker::<SystemBroker>::issue_async(MessageValidator {
                                height: validator_result.height,
                                operator_address: v.operator_address.clone(),
                                validator: v.clone(),
                                tendermint: tendermint.get(&v.consensus_pubkey).cloned(),
                            });
                        })
                    }
                    Err(e) => log::error!("can't obtain validators {}", e),
                }
            }
            Err(e) => log::error!("LCD? {}", e),
        }
        interval.tick().await;
    }
}
