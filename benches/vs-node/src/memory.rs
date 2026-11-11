//! Scenario 3: the memory taken by the guilds a bot receives at startup.
//!
//! djs-rs has no cache layer yet (Phase 5), so the bot here keeps every decoded
//! [`AvailableGuild`] — members, users, roles, channels, emojis — in a `HashMap`, which is
//! everything a cache could store.

use std::collections::HashMap;

use djs_gateway::{Compression, Config, Shard};
use djs_model::{
    gateway::{
        DispatchEvent, GatewayEvent, Intents, ShardId,
        payload::{AvailableGuild, GuildCreate},
    },
    id::GuildId,
};
use serde_json::json;

use crate::{Options, TOKEN, metrics::status_kib};

/// Connects to the mock gateway, keeps the `--guilds` guilds it sends and prints the
/// resident memory before and after.
pub(crate) async fn run(options: &Options, allocator: &str) {
    let expected: usize = options.get("guilds", 100);
    let config = Config::builder(TOKEN, Intents::GUILDS | Intents::GUILD_MEMBERS)
        .gateway_url(options.required("gateway"))
        .compression(Compression::None)
        .build();
    let mut shard = Shard::with_config(ShardId::ONE, config);
    let mut guilds: HashMap<GuildId, Box<AvailableGuild>> = HashMap::new();
    let mut connected_rss = 0;
    while let Some(event) = shard.next_event().await {
        match event.expect("the mock gateway sends valid payloads") {
            GatewayEvent::Dispatch(_, DispatchEvent::Ready(_)) => {
                connected_rss = status_kib("VmRSS");
                if expected == 0 {
                    break;
                }
            }
            GatewayEvent::Dispatch(_, DispatchEvent::GuildCreate(guild)) => {
                if let GuildCreate::Available(guild) = *guild {
                    guilds.insert(guild.id, guild);
                }
                if guilds.len() == expected {
                    break;
                }
            }
            _ => {}
        }
    }
    let members: usize = guilds.values().map(|guild| guild.members.len()).sum();
    println!(
        "{}",
        json!({
            "scenario": "memory",
            "library": "djs-rs",
            "variant": "decoded guilds kept in a HashMap",
            "allocator": allocator,
            "guilds": guilds.len(),
            "members": members,
            "connected_rss_kib": connected_rss,
            "rss_kib": status_kib("VmRSS"),
            "peak_rss_kib": status_kib("VmHWM"),
        })
    );
    std::process::exit(0);
}
