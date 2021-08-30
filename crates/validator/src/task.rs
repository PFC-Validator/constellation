use actix_broker::{Broker, SystemBroker};
use constellation_observer::messages::MessageValidator;
use constellation_shared::state::AppState;
use std::time::Duration;
use terra_rust_api::Terra;
use tokio::time;

pub async fn run(
    _state: AppState,
    period: Duration,
    chain_id: String,
    lcd_endpoint: String,
) -> anyhow::Result<()> {
    let mut interval = time::interval(period);
    loop {
        match Terra::lcd_client_no_tx(&lcd_endpoint, &chain_id).await {
            Ok(terra) => {
                let validator_result = terra.staking().validators().await?;
                validator_result.result.iter().for_each(|v| {
                    Broker::<SystemBroker>::issue_async(MessageValidator {
                        height: validator_result.height,
                        operator_address: v.operator_address.clone(),
                        validator: v.clone(),
                    });
                })
            }
            Err(e) => log::error!("LCD? {}", e),
        }

        interval.tick().await;
    }
}
