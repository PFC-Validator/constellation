use crate::commands::*;
//use actix_broker::{Broker, SystemBroker};
use actix::Actor;
use anyhow::Result;
use constellation_shared::state::AppState;
use serenity::framework::standard::{macros::group, StandardFramework};
use serenity::http::{GuildPagination, Http};
use serenity::model::prelude::{GuildId, GuildInfo};
//use serenity::prelude::*;
use crate::actor::validator::DiscordValidatorActor;
use serenity::Client;
use std::env;
use std::time::Duration;

#[group]
#[commands(ping)]
struct General;

pub async fn run(_state: AppState, _period: Duration) -> Result<()> {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let category_name = env::var("DISCORD_CATEGORY_NAME").unwrap_or("Validators".into());
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    match DiscordValidatorActor::create(&token, framework, &category_name).await {
        Ok((discord_actor)) => {
            log::info!("Discord Actor starting");
            discord_actor.start();
            /*     if let Err(why) = client.start().await {
                println!("An error occurred while running the client: {:?}", why);
            }*/
        }
        Err(e) => log::error!("Starting Discord actor:{}", e),
    }

    Ok(())

    /*
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Could not register ctrl+c handler");
            shard_manager.lock().await.shutdown_all().await;
        });
    */
}
pub async fn _get_all_guilds(client: &Http) -> Result<Vec<GuildInfo>> {
    let mut last_guild_id = Some(0u64);
    let mut guilds: Vec<GuildInfo> = vec![];
    while let Some(after) = last_guild_id {
        let mut batch = client
            .get_guilds(&GuildPagination::After(GuildId(after)), 100)
            .await?;
        guilds.append(&mut batch);
        last_guild_id = batch.last().map(|guild| *guild.id.as_u64());
    }
    Ok(guilds)
}
