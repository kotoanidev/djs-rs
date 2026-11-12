//! Applying gateway events to the cache, with payloads shaped like Discord's.

#[expect(
    dead_code,
    reason = "shared with the djs-model tests, which use other helpers"
)]
#[path = "../../djs-model/tests/common/payloads.rs"]
mod payloads;

use std::sync::Arc;

use djs_cache::{Cache, CacheConfig, CacheConfigBuilder, Limit, Previous, Resource};
use djs_model::{
    channel::Channel,
    gateway::{
        DispatchEvent,
        payload::{
            ChannelCreate, ChannelDelete, ChannelPinsUpdate, ChannelUpdate, GuildCreate,
            GuildDelete, GuildEmojisUpdate, GuildMemberAdd, GuildMemberRemove, GuildMemberUpdate,
            GuildMembersChunk, GuildRoleCreate, GuildRoleDelete, GuildRoleUpdate, GuildUpdate,
            Ready, ThreadCreate, UserUpdate,
        },
    },
    guild::Guild,
    id::{ChannelId, GuildId, RoleId, UserId},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

const GUILD: GuildId = GuildId::new(payloads::GUILD);

/// Decodes the data of a dispatch payload built by `payloads`.
fn data<T: DeserializeOwned>(dispatch: &str) -> T {
    let value: Value = serde_json::from_str(dispatch).unwrap();
    serde_json::from_value(value["d"].clone()).unwrap()
}

fn decode<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

/// A `GUILD_CREATE` with 10 members, 5 channels, 4 roles and 50 emojis.
fn guild_create() -> GuildCreate {
    data(&payloads::guild_create(1, 10, 10, 5, 4))
}

fn everything() -> Cache {
    Cache::with_config(CacheConfig::discord_js())
}

fn role_json(id: u64, name: &str) -> Value {
    json!({ "id": id.to_string(), "name": name, "color": 0, "hoist": false, "position": 1,
            "permissions": "0", "managed": false, "mentionable": false, "flags": 0,
            "colors": { "primary_color": 0, "secondary_color": null, "tertiary_color": null } })
}

fn member_user_id(create: &GuildCreate) -> UserId {
    let GuildCreate::Available(guild) = create else {
        unreachable!()
    };
    guild.members[0].user.id
}

#[test]
fn the_default_cache_keeps_guilds_channels_and_roles() {
    let cache = Cache::new();
    assert!(cache.update(&guild_create()).is_none());

    let guild = cache.guilds().get(GUILD).unwrap();
    assert_eq!(guild.name, "Big Guild");
    assert_eq!(guild.member_count, 10);
    assert!(guild.large);
    assert_eq!(cache.channels().in_guild(GUILD).len(), 5);
    assert_eq!(cache.roles(GUILD).len(), 4);

    // Not cached by default (D-152).
    assert!(!cache.emojis(GUILD).is_enabled());
    assert_eq!(cache.emojis(GUILD).len(), 0);
    assert!(!cache.members(GUILD).is_enabled());
    assert_eq!(cache.users().len(), 0);
}

#[test]
fn guild_channels_get_their_guild_id() {
    let cache = Cache::new();
    cache.update(&guild_create());
    for id in cache.channels().in_guild(GUILD) {
        let channel = cache.channels().get(id).unwrap();
        assert_eq!(channel.guild_id(), Some(GUILD));
    }
}

#[test]
fn members_share_their_user_with_the_user_resource() {
    let cache = everything();
    let create = guild_create();
    cache.update(&create);
    assert_eq!(cache.members(GUILD).len(), 10);
    assert_eq!(cache.users().len(), 10);
    assert_eq!(cache.emojis(GUILD).len(), 50);

    let id = member_user_id(&create);
    let member = cache.members(GUILD).get(id).unwrap();
    let user = cache.users().get(id).unwrap();
    assert!(Arc::ptr_eq(&member.user, &user));
    assert_eq!(member.display_name(), "Member 0");
}

#[test]
fn users_are_cached_without_members() {
    let cache = Cache::with_config(CacheConfig::builder().users(Limit::Unlimited).build());
    cache.update(&guild_create());
    assert!(!cache.members(GUILD).is_enabled());
    assert_eq!(cache.users().len(), 10);
}

#[test]
fn guild_update_keeps_the_guild_create_extras() {
    let cache = Cache::new();
    let create = guild_create();
    cache.update(&create);
    let GuildCreate::Available(available) = &create else {
        unreachable!()
    };
    let mut guild: Guild = available.guild.clone();
    guild.name = "Renamed".into();
    guild.roles.truncate(1);
    let update = GuildUpdate(guild);

    let previous = cache.update(&update).unwrap();
    assert_eq!(previous.name, "Big Guild");
    let guild = cache.guilds().get(GUILD).unwrap();
    assert_eq!(guild.name, "Renamed");
    assert_eq!(guild.member_count, 10);
    assert_eq!(guild.joined_at, previous.joined_at);
    // The update's roles replace the cached ones.
    assert_eq!(cache.roles(GUILD).len(), 1);
}

#[test]
fn guild_delete_drops_everything_in_the_guild() {
    let cache = everything();
    cache.update(&guild_create());
    let left: GuildDelete = decode(json!({ "id": GUILD.to_string() }));

    let previous = cache.update(&left).unwrap();
    assert_eq!(previous.id, GUILD);
    assert!(cache.guilds().get(GUILD).is_none());
    assert_eq!(cache.channels().len(), 0);
    assert_eq!(cache.channels().in_guild(GUILD).len(), 0);
    assert_eq!(cache.roles(GUILD).len(), 0);
    assert_eq!(cache.emojis(GUILD).len(), 0);
    assert_eq!(cache.members(GUILD).len(), 0);
    // Users are kept, like in discord.js.
    assert_eq!(cache.users().len(), 10);
}

#[test]
fn an_outage_keeps_the_guild_and_marks_it_unavailable() {
    let cache = Cache::new();
    cache.update(&guild_create());
    let outage: GuildDelete = decode(json!({ "id": GUILD.to_string(), "unavailable": true }));

    assert!(cache.update(&outage).is_none());
    assert!(cache.guilds().contains(GUILD));
    assert_eq!(cache.unavailable_guilds(), [GUILD]);

    // Available again.
    assert_eq!(cache.update(&guild_create()).unwrap().id, GUILD);
    assert_eq!(cache.unavailable_guilds().len(), 0);
}

#[test]
fn ready_sets_the_current_user_and_lists_guilds_as_unavailable() {
    let cache = Cache::new();
    let ready: Ready = decode(json!({
        "v": 10,
        "user": { "id": "7", "username": "bot", "discriminator": "0", "global_name": null,
                  "avatar": null, "bot": true },
        "guilds": [{ "id": GUILD.to_string(), "unavailable": true }],
        "session_id": "s", "resume_gateway_url": "wss://example.invalid",
        "shard": [0, 1], "application": { "id": "7", "flags": 0 }
    }));
    assert!(cache.update(&ready).is_none());
    assert_eq!(cache.current_user().unwrap().username, "bot");
    assert_eq!(cache.unavailable_guilds(), [GUILD]);

    let mut user = (*cache.current_user().unwrap()).clone();
    user.username = "renamed".into();
    let previous = cache.update(&UserUpdate(user)).unwrap();
    assert_eq!(previous.username, "bot");
    assert_eq!(cache.current_user().unwrap().username, "renamed");

    cache.update(&guild_create());
    assert_eq!(cache.unavailable_guilds().len(), 0);
}

#[test]
fn channel_events_return_the_previous_channel() {
    let cache = Cache::new();
    let channel: Channel = decode(json!({
        "id": "50", "type": 0, "guild_id": GUILD.to_string(), "name": "general",
        "position": 0, "permission_overwrites": [], "nsfw": false
    }));
    assert!(cache.update(&ChannelCreate(channel.clone())).is_none());
    assert_eq!(cache.channels().in_guild(GUILD), [ChannelId::new(50)]);

    let mut renamed = channel.clone();
    if let Channel::Text(text) = &mut renamed {
        text.name = "renamed".into();
    }
    let previous = cache.update(&ChannelUpdate(renamed)).unwrap();
    assert_eq!(previous.name(), Some("general"));
    assert_eq!(
        cache.channels().get(ChannelId::new(50)).unwrap().name(),
        Some("renamed")
    );

    let pins: ChannelPinsUpdate = decode(json!({
        "guild_id": GUILD.to_string(), "channel_id": "50",
        "last_pin_timestamp": "2026-11-12T10:00:00.000000+00:00"
    }));
    assert!(cache.update(&pins).is_some());
    let Channel::Text(text) = &*cache.channels().get(ChannelId::new(50)).unwrap() else {
        unreachable!()
    };
    assert!(text.last_pin_timestamp.is_some());

    let deleted = cache.update(&ChannelDelete(channel)).unwrap();
    assert_eq!(deleted.name(), Some("renamed"));
    assert_eq!(cache.channels().len(), 0);
    assert_eq!(cache.channels().in_guild(GUILD).len(), 0);
}

#[test]
fn threads_are_not_cached_as_channels() {
    let cache = Cache::new();
    let thread: ThreadCreate = decode(json!({
        "id": "60", "type": 11, "guild_id": GUILD.to_string(), "parent_id": "50",
        "name": "a thread", "owner_id": "7", "message_count": 0, "member_count": 1,
        "thread_metadata": { "archived": false, "auto_archive_duration": 1440,
                             "archive_timestamp": "2026-11-12T10:00:00.000000+00:00",
                             "locked": false }
    }));
    let channel = ChannelCreate(Channel::clone(&thread));
    assert!(cache.update(&channel).is_none());
    assert_eq!(cache.channels().len(), 0);
}

#[test]
fn role_events() {
    let cache = Cache::new();
    let create: GuildRoleCreate = decode(json!({
        "guild_id": GUILD.to_string(), "role": role_json(3, "mods")
    }));
    assert!(cache.update(&create).is_none());
    let update: GuildRoleUpdate = decode(json!({
        "guild_id": GUILD.to_string(), "role": role_json(3, "admins")
    }));
    assert_eq!(cache.update(&update).unwrap().name, "mods");
    assert_eq!(
        cache.roles(GUILD).get(RoleId::new(3)).unwrap().name,
        "admins"
    );
    // Roles are per guild: another guild does not see it.
    assert!(cache.roles(GuildId::new(99)).get(RoleId::new(3)).is_none());

    let delete: GuildRoleDelete = decode(json!({
        "guild_id": GUILD.to_string(), "role_id": "3"
    }));
    assert_eq!(cache.update(&delete).unwrap().name, "admins");
    assert_eq!(cache.roles(GUILD).len(), 0);
}

#[test]
fn emojis_update_replaces_the_set() {
    let cache = everything();
    cache.update(&guild_create());
    let update: GuildEmojisUpdate = decode(json!({
        "guild_id": GUILD.to_string(),
        "emojis": [{ "id": "70", "name": "new", "roles": [], "require_colons": true,
                     "managed": false, "animated": false, "available": true }]
    }));
    assert_eq!(cache.update(&update).len(), 50);
    assert_eq!(cache.emojis(GUILD).keys().len(), 1);
}

#[test]
fn member_events_keep_the_member_count() {
    let cache = everything();
    cache.update(&guild_create());
    let add: GuildMemberAdd = {
        let mut member: Value = serde_json::from_str(&payloads::member(100)).unwrap();
        member["guild_id"] = json!(GUILD.to_string());
        member["deaf"] = json!(true);
        decode(member)
    };
    let id = add.user.id;
    assert!(cache.update(&add).is_none());
    assert_eq!(cache.guilds().get(GUILD).unwrap().member_count, 11);

    // A partial update keeps the fields it omits.
    let update: GuildMemberUpdate = decode(json!({
        "guild_id": GUILD.to_string(), "roles": [], "user": add.user.clone(),
        "nick": "nickname", "joined_at": "2024-05-01T12:00:00.000000+00:00"
    }));
    let previous = cache.update(&update).unwrap();
    assert_eq!(previous.nick, None);
    let member = cache.members(GUILD).get(id).unwrap();
    assert_eq!(member.nick.as_deref(), Some("nickname"));
    assert!(member.deaf);
    assert_eq!(member.roles.len(), 0);
    assert_eq!(member.display_name(), "nickname");

    let remove: GuildMemberRemove = decode(json!({
        "guild_id": GUILD.to_string(), "user": add.user.clone()
    }));
    assert_eq!(
        cache.update(&remove).unwrap().nick.as_deref(),
        Some("nickname")
    );
    assert!(cache.members(GUILD).get(id).is_none());
    assert_eq!(cache.guilds().get(GUILD).unwrap().member_count, 10);
}

#[test]
fn a_member_update_adds_an_uncached_member() {
    let cache = everything();
    let user: Value = serde_json::from_str(&payloads::user(5)).unwrap();
    let update: GuildMemberUpdate = decode(json!({
        "guild_id": GUILD.to_string(), "roles": ["3"], "user": user,
        "joined_at": "2024-05-01T12:00:00.000000+00:00"
    }));
    assert!(cache.update(&update).is_none());
    assert_eq!(cache.members(GUILD).len(), 1);
}

#[test]
fn members_chunk_returns_the_replaced_members() {
    let cache = everything();
    let members: Vec<Value> = (0..4)
        .map(|i| serde_json::from_str(&payloads::member(i)).unwrap())
        .collect();
    let chunk: GuildMembersChunk = decode(json!({
        "guild_id": GUILD.to_string(), "members": members[..2],
        "chunk_index": 0, "chunk_count": 2
    }));
    assert_eq!(cache.update(&chunk).len(), 0);
    let chunk: GuildMembersChunk = decode(json!({
        "guild_id": GUILD.to_string(), "members": members[1..],
        "chunk_index": 1, "chunk_count": 2
    }));
    assert_eq!(cache.update(&chunk).len(), 1);
    assert_eq!(cache.members(GUILD).len(), 4);
}

#[test]
fn member_limits_count_per_guild() {
    let config = CacheConfig::builder().members(Limit::Max(3)).build();
    let cache = Cache::with_config(config);
    cache.update(&guild_create());
    assert_eq!(cache.members(GUILD).len(), 3);

    let other: GuildCreate = {
        let mut value: Value =
            serde_json::from_str(&payloads::guild_create(2, 2, 0, 1, 1)).unwrap();
        value["d"]["id"] = json!("77");
        decode(value["d"].clone())
    };
    cache.update(&other);
    assert_eq!(cache.members(GuildId::new(77)).len(), 2);
    assert_eq!(cache.members(GUILD).len(), 3);
}

#[test]
fn an_evicted_guild_takes_its_parts_along() {
    let cache = Cache::with_config(CacheConfig::builder().guilds(Limit::Max(1)).build());
    cache.update(&guild_create());
    let other: GuildCreate = {
        let mut value: Value =
            serde_json::from_str(&payloads::guild_create(2, 2, 0, 1, 1)).unwrap();
        value["d"]["id"] = json!("77");
        value["d"]["channels"][0]["id"] = json!("78");
        decode(value["d"].clone())
    };
    cache.update(&other);
    assert_eq!(cache.guilds().keys(), [GuildId::new(77)]);
    assert_eq!(cache.roles(GUILD).len(), 0);
    assert_eq!(cache.channels().in_guild(GUILD).len(), 0);
    assert_eq!(cache.channels().len(), 1);
}

#[test]
fn update_dispatch_wraps_the_previous_state() {
    let cache = everything();
    let create = guild_create();
    let event = DispatchEvent::GuildCreate(Box::new(create.clone()));
    assert_eq!(cache.update_dispatch(&event), Previous::None);
    let Previous::Guild(previous) = cache.update_dispatch(&event) else {
        panic!("expected the previous guild");
    };
    assert_eq!(previous.id, GUILD);

    let delete: GuildRoleDelete = decode(json!({
        "guild_id": GUILD.to_string(),
        "role_id": cache.roles(GUILD).keys()[0].to_string()
    }));
    assert!(matches!(
        cache.update_dispatch(&DispatchEvent::GuildRoleDelete(delete)),
        Previous::Role(_)
    ));
    let resumed = DispatchEvent::Resumed;
    assert_eq!(cache.update_dispatch(&resumed), Previous::None);
}

#[test]
fn disabled_resources_are_not_touched() {
    let config = CacheConfigBuilder::from(CacheConfig::default())
        .guilds(Limit::Disabled)
        .channels(Limit::Disabled)
        .roles(Limit::Disabled)
        .build();
    let cache = Cache::with_config(config);
    assert!(cache.update(&guild_create()).is_none());
    assert!(!cache.guilds().is_enabled());
    assert_eq!(cache.guilds().len(), 0);
    assert_eq!(cache.channels().in_guild(GUILD).len(), 0);
    assert!(!cache.config().is_enabled(Resource::Role));
}

/// `CachedGuild` and `CachedMember` copy every field of the model types (D-160): a field
/// added to the model must be added to the cached type too.
#[test]
fn cached_types_cover_every_model_field() {
    let cache = everything();
    let create = guild_create();
    cache.update(&create);
    let GuildCreate::Available(available) = &create else {
        unreachable!()
    };

    let guild_debug = format!("{:?}", cache.guilds().get(GUILD).unwrap());
    let guild_json = serde_json::to_value(&available.guild).unwrap();
    for key in guild_json.as_object().unwrap().keys() {
        if ["roles", "emojis", "stickers"].contains(&key.as_str()) {
            continue;
        }
        assert!(
            guild_debug.contains(&format!(" {key}: ")),
            "CachedGuild lacks `{key}`"
        );
    }

    let id = member_user_id(&create);
    let member_debug = format!("{:?}", cache.members(GUILD).get(id).unwrap());
    let member_json = serde_json::to_value(&available.members[0]).unwrap();
    for key in member_json.as_object().unwrap().keys() {
        assert!(
            member_debug.contains(&format!(" {key}: ")),
            "CachedMember lacks `{key}`"
        );
    }
}
