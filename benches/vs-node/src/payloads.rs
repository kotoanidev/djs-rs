//! Gateway payloads shaped like the ones Discord sends to bots, generated deterministically so
//! that both libraries receive byte-for-byte the same input.
//!
//! Every ID is unique across the generated guilds (the guild index is part of each ID), so no
//! library can deduplicate users or channels that real Discord would send as different ones.

use std::fmt::Write as _;

const GUILD_BASE: u64 = 1_400_000_000_000_000_000;
const CHANNEL_BASE: u64 = 1_200_000_000_000_000_000;
const ROLE_BASE: u64 = 1_100_000_000_000_000_000;
const EMOJI_BASE: u64 = 1_250_000_000_000_000_000;
const USER_BASE: u64 = 1_000_000_000_000_000_000;
const MESSAGE_BASE: u64 = 1_500_000_000_000_000_000;
/// The bot's own user ID.
const BOT: u64 = 1_437_000_000_000_000_001;
const TIME: &str = "2026-11-11T12:00:00.000000+00:00";

/// The size of each generated guild.
#[derive(Clone, Copy, Debug)]
pub struct GuildShape {
    /// Members (each with its user), sent in full like a large bot with the members intent
    /// sees them.
    pub members: u64,
    /// Text channels.
    pub channels: u64,
    /// Roles, including `@everyone`.
    pub roles: u64,
    /// Custom emojis.
    pub emojis: u64,
}

impl Default for GuildShape {
    fn default() -> Self {
        Self {
            members: 1000,
            channels: 20,
            roles: 10,
            emojis: 20,
        }
    }
}

/// The ID of guild `g`.
#[must_use]
pub const fn guild_id(g: u64) -> u64 {
    GUILD_BASE + g
}

/// The ID of channel `i` of guild `g`.
#[must_use]
pub const fn channel_id(g: u64, i: u64) -> u64 {
    CHANNEL_BASE + g * 1000 + i
}

/// The ID of role `i` of guild `g`; role 0 is `@everyone`, whose ID is the guild's.
#[must_use]
pub const fn role_id(g: u64, i: u64) -> u64 {
    if i == 0 {
        guild_id(g)
    } else {
        ROLE_BASE + g * 1000 + i
    }
}

/// The ID of member `i` of guild `g`.
#[must_use]
pub const fn user_id(g: u64, i: u64) -> u64 {
    USER_BASE + g * 1_000_000 + i
}

