//! A minimal bot that answers `!ping` with `Pong!`.
//!
//! # Running
//!
//! 1. Create a bot in the [Developer Portal](https://discord.com/developers/applications),
//!    enable the **Message Content** privileged intent and invite the bot to a test server.
//! 2. Put the token in a `.env` file at the repository root (see `.env.example`):
//!    `DISCORD_TOKEN=your-token`. The file is git-ignored.
//! 3. Run `cargo run --example ping` and send `!ping` in a channel the bot can see.
//!
//! Library logs are printed according to `RUST_LOG` (default: `info`), e.g.
//! `RUST_LOG=djs_gateway=debug,djs_rest=debug cargo run --example ping`.
//!
//! Press Ctrl+C to disconnect cleanly.

use djs_rs::prelude::*;

#[path = "common/mod.rs"]
mod common;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    common::init_tracing();

    let client = Client::builder(common::token()?)
        .intents(Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT)
        .build();

    client.on(Events::ClientReady, |_ctx, ready| async move {
        println!(
            "Logged in as {} — in {} guild(s). Send `!ping` to test.",
            ready.user.tag(),
            ready.guilds.len()
        );
        Ok(())
    });

    client.on(Events::MessageCreate, |ctx, msg| async move {
        if msg.content == "!ping" {
            let started = std::time::Instant::now();
            let reply = msg.reply(&ctx, "Pong!").await?;
            println!(
                "Replied to {} with message {} in {:?}",
                msg.author.tag(),
                reply.id,
                started.elapsed()
            );
        }
        Ok(())
    });

    // Disconnect cleanly on Ctrl+C.
    common::shutdown_on_ctrl_c(&client);

    client.login().await?;
    Ok(())
}
