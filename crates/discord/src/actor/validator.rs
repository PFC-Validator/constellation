use std::collections::HashMap;

use actix::prelude::*;
use actix_broker::BrokerSubscribe;

use chrono::{DateTime, Utc};
use constellation_observer::messages::{MessageValidator, MessageValidatorEvent};
use constellation_observer::BrokerType;
use serenity::http::Http;
use serenity::model::id::{ChannelId, GuildId};
use serenity::Client;

pub struct BlockHeightTime {
    pub datetime: DateTime<Utc>,
    pub height: Option<u64>,
}
pub struct DiscordValidatorActor {
    pub discord_client: Client,
    //todo deal with multiple Guilds/Servers
    pub guild_id: GuildId,
    pub channel_map: HashMap<String, ChannelId>,
    pub channel_last_seen: HashMap<String, BlockHeightTime>,
    pub channel_last_alert: HashMap<String, BlockHeightTime>,
    pub category_prefix: String,
}
impl DiscordValidatorActor {
    pub async fn create(
        token: &str,
        guild_id: &GuildId,
        category_prefix: &str,
    ) -> anyhow::Result<DiscordValidatorActor> {
        //tODO fix this
        let client = Client::builder(token).await?;
        Ok(DiscordValidatorActor {
            discord_client: client,
            guild_id: guild_id.clone(),
            channel_last_alert: Default::default(),
            channel_last_seen: Default::default(),
            channel_map: Default::default(),
            category_prefix: category_prefix.into(),
        })
    }
}
impl Actor for DiscordValidatorActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.subscribe_sync::<BrokerType, MessageValidator>(ctx);
        self.subscribe_sync::<BrokerType, MessageValidatorEvent>(ctx);
    }
}
impl Handler<MessageValidator> for DiscordValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: MessageValidator, _ctx: &mut Self::Context) {
        let height = msg.height;
    }
}

impl Handler<MessageValidatorEvent> for DiscordValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: MessageValidatorEvent, _ctx: &mut Self::Context) {
        let height = msg.height;
    }
}
