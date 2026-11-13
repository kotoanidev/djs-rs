//! The smallest serenity bot: answers `!ping` with `Pong!`. The same bot as `../djs-rs`.

use serenity::{async_trait, model::channel::Message, prelude::*};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(error) = msg.reply(&ctx, "Pong!").await {
                eprintln!("could not reply: {error}");
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), serenity::Error> {
    let token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN is set");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await?;
    client.start().await
}
