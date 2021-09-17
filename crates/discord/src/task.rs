//use crate::commands::*;
//use actix_broker::{Broker, SystemBroker};
//use actix::Actor;
use actor_discord::DiscordAPI;
use actor_discord::DiscordBot;
use actor_discord::GatewayIntents;
use constellation_shared::state::AppState;
use tokio::time::Duration;

pub async fn run(_state: AppState, discord_token: String, discord_url: String, max_retries: usize) {
    let intents: GatewayIntents = GatewayIntents::GUILDS
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::DIRECT_MESSAGE_REACTIONS;
    loop {
        match DiscordAPI::create(&discord_token, &discord_url, max_retries) {
            Ok(discord_api) => match DiscordBot::create(&discord_api, intents).await {
                Ok(mut discord_bot) => match discord_bot.start_websocket().await {
                    Ok(_) => {}
                    Err(e) => log::error!("Error websocket:{}", e),
                },
                Err(e) => log::error!("Error creating bot:{}", e),
            },
            Err(e) => log::error!("Error creatung api:{}", e),
        };
        log::warn!("Discord Disconnected..waiting 2 secs and trying again");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
