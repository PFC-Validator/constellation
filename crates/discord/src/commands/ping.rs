use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::Message;
use serenity::prelude::*;
#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;

    Ok(())
}
