use actix::prelude::*;

#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct MessageStop {}