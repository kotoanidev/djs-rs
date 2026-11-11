//! Every gateway dispatch event decoded from a complete envelope (synthetic payloads built from
//! the docs' field tables, plus the docs' examples), the events that need custom decoding, and
//! the gateway commands compared with the docs' examples.

use djs_model::{
    gateway::{
        Activity, ActivityType, DispatchEvent, GatewayEvent, GatewayEventDeserializer, Intents,
        ShardId, Status,
        payload::{
            ChannelInfoField, CommandPayload, GatewayCommand, GuildCreate, Identify,
            IdentifyProperties, RequestChannelInfo, RequestGuildMembers, RequestSoundboardSounds,
            UpdatePresence, UpdateVoiceState,
        },
    },
    id::{ChannelId, GuildId},
    util::Timestamp,
};
use serde::{Serialize, de::DeserializeSeed};
use serde_json::{Value, json};

mod common;

use common::assert_subset;

const GUILD: &str = "1300000000000000001";
const CHANNEL: &str = "1300000000000000002";
const USER: &str = "1300000000000000003";
const MESSAGE: &str = "1300000000000000004";
const ROLE: &str = "1300000000000000005";
const THREAD: &str = "1300000000000000006";
const TIME: &str = "2026-11-11T12:00:00.000000+00:00";
const LATER: &str = "2026-11-12T12:00:00.000000+00:00";
/// [`TIME`] as Unix seconds.
const TIME_SECS: i64 = 1_794_398_400;

/// Decodes one raw gateway payload.
fn decode(json: &str) -> Result<GatewayEvent, serde_json::Error> {
    let seed = GatewayEventDeserializer::from_json(json).expect("top-level scan failed");
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let event = seed.deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(event)
}

