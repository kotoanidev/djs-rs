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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let token = config_value("DISCORD_TOKEN")
        .ok_or("DISCORD_TOKEN is not set (put it in .env or the environment)")?;

    let client = Client::builder(token)
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
    let shutdown = client.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            println!("Shutting down…");
            shutdown.shutdown();
        }
    });

    client.login().await?;
    Ok(())
}

/// Reads a setting from the environment, falling back to a `.env` file in the current directory
/// or one of its parents (`KEY=VALUE` lines, `#` comments). Kept dependency-free on purpose; the
/// environment is not modified.
fn config_value(key: &str) -> Option<String> {
    if let Ok(value) = std::env::var(key) {
        return Some(value);
    }
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if let Ok(contents) = std::fs::read_to_string(dir.join(".env")) {
            return contents.lines().find_map(|line| {
                let (name, value) = line.trim().split_once('=')?;
                (name.trim() == key).then(|| value.trim().trim_matches('"').to_owned())
            });
        }
        if !dir.pop() {
            return None;
        }
    }
}
