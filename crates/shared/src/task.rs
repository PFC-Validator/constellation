use crate::messages::MessageTick;
use actix_broker::{Broker, SystemBroker};
use std::time::Duration;
use tokio::time;

pub async fn run(period: Duration) {
    log::info!("Tick task starting");
    let mut interval = time::interval(period);
    loop {
        Broker::<SystemBroker>::issue_async(MessageTick {});
        interval.tick().await;
    }
}
