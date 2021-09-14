use std::collections::HashMap;

use actix::prelude::*;
use actix_broker::BrokerSubscribe;
use actor_discord::types::events::{
    ChannelEvent, ChannelType, Event, GuildChannelCreate, MessageCreate, MessageEvent, SnowflakeID,
};
use actor_discord::DiscordAPI;
use chrono::{DateTime, Utc};
use constellation_observer::messages::{
    MessageValidator, MessageValidatorEvent, ValidatorEventType,
};
use constellation_observer::BrokerType;
use std::collections::hash_map::Entry;

pub struct BlockHeightTime {
    pub datetime: DateTime<Utc>,
    pub height: Option<u64>,
}
#[derive(Copy, Clone, Debug)]
pub struct GuildChannel {
    pub guild_id: SnowflakeID,
    pub channel_id: SnowflakeID,
}
pub struct DiscordValidatorActor {
    //todo deal with multiple Guilds/Servers
    pub guild_id: Option<SnowflakeID>,
    pub channel_map: HashMap<String, GuildChannel>, // channel name to Guild info
    pub channel_last_seen: HashMap<String, BlockHeightTime>, // operator address -> block-height
    pub channel_last_alert: HashMap<String, BlockHeightTime>, // operator address ->
    pub discord_validator_map: HashMap<SnowflakeID, String>, // discord channel id -> operator id
    pub validator_discord_map: HashMap<String, SnowflakeID>, // operator id -> discord channel id
    pub validator_moniker_map: HashMap<String, String>, // operator id -> moniker
    //   pub category_prefix: String,
    pub token: String,
    pub connect_addr: String,
    pub announcement_channel: Option<GuildChannel>,
    pub max_retries: usize,
}

impl DiscordValidatorActor {
    pub async fn create(
        token: &str,
        //   category_prefix: &str,
        connect_addr: &str,
        max_retries: usize,
    ) -> anyhow::Result<DiscordValidatorActor> {
        log::info!("Discord Starting");

        Ok(DiscordValidatorActor {
            channel_last_alert: Default::default(),
            channel_last_seen: Default::default(),
            channel_map: Default::default(),
            //         category_prefix: category_prefix.into(),
            token: token.into(),
            connect_addr: connect_addr.into(),
            discord_validator_map: Default::default(),
            validator_discord_map: Default::default(),
            validator_moniker_map: Default::default(),
            guild_id: None,
            announcement_channel: None,
            max_retries,
        })
    }
}
impl Actor for DiscordValidatorActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.subscribe_sync::<BrokerType, MessageValidator>(ctx);
        self.subscribe_sync::<BrokerType, MessageValidatorEvent>(ctx);
        self.subscribe_sync::<BrokerType, Event>(ctx);
        self.subscribe_sync::<BrokerType, MessageEvent>(ctx);
        self.subscribe_sync::<BrokerType, ChannelEvent>(ctx);
        log::info!("Discord Validator Actor started!");
    }
}
impl Handler<MessageValidator> for DiscordValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: MessageValidator, ctx: &mut Self::Context) {
        let height = msg.height;
        log::debug!(
            "Discord Validator {} {}",
            height,
            msg.validator.description.moniker
        );
        //TODO check if moniker has changed, and if so.. rename the channel
        self.validator_moniker_map.insert(
            msg.operator_address.clone(),
            msg.validator.description.moniker.clone(),
        );
        let sanitized = DiscordAPI::sanitize(&msg.validator.description.moniker);
        if let Some(guild) = self.guild_id {
            if !self.channel_map.contains_key(&sanitized) {
                match DiscordAPI::create(&self.token, &self.connect_addr, self.max_retries) {
                    Ok(discord_api) => {
                        let details = GuildChannelCreate::simple(
                            ChannelType::GuildText,
                            &sanitized,
                            Some(msg.operator_address),
                            None,
                        );
                        log::info!(
                            "Attempting to create channel {} {} {}",
                            self.channel_map.len(),
                            &msg.validator.description.moniker,
                            sanitized
                        );

                        async move {
                            let create_ch = discord_api.create_channel(guild, details).await;
                            match create_ch {
                                Ok(ch) => log::info!("Channel {} being created", ch.name),
                                Err(e) => log::error!("Error {}", e),
                            }
                        }
                        .into_actor(self)
                        .spawn(ctx)
                    }
                    Err(e) => {
                        log::error!("Unable to build discord API? {}", e)
                    }
                }
            }
        }
    }
}

impl Handler<MessageValidatorEvent> for DiscordValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: MessageValidatorEvent, ctx: &mut Self::Context) {
        let height = msg.height;
        let channel_opt = self.validator_discord_map.get(&msg.operator_address);