/// Decodes the dispatch payload `{"t": name, "d": raw_data}`.
fn dispatch_raw(name: &str, raw_data: &str) -> Result<DispatchEvent, serde_json::Error> {
    let json = format!(r#"{{"op":0,"s":7,"t":"{name}","d":{raw_data}}}"#);
    decode(&json).map(|event| match event {
        GatewayEvent::Dispatch(7, event) => event,
        other => panic!("{name}: expected a dispatch, got {other:?}"),
    })
}

/// Decodes the dispatch payload `{"t": name, "d": data}`, checks that it is routed to the
/// event's own variant, and that serializing the event keeps every field of `data`.
fn dispatch(name: &str, data: &Value) -> DispatchEvent {
    let event = dispatch_raw(name, &data.to_string()).unwrap_or_else(|error| {
        panic!("{name}: {error}");
    });
    assert!(
        !matches!(event, DispatchEvent::Unknown(_)),
        "{name} was not routed"
    );
    assert_eq!(event.name(), name);
    let actual = serde_json::to_value(&event).unwrap();
    assert_subset(data, &actual, name);
    event
}

fn fixture(json: &str) -> Value {
    serde_json::from_str(json).unwrap()
}

/// Merges the fields of `extra` into the object `base`.
fn with(mut base: Value, extra: &Value) -> Value {
    let object = base.as_object_mut().unwrap();
    for (key, value) in extra.as_object().unwrap() {
        object.insert(key.clone(), value.clone());
    }
    base
}

fn user() -> Value {
    json!({ "id": USER, "username": "mixo", "discriminator": "0", "global_name": "Mixo",
            "avatar": null })
}

fn member() -> Value {
    json!({ "user": user(), "nick": "Mix", "roles": [ROLE], "joined_at": TIME,
            "deaf": false, "mute": false, "flags": 0, "pending": false })
}

fn role() -> Value {
    json!({ "id": ROLE, "name": "Tester", "color": 0,
            "colors": { "primary_color": 0, "secondary_color": null, "tertiary_color": null },
            "hoist": false, "position": 1, "permissions": "0", "managed": false,
            "mentionable": false, "flags": 0 })
}

fn presence() -> Value {
    json!({ "user": { "id": USER }, "guild_id": GUILD, "status": "idle",
            "activities": [{ "name": "Custom Status", "type": 4, "state": "Writing Rust",
                             "emoji": { "name": "🦀" }, "created_at": 1_794_398_400_000_u64 }],
            "client_status": { "desktop": "idle", "mobile": "online" } })
}

fn thread_member() -> Value {
    json!({ "id": THREAD, "user_id": USER, "join_timestamp": TIME, "flags": 0 })
}

fn thread() -> Value {
    fixture(include_str!("fixtures/docs/channel_thread_channel.json"))
}

fn text_channel() -> Value {
    fixture(include_str!(
        "fixtures/docs/channel_guild_text_channel.json"
    ))
}

fn guild() -> Value {
    fixture(include_str!("fixtures/docs/guild.json"))
}

fn sound() -> Value {
    let mut sound = fixture(include_str!("fixtures/docs/soundboard_guild_sound.json"));
    // `volume` is written as a float, which `serde_json::Value` does not equate with `1`.
    sound["volume"] = json!(1.0);
    sound
}

fn scheduled_event() -> Value {
    json!({ "id": "1300000000000000020", "guild_id": GUILD, "channel_id": null,
            "creator_id": USER, "name": "Book club", "description": "Chapter 4",
            "scheduled_start_time": TIME, "scheduled_end_time": LATER, "privacy_level": 2,
            "status": 1, "entity_type": 3, "entity_id": null,
            "entity_metadata": { "location": "Library" }, "user_count": 7, "image": null,
            "recurrence_rule": null })
}

fn interaction() -> Value {
    json!({
        "id": "1300000000000000030", "application_id": "1300000000000000031", "type": 2,
        "token": "token", "version": 1, "app_permissions": "0", "locale": "pl",
        "guild_id": GUILD, "guild_locale": "en-US",
        "guild": { "id": GUILD, "locale": "en-US", "features": [] },
        "channel_id": CHANNEL,
        "channel": { "id": CHANNEL, "type": 0, "name": "general", "guild_id": GUILD,
                     "nsfw": false, "flags": 0, "permissions": "1024", "position": 3 },
        "member": with(member(), &json!({ "permissions": "8" })),
        "entitlements": [],
        "authorizing_integration_owners": { "0": GUILD },
        "context": 0,
        "attachment_size_limit": 10_485_760,
        "data": { "id": "1300000000000000032", "name": "ping", "type": 1 },
    })
}

/// The `d` of one synthetic payload per dispatch event.
#[expect(clippy::too_many_lines, reason = "one entry per event")]
fn every_event() -> Vec<(&'static str, Value)> {
    let rule = fixture(include_str!("fixtures/docs/auto_moderation_rule.json"));
    let mut entitlement = fixture(include_str!("fixtures/docs/entitlement_entitlement.json"));
    // Shown in the docs' example but documented nowhere (see tests/application.rs).
    for field in ["promotion_id", "gift_code_flags", "subscription_id"] {
        entitlement.as_object_mut().unwrap().remove(field);
    }
    let message = fixture(include_str!("fixtures/docs/message_message.json"));
    let stage = fixture(include_str!(
        "fixtures/docs/stage_instance_stage_instance.json"
    ));
    let subscription = fixture(include_str!("fixtures/docs/subscription_subscription.json"));
    let voice_state = fixture(include_str!("fixtures/docs/voice_voice_state.json"));
    let integration = json!({
        "id": "1300000000000000040", "name": "djs-rs", "type": "discord", "enabled": true,
        "account": { "id": "1300000000000000041", "name": "djs-rs" }, "user": user(),
        "scopes": ["bot", "applications.commands"], "guild_id": GUILD,
    });
    let guild_member_update = json!({
        "guild_id": GUILD, "roles": [ROLE], "user": user(), "nick": "Mix", "avatar": null,
        "banner": null, "joined_at": TIME, "premium_since": null, "deaf": false,
        "mute": false, "pending": false, "communication_disabled_until": LATER, "flags": 0,
    });
    let reaction_emoji = json!({ "id": null, "name": "🔥" });
    let poll_vote = json!({ "user_id": USER, "channel_id": CHANNEL, "message_id": MESSAGE,
                            "guild_id": GUILD, "answer_id": 1 });
    vec![
        (
            "READY",
            json!({
                "v": 10, "user": with(user(), &json!({ "bot": true, "verified": true })),
                "guilds": [{ "id": GUILD, "unavailable": true }],
                "session_id": "0123456789abcdef", "resume_gateway_url": "wss://gateway.discord.gg",
                "shard": [0, 1], "application": { "id": "1300000000000000031", "flags": 8_912_896 },
            }),
        ),
        (
            "RATE_LIMITED",
            json!({ "opcode": 8, "retry_after": 25.5,
                                 "meta": { "guild_id": GUILD, "nonce": "n1" } }),
        ),
        (
            "APPLICATION_COMMAND_PERMISSIONS_UPDATE",
            json!({
                "id": "1300000000000000031", "application_id": "1300000000000000031",
                "guild_id": GUILD, "permissions": [{ "id": ROLE, "type": 1, "permission": true }],
            }),
        ),
        ("AUTO_MODERATION_RULE_CREATE", rule.clone()),
        ("AUTO_MODERATION_RULE_UPDATE", rule.clone()),
        ("AUTO_MODERATION_RULE_DELETE", rule),
        (
            "AUTO_MODERATION_ACTION_EXECUTION",
            json!({
                "guild_id": GUILD, "action": { "type": 1, "metadata": { "custom_message": "No" } },
                "rule_id": "1300000000000000050", "rule_trigger_type": 1, "user_id": USER,
                "channel_id": CHANNEL, "message_id": MESSAGE,
                "alert_system_message_id": "1300000000000000051", "content": "a bad word",
                "matched_keyword": "bad", "matched_content": "bad",
            }),
        ),
        ("CHANNEL_CREATE", text_channel()),
        ("CHANNEL_UPDATE", text_channel()),
        ("CHANNEL_DELETE", text_channel()),
        (
            "CHANNEL_INFO",
            json!({ "guild_id": GUILD, "channels": [
            { "id": CHANNEL, "status": "Chilling", "voice_start_time": TIME_SECS },
            { "id": "1300000000000000007" },
        ] }),
        ),
        (
            "CHANNEL_PINS_UPDATE",
            json!({ "guild_id": GUILD, "channel_id": CHANNEL,
                                        "last_pin_timestamp": TIME }),
        ),
        (
            "THREAD_CREATE",
            with(thread(), &json!({ "newly_created": true })),
        ),
        ("THREAD_UPDATE", thread()),
        (
            "THREAD_DELETE",
            json!({ "id": THREAD, "guild_id": GUILD, "parent_id": CHANNEL,
                                  "type": 11 }),
        ),
        (
            "THREAD_LIST_SYNC",
            json!({ "guild_id": GUILD, "channel_ids": [CHANNEL],
                                     "threads": [thread()], "members": [thread_member()] }),
        ),
        (
            "THREAD_MEMBER_UPDATE",
            with(thread_member(), &json!({ "guild_id": GUILD })),
        ),
        (
            "THREAD_MEMBERS_UPDATE",
            json!({
                "id": THREAD, "guild_id": GUILD, "member_count": 2,
                "added_members": [
                    with(thread_member(), &json!({ "member": member(), "presence": presence() })),
                    with(thread_member(), &json!({ "member": member(), "presence": null })),
                ],
                "removed_member_ids": ["1300000000000000008"],
            }),
        ),
        ("ENTITLEMENT_CREATE", entitlement.clone()),
        ("ENTITLEMENT_UPDATE", entitlement.clone()),
        ("ENTITLEMENT_DELETE", entitlement),
        (
            "GUILD_CREATE",
            with(
                guild(),
                &json!({
                    "joined_at": TIME, "large": true, "member_count": 2,
                    "voice_states": [voice_state.clone()], "members": [member()],
                    "channels": [text_channel()], "threads": [thread()], "presences": [presence()],
                    "stage_instances": [stage.clone()], "guild_scheduled_events": [scheduled_event()],
                    "soundboard_sounds": [sound()],
                }),
            ),
        ),
        ("GUILD_UPDATE", guild()),
        ("GUILD_DELETE", json!({ "id": GUILD, "unavailable": true })),
        (
            "GUILD_AUDIT_LOG_ENTRY_CREATE",
            json!({
                "id": "1300000000000000060", "action_type": 25, "target_id": USER,
                "user_id": USER, "changes": [{ "key": "$add",
                                                "new_value": [{ "id": ROLE, "name": "Tester" }] }],
                "reason": "promotion", "guild_id": GUILD,
            }),
        ),
        (
            "GUILD_BAN_ADD",
            json!({ "guild_id": GUILD, "user": user() }),
        ),
        (
            "GUILD_BAN_REMOVE",
            json!({ "guild_id": GUILD, "user": user() }),
        ),
        (
            "GUILD_EMOJIS_UPDATE",
            json!({ "guild_id": GUILD, "emojis": [{
            "id": "1300000000000000070", "name": "blob", "roles": [ROLE],
            "require_colons": true, "managed": false, "animated": true, "available": true,
        }] }),
        ),
        (
            "GUILD_STICKERS_UPDATE",
            json!({ "guild_id": GUILD, "stickers": [{
            "id": "1300000000000000071", "name": "wave", "description": null, "tags": "wave",
            "type": 2, "format_type": 1, "available": true, "guild_id": GUILD,
        }] }),
        ),
        ("GUILD_INTEGRATIONS_UPDATE", json!({ "guild_id": GUILD })),
        (
            "GUILD_MEMBER_ADD",
            with(member(), &json!({ "guild_id": GUILD })),
        ),
        (
            "GUILD_MEMBER_REMOVE",
            json!({ "guild_id": GUILD, "user": user() }),
        ),
        ("GUILD_MEMBER_UPDATE", guild_member_update),
        (
            "GUILD_MEMBERS_CHUNK",
            json!({
                "guild_id": GUILD, "members": [member()], "chunk_index": 0, "chunk_count": 1,
                "not_found": ["1300000000000000009"], "presences": [presence()], "nonce": "n1",
            }),
        ),
        (
            "GUILD_ROLE_CREATE",
            json!({ "guild_id": GUILD, "role": role() }),
        ),
        (
            "GUILD_ROLE_UPDATE",
            json!({ "guild_id": GUILD, "role": role() }),
        ),
        (
            "GUILD_ROLE_DELETE",
            json!({ "guild_id": GUILD, "role_id": ROLE }),
        ),
        ("GUILD_SCHEDULED_EVENT_CREATE", scheduled_event()),
        ("GUILD_SCHEDULED_EVENT_UPDATE", scheduled_event()),
        ("GUILD_SCHEDULED_EVENT_DELETE", scheduled_event()),
        (
            "GUILD_SCHEDULED_EVENT_USER_ADD",
            json!({
                "guild_scheduled_event_id": "1300000000000000020", "user_id": USER,
                "guild_id": GUILD,
            }),
        ),
        (
            "GUILD_SCHEDULED_EVENT_USER_REMOVE",
            json!({
                "guild_scheduled_event_id": "1300000000000000020", "user_id": USER,
                "guild_id": GUILD,
            }),
        ),
        ("GUILD_SOUNDBOARD_SOUND_CREATE", sound()),
        ("GUILD_SOUNDBOARD_SOUND_UPDATE", sound()),
        (
            "GUILD_SOUNDBOARD_SOUND_DELETE",
            json!({ "sound_id": "1300000000000000080",
                                                  "guild_id": GUILD }),
        ),
        (
            "GUILD_SOUNDBOARD_SOUNDS_UPDATE",
            json!({ "soundboard_sounds": [sound()],
                                                   "guild_id": GUILD }),
        ),
        (
            "SOUNDBOARD_SOUNDS",
            json!({ "soundboard_sounds": [sound()], "guild_id": GUILD }),
        ),
        ("INTEGRATION_CREATE", integration.clone()),
        ("INTEGRATION_UPDATE", integration),
        (
            "INTEGRATION_DELETE",
            json!({ "id": "1300000000000000040", "guild_id": GUILD,
                                       "application_id": "1300000000000000031" }),
        ),
        ("INTERACTION_CREATE", interaction()),
        (
            "INVITE_CREATE",
            json!({
                "channel_id": CHANNEL, "code": "djsrs", "created_at": TIME, "guild_id": GUILD,
                "inviter": user(), "max_age": 86_400, "max_uses": 0, "target_type": 1,
                "target_user": user(), "temporary": false, "uses": 0, "expires_at": LATER,
                "role_ids": [ROLE],
            }),
        ),
        (
            "INVITE_DELETE",
            json!({ "channel_id": CHANNEL, "guild_id": GUILD,
                                  "code": "djsrs" }),
        ),
        ("MESSAGE_CREATE", message.clone()),
        ("MESSAGE_UPDATE", message),
        (
            "MESSAGE_DELETE",
            json!({ "id": MESSAGE, "channel_id": CHANNEL, "guild_id": GUILD }),
        ),
        (
            "MESSAGE_DELETE_BULK",
            json!({ "ids": [MESSAGE, "1300000000000000010"],
                                        "channel_id": CHANNEL, "guild_id": GUILD }),
        ),
        (
            "MESSAGE_REACTION_ADD",
            json!({
                "user_id": USER, "channel_id": CHANNEL, "message_id": MESSAGE, "guild_id": GUILD,
                "member": member(), "emoji": reaction_emoji, "message_author_id": USER,
                "burst": true, "burst_colors": ["#ff6a00", "#ffc800"], "type": 1,
            }),
        ),
        (
            "MESSAGE_REACTION_REMOVE",
            json!({
                "user_id": USER, "channel_id": CHANNEL, "message_id": MESSAGE, "guild_id": GUILD,
                "emoji": { "id": "1300000000000000070", "name": "blob", "animated": true },
                "burst": false, "type": 0,
            }),
        ),
        (
            "MESSAGE_REACTION_REMOVE_ALL",
            json!({ "channel_id": CHANNEL, "message_id": MESSAGE,
                                                "guild_id": GUILD }),
        ),
        (
            "MESSAGE_REACTION_REMOVE_EMOJI",
            json!({
                "channel_id": CHANNEL, "guild_id": GUILD, "message_id": MESSAGE,
                "emoji": { "id": null, "name": "🔥" },
            }),
        ),
        ("MESSAGE_POLL_VOTE_ADD", poll_vote.clone()),
        ("MESSAGE_POLL_VOTE_REMOVE", poll_vote),
        ("PRESENCE_UPDATE", presence()),
        ("STAGE_INSTANCE_CREATE", stage.clone()),
        ("STAGE_INSTANCE_UPDATE", stage.clone()),
        ("STAGE_INSTANCE_DELETE", stage),
        ("SUBSCRIPTION_CREATE", subscription.clone()),
        ("SUBSCRIPTION_UPDATE", subscription.clone()),
        ("SUBSCRIPTION_DELETE", subscription),
        (
            "TYPING_START",
            json!({ "channel_id": CHANNEL, "guild_id": GUILD, "user_id": USER,
                                 "timestamp": TIME_SECS, "member": member() }),
        ),
        (
            "USER_UPDATE",
            with(
                user(),
                &json!({ "bot": true, "mfa_enabled": true,
                                              "verified": true, "flags": 0 }),
            ),
        ),
        (
            "VOICE_CHANNEL_EFFECT_SEND",
            json!({
                "channel_id": CHANNEL, "guild_id": GUILD, "user_id": USER,
                "emoji": { "id": null, "name": "🎉" }, "animation_type": 0, "animation_id": 5,
                "sound_id": "1300000000000000080", "sound_volume": 0.5,
            }),
        ),
        (
            "VOICE_CHANNEL_START_TIME_UPDATE",
            json!({ "id": CHANNEL, "guild_id": GUILD,
                                                    "voice_start_time": TIME_SECS }),
        ),
        (
            "VOICE_CHANNEL_STATUS_UPDATE",
            json!({ "id": CHANNEL, "guild_id": GUILD,
                                                "status": "Gaming" }),
        ),
        ("VOICE_STATE_UPDATE", voice_state),
        (
            "VOICE_SERVER_UPDATE",
            json!({ "token": "my_token", "guild_id": GUILD,
                                        "endpoint": "sweetwater-12345.discord.media:2048" }),
        ),
        (
            "WEBHOOKS_UPDATE",
            json!({ "guild_id": GUILD, "channel_id": CHANNEL }),
        ),
    ]
}

/// The `d` of the synthetic `name` payload.
fn data_of(name: &str) -> Value {
    every_event()
        .into_iter()
        .find(|(event, _)| *event == name)
        .unwrap_or_else(|| panic!("no {name} payload"))
        .1
}

#[test]
fn every_dispatch_event_is_routed_and_round_trips() {
    let events = every_event();
    // One entry per typed variant of `DispatchEvent` (all but `Resumed` and `Unknown`).
    assert_eq!(events.len(), 78);
    let mut names: Vec<&str> = events.iter().map(|(name, _)| *name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), events.len(), "duplicate event names");
    for (name, data) in &events {
        dispatch(name, data);
    }
}

