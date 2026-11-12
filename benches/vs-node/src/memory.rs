//! Scenario 3: the memory taken by the guilds a bot receives at startup.
//!
//! The bot is a djs-rs [`Client`] with the cache discord.js keeps by default
//! ([`CacheConfig::discord_js`]: guilds, members, users, channels, roles, emojis, …; D-174),
//! so both sides keep the same data.

use djs_model::gateway::Intents;
use djs_rs::{Client, Shards, cache::CacheConfig, gateway::Compression};
use serde_json::json;

use crate::{Options, TOKEN, metrics::status_kib};

/// Connects to the mock gateway, caches the `--guilds` guilds it sends and prints the
/// resident memory before and after.
pub(crate) async fn run(options: &Options, allocator: &str) {
    let expected: usize = options.get("guilds", 100);
    let client = Client::builder(TOKEN)
        .intents(Intents::GUILDS | Intents::GUILD_MEMBERS)
        .gateway_url(options.required("gateway"))
        .shards(Shards::Count(1))
        .compression(Compression::None)
        .cache(CacheConfig::discord_js())
        .build();
    // Subscribers see an event after the cache has applied it.
    let mut events = client.events();
    crate::login(&client);

    let mut connected_rss = 0;
    let mut received = 0;
    while let Some(event) = events.next().await {
        // Guilds listed in READY arrive as `guildAvailable`, others as `guildCreate` (D-223).
        match event.event.name() {
            "shardReady" => {
                connected_rss = status_kib("VmRSS");
                if expected == 0 {
                    break;
                }
            }
            "guildAvailable" | "guildCreate" => {
                received += 1;
                if received == expected {
                    break;
                }
            }
            _ => {}
        }
    }
    let cache = client.cache().expect("the client has a cache");
    let members: usize = cache
        .guilds()
        .keys()
        .into_iter()
        .map(|guild| cache.members(guild).len())
        .sum();
    println!(
        "{}",
        json!({
            "scenario": "memory",
            "library": "djs-rs",
            "variant": "Client + CacheConfig::discord_js()",
            "allocator": allocator,
            "guilds": cache.guilds().len(),
            "members": members,
            "connected_rss_kib": connected_rss,
            "rss_kib": status_kib("VmRSS"),
            "peak_rss_kib": status_kib("VmHWM"),
        })
    );
    std::process::exit(0);
}
