//! Storing objects received from the REST API (D-230, D-233): converted, stored when the
//! resource is enabled, and returned as stored.

#[expect(
    dead_code,
    reason = "shared with the djs-model tests, which use other helpers"
)]
#[path = "../../djs-model/tests/common/payloads.rs"]
mod payloads;

use std::sync::Arc;

use djs_cache::{Cache, CacheConfig, Limit};
use djs_model::{
    channel::Channel,
    emoji::Emoji,
    gateway::payload::GuildCreate,
    guild::{Guild, Member, Role},
    id::{ChannelId, EmojiId, GuildId, MessageId, RoleId, StickerId, UserId},
    message::Message,
    sticker::Sticker,
    user::User,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

const GUILD: GuildId = GuildId::new(payloads::GUILD);

fn decode<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn user(id: u64, name: &str) -> User {
    decode(
        json!({"id": id.to_string(), "username": name, "discriminator": "0",
                  "global_name": null, "avatar": null}),
    )
}

fn member(id: u64, nick: &str) -> Member {
    decode(
        json!({"user": {"id": id.to_string(), "username": "nelly", "discriminator": "0",
                           "global_name": null, "avatar": null},
                  "nick": nick, "roles": ["11"], "joined_at": "2026-01-01T00:00:00.000000+00:00",
                  "deaf": false, "mute": false, "flags": 0, "pending": false}),
    )
}

fn message() -> Message {
    let value: Value = serde_json::from_str(payloads::MESSAGE_CREATE).unwrap();
    decode(value["d"].clone())
}

fn role(id: u64, name: &str) -> Role {
    decode(
        json!({ "id": id.to_string(), "name": name, "color": 0, "hoist": false, "position": 1,
                   "permissions": "0", "managed": false, "mentionable": false, "flags": 0,
                   "colors": { "primary_color": 0, "secondary_color": null,
                               "tertiary_color": null } }),
    )
}

fn everything() -> Cache {
    Cache::with_config(CacheConfig::discord_js())
}

/// A cache keeping nothing at all.
fn nothing() -> Cache {
    Cache::with_config(
        CacheConfig::builder()
            .guilds(Limit::Disabled)
            .channels(Limit::Disabled)
            .roles(Limit::Disabled)
            .build(),
    )
}

#[test]
fn users_are_stored_once() {
    let cache = Cache::with_config(CacheConfig::builder().users(Limit::Unlimited).build());
    let first = cache.store_user(&user(7, "nelly"));
    let again = cache.store_user(&user(7, "nelly"));
    assert!(Arc::ptr_eq(&first, &again), "an equal user is reused");
    assert!(Arc::ptr_eq(
        &cache.users().get(UserId::new(7)).unwrap(),
        &first
    ));
    // A changed user replaces the stored one.
    let renamed = cache.store_user(&user(7, "nelly2"));
    assert_eq!(
        cache.users().get(UserId::new(7)).unwrap().username,
        "nelly2"
    );
    assert!(!Arc::ptr_eq(&renamed, &first));
}

#[test]
fn members_are_stored_without_counting_a_join() {
    let cache = everything();
    cache.update(&decode::<GuildCreate>(
        serde_json::from_str::<Value>(&payloads::guild_create(1, 3, 0, 1, 1)).unwrap()["d"].clone(),
    ));
    let before = cache.guilds().get(GUILD).unwrap().member_count;

    let stored = cache.store_member(GUILD, &member(9, "nick"));
    assert_eq!(stored.nick.as_deref(), Some("nick"));
    assert_eq!(stored.guild_id, GUILD);
    let cached = cache.members(GUILD).get(UserId::new(9)).unwrap();
    assert!(Arc::ptr_eq(&cached, &stored));
    // The member's user is the one the user resource holds.
    assert!(Arc::ptr_eq(
        &cache.users().get(UserId::new(9)).unwrap(),
        &stored.user
    ));
    assert_eq!(cache.guilds().get(GUILD).unwrap().member_count, before);
}

#[test]
fn a_disabled_resource_converts_without_storing() {
    let cache = nothing();
    let member = cache.store_member(GUILD, &member(9, "nick"));
    assert_eq!(member.user.id, UserId::new(9));
    assert!(!cache.members(GUILD).is_enabled());
    let message = cache.store_message(&message());
    assert!(!message.content.is_empty());
    let role = cache.store_role(GUILD, &role(5, "mods"));
    assert_eq!(role.name, "mods");
    assert_eq!(cache.roles(GUILD).len(), 0);
    assert_eq!(cache.users().len(), 0);
    assert_eq!(cache.guilds().len(), 0);
}

#[test]
fn messages_are_stored_with_their_author_and_member() {
    let cache = everything();
    let message = message();
    let stored = cache.store_message(&message);
    let cached = cache.messages(message.channel_id).get(message.id).unwrap();
    assert!(Arc::ptr_eq(&cached, &stored));
    assert!(Arc::ptr_eq(
        &cache.users().get(message.author.id).unwrap(),
        &stored.author
    ));
    if let Some(guild_id) = message.guild_id {
        assert!(cache.members(guild_id).get(message.author.id).is_some());
    }
    assert!(
        cache
            .messages(ChannelId::new(1))
            .get(MessageId::new(1))
            .is_none()
    );
}

#[test]
fn channels_and_threads_go_to_their_resources() {
    let cache = everything();
    let channel: Channel = decode(json!({
        "id": "30", "type": 0, "guild_id": GUILD.to_string(), "name": "general",
        "position": 0, "permission_overwrites": [], "nsfw": false
    }));
    let stored = cache.store_channel(&channel);
    assert!(Arc::ptr_eq(
        &cache.channels().get(ChannelId::new(30)).unwrap(),
        &stored
    ));
    assert_eq!(cache.channels().in_guild(GUILD).len(), 1);

    let thread: Channel = decode(json!({
        "id": "31", "type": 11, "guild_id": GUILD.to_string(), "parent_id": "30",
        "name": "a thread", "owner_id": "7", "message_count": 0, "member_count": 1,
        "thread_metadata": { "archived": false, "auto_archive_duration": 1440,
                             "archive_timestamp": "2026-11-12T10:00:00.000000+00:00",
                             "locked": false }
    }));
    let stored = cache.store_channel(&thread);
    assert!(Arc::ptr_eq(
        &cache.threads(GUILD).get(ChannelId::new(31)).unwrap(),
        &stored
    ));
    assert!(cache.channels().get(ChannelId::new(31)).is_none());
}

#[test]
fn guilds_keep_what_only_guild_create_sends() {
    let cache = everything();
    let create: Value = serde_json::from_str(&payloads::guild_create(1, 3, 0, 1, 2)).unwrap();
    cache.update(&decode::<GuildCreate>(create["d"].clone()));
    let before = cache.guilds().get(GUILD).unwrap();

    // Get Guild: the same guild renamed, with one role.
    let mut guild = create["d"].clone();
    guild["name"] = json!("Renamed");
    guild["roles"] = json!([serde_json::to_value(role(5, "only")).unwrap()]);
    let guild: Guild = decode(guild);
    let stored = cache.store_guild(&guild);
    assert_eq!(stored.name, "Renamed");
    assert_eq!(stored.member_count, before.member_count);
    assert_eq!(stored.joined_at, before.joined_at);
    assert_eq!(stored.large, before.large);
    assert!(Arc::ptr_eq(&cache.guilds().get(GUILD).unwrap(), &stored));
    assert_eq!(cache.roles(GUILD).len(), 1);
    assert!(cache.roles(GUILD).get(RoleId::new(5)).is_some());

    // A guild the cache did not hold takes its approximate member count.
    let mut other = create["d"].clone();
    other["id"] = json!("77");
    other["approximate_member_count"] = json!(1234);
    let stored = cache.store_guild(&decode(other));
    assert_eq!(stored.member_count, 1234);
    assert_eq!(stored.joined_at, None);
}

#[test]
fn emojis_and_stickers_are_stored_per_guild() {
    let cache = Cache::with_config(
        CacheConfig::builder()
            .emojis(Limit::Unlimited)
            .stickers(Limit::Unlimited)
            .build(),
    );
    let emoji: Emoji = decode(json!({"id": "41", "name": "ferris", "roles": [],
                                      "require_colons": true, "managed": false,
                                      "animated": false, "available": true}));
    let stored = cache.store_emoji(GUILD, &emoji);
    assert!(Arc::ptr_eq(
        &cache.emojis(GUILD).get(EmojiId::new(41)).unwrap(),
        &stored
    ));
    assert!(
        cache
            .emojis(GuildId::new(9))
            .get(EmojiId::new(41))
            .is_none()
    );

    let sticker: Sticker = decode(json!({"id": "42", "name": "wave", "tags": "wave",
                                          "type": 2, "format_type": 1, "available": true,
                                          "guild_id": GUILD.to_string()}));
    let stored = cache.store_sticker(GUILD, &sticker);
    assert!(Arc::ptr_eq(
        &cache.stickers(GUILD).get(StickerId::new(42)).unwrap(),
        &stored
    ));

    // Disabled by default: converted only.
    let nothing = nothing();
    assert_eq!(nothing.store_emoji(GUILD, &emoji).name, "ferris");
    assert!(!nothing.emojis(GUILD).is_enabled());
}

#[test]
fn events_stages_and_sounds_are_stored_per_guild() {
    use djs_model::{
        guild_scheduled_event::GuildScheduledEvent,
        id::{ScheduledEventId, SoundboardSoundId, StageInstanceId},
        soundboard::SoundboardSound,
        stage_instance::StageInstance,
    };

    let cache = Cache::with_config(
        CacheConfig::builder()
            .scheduled_events(Limit::Unlimited)
            .stage_instances(Limit::Unlimited)
            .soundboard_sounds(Limit::Unlimited)
            .build(),
    );
    let event: GuildScheduledEvent = decode(json!({
        "id": "50", "guild_id": GUILD.to_string(), "channel_id": null, "creator_id": null,
        "name": "Launch", "scheduled_start_time": "2026-12-01T18:00:00.000000+00:00",
        "scheduled_end_time": "2026-12-01T19:00:00.000000+00:00", "privacy_level": 2,
        "status": 1, "entity_type": 3, "entity_id": null,
        "entity_metadata": {"location": "Online"}
    }));
    let stored = cache.store_scheduled_event(GUILD, &event);
    assert!(Arc::ptr_eq(
        &cache
            .scheduled_events(GUILD)
            .get(ScheduledEventId::new(50))
            .unwrap(),
        &stored
    ));
    let stage: StageInstance = decode(json!({
        "id": "51", "guild_id": GUILD.to_string(), "channel_id": "52", "topic": "Q&A",
        "privacy_level": 2, "discoverable_disabled": false, "guild_scheduled_event_id": null
    }));
    let stored = cache.store_stage_instance(GUILD, &stage);
    assert!(Arc::ptr_eq(
        &cache
            .stage_instances(GUILD)
            .get(StageInstanceId::new(51))
            .unwrap(),
        &stored
    ));
    let sound: SoundboardSound = decode(json!({
        "name": "quack", "sound_id": "53", "volume": 1.0, "emoji_id": null,
        "emoji_name": "🦆", "guild_id": GUILD.to_string(), "available": true
    }));
    let stored = cache.store_soundboard_sound(GUILD, &sound);
    assert!(Arc::ptr_eq(
        &cache
            .soundboard_sounds(GUILD)
            .get(SoundboardSoundId::new(53))
            .unwrap(),
        &stored
    ));
}
