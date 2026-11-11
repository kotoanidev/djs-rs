//! Realistic gateway payloads, shaped like the ones Discord sends to bots, for the benchmarks
//! (`benches/decode.rs` includes this file) and the extreme-load tests (`tests/stress.rs`).

use std::fmt::Write as _;

/// The guild every generated payload belongs to.
pub(crate) const GUILD: u64 = 1_400_000_000_000_000_002;
const CHANNEL: u64 = 1_200_000_000_000_000_020;
const USER_BASE: u64 = 1_000_000_000_000_000_000;
const ROLE_BASE: u64 = 1_100_000_000_000_000_000;
const CHANNEL_BASE: u64 = 1_200_000_000_000_000_000;
const EMOJI_BASE: u64 = 1_250_000_000_000_000_000;
const MESSAGE_BASE: u64 = 1_500_000_000_000_000_000;
const TIME: &str = "2026-11-11T12:00:00.000000+00:00";

/// Wraps the data `d` of a dispatch event in a gateway payload.
pub(crate) fn dispatch(name: &str, sequence: u64, d: &str) -> String {
    format!(r#"{{"t":"{name}","s":{sequence},"op":0,"d":{d}}}"#)
}

/// A user, with the fields Discord sends in members and messages.
pub(crate) fn user(i: u64) -> String {
    format!(
        r#"{{"id":"{}","username":"member{i}","discriminator":"0","global_name":"Member {i}","avatar":"a_1269e74af4df7417b13759eae50c83dc","public_flags":64,"avatar_decoration_data":null,"collectibles":null,"primary_guild":null,"clan":null}}"#,
        USER_BASE + i
    )
}

/// A guild member with its user.
pub(crate) fn member(i: u64) -> String {
    format!(
        r#"{{"user":{},"roles":["{}","{}"],"nick":null,"avatar":null,"banner":null,"premium_since":null,"joined_at":"2024-05-01T12:00:00.000000+00:00","deaf":false,"mute":false,"pending":false,"flags":0,"communication_disabled_until":null}}"#,
        user(i),
        ROLE_BASE + 1,
        ROLE_BASE + 2
    )
}

fn role(i: u64) -> String {
    format!(
        r#"{{"id":"{}","name":"role {i}","description":null,"color":3447003,"colors":{{"primary_color":3447003,"secondary_color":null,"tertiary_color":null}},"hoist":true,"icon":null,"unicode_emoji":null,"position":{i},"permissions":"1071698660929","managed":false,"mentionable":true,"flags":0}}"#,
        ROLE_BASE + i
    )
}

fn channel(i: u64) -> String {
    format!(
        r#"{{"id":"{}","type":0,"name":"channel-{i}","position":{i},"permission_overwrites":[{{"id":"{}","type":0,"allow":"1024","deny":"0"}},{{"id":"{}","type":1,"allow":"0","deny":"2048"}}],"rate_limit_per_user":0,"nsfw":false,"topic":"What channel {i} is about","last_message_id":"{}","parent_id":"{}","flags":0,"default_auto_archive_duration":1440}}"#,
        CHANNEL_BASE + i,
        ROLE_BASE + 1,
        USER_BASE + i,
        MESSAGE_BASE + i,
        CHANNEL_BASE
    )
}

fn emoji(i: u64) -> String {
    format!(
        r#"{{"id":"{}","name":"emoji_{i}","roles":[],"require_colons":true,"managed":false,"animated":false,"available":true}}"#,
        EMOJI_BASE + i
    )
}

/// The presence of user `i`, as sent in `GUILD_CREATE` (`guild_id: None`) or in
/// `PRESENCE_UPDATE` (`guild_id: Some`).
pub(crate) fn presence(i: u64, guild_id: Option<u64>) -> String {
    let guild = guild_id.map_or_else(String::new, |id| format!(r#""guild_id":"{id}","#));
    format!(
        r#"{{"user":{{"id":"{}"}},{guild}"status":"online","client_status":{{"desktop":"online","mobile":"idle"}},"activities":[{{"id":"custom","name":"Custom Status","type":4,"state":"Writing Rust","emoji":{{"name":"🦀"}},"created_at":1794398400000}},{{"id":"ec0b28a579ecb4bd","name":"Visual Studio Code","type":0,"application_id":"383226320970055681","details":"Editing main.rs","state":"Workspace: djs-rs","timestamps":{{"start":1794398000000}},"assets":{{"large_image":"565945350376841216","large_text":"Editing a RUST file"}},"created_at":1794398400000}}]}}"#,
        USER_BASE + i
    )
}

fn voice_state(i: u64) -> String {
    format!(
        r#"{{"user_id":"{}","channel_id":"{}","session_id":"0123456789abcdef0123456789abcdef","deaf":false,"mute":false,"self_deaf":false,"self_mute":true,"self_video":false,"suppress":false,"request_to_speak_timestamp":null}}"#,
        USER_BASE + i,
        CHANNEL_BASE + 1
    )
}

fn join(count: u64, item: impl Fn(u64) -> String) -> String {
    let mut out = String::new();
    for i in 0..count {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&item(i));
    }
    out
}

/// A `GUILD_CREATE` for a large guild: `members` members (with as many presences, capped by
/// `presences`), `channels` text channels, `roles` roles, 50 emojis and 20 voice states.
pub(crate) fn guild_create(
    sequence: u64,
    members: u64,
    presences: u64,
    channels: u64,
    roles: u64,
) -> String {
    let mut d = String::with_capacity(usize::try_from(members * 500).unwrap_or(0));
    write!(
        d,
        r#"{{"id":"{GUILD}","name":"Big Guild","icon":"f64c482b807da4f539cff778d174971c","description":"A large guild","splash":null,"discovery_splash":null,"features":["ANIMATED_ICON","COMMUNITY","NEWS","INVITE_SPLASH"],"emojis":[{}],"stickers":[],"banner":null,"owner_id":"{}","application_id":null,"region":null,"afk_channel_id":null,"afk_timeout":300,"system_channel_id":"{CHANNEL}","widget_enabled":false,"widget_channel_id":null,"verification_level":2,"roles":[{}],"default_message_notifications":1,"mfa_level":1,"explicit_content_filter":2,"max_presences":null,"max_members":500000,"max_video_channel_users":25,"vanity_url_code":null,"premium_tier":2,"premium_subscription_count":12,"system_channel_flags":0,"preferred_locale":"en-US","rules_channel_id":null,"public_updates_channel_id":null,"safety_alerts_channel_id":null,"nsfw_level":0,"nsfw":false,"premium_progress_bar_enabled":false,"hub_type":null,"incidents_data":null,"latest_onboarding_question_id":null,"home_header":null,"inventory_settings":null,"version":1794398400000,"joined_at":"{TIME}","large":true,"unavailable":false,"lazy":true,"member_count":{members},"application_command_counts":{{"1":12}},"embedded_activities":[],"activity_instances":[],"stage_instances":[],"guild_scheduled_events":[],"soundboard_sounds":[],"threads":[],"voice_states":[{}],"channels":[{}],"presences":[{}],"members":[{}]}}"#,
        join(50, emoji),
        USER_BASE,
        join(roles, role),
        join(20.min(members), voice_state),
        join(channels, channel),
        join(presences.min(members), |i| presence(i, None)),
        join(members, member),
    )
    .unwrap();
    dispatch("GUILD_CREATE", sequence, &d)
}

/// The `MESSAGE_CREATE` fixture (1.5 KB, a reply with a member, mentions and an embed).
pub(crate) const MESSAGE_CREATE: &str = include_str!("../fixtures/message_create.json");

/// A slash command interaction with two options and resolved users and members.
pub(crate) fn interaction_create(sequence: u64) -> String {
    let d = format!(
        r#"{{"version":1,"type":2,"token":"interaction-token-for-tests","locale":"pl","id":"{}","guild_locale":"en-US","guild_id":"{GUILD}","guild":{{"locale":"en-US","id":"{GUILD}","features":["COMMUNITY"]}},"entitlements":[],"entitlement_sku_ids":[],"context":0,"channel_id":"{CHANNEL}","channel":{{"type":0,"topic":null,"rate_limit_per_user":0,"position":3,"permissions":"2248473465835073","parent_id":"{CHANNEL_BASE}","nsfw":false,"name":"general","last_message_id":"{}","id":"{CHANNEL}","guild_id":"{GUILD}","flags":0}},"authorizing_integration_owners":{{"0":"{GUILD}"}},"attachment_size_limit":10485760,"application_id":"1437000000000000001","app_permissions":"2248473465835073","member":{{"user":{},"roles":["{}"],"premium_since":null,"permissions":"2248473465835073","pending":false,"nick":null,"mute":false,"joined_at":"2024-05-01T12:00:00.000000+00:00","flags":0,"deaf":false,"communication_disabled_until":null,"banner":null,"avatar":null}},"data":{{"type":1,"name":"warn","id":"1437000000000000100","guild_id":"{GUILD}","options":[{{"value":"{}","type":6,"name":"user"}},{{"value":"Please read the rules","type":3,"name":"reason"}},{{"value":3,"type":4,"name":"days"}}],"resolved":{{"users":{{"{}":{}}},"members":{{"{}":{{"roles":[],"premium_since":null,"permissions":"0","pending":false,"nick":null,"joined_at":"2025-01-01T00:00:00.000000+00:00","flags":0,"communication_disabled_until":null,"banner":null,"avatar":null}}}}}}}}}}"#,
        MESSAGE_BASE + sequence,
        MESSAGE_BASE,
        user(1),
        ROLE_BASE + 1,
        USER_BASE + 2,
        USER_BASE + 2,
        user(2),
        USER_BASE + 2,
    );
    dispatch("INTERACTION_CREATE", sequence, &d)
}

/// The frequent small events a bot receives, as `(name, payload)` pairs, with `sequence` mixed
/// into their IDs.
pub(crate) fn hot_events(sequence: u64) -> Vec<(&'static str, String)> {
    let message = MESSAGE_BASE + sequence;
    let user_id = USER_BASE + sequence % 1000;
    let mut events = vec![
        ("MESSAGE_CREATE", MESSAGE_CREATE.to_owned()),
        (
            "MESSAGE_UPDATE",
            MESSAGE_CREATE.replacen("MESSAGE_CREATE", "MESSAGE_UPDATE", 1),
        ),
        ("INTERACTION_CREATE", interaction_create(sequence)),
        (
            "PRESENCE_UPDATE",
            dispatch(
                "PRESENCE_UPDATE",
                sequence,
                &presence(sequence % 1000, Some(GUILD)),
            ),
        ),
        (
            "TYPING_START",
            dispatch(
                "TYPING_START",
                sequence,
                &format!(
                    r#"{{"user_id":"{user_id}","timestamp":1794398400,"member":{},"channel_id":"{CHANNEL}","guild_id":"{GUILD}"}}"#,
                    member(sequence % 1000)
                ),
            ),
        ),
        (
            "MESSAGE_REACTION_ADD",
            dispatch(
                "MESSAGE_REACTION_ADD",
                sequence,
                &format!(
                    r#"{{"user_id":"{user_id}","type":0,"message_id":"{message}","message_author_id":"{USER_BASE}","member":{},"emoji":{{"name":"🔥","id":null}},"channel_id":"{CHANNEL}","burst":false,"guild_id":"{GUILD}","burst_colors":[]}}"#,
                    member(sequence % 1000)
                ),
            ),
        ),
        (
            "MESSAGE_DELETE",
            dispatch(
                "MESSAGE_DELETE",
                sequence,
                &format!(r#"{{"id":"{message}","channel_id":"{CHANNEL}","guild_id":"{GUILD}"}}"#),
            ),
        ),
        (
            "GUILD_MEMBER_ADD",
            dispatch(
                "GUILD_MEMBER_ADD",
                sequence,
                &member(sequence % 1000).replacen('{', &format!(r#"{{"guild_id":"{GUILD}","#), 1),
            ),
        ),
        (
            "GUILD_MEMBER_UPDATE",
            dispatch(
                "GUILD_MEMBER_UPDATE",
                sequence,
                &member(sequence % 1000).replacen('{', &format!(r#"{{"guild_id":"{GUILD}","#), 1),
            ),
        ),
        (
            "VOICE_STATE_UPDATE",
            dispatch(
                "VOICE_STATE_UPDATE",
                sequence,
                &voice_state(sequence % 1000).replacen(
                    '{',
                    &format!(
                        r#"{{"guild_id":"{GUILD}","member":{},"#,
                        member(sequence % 1000)
                    ),
                    1,
                ),
            ),
        ),
        (
            "CHANNEL_UPDATE",
            dispatch(
                "CHANNEL_UPDATE",
                sequence,
                &channel(sequence % 100).replacen('{', &format!(r#"{{"guild_id":"{GUILD}","#), 1),
            ),
        ),
    ];
    events.shrink_to_fit();
    events
}
