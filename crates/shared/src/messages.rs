use actix::prelude::*;
use chrono::{DateTime, Utc};
use tokio::time::Duration;

#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessageStop {}
#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessageTick {
    pub duration: Duration,
    pub now: DateTime<Utc>,
}
