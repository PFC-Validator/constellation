use constellation_shared::state::AppState;
use std::time::Duration;
use terra_rust_api::Terra;
use tokio::time;

pub async fn run(
    state: AppState,
    period: Duration,
    chain_id: String,
    lcd_endpoint: String,
    rpc_endpoint: String,
    //  fcd_endpoint: String,
    //gas_denom: String,
    // gas_multiplier: f64,
) -> anyhow::Result<()> {
    log::info!("{} {}", lcd_endpoint, rpc_endpoint);
    let mut interval = time::interval(period);
    loop {
        match Terra::lcd_client_no_tx(&lcd_endpoint, &chain_id).await {
            Ok(terra) => match run_task(&state, &terra, &rpc_endpoint).await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("RPC Crawler: {}", e)
                }
            },
            Err(e) => {
                log::error!("RPC Crawler: {}", e)
            }
        }
        interval.tick().await;
    }
}
pub async fn run_task(
    _state: &AppState,
    terra: &Terra<'_>,
    rpc_endpoint: &str,
) -> anyhow::Result<()> {
    let rpc = terra.rpc(rpc_endpoint);
    match rpc.net_info().await {
        Ok(net_info) => {
            let open_rpc = net_info
                .peers
                .iter()
                .flat_map(|peer| {
                    let rpc_address = &peer.node_info.other.rpc_address;
                    if rpc_address.starts_with("tcp://127.0.0.1")
                        || rpc_address.starts_with("tcp://0.0.0.0")
                        || rpc_address.starts_with("tcp://10.")
                        || rpc_address.starts_with("tcp://192.168.")
                        || rpc_address.starts_with("tcp://172.16.")
                    {
                        None
                    } else {
                        Some(rpc_address.clone())
                    }
                    //    peer.node_info.other.rpc_address.clone()
                })
                .collect::<Vec<_>>();
            let open_peer = net_info
                .peers
                .iter()
                .flat_map(|peer| {
                    let listen_address = &peer.node_info.listen_addr;
                    if listen_address.starts_with("tcp://127.0.0.1")
                        || listen_address.starts_with("tcp://0.0.0.0")
                        || listen_address.starts_with("tcp://10.")
                        || listen_address.starts_with("tcp://192.168.")
                        || listen_address.starts_with("tcp://172.16.")
                    {
                        None
                    } else {
                        Some(format!(
                            "{}@{}",
                            &peer.node_info.id,
                            listen_address
                                .strip_prefix("tcp://")
                                .unwrap_or(listen_address)
                        ))
                    }
                    //    peer.node_info.other.rpc_address.clone()
                })
                .collect::<Vec<_>>();
            if !open_rpc.is_empty() {
                log::warn!("found {} open rpc {:?}", open_rpc.len(), open_rpc);
            }
            if !open_peer.is_empty() {
                log::info!("found {} open peers", open_peer.len());
            }
            log::info!("{} connections", net_info.peers.len())
        }
        Err(e) => {
            log::error!("RPC: {} {}", rpc_endpoint, e);
        }
    }

    Ok(())
}
