//! Messages, reactions, presences, voice states, threads and the other guild resources.

#[expect(
    dead_code,
    reason = "shared with the djs-model tests, which use other helpers"
)]
#[path = "../../djs-model/tests/common/payloads.rs"]
mod payloads;

use std::sync::Arc;

use djs_cache::{Cache, CacheConfig, Limit, Previous};
use djs_model::{
    channel::Channel,
    emoji::PartialEmoji,
    gateway::{
        DispatchEvent, Status,
        payload::{
            ChannelDelete, GuildCreate, GuildDelete, GuildScheduledEventCreate,
            GuildScheduledEventDelete, GuildSoundboardSoundCreate, GuildSoundboardSoundDelete,
            MessageCreate, MessageDelete, MessageDeleteBulk, MessageReactionAdd,
            MessageReactionRemove, MessageReactionRemoveAll, MessageReactionRemoveEmoji,
            MessageUpdate, PresenceUpdate, Ready, StageInstanceCreate, StageInstanceDelete,
            ThreadCreate, ThreadDelete, ThreadListSync, ThreadUpdate, VoiceStateUpdate,
        },
    },
    id::{ChannelId, GuildId, MessageId, ScheduledEventId, SoundboardSoundId, UserId},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

const GUILD: GuildId = GuildId::new(payloads::GUILD);

fn decode<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn everything() -> Cache {
    Cache::with_config(CacheConfig::discord_js())
}

/// The `MESSAGE_CREATE` fixture's data, with its IDs changed to `channel` / `id`.
fn message_json(channel: u64, id: u64) -> Value {
    let mut value: Value = serde_json::from_str(payloads::MESSAGE_CREATE).unwrap();
    let mut data = value["d"].take();
    data["channel_id"] = json!(channel.to_string());
    data["id"] = json!(id.to_string());
    data
}

fn message(channel: u64, id: u64) -> MessageCreate {
    decode(message_json(channel, id))
}

fn guild_create() -> GuildCreate {
    let value: Value = serde_json::from_str(&payloads::guild_create(1, 10, 10, 5, 4)).unwrap();
    decode(value["d"].clone())
}

#[test]
fn messages_are_cached_with_their_author_and_member() {
    let cache = everything();
    let event = message(10, 1);
    assert!(cache.update(&event).is_none());

    let cached = cache
        .messages(ChannelId::new(10))
        .get(MessageId::new(1))
        .unwrap();
    assert_eq!(cached.content, event.content);
    let user = cache.users().get(event.author.id).unwrap();
    assert!(Arc::ptr_eq(&cached.author, &user));
    let guild_id = event.guild_id.unwrap();
    let member = cache.members(guild_id).get(event.author.id).unwrap();
    assert!(Arc::ptr_eq(&member.user, &user));
}

#[test]
fn messages_count_per_channel() {
    let cache = Cache::with_config(CacheConfig::builder().messages(Limit::Max(2)).build());
    for id in 1..=3 {
        cache.update(&message(10, id));
    }
    cache.update(&message(11, 4));
    let mut kept = cache.messages(ChannelId::new(10)).keys();
    kept.sort_unstable();
    assert_eq!(kept, [MessageId::new(2), MessageId::new(3)]);
    assert_eq!(cache.messages(ChannelId::new(11)).len(), 1);
}

#[test]
fn message_update_replaces_only_cached_messages() {
    let cache = everything();
    let mut edited = message_json(10, 1);
    edited["content"] = json!("edited");
    let update: MessageUpdate = decode(edited.clone());
    assert!(cache.update(&update).is_none());
    assert!(cache.messages(ChannelId::new(10)).is_empty());

    cache.update(&message(10, 1));
    let previous = cache.update(&update).unwrap();
    assert_ne!(previous.content, "edited");
    let cached = cache
        .messages(ChannelId::new(10))
        .get(MessageId::new(1))
        .unwrap();
    assert_eq!(cached.content, "edited");
}

#[test]
fn message_deletes_return_the_messages() {
    let cache = everything();
    for id in 1..=3 {
        cache.update(&message(10, id));
    }
    let delete: MessageDelete = decode(json!({ "id": "1", "channel_id": "10" }));
    assert_eq!(cache.update(&delete).unwrap().id, MessageId::new(1));
    let bulk: MessageDeleteBulk = decode(json!({ "ids": ["2", "3", "4"], "channel_id": "10" }));
    assert_eq!(cache.update(&bulk).len(), 2);
    assert_eq!(cache.messages(ChannelId::new(10)).len(), 0);
}

#[test]
fn deleting_a_channel_or_guild_drops_its_messages() {
    let cache = everything();
    let event = message(10, 1);
    let guild_id = event.guild_id.unwrap();
    cache.update(&event);
    cache.update(&message(11, 2));

    let channel: Channel = decode(json!({
        "id": "10", "type": 0, "guild_id": guild_id.to_string(), "name": "gone",
        "position": 0, "permission_overwrites": [], "nsfw": false
    }));
    cache.update(&ChannelDelete(channel));
    assert_eq!(cache.messages(ChannelId::new(10)).len(), 0);
    assert_eq!(cache.messages(ChannelId::new(11)).len(), 1);

    let left: GuildDelete = decode(json!({ "id": guild_id.to_string() }));
    cache.update(&left);
    assert_eq!(cache.messages(ChannelId::new(11)).len(), 0);
}

#[test]
fn webhook_authors_are_not_cached_as_users() {
    let cache = everything();
    let mut value = message_json(10, 1);
    value["webhook_id"] = json!("99");
    value.as_object_mut().unwrap().remove("member");
    cache.update(&decode::<MessageCreate>(value));
    assert_eq!(cache.users().len(), 0);
    assert_eq!(cache.messages(ChannelId::new(10)).len(), 1);
}

#[test]
fn reactions_update_the_cached_message() {
    let cache = everything();
    cache.update(&message(10, 1));
    let ready: Ready = decode(json!({
        "v": 10,
        "user": { "id": "7", "username": "bot", "discriminator": "0", "global_name": null,
                  "avatar": null, "bot": true },
        "guilds": [], "session_id": "s", "resume_gateway_url": "wss://example.invalid",
        "shard": [0, 1], "application": { "id": "7", "flags": 0 }
    }));
    cache.update(&ready);
    let reaction = |user: &str, emoji: Value| -> MessageReactionAdd {
        decode(
            json!({ "user_id": user, "channel_id": "10", "message_id": "1", "emoji": emoji,
                       "burst": false, "type": 0, "burst_colors": [] }),
        )
    };
    let fire = json!({ "id": null, "name": "🔥" });
    let custom = json!({ "id": "55", "name": "party", "animated": false });

    assert!(cache.update(&reaction("7", fire.clone())).is_some());
    cache.update(&reaction("8", fire.clone()));
    cache.update(&reaction("8", custom.clone()));
    let reactions = || {
        cache
            .messages(ChannelId::new(10))
            .get(MessageId::new(1))
            .unwrap()
            .reactions
            .clone()
    };
    let now = reactions();
    assert_eq!(now.len(), 2);
    assert_eq!(
        (now[0].count, now[0].count_details.normal, now[0].me),
        (2, 2, true)
    );
    assert_eq!(now[1].count, 1);

    let remove: MessageReactionRemove = decode(json!({
        "user_id": "7", "channel_id": "10", "message_id": "1", "emoji": fire,
        "burst": false, "type": 0
    }));
    let previous = cache.update(&remove).unwrap();
    assert_eq!(previous.reactions[0].count, 2);
    assert_eq!((reactions()[0].count, reactions()[0].me), (1, false));

    let remove_emoji: MessageReactionRemoveEmoji = decode(json!({
        "channel_id": "10", "message_id": "1", "emoji": { "id": "55", "name": null }
    }));
    cache.update(&remove_emoji);
    assert_eq!(reactions().len(), 1);
    assert_eq!(
        reactions()[0].emoji,
        PartialEmoji::Unicode {
            name: "🔥".into()
        }
    );

    let remove_all: MessageReactionRemoveAll =
        decode(json!({ "channel_id": "10", "message_id": "1" }));
    cache.update(&remove_all);
    assert_eq!(reactions().len(), 0);
}

#[test]
fn presences_are_cached_and_offline_ones_removed() {
    let cache = everything();
    cache.update(&guild_create());
    assert_eq!(cache.presences(GUILD).len(), 10);

    let mut value: Value =
        serde_json::from_str(&payloads::presence(0, Some(payloads::GUILD))).unwrap();
    let user_id: UserId = decode(value["user"]["id"].clone());
    let presence = cache.presences(GUILD).get(user_id).unwrap();
    assert_eq!(presence.status, Status::Online);
    assert_eq!(presence.activities.len(), 2);

    value["status"] = json!("offline");
    value["activities"] = json!([]);
    let offline: PresenceUpdate = decode(value);
    assert_eq!(cache.update(&offline).unwrap().status, Status::Online);
    assert!(cache.presences(GUILD).get(user_id).is_none());
}

#[test]
fn voice_states_are_cached_without_their_member() {
    let cache = everything();
    cache.update(&guild_create());
    assert_eq!(cache.voice_states(GUILD).len(), 10);

    let user: Value = serde_json::from_str(&payloads::user(42)).unwrap();
    let join: VoiceStateUpdate = decode(json!({
        "guild_id": GUILD.to_string(), "channel_id": "5", "user_id": user["id"],
        "session_id": "x", "deaf": false, "mute": false, "self_deaf": false,
        "self_mute": false, "self_video": false, "suppress": false,
        "request_to_speak_timestamp": null,
        "member": { "user": user, "roles": [], "joined_at": "2024-05-01T12:00:00.000000+00:00",
                    "deaf": false, "mute": false }
    }));
    let user_id = join.user_id;
    assert!(cache.update(&join).is_none());
    let state = cache.voice_states(GUILD).get(user_id).unwrap();
    assert!(state.member.is_none());
    assert!(cache.members(GUILD).get(user_id).is_some());

    let mut leave = join.clone();
    leave.channel_id = None;
    assert!(cache.update(&leave).is_some());
    assert!(cache.voice_states(GUILD).get(user_id).is_none());
}

fn thread_json(id: u64, parent: u64, archived: bool) -> Value {
    json!({
        "id": id.to_string(), "type": 11, "guild_id": GUILD.to_string(),
        "parent_id": parent.to_string(), "name": "a thread", "owner_id": "7",
        "message_count": 0, "member_count": 1,
        "thread_metadata": { "archived": archived, "auto_archive_duration": 1440,
                             "archive_timestamp": "2026-11-12T10:00:00.000000+00:00",
                             "locked": false }
    })
}

#[test]
fn thread_events() {
    let cache = everything();
    let create: ThreadCreate = decode(thread_json(60, 50, false));
    assert!(cache.update(&create).is_none());
    assert_eq!(cache.threads(GUILD).keys(), [ChannelId::new(60)]);
    // Threads are not channels.
    assert_eq!(cache.channels().len(), 0);

    let mut renamed = thread_json(60, 50, false);
    renamed["name"] = json!("renamed");
    let update: ThreadUpdate = decode(renamed);
    assert_eq!(cache.update(&update).unwrap().name(), Some("a thread"));

    cache.update(&message(60, 1));
    let delete: ThreadDelete = decode(json!({
        "id": "60", "guild_id": GUILD.to_string(), "parent_id": "50", "type": 11
    }));
    assert_eq!(cache.update(&delete).unwrap().name(), Some("renamed"));
    assert_eq!(cache.threads(GUILD).len(), 0);
    assert_eq!(cache.messages(ChannelId::new(60)).len(), 0);
}

#[test]
fn thread_list_sync_replaces_the_active_threads_of_its_channels() {
    let cache = everything();
    for (id, parent, archived) in [(60, 50, false), (61, 50, true), (62, 51, false)] {
        cache.update(&decode::<ThreadCreate>(thread_json(id, parent, archived)));
    }
    let sync: ThreadListSync = decode(json!({
        "guild_id": GUILD.to_string(), "channel_ids": ["50"],
        "threads": [thread_json(63, 50, false)], "members": []
    }));
    let replaced = cache.update(&sync);
    assert_eq!(replaced.len(), 1);
    assert_eq!(replaced[0].id(), ChannelId::new(60));
    let mut threads = cache.threads(GUILD).keys();
    threads.sort_unstable();
    // The archived thread and the other channel's thread stay.
    assert_eq!(
        threads,
        [ChannelId::new(61), ChannelId::new(62), ChannelId::new(63)]
    );
}

#[test]
fn scheduled_events_stage_instances_and_sounds() {
    let cache = everything();
    let event = json!({
        "id": "80", "guild_id": GUILD.to_string(), "channel_id": "5", "creator_id": "7",
        "name": "Game night", "scheduled_start_time": "2026-12-01T20:00:00.000000+00:00",
        "privacy_level": 2, "status": 1, "entity_type": 2, "entity_id": null,
        "entity_metadata": null
    });
    cache.update(&decode::<GuildScheduledEventCreate>(event.clone()));
    assert!(
        cache
            .scheduled_events(GUILD)
            .contains(ScheduledEventId::new(80))
    );
    assert!(
        cache
            .update(&decode::<GuildScheduledEventDelete>(event))
            .is_some()
    );
    assert_eq!(cache.scheduled_events(GUILD).len(), 0);

    let stage = json!({
        "id": "81", "guild_id": GUILD.to_string(), "channel_id": "5", "topic": "Q&A",
        "privacy_level": 2, "discoverable_disabled": false, "guild_scheduled_event_id": null
    });
    cache.update(&decode::<StageInstanceCreate>(stage.clone()));
    assert_eq!(cache.stage_instances(GUILD).len(), 1);
    cache.update(&decode::<StageInstanceDelete>(stage));
    assert_eq!(cache.stage_instances(GUILD).len(), 0);

    let sound = json!({
        "name": "quack", "sound_id": "82", "volume": 1.0, "emoji_id": null,
        "emoji_name": "🦆", "guild_id": GUILD.to_string(), "available": true
    });
    cache.update(&decode::<GuildSoundboardSoundCreate>(sound));
    assert!(
        cache
            .soundboard_sounds(GUILD)
            .contains(SoundboardSoundId::new(82))
    );
    let delete: GuildSoundboardSoundDelete =
        decode(json!({ "sound_id": "82", "guild_id": GUILD.to_string() }));
    assert!(cache.update(&delete).is_some());
    assert_eq!(cache.soundboard_sounds(GUILD).len(), 0);
}

#[test]
fn update_dispatch_handles_messages() {
    let cache = everything();
    let event = DispatchEvent::MessageCreate(Box::new(message(10, 1)));
    assert_eq!(cache.update_dispatch(&event), Previous::None);
    assert!(matches!(
        cache.update_dispatch(&event),
        Previous::Message(_)
    ));
}

/// `CachedMessage` copies every field of `Message` except `member` (D-165).
#[test]
fn cached_messages_cover_every_model_field() {
    let cache = everything();
    let event = message(10, 1);
    cache.update(&event);
    let debug = format!(
        "{:?}",
        cache
            .messages(ChannelId::new(10))
            .get(MessageId::new(1))
            .unwrap()
    );
    let json = serde_json::to_value(&*event).unwrap();
    for key in json.as_object().unwrap().keys() {
        let field = if key == "type" { "kind" } else { key.as_str() };
        if field == "member" {
            continue;
        }
        assert!(
            debug.contains(&format!(" {field}: ")),
            "CachedMessage lacks `{field}`"
        );
    }
}
