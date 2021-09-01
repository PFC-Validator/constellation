use std::collections::HashMap;

use actix::prelude::*;
use actix_broker::BrokerSubscribe;

use crate::handler::SerentityHandler;
use chrono::{DateTime, Utc};
use constellation_observer::messages::{MessageValidator, MessageValidatorEvent};
use constellation_observer::BrokerType;
use serenity::framework::StandardFramework;
use serenity::model::id::{ChannelId, GuildId};
use serenity::Client;

pub struct BlockHeightTime {
    pub datetime: DateTime<Utc>,
    pub height: Option<u64>,
}
pub struct GuildChannel {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
}
pub struct DiscordValidatorActor {
    pub discord_client: Client,
    //todo deal with multiple Guilds/Servers
    pub channel_map: HashMap<String, GuildChannel>,
    pub channel_last_seen: HashMap<String, BlockHeightTime>,
    pub channel_last_alert: HashMap<String, BlockHeightTime>,
    pub category_prefix: String,
}

impl DiscordValidatorActor {
    pub async fn create(
        token: &str,
        framework: StandardFramework,
        category_prefix: &str,
    ) -> anyhow::Result<(DiscordValidatorActor)> {
        log::info!("Discord Starting");
        let client = Client::builder(&token)
            .event_handler(SerentityHandler)
            .framework(framework)
            .await
            .expect("Err creating client");

        Ok((DiscordValidatorActor {
            discord_client: client,
            channel_last_alert: Default::default(),
            channel_last_seen: Default::default(),
            channel_map: Default::default(),
            category_prefix: category_prefix.into(),
        }))
    }
}
impl Actor for DiscordValidatorActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.subscribe_sync::<BrokerType, MessageValidator>(ctx);
        self.subscribe_sync::<BrokerType, MessageValidatorEvent>(ctx);
        log::info!("Discord Validator Actor started!");
        let _ignore = self.discord_client.start();
        log::info!("Discord Client Actor started!");
    }
}
impl Handler<MessageValidator> for DiscordValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: MessageValidator, _ctx: &mut Self::Context) {
        let height = msg.height;
        log::info!(
            "Discord Validator {} {}",
            height,
            msg.validator.description.moniker
        );
    }
}

impl Handler<MessageValidatorEvent> for DiscordValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: MessageValidatorEvent, _ctx: &mut Self::Context) {
        let height = msg.height;
    }
}
