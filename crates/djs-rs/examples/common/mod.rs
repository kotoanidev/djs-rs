//! Helpers shared by the examples: settings from `.env`, logging and a clean Ctrl+C.
//!
//! Each example includes this file with `#[path = "common/mod.rs"] mod common;`. Cargo only
//! treats `examples/*.rs` files (and directories with a `main.rs`) as examples, so this
//! directory is not built on its own.

// Each example uses only some of the helpers.
#![allow(dead_code)]

use djs_rs::{Client, model::id::GuildId};

/// Prints the library's logs according to `RUST_LOG` (default: `info`), e.g.
/// `RUST_LOG=djs_gateway=debug,djs_rest=debug`.
pub(crate) fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
}

/// Reads a setting from the environment, falling back to a `.env` file in the current
/// directory or one of its parents (`KEY=VALUE` lines, `#` comments). Empty values count as
/// unset. Kept dependency-free on purpose; the environment is not modified.
pub(crate) fn config_value(key: &str) -> Option<String> {
    if let Ok(value) = std::env::var(key) {
        return Some(value).filter(|value| !value.is_empty());
    }
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if let Ok(contents) = std::fs::read_to_string(dir.join(".env")) {
            return contents.lines().find_map(|line| {
                let (name, value) = line.trim().split_once('=')?;
                let value = value.trim().trim_matches('"');
                (name.trim() == key && !value.is_empty()).then(|| value.to_owned())
            });
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// The bot token, from `DISCORD_TOKEN`.
///
/// # Errors
///
/// When `DISCORD_TOKEN` is not set.
pub(crate) fn token() -> Result<String, &'static str> {
    config_value("DISCORD_TOKEN")
        .ok_or("DISCORD_TOKEN is not set (put it in .env or the environment)")
}

/// The guild to register the examples' commands in: `DISCORD_TEST_GUILD_ID` if set, else
/// `None` (the examples then take the first guild of `READY`).
///
/// # Errors
///
/// When `DISCORD_TEST_GUILD_ID` is not an ID.
pub(crate) fn test_guild() -> Result<Option<GuildId>, &'static str> {
    config_value("DISCORD_TEST_GUILD_ID")
        .map(|id| {
            id.parse()
                .map_err(|_| "DISCORD_TEST_GUILD_ID is not a guild ID")
        })
        .transpose()
}

/// Whether the example was started with `-- --cleanup` (remove what it registered, then
/// exit).
pub(crate) fn cleanup_requested() -> bool {
    std::env::args().any(|argument| argument == "--cleanup")
}

/// Disconnects `client` cleanly when Ctrl+C is pressed: [`Client::login`] then returns.
pub(crate) fn shutdown_on_ctrl_c(client: &Client) {
    let client = client.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            println!("Shutting down…");
            client.shutdown();
        }
    });
}
