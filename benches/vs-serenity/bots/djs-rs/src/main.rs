//! The smallest djs-rs bot: answers `!ping` with `Pong!`. The same bot as `../serenity`.

use djs_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), djs_rs::Error> {
    let token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN is set");
    let client = Client::builder(token)
        .intents(Intents::GUILD_MESSAGES | Intents::DIRECT_MESSAGES | Intents::MESSAGE_CONTENT)
        .build();
    client.on(Events::MessageCreate, |ctx, msg| async move {
        if msg.content == "!ping" {
            msg.reply(&ctx, "Pong!").await?;
        }
        Ok(())
    });
    client.login().await
}
