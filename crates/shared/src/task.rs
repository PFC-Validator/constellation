use crate::messages::MessageTick;
use actix_broker::{Broker, SystemBroker};
use chrono::Utc;
use std::time::Duration;
use tokio::time;

pub async fn run(period: Duration) {
    log::info!("Tick task starting");
    let mut interval = time::interval(period);
    loop {
        let now = Utc::now();
        Broker::<SystemBroker>::issue_async(MessageTick {
            duration: period,
            now,
        });
        interval.tick().await;
    }
}
