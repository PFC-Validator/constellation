use constellation_shared::state::AppState;
use std::time::Duration;
use terra_rust_api::Terra;
use tokio::time;

pub async fn run(
    state: AppState,
    period: Duration,
    chain_id: String,
    lcd_endpoint: String,
) -> anyhow::Result<()> {
    log::debug!("LCD {} ", lcd_endpoint);
    let mut interval = time::interval(period);
    loop {
        match Terra::lcd_client_no_tx(&lcd_endpoint, &chain_id).await {
            Ok(terra) => match run_task(&state, &terra).await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("running task: {}", e)
                }
            },
            Err(e) => {
                log::error!("creating LCD: {}", e)
            }
        }
        interval.tick().await;
    }
}
async fn run_task(_state: &AppState, terra: &Terra<'_>) -> anyhow::Result<()> {
    let _validator = terra.staking().validators().await?;

    Ok(())
}
