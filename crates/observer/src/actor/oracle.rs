use crate::{BrokerType, MessageTX};
use actix::prelude::*;
use actix_broker::BrokerSubscribe;

pub struct OracleActor;
impl Actor for OracleActor {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        self.subscribe_sync::<BrokerType, MessageTX>(ctx);
        //   self.issue_async::<BrokerType, _>(MessageOne("hello".to_string()));
    }
}
impl Handler<MessageTX> for OracleActor {
    type Result = ();

    fn handle(&mut self, msg: MessageTX, _ctx: &mut Self::Context) {
        if msg.tx.tx.s_type == "core/StdTx" {
            let messages = msg.tx.tx.value;
            messages.msg.iter().for_each(|message| {
                if message.s_type == "oracle/MsgAggregateExchangeRateVote" {
                    log::info!("{} {}", message.s_type, message.value)
                }
                if message.s_type == "oracle/MsgAggregateExchangeRatePrevote" {
                    log::info!("{} {}", message.s_type, message.value)
                }
            })
        } else {
            log::info!("Height: {} Type {}", msg.tx.height, msg.tx.tx.s_type);
        }
    }
}