/// Wraps the data `d` of a dispatch event in a gateway payload.
#[must_use]
pub fn dispatch(name: &str, sequence: u64, d: &str) -> String {
    format!(r#"{{"t":"{name}","s":{sequence},"op":0,"d":{d}}}"#)
}

/// The `HELLO` payload. The heartbeat interval is long enough that no heartbeat is due while
/// a scenario runs.
pub const HELLO: &str = r#"{"t":null,"s":null,"op":10,"d":{"heartbeat_interval":600000}}"#;

/// The `READY` dispatch, listing `guilds` guilds as unavailable (they follow as
/// `GUILD_CREATE`s, as on a real connection).
#[must_use]
pub fn ready(guilds: u64) -> String {
    let mut list = String::new();
    for g in 0..guilds {
        if g > 0 {
            list.push(',');
        }
        write!(list, r#"{{"id":"{}","unavailable":true}}"#, guild_id(g)).unwrap();
    }
    let d = format!(
        r#"{{"v":10,"user":{{"id":"{BOT}","username":"showcase-bot","discriminator":"0","global_name":null,"avatar":null,"bot":true,"verified":true,"mfa_enabled":false,"flags":0}},"guilds":[{list}],"session_id":"0123456789abcdef0123456789abcdef","resume_gateway_url":"ws://127.0.0.1:9","shard":[0,1],"application":{{"id":"{BOT}","flags":0}},"private_channels":[],"presences":[],"relationships":[],"guild_join_requests":[],"user_settings":{{}},"auth":{{}},"geo_ordered_rtc_regions":["frankfurt"]}}"#
    );
    dispatch("READY", 1, &d)
}

/// A user, with the fields Discord sends in members and messages.
fn user(g: u64, i: u64) -> String {
    format!(
        r#"{{"id":"{}","username":"member{i}","discriminator":"0","global_name":"Member {i}","avatar":"a_1269e74af4df7417b13759eae50c83dc","public_flags":64,"avatar_decoration_data":null,"collectibles":null,"primary_guild":null,"clan":null}}"#,
        user_id(g, i)
    )
}

fn member(g: u64, i: u64) -> String {
    format!(
        r#"{{"user":{},"roles":["{}","{}"],"nick":null,"avatar":null,"banner":null,"premium_since":null,"joined_at":"2024-05-01T12:00:00.000000+00:00","deaf":false,"mute":false,"pending":false,"flags":0,"communication_disabled_until":null}}"#,
        user(g, i),
        role_id(g, 1),
        role_id(g, 2)
    )
}

fn role(g: u64, i: u64) -> String {
    let name = if i == 0 {
        "@everyone".to_owned()
    } else {
        format!("role {i}")
    };
    format!(
        r#"{{"id":"{}","name":"{name}","description":null,"color":3447003,"colors":{{"primary_color":3447003,"secondary_color":null,"tertiary_color":null}},"hoist":true,"icon":null,"unicode_emoji":null,"position":{i},"permissions":"1071698660929","managed":false,"mentionable":true,"flags":0}}"#,
        role_id(g, i)
    )
}

fn channel(g: u64, i: u64) -> String {
    format!(
        r#"{{"id":"{}","type":0,"name":"channel-{i}","position":{i},"permission_overwrites":[{{"id":"{}","type":0,"allow":"1024","deny":"0"}},{{"id":"{}","type":1,"allow":"0","deny":"2048"}}],"rate_limit_per_user":0,"nsfw":false,"topic":"What channel {i} is about","last_message_id":"{}","parent_id":null,"flags":0,"default_auto_archive_duration":1440}}"#,
        channel_id(g, i),
        role_id(g, 1),
        user_id(g, i),
        MESSAGE_BASE + i,
    )
}

fn emoji(g: u64, i: u64) -> String {
    format!(
        r#"{{"id":"{}","name":"emoji_{i}","roles":[],"require_colons":true,"managed":false,"animated":false,"available":true}}"#,
        EMOJI_BASE + g * 1000 + i
    )
}

fn join(out: &mut String, count: u64, item: impl Fn(u64) -> String) {
    for i in 0..count {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&item(i));
    }
}

/// The `GUILD_CREATE` dispatch of guild `g`.
#[must_use]
pub fn guild_create(sequence: u64, g: u64, shape: &GuildShape) -> String {
    let mut d = String::with_capacity(usize::try_from(shape.members * 480).unwrap_or(0) + 16_384);
    write!(
        d,
        r#"{{"id":"{}","name":"Guild {g}","icon":"f64c482b807da4f539cff778d174971c","description":"A large guild","splash":null,"discovery_splash":null,"features":["ANIMATED_ICON","COMMUNITY","NEWS","INVITE_SPLASH"],"stickers":[],"banner":null,"owner_id":"{}","application_id":null,"region":null,"afk_channel_id":null,"afk_timeout":300,"system_channel_id":"{}","widget_enabled":false,"widget_channel_id":null,"verification_level":2,"default_message_notifications":1,"mfa_level":1,"explicit_content_filter":2,"max_presences":null,"max_members":500000,"max_video_channel_users":25,"vanity_url_code":null,"premium_tier":2,"premium_subscription_count":12,"system_channel_flags":0,"preferred_locale":"en-US","rules_channel_id":null,"public_updates_channel_id":null,"safety_alerts_channel_id":null,"nsfw_level":0,"nsfw":false,"premium_progress_bar_enabled":false,"hub_type":null,"incidents_data":null,"latest_onboarding_question_id":null,"home_header":null,"inventory_settings":null,"version":1794398400000,"joined_at":"{TIME}","large":true,"unavailable":false,"lazy":true,"member_count":{},"application_command_counts":{{"1":12}},"embedded_activities":[],"activity_instances":[],"stage_instances":[],"guild_scheduled_events":[],"soundboard_sounds":[],"threads":[],"voice_states":[],"presences":[],"#,
        guild_id(g),
        user_id(g, 0),
        channel_id(g, 0),
        shape.members,
    )
    .unwrap();
    d.push_str(r#""emojis":["#);
    join(&mut d, shape.emojis, |i| emoji(g, i));
    d.push_str(r#"],"roles":["#);
    join(&mut d, shape.roles, |i| role(g, i));
    d.push_str(r#"],"channels":["#);
    join(&mut d, shape.channels, |i| channel(g, i));
    d.push_str(r#"],"members":["#);
    join(&mut d, shape.members, |i| member(g, i));
    d.push_str("]}");
    dispatch("GUILD_CREATE", sequence, &d)
}

/// A typical `MESSAGE_CREATE` (≈ 0.9 KB): a plain text message by member `author` of guild 0
/// in its channel 1, with the member and user objects Discord includes.
#[must_use]
pub fn message_create(sequence: u64, author: u64) -> String {
    let id = MESSAGE_BASE + sequence;
    format!(
        r#"{{"t":"MESSAGE_CREATE","s":{sequence},"op":0,"d":{{"type":0,"tts":false,"timestamp":"2026-11-10T19:42:07.123000+00:00","pinned":false,"nonce":"{id}","mentions":[],"mention_roles":[],"mention_everyone":false,"member":{{"roles":["{}","{}"],"premium_since":null,"pending":false,"nick":null,"mute":false,"joined_at":"2024-05-01T12:00:00.000000+00:00","flags":0,"deaf":false,"communication_disabled_until":null,"banner":null,"avatar":null}},"id":"{id}","flags":0,"embeds":[],"edited_timestamp":null,"content":"message number {sequence}: zażółć gęślą jaźń 🦀 and a tail long enough to be a real sentence","components":[],"channel_type":0,"channel_id":"{}","author":{},"attachments":[],"guild_id":"{}"}}}}"#,
        role_id(0, 1),
        role_id(0, 2),
        channel_id(0, 1),
        user(0, author),
        guild_id(0),
    )
}

/// The JSON body of the message returned by the mock Create Message endpoint.
#[must_use]
pub fn sent_message(channel: &str, sequence: u64) -> String {
    let id = MESSAGE_BASE + sequence;
    format!(
        r#"{{"type":0,"tts":false,"timestamp":"2026-11-10T19:42:07.123000+00:00","pinned":false,"nonce":null,"mentions":[],"mention_roles":[],"mention_everyone":false,"id":"{id}","flags":0,"embeds":[],"edited_timestamp":null,"content":"benchmark","components":[],"channel_id":"{channel}","author":{{"id":"{BOT}","username":"showcase-bot","discriminator":"0","global_name":null,"avatar":null,"bot":true,"public_flags":0}},"attachments":[]}}"#
    )
}
