//! The guide's first bot ("Your first bot"): it answers `!ping` with `Pong!`.
//!
//! Run it with `DISCORD_TOKEN=<token> cargo run -p djs-rs-guide --example first_bot`.

// ANCHOR: all
use djs_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: logs
    // Print the library's logs (connections, warnings, a handler's errors).
    tracing_subscriber::fmt::init();
    // ANCHOR_END: logs

    // ANCHOR: client
    let token = std::env::var("DISCORD_TOKEN")?;
    let client = Client::builder(token)
        .intents(Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT)
        .build();
    // ANCHOR_END: client

    // ANCHOR: ready
    client.on(Events::ClientReady, |_ctx, ready| async move {
        println!("Logged in as {}", ready.user.tag());
        Ok(())
    });
    // ANCHOR_END: ready

    // ANCHOR: message
    client.on(Events::MessageCreate, |ctx, msg| async move {
        if msg.content == "!ping" {
            msg.reply(&ctx, "Pong!").await?;
        }
        Ok(())
    });
    // ANCHOR_END: message

    // ANCHOR: shutdown
    // Ctrl+C disconnects the bot cleanly; `login` then returns.
    let handle = client.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            handle.shutdown();
        }
    });
    // ANCHOR_END: shutdown

    // ANCHOR: login
    client.login().await?;
    Ok(())
    // ANCHOR_END: login
}
// ANCHOR_END: all