#[test]
fn resumed_and_unknown_events() {
    let resumed = dispatch_raw("RESUMED", "{}").unwrap();
    assert_eq!(resumed, DispatchEvent::Resumed);
    assert_eq!(resumed.name(), "RESUMED");
    assert_eq!(serde_json::to_value(&resumed).unwrap(), Value::Null);

    let unknown = dispatch_raw("FUTURE_EVENT", r#"{"nested":{"a":[1,2,3]}}"#).unwrap();
    assert_eq!(unknown, DispatchEvent::Unknown("FUTURE_EVENT".into()));
    assert_eq!(unknown.name(), "FUTURE_EVENT");
}

#[test]
fn guild_create_available() {
    let DispatchEvent::GuildCreate(event) = dispatch("GUILD_CREATE", &data_of("GUILD_CREATE"))
    else {
        panic!("expected GUILD_CREATE");
    };
    let GuildCreate::Available(guild) = *event else {
        panic!("expected an available guild");
    };
    assert_eq!(guild.id, guild.guild.id);
    assert_eq!(guild.name, "Discord Testers");
    assert!(guild.large);
    assert_eq!(guild.member_count, 2);
    assert_eq!(guild.joined_at, Some(TIME.parse().unwrap()));
    assert_eq!(
        (
            guild.members.len(),
            guild.channels.len(),
            guild.threads.len()
        ),
        (1, 1, 1)
    );
    assert_eq!(guild.presences[0].activities[0].kind, ActivityType::Custom);
    assert_eq!(guild.soundboard_sounds.len(), 1);
}

#[test]
fn guild_create_unavailable() {
    for raw in [
        format!(r#"{{"id":"{GUILD}","unavailable":true}}"#),
        format!(r#"{{"unavailable":true,"id":"{GUILD}"}}"#),
    ] {
        let event = dispatch_raw("GUILD_CREATE", &raw).unwrap();
        let DispatchEvent::GuildCreate(event) = event else {
            panic!("expected GUILD_CREATE");
        };
        assert_eq!(event.id(), GuildId::new(1_300_000_000_000_000_001));
        let GuildCreate::Unavailable(guild) = *event else {
            panic!("expected an unavailable guild");
        };
        assert!(guild.unavailable);
        assert_eq!(
            serde_json::to_value(guild).unwrap(),
            json!({ "id": GUILD, "unavailable": true })
        );
    }
}

#[test]
fn guild_create_without_guild_fields_is_an_error() {
    for raw in [
        format!(r#"{{"id":"{GUILD}"}}"#),
        format!(r#"{{"id":"{GUILD}","unavailable":false}}"#),
        r#"{"unavailable":true}"#.to_owned(),
    ] {
        let error = dispatch_raw("GUILD_CREATE", &raw).unwrap_err();
        assert!(
            error.to_string().contains("missing field"),
            "{raw}: {error}"
        );
    }
}

#[test]
fn extra_fields_decode_in_any_order_and_with_escaped_keys() {
    let member = r#""user":{"id":"1300000000000000003","username":"mixo","discriminator":"0","global_name":null,"avatar":null},"roles":[],"joined_at":null"#;
    for raw in [
        format!(r#"{{"guild_id":"{GUILD}",{member}}}"#),
        format!(r#"{{{member},"guild_id":"{GUILD}"}}"#),
        format!(r#"{{{member},"guild_id":"{GUILD}"}}"#),
    ] {
        let DispatchEvent::GuildMemberAdd(event) = dispatch_raw("GUILD_MEMBER_ADD", &raw).unwrap()
        else {
            panic!("expected GUILD_MEMBER_ADD");
        };
        assert_eq!(event.guild_id, GuildId::new(1_300_000_000_000_000_001));
        assert_eq!(event.user.username, "mixo");
        assert_eq!(event.joined_at, None);
    }

    let thread = thread().to_string();
    let base = thread.strip_prefix('{').unwrap();
    for (raw, newly_created) in [
        (format!(r#"{{"newly_created":true,{base}"#), true),
        (format!(r#"{{"newly_created":true,{base}"#), true),
        (thread.clone(), false),
    ] {
        let DispatchEvent::ThreadCreate(event) = dispatch_raw("THREAD_CREATE", &raw).unwrap()
        else {
            panic!("expected THREAD_CREATE");
        };
        assert_eq!(event.newly_created, newly_created);
        assert_eq!(event.thread.id().get(), 41_771_983_423_143_937);
    }
}

#[test]
fn thread_members_update_presences() {
    let data = data_of("THREAD_MEMBERS_UPDATE");
    let DispatchEvent::ThreadMembersUpdate(event) = dispatch("THREAD_MEMBERS_UPDATE", &data) else {
        panic!("expected THREAD_MEMBERS_UPDATE");
    };
    assert_eq!(event.added_members.len(), 2);
    let first = &event.added_members[0];
    assert_eq!(
        first.member.user_id.map(djs_model::id::Id::get),
        Some(1_300_000_000_000_000_003)
    );
    assert!(first.member.member.is_some());
    assert_eq!(first.presence.as_ref().unwrap().status, Status::Idle);
    assert!(event.added_members[1].presence.is_none());
}

#[test]
fn presence_update_with_a_partial_user_and_the_docs_activities() {
    let examples: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/docs/gateway_events_examples.json")).unwrap();
    let example = |title: &str| {
        examples
            .iter()
            .find(|example| example["title"] == title)
            .unwrap_or_else(|| panic!("no example {title}"))["json"]
            .clone()
    };
    let data = json!({
        "user": { "id": USER, "username": "mixo" }, "guild_id": GUILD, "status": "dnd",
        "activities": [example("Example Activity"), example("Example Activity with Rich Presence")],
        "client_status": { "desktop": "dnd", "web": "online" },
    });
    let DispatchEvent::PresenceUpdate(presence) = dispatch("PRESENCE_UPDATE", &data) else {
        panic!("expected PRESENCE_UPDATE");
    };
    assert_eq!(presence.user.username.as_deref(), Some("mixo"));
    assert_eq!(presence.user.discriminator, None);
    assert_eq!(presence.status, Status::DoNotDisturb);
    assert_eq!(presence.client_status.web, Some(Status::Online));
    assert_eq!(presence.client_status.mobile, None);
    let [stream, rich] = presence.activities.as_slice() else {
        panic!("expected two activities");
    };
    assert_eq!(stream.kind, ActivityType::Streaming);
    assert_eq!(stream.url.as_deref(), Some("https://www.twitch.tv/discord"));
    assert_eq!(rich.party.as_ref().unwrap().size, Some((2, 2)));
    assert_eq!(
        rich.timestamps
            .unwrap()
            .start
            .map(Timestamp::as_unix_millis),
        Some(15_112_000_660_000)
    );
    let secrets = format!("{:?}", rich.secrets.as_ref().unwrap());
    assert!(!secrets.contains("025ed05c"), "{secrets}");
}

#[test]
fn unix_times_are_seconds() {
    let DispatchEvent::TypingStart(typing) = dispatch("TYPING_START", &data_of("TYPING_START"))
    else {
        panic!("expected TYPING_START");
    };
    assert_eq!(typing.timestamp.as_unix_secs(), TIME_SECS);
    assert_eq!(typing.timestamp, TIME.parse().unwrap());
    assert!(typing.member.is_some());

    let DispatchEvent::ChannelInfo(info) = dispatch("CHANNEL_INFO", &data_of("CHANNEL_INFO"))
    else {
        panic!("expected CHANNEL_INFO");
    };
    assert_eq!(
        info.channels[0]
            .voice_start_time
            .map(Timestamp::as_unix_secs),
        Some(TIME_SECS)
    );
    assert_eq!(info.channels[1].voice_start_time, None);
}

#[test]
fn voice_server_update_debug_redacts_the_token() {
    let data = data_of("VOICE_SERVER_UPDATE");
    let DispatchEvent::VoiceServerUpdate(update) = dispatch("VOICE_SERVER_UPDATE", &data) else {
        panic!("expected VOICE_SERVER_UPDATE");
    };
    assert_eq!(update.token, "my_token");
    let debug = format!("{update:?}");
    assert!(!debug.contains("my_token"), "{debug}");
}

#[test]
fn dispatch_event_size() {
    assert!(
        size_of::<DispatchEvent>() <= 32,
        "{}",
        size_of::<DispatchEvent>()
    );
    assert!(
        size_of::<GatewayEvent>() <= 40,
        "{}",
        size_of::<GatewayEvent>()
    );
}

/// Serializes `command` the way a shard sends it.
fn payload<T: GatewayCommand + Serialize>(command: &T) -> Value {
    serde_json::to_value(CommandPayload::new(command)).unwrap()
}

fn docs_example(title: &str) -> Value {
    let examples: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/docs/gateway_events_examples.json")).unwrap();
    examples
        .into_iter()
        .find(|example| example["title"] == title)
        .unwrap_or_else(|| panic!("no example {title}"))["json"]
        .take()
}

#[test]
fn commands_match_the_docs_examples() {
    let guild = GuildId::new(41_771_983_444_115_456);
    assert_eq!(
        payload(&RequestGuildMembers::query(guild, "", 0)),
        docs_example("Example Request Guild Members")
    );
    assert_eq!(
        payload(&RequestSoundboardSounds::new([
            GuildId::new(613_425_648_685_547_541),
            GuildId::new(81_384_788_765_712_384),
        ])),
        docs_example("Example Request Soundboard Sounds")
    );
    assert_eq!(
        payload(&RequestChannelInfo::new(
            GuildId::new(613_425_648_685_547_541),
            [ChannelInfoField::Status, ChannelInfoField::VoiceStartTime],
        )),
        docs_example("Example Request Channel Info")
    );
    assert_eq!(
        payload(&UpdateVoiceState::new(
            GuildId::new(41_771_983_423_143_937),
            Some(ChannelId::new(127_121_515_262_115_840)),
            false,
            false,
        )),
        docs_example("Example Gateway Voice State Update")
    );

    let mut presence = UpdatePresence::new(
        Status::Online,
        [Activity::new(
            ActivityType::Playing,
            "Save the Oxford Comma",
        )],
    );
    presence.since = Timestamp::from_unix_millis(91_879_201);
    assert_eq!(
        payload(&presence),
        docs_example("Example Gateway Presence Update")
    );
}

#[test]
fn identify_with_a_presence_matches_the_docs_example() {
    let mut presence = UpdatePresence::new(
        Status::DoNotDisturb,
        [Activity::new(
            ActivityType::Playing,
            "Cards Against Humanity",
        )],
    );
    presence.since = Timestamp::from_unix_millis(91_879_201);
    let identify = Identify {
        token: "my_token".into(),
        properties: IdentifyProperties {
            os: "linux".into(),
            browser: "disco".into(),
            device: "disco".into(),
        },
        large_threshold: Some(250),
        shard: Some(ShardId::new(0, 1)),
        intents: Intents::GUILDS | Intents::GUILD_MODERATION,
        presence: Some(presence),
        capabilities: djs_model::gateway::payload::GatewayCapabilities::empty(),
    };
    let mut expected = docs_example("Example Identify");
    // Per-payload compression is not modelled: transport compression is used instead (D-015).
    expected["d"].as_object_mut().unwrap().remove("compress");
    assert_eq!(payload(&identify), expected);
}

#[test]
fn update_presence_without_since_writes_null() {
    let presence = UpdatePresence::new(Status::Invisible, []);
    assert_eq!(
        payload(&presence),
        json!({ "op": 3, "d": { "since": null, "activities": [], "status": "invisible",
                                "afk": false } })
    );
}
