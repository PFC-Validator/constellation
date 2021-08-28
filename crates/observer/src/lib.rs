pub mod actor;
mod b64;
mod observer_intake;
pub mod types;

pub use crate::types::MessageTX;
use actix_broker::SystemBroker;
pub use observer_intake::run;
pub type BrokerType = SystemBroker;
