use crate::actor::validator::DiscordValidatorActor;
use crate::errors::ConstellationDiscordError::ChannelList;
use actix::Actor;
use anyhow::Result;
use serde_json::Value;
use serenity::model::guild::GuildStatus;
use serenity::model::id::{ChannelId, GuildId};
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::env;

pub(crate) struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            // Sending a message can fail, due to a network error, an
            // authentication error, or lack of permissions to post in the
            // channel, so log to stdout when some error happens, with a
            // description of it.
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                log::warn!("Error sending message: {:?}", why);
            }
        }
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, ctx: Context, ready: Ready) {
        let category_name = env::var("DISCORD_CATEGORY_NAME").unwrap_or("Validators".into());
        log::info!("{} is connected!", ready.user.name);
        ready
            .private_channels
            .iter()
            .for_each(|c| log::info!("{} {}/_", c.0 .0, c.1.id()));
        ready.guilds.iter().for_each(|g| match g {
            GuildStatus::OnlinePartialGuild(pg) => {
                log::info!("Partial {} {} ", pg.id, pg.name);
            }
            GuildStatus::OnlineGuild(g) => {
                log::info!("Online {} {} #{}", g.id, g.name, g.channels.len());
            }
            GuildStatus::Offline(og) => {
                log::info!("Offline {} {}", og.id, og.unavailable);
            }
            _ => {
                log::warn!("Unknown guild status {}", g.id())
            }
        });
        let guilds = ctx.cache.guilds().await;
        //   let guild_opt = guilds.first();
        if let Some(guildid) = guilds.first() {
            match DiscordValidatorActor::create(&ctx.http.token, guildid, &category_name).await {
                Ok(discord_actor) => {
                    discord_actor.start();
                    log::info!("Discord Actor started")
                }
                Err(e) => log::error!("Starting Discord actor:{}", e),
            }

            match find_or_create_validator_channel_categories(&ctx, guildid, &category_name).await {
                Ok(channel_id) => log::info!("Channel ID: {}", channel_id),
                Err(e) => log::error!("Channel:{}", e),
            }
        }
    }
}

async fn find_or_create_validator_channel_categories(
    ctx: &Context,
    guild_id: &GuildId,
    category_name: &str,
) -> Result<ChannelId> {
    let channels = ctx.cache.guild_channels(guild_id).await;
    if let Some(channellist) = channels {
        let v_channel_category = channellist
            .values()
            .filter(|channel| {
                if channel.category_id.is_none() && channel.name.eq(&category_name) {
                    log::info!("**Category ID: {} - name {}", channel.id, channel.name);
                } else {
                    log::info!(
                        "Category ID: {} {} name '{}'",
                        channel.id,
                        channel.category_id.is_none(),
                        channel.name,
                    );
                }
                channel.name.eq(&category_name) && channel.category_id.is_none()
            })
            .collect::<Vec<_>>();
        log::info!("{}", v_channel_category.len());
        if v_channel_category.is_empty() {
            let new_channel: Value = serde_json::from_str(&format!(
                "{{\"name\":\"{}\",\"category\":4,\"type\":4}}",
                category_name
            ))?;

            let channel = ctx
                .http
                .create_channel(guild_id.0, new_channel.as_object().unwrap())
                .await?;
            let new_channel_2: Value = serde_json::from_str(&format!(
                "{{\"name\":\"foobar2\",\"category\":0, \"parent_id\":{}}}",
                channel.id
            ))?;

            let _channel2 = ctx
                .http
                .create_channel(guild_id.0, new_channel_2.as_object().unwrap())
                .await?;
            let new_channel_3: Value = serde_json::from_str(&format!(
                "{{\"name\":\"foobar3\",\"category\":0, \"parent_id\":{}}}",
                channel.id
            ))?;

            let _channel3 = ctx
                .http
                .create_channel(guild_id.0, new_channel_3.as_object().unwrap())
                .await?;

            Ok(channel.id)
        } else {
            log::info!("Found Channel!");
            let channel = v_channel_category.first().unwrap();
            Ok(channel.id)
        }
    } else {
        Err(ChannelList(guild_id.to_string()).into())
    }
}
