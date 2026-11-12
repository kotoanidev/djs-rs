//! Sweeping the cache (D-167, D-168).

#[expect(
    dead_code,
    reason = "shared with the djs-model tests, which use other helpers"
)]
#[path = "../../djs-model/tests/common/payloads.rs"]
mod payloads;

use core::time::Duration;

use djs_cache::{Cache, CacheConfig, Limit};
use djs_model::{
    gateway::payload::{GuildCreate, MessageCreate, ThreadCreate},
    id::{ChannelId, GuildId, MessageId},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

const GUILD: GuildId = GuildId::new(payloads::GUILD);
const HOUR: Duration = Duration::from_secs(3_600);

fn decode<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn guild_create() -> GuildCreate {
    let value: Value = serde_json::from_str(&payloads::guild_create(1, 10, 10, 5, 4)).unwrap();
    decode(value["d"].clone())
}

/// The `MESSAGE_CREATE` fixture (sent 2026-11-10), in `channel`, with `id`.
fn message(channel: u64, id: u64, timestamp: Option<&str>) -> MessageCreate {
    let mut value: Value = serde_json::from_str(payloads::MESSAGE_CREATE).unwrap();
    let mut data = value["d"].take();
    data["channel_id"] = json!(channel.to_string());
    data["id"] = json!(id.to_string());
    if let Some(timestamp) = timestamp {
        data["timestamp"] = json!(timestamp);
    }
    decode(data)
}

fn thread(id: u64, archived: bool, archived_at: &str) -> ThreadCreate {
    decode(json!({
        "id": id.to_string(), "type": 11, "guild_id": GUILD.to_string(), "parent_id": "50",
        "name": "a thread", "owner_id": "7", "message_count": 0, "member_count": 1,
        "thread_metadata": { "archived": archived, "auto_archive_duration": 1440,
                             "archive_timestamp": archived_at, "locked": false }
    }))
}

#[test]
fn sweeping_a_guild_takes_its_parts_along() {
    let cache = Cache::with_config(CacheConfig::discord_js());
    cache.update(&guild_create());
    assert_eq!(cache.sweep_guilds(|guild| guild.large), 1);
    assert_eq!(cache.guilds().len(), 0);
    assert_eq!(cache.channels().len(), 0);
    assert_eq!(cache.members(GUILD).len(), 0);
    assert_eq!(cache.roles(GUILD).len(), 0);
    // Users are not part of a guild.
    assert_eq!(cache.users().len(), 10);
}

#[test]
fn sweeping_a_channel_drops_its_messages() {
    let cache = Cache::with_config(CacheConfig::discord_js());
    cache.update(&guild_create());
    let channel = cache.channels().in_guild(GUILD)[0];
    cache.update(&message(channel.get(), 1, None));
    assert_eq!(
        cache.sweep_channels(|candidate| candidate.id() == channel),
        1
    );
    assert_eq!(cache.messages(channel).len(), 0);
    assert!(!cache.channels().in_guild(GUILD).contains(&channel));
}

#[test]
fn filters_see_the_guild_of_guild_resources() {
    let cache = Cache::with_config(CacheConfig::discord_js());
    cache.update(&guild_create());
    assert_eq!(cache.sweep_members(|guild_id, _| guild_id != GUILD), 0);
    assert_eq!(cache.sweep_roles(|guild_id, _| guild_id == GUILD), 4);
    assert_eq!(cache.roles(GUILD).len(), 0);
    let presences = cache.presences(GUILD).len();
    assert_eq!(cache.sweep_presences(|_, _| true), presences);
    assert_eq!(cache.sweep_users(|user| user.id.get() % 2 == 0), 5);
}

#[test]
fn old_messages_are_swept_by_their_last_change() {
    let cache = Cache::with_config(CacheConfig::builder().messages(Limit::Max(3)).build());
    cache.update(&message(10, 1, None));
    cache.update(&message(10, 2, Some("2100-01-01T00:00:00.000000+00:00")));
    // The fixture is from 2026-11-10: older than an hour, younger than a century.
    assert_eq!(cache.sweep_messages_older_than(HOUR * 24 * 365 * 100), 0);
    assert_eq!(cache.sweep_messages_older_than(HOUR), 1);
    assert_eq!(
        cache.messages(ChannelId::new(10)).keys(),
        [MessageId::new(2)]
    );

    // The swept message freed its room in the channel's limit.
    for id in 3..=4 {
        cache.update(&message(10, id, None));
    }
    assert_eq!(cache.messages(ChannelId::new(10)).len(), 3);
}

#[test]
fn only_long_archived_threads_are_swept() {
    let config = CacheConfig::builder()
        .threads(Limit::Unlimited)
        .messages(Limit::Unlimited)
        .build();
    let cache = Cache::with_config(config);
    cache.update(&thread(60, true, "2020-01-01T00:00:00.000000+00:00"));
    cache.update(&thread(61, true, "2100-01-01T00:00:00.000000+00:00"));
    cache.update(&thread(62, false, "2020-01-01T00:00:00.000000+00:00"));
    cache.update(&message(60, 1, None));

    let lifetime = cache.config().sweepers().threads.unwrap().lifetime;
    assert_eq!(cache.sweep_archived_threads(lifetime), 1);
    let mut kept = cache.threads(GUILD).keys();
    kept.sort_unstable();
    assert_eq!(kept, [ChannelId::new(61), ChannelId::new(62)]);
    assert_eq!(cache.messages(ChannelId::new(60)).len(), 0);
}

#[test]
fn sweeping_a_disabled_resource_removes_nothing() {
    let cache = Cache::new();
    assert_eq!(cache.sweep_members(|_, _| true), 0);
    assert_eq!(cache.sweep_messages_older_than(Duration::ZERO), 0);
    assert_eq!(cache.sweep_users(|_| true), 0);
}