        let operator = msg.operator_address;
        let moniker = match msg.moniker {
            Some(m) => m,
            None => match self.validator_moniker_map.get(&operator) {
                Some(m) => m.clone(),
                None => {
                    log::info!(
                        "map len {} not found {}",
                        self.validator_moniker_map.len(),
                        &operator
                    );
                    operator.clone()
                }
            },
        };
        let hash_url = msg.hash.map(|hash| {
            format!(
                "[hash](https://finder.extraterrestrial.money/columbus-4/tx/{})",
                hash
            )
        });
        let announce = if self.announcement_channel.is_some() {
            match msg.event_type {
                ValidatorEventType::WARN => {
                    let warn_message = format!(
                        "WARN @{} {} {}",
                        height,
                        moniker,
                        &msg.message,
                        //    hash_url
                    );
                    Some(MessageCreate::markdown(warn_message, &hash_url))
                }
                ValidatorEventType::ERROR => {
                    let err_message = format!(
                        "ERROR @{} {} {}",
                        height,
                        moniker,
                        &msg.message,
                        //   hash_url
                    );
                    Some(MessageCreate::markdown(err_message, &hash_url))
                }
                ValidatorEventType::CRITICAL => {
                    let err_message = format!(
                        "CRITICAL @{} {} {}",
                        height,
                        moniker,
                        &msg.message,
                        // hash_url
                    );
                    Some(MessageCreate::markdown(err_message, &hash_url))
                }
                _ => None,
            }
        } else {
            None
        };
        let formatted_message = format!("Height:{} {}", height, &msg.message);
        log::debug!("WE have a message! {} {}", &moniker, msg.message);
        if let Some(channel_id) = channel_opt {
            match DiscordAPI::create(&self.token, &self.connect_addr, self.max_retries) {
                Ok(api) => {
                    let message = MessageCreate::markdown(formatted_message, &hash_url);
                    let channel = *channel_id;
                    let announcement_channel = self.announcement_channel;

                    //  let moniker = msg.moniker.unwrap_or(operator.clone()).clone();
                    async move {
                        if let Some(announce_msg) = announce {
                            let _ann_result = api
                                .create_message(
                                    announcement_channel.unwrap().channel_id,
                                    announce_msg,
                                )
                                .await;
                        };
                        let msg_result = api.create_message(channel, message).await;
                        match msg_result {
                            Ok(m) => {
                                log::debug!(
                                    "message sent to {} {}",
                                    moniker.clone(),
                                    m.id.to_string()
                                )
                            }
                            Err(e) => log::error!("Error sending message {}", e),
                        };
                    }
                    .into_actor(self)
                    .spawn(ctx)
                }
                Err(e) => {
                    log::error!("Unable to create discord api {}", e)
                }
            }
        } else {
            log::warn!("Channel for {} not found", operator);
        }
    }
}
impl Handler<Event> for DiscordValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: Event, ctx: &mut Self::Context) {
        match msg {
            Event::INIT => {
                log::info!("IN INIT");
            }
            Event::GuildCreate(gc) => {
                let channels: HashMap<String, GuildChannel> = gc
                    .channels
                    .iter()
                    .map(|channel| {
                        (
                            channel.name.clone(),
                            GuildChannel {
                                guild_id: gc.id,
                                channel_id: channel.id,
                            },
                        )
                    })
                    .collect::<_>();
                self.channel_map = channels;
                self.guild_id = Some(gc.id);

                for c in gc.channels {
                    if c.name == "announcements" {
                        self.announcement_channel = Some(GuildChannel {
                            guild_id: gc.id,
                            channel_id: c.id,
                        })
                    }
                    if let Some(topic) = c.topic {
                        if topic.starts_with("terravaloper1") {
                            self.validator_discord_map.insert(topic.clone(), c.id);
                            self.discord_validator_map.insert(c.id, topic);
                        }
                    }
                }

                if !self.channel_last_seen.is_empty() {
                    let names = self
                        .channel_last_seen
                        .iter()
                        .filter(|c| self.channel_map.contains_key(c.0))
                        .map(|c| c.0.clone())
                        .collect::<Vec<_>>();
                    async move {
                        log::info!("{} missing", names.join(","));
                    }
                    .into_actor(self)
                    .spawn(ctx)
                }
            }
        }
    }
}
impl Handler<ChannelEvent> for DiscordValidatorActor {
    type Result = ();

    fn handle(&mut self, msg: ChannelEvent, _ctx: &mut Self::Context) {
        if let Some(guild_id) = self.guild_id {
            match msg {
                ChannelEvent::ChannelCreate(gc) | ChannelEvent::ChannelUpdate(gc) => {
                    if let Some(topic) = gc.topic {
                        if topic.starts_with("terravaloper1") {
                            self.channel_map.insert(
                                gc.name.clone(),
                                GuildChannel {
                                    guild_id,
                                    channel_id: gc.id,
                                },
                            );
                            log::info!("Channel created/updated {}", gc.name);
                            let channel_id = gc.id;
                            let name = gc.name;

                            match self.validator_discord_map.entry(topic.clone()) {
                                Entry::Occupied(mut oc) => {
                                    if oc.get().id != channel_id.id {
                                        log::warn!("Channel changed? {} {}", topic, name)
                                    } else {
                                        oc.insert(channel_id);
                                    }
                                }
                                Entry::Vacant(ve) => {
                                    ve.insert(channel_id);
                                }
                            }
                            match self.discord_validator_map.entry(channel_id) {
                                Entry::Occupied(mut oc) => {
                                    if oc.get() != &topic {
                                        log::warn!("Channel changed? {} {}", topic, name)
                                    } else {
                                        oc.insert(topic);
                                    }
                                }
                                Entry::Vacant(ve) => {
                                    ve.insert(topic);
                                }
                            }
                        }
                    }
                }

                ChannelEvent::ChannelDelete(gc) => {
                    if let Some(topic) = gc.topic {
                        if topic.starts_with("terravaloper1") {
                            self.channel_map.remove(&gc.name);
                            self.discord_validator_map.remove(&gc.id);
                            self.validator_discord_map.remove(&topic);
                            log::info!("Channel {} removed", gc.name)
                        }
                    }
                }
            };
        }
    }
}
impl Handler<MessageEvent> for DiscordValidatorActor {
    type Result = ();

    fn handle(&mut self, _msg: MessageEvent, _ctx: &mut Self::Context) {}
}
