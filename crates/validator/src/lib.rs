pub mod actor;

mod task;

use actix_broker::SystemBroker;
pub use task::run;
pub(crate) type BrokerType = SystemBroker;
