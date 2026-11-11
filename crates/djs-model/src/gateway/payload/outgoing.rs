//! Payloads (commands) sent to the gateway.

use core::fmt;

use compact_str::CompactString;
use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::{
    gateway::{Activity, Intents, OpCode, ShardId, Status},
    id::{ChannelId, GuildId, UserId},
    macros::{int_flags, str_enum},
    util::Timestamp,
};

/// A payload that can be sent to the gateway.
///
/// Each command knows its [opcode](Self::OPCODE). Wrap a command in [`CommandPayload`] to get
/// the `{"op": …, "d": …}` envelope the gateway expects.
pub trait GatewayCommand: Serialize {
    /// The opcode of the command.
    const OPCODE: OpCode;
}

/// The `{"op": …, "d": …}` envelope around a [`GatewayCommand`], ready to be serialized.
///
/// # Examples
///
/// ```
/// use djs_model::gateway::payload::{CommandPayload, Heartbeat};
///
/// let json = serde_json::to_string(&CommandPayload::new(&Heartbeat(Some(251)))).unwrap();
/// assert_eq!(json, r#"{"op":1,"d":251}"#);
/// ```
#[derive(Debug)]
pub struct CommandPayload<'a, T> {
    command: &'a T,
}

impl<'a, T: GatewayCommand> CommandPayload<'a, T> {
    /// Wraps a command.
    #[must_use]
    pub const fn new(command: &'a T) -> Self {
        Self { command }
    }
}

impl<T: GatewayCommand> Serialize for CommandPayload<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut payload = serializer.serialize_struct("CommandPayload", 2)?;
        payload.serialize_field("op", &T::OPCODE)?;
        payload.serialize_field("d", self.command)?;
        payload.end()
    }
}

/// A [Heartbeat](https://docs.discord.com/developers/events/gateway-events#heartbeat)
/// (opcode 1): the last sequence number received, or `None` if no dispatch has been received
/// yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Heartbeat(pub Option<u64>);

impl GatewayCommand for Heartbeat {
    const OPCODE: OpCode = OpCode::Heartbeat;
}

/// An [Identify](https://docs.discord.com/developers/events/gateway-events#identify)
/// (opcode 2): starts a new session.
///
/// Identifying is limited to `max_concurrency` sessions per 5 seconds and 1000 per day; exceeding
/// the daily limit resets the bot token.
///
/// The [`Debug`] implementation redacts the token.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct Identify {
    /// The bot token, **without** the `Bot ` prefix.
    pub token: CompactString,
    /// Connection properties describing the client.
    pub properties: IdentifyProperties,
    /// The member count above which offline members are not sent in a guild's member list
    /// (50–250, Discord's default is 50).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_threshold: Option<u8>,
    /// The shard this session handles. Omit for unsharded bots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shard: Option<ShardId>,
    /// The gateway intents to subscribe to.
    pub intents: Intents,
    /// The presence to start with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence: Option<UpdatePresence>,
    /// Opt-in gateway behaviours.
    #[serde(skip_serializing_if = "GatewayCapabilities::is_empty")]
    pub capabilities: GatewayCapabilities,
}

int_flags! {
    /// [Gateway capabilities](https://docs.discord.com/developers/events/gateway-events#identify-gateway-capabilities)
    /// a client opts into with [`Identify::capabilities`].
    pub struct GatewayCapabilities: u64 {
        /// Receive obfuscated metadata for channels the bot cannot view.
        const CHANNEL_OBFUSCATION = 1 << 15;
    }
}

impl GatewayCommand for Identify {
    const OPCODE: OpCode = OpCode::Identify;
}

impl fmt::Debug for Identify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Identify")
            .field("token", &"<redacted>")
            .field("properties", &self.properties)
            .field("large_threshold", &self.large_threshold)
            .field("shard", &self.shard)
            .field("intents", &self.intents)
            .field("presence", &self.presence)
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

/// The [connection properties](https://docs.discord.com/developers/events/gateway-events#identify-identify-connection-properties)
/// sent in [`Identify`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IdentifyProperties {
    /// The operating system, e.g. `linux`.
    pub os: CompactString,
    /// The library name.
    pub browser: CompactString,
    /// The library name.
    pub device: CompactString,
}

impl IdentifyProperties {
    /// Creates properties with the given library name and the current operating system
    /// ([`std::env::consts::OS`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::gateway::payload::IdentifyProperties;
    ///
    /// let properties = IdentifyProperties::new("djs-rs");
    /// assert_eq!(properties.browser, "djs-rs");
    /// assert_eq!(properties.os, std::env::consts::OS);
    /// ```
    #[must_use]
    pub fn new(library: &str) -> Self {
        Self {
            os: CompactString::const_new(std::env::consts::OS),
            browser: library.into(),
            device: library.into(),
        }
    }
}

/// A [Resume](https://docs.discord.com/developers/events/gateway-events#resume) (opcode 6):
/// continues a disconnected session and replays missed events.
///
/// The [`Debug`] implementation redacts the token.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct Resume {
    /// The bot token, **without** the `Bot ` prefix.
    pub token: CompactString,
    /// The session ID from [`Ready`](super::Ready).
    pub session_id: CompactString,
    /// The last sequence number received.
    pub seq: u64,
}

impl GatewayCommand for Resume {
    const OPCODE: OpCode = OpCode::Resume;
}

impl fmt::Debug for Resume {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resume")
            .field("token", &"<redacted>")
            .field("session_id", &self.session_id)
            .field("seq", &self.seq)
            .finish()
    }
}

/// [Request Guild Members](https://docs.discord.com/developers/events/gateway-events#request-guild-members)
/// (opcode 8): asks for members of a guild; they arrive in
/// [`GuildMembersChunk`](super::GuildMembersChunk) events.
///
/// Requesting all members (or presences) requires the `GUILD_MEMBERS` (or `GUILD_PRESENCES`)
/// intent; it is rate limited ([`RateLimited`](super::RateLimited)).
///
/// discord.js counterpart: `GuildMemberManager#fetch` (over the gateway).
///
/// # Examples
///
/// ```
/// use djs_model::{gateway::payload::{CommandPayload, RequestGuildMembers}, id::GuildId};
///
/// let request = RequestGuildMembers::query(GuildId::new(41771983423143937), "mix", 10);
/// assert_eq!(
///     serde_json::to_string(&CommandPayload::new(&request)).unwrap(),
///     r#"{"op":8,"d":{"guild_id":"41771983423143937","query":"mix","limit":10}}"#
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct RequestGuildMembers {
    /// The guild.
    pub guild_id: GuildId,
    /// The prefix usernames must start with (`""` for all members).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<CompactString>,
    /// The maximum number of members for a query (`0` with an empty query: all members).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Whether to include the members' presences.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub presences: bool,
    /// The users to fetch (instead of a query).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub user_ids: Vec<UserId>,
    /// A nonce returned in the chunks (max 32 bytes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<CompactString>,
}

impl RequestGuildMembers {
    /// Requests the members whose username starts with `query`, at most `limit`.
    #[must_use]
    pub fn query(guild_id: GuildId, query: impl Into<CompactString>, limit: u32) -> Self {
        Self {
            guild_id,
            query: Some(query.into()),
            limit: Some(limit),
            presences: false,
            user_ids: Vec::new(),
            nonce: None,
        }
    }

    /// Requests all members (requires the `GUILD_MEMBERS` intent).
    #[must_use]
    pub fn all(guild_id: GuildId) -> Self {
        Self::query(guild_id, "", 0)
    }

    /// Requests the given users (up to 100).
    #[must_use]
    pub fn users(guild_id: GuildId, user_ids: impl IntoIterator<Item = UserId>) -> Self {
        Self {
            guild_id,
            query: None,
            limit: None,
            presences: false,
            user_ids: user_ids.into_iter().collect(),
            nonce: None,
        }
    }
}

impl GatewayCommand for RequestGuildMembers {
    const OPCODE: OpCode = OpCode::RequestGuildMembers;
}

/// [Request Soundboard Sounds](https://docs.discord.com/developers/events/gateway-events#request-soundboard-sounds)
/// (opcode 31): asks for the soundboard sounds of guilds; they arrive in
/// [`SoundboardSounds`](super::SoundboardSounds) events.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct RequestSoundboardSounds {
    /// The guilds.
    pub guild_ids: Vec<GuildId>,
}

impl RequestSoundboardSounds {
    /// Requests the sounds of the given guilds.
    #[must_use]
    pub fn new(guild_ids: impl IntoIterator<Item = GuildId>) -> Self {
        Self {
            guild_ids: guild_ids.into_iter().collect(),
        }
    }
}

impl GatewayCommand for RequestSoundboardSounds {
    const OPCODE: OpCode = OpCode::RequestSoundboardSounds;
}

/// [Request Channel Info](https://docs.discord.com/developers/events/gateway-events#request-channel-info)
/// (opcode 43): asks for ephemeral data of a guild's channels; it arrives in a
/// [`ChannelInfo`](super::ChannelInfo) event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct RequestChannelInfo {
    /// The guild.
    pub guild_id: GuildId,
    /// The fields to request.
    pub fields: Vec<ChannelInfoField>,
}

impl RequestChannelInfo {
    /// Requests the given fields of the guild's channels.
    #[must_use]
    pub fn new(guild_id: GuildId, fields: impl IntoIterator<Item = ChannelInfoField>) -> Self {
        Self {
            guild_id,
            fields: fields.into_iter().collect(),
        }
    }
}

impl GatewayCommand for RequestChannelInfo {
    const OPCODE: OpCode = OpCode::RequestChannelInfo;
}

str_enum! {
    /// A field that [`RequestChannelInfo`] can request.
    pub enum ChannelInfoField {
        /// The voice channel status.
        Status = "status",
        /// When the voice session started.
        VoiceStartTime = "voice_start_time",
    }
}

/// [Update Voice State](https://docs.discord.com/developers/events/gateway-events#update-voice-state)
/// (opcode 4): joins, moves between or leaves voice channels.
///
/// discord.js counterpart: what `@discordjs/voice` sends through the guild's shard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct UpdateVoiceState {
    /// The guild.
    pub guild_id: GuildId,
    /// The voice channel to join (`None` to disconnect).
    pub channel_id: Option<ChannelId>,
    /// Whether the bot is muted.
    pub self_mute: bool,
    /// Whether the bot is deafened.
    pub self_deaf: bool,
}

impl UpdateVoiceState {
    /// Joins `channel_id` (or disconnects with `None`).
    #[must_use]
    pub const fn new(
        guild_id: GuildId,
        channel_id: Option<ChannelId>,
        self_mute: bool,
        self_deaf: bool,
    ) -> Self {
        Self {
            guild_id,
            channel_id,
            self_mute,
            self_deaf,
        }
    }
}

impl GatewayCommand for UpdateVoiceState {
    const OPCODE: OpCode = OpCode::VoiceStateUpdate;
}

/// [Update Presence](https://docs.discord.com/developers/events/gateway-events#update-presence)
/// (opcode 3): sets the bot's status and activities (5 updates per 20 seconds).
///
/// discord.js counterpart: `ClientUser#setPresence`.
///
/// # Examples
///
/// ```
/// use djs_model::gateway::{Activity, ActivityType, Status, payload::UpdatePresence};
///
/// let presence = UpdatePresence::new(Status::Online, [Activity::new(ActivityType::Watching, "you")]);
/// assert_eq!(
///     serde_json::to_string(&presence).unwrap(),
///     r#"{"since":null,"activities":[{"name":"you","type":3}],"status":"online","afk":false}"#
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct UpdatePresence {
    /// When the bot went idle (sent as Unix milliseconds, D-075), `None` if not idle.
    #[serde(with = "crate::serde_util::option_unix_millis")]
    pub since: Option<Timestamp>,
    /// The activities (bots can set the name, state, type and URL).
    pub activities: Vec<Activity>,
    /// The status.
    pub status: Status,
    /// Whether the bot is AFK.
    pub afk: bool,
}

impl UpdatePresence {
    /// Sets the status and activities (not idle, not AFK).
    #[must_use]
    pub fn new(status: Status, activities: impl IntoIterator<Item = Activity>) -> Self {
        Self {
            since: None,
            activities: activities.into_iter().collect(),
            status,
            afk: false,
        }
    }
}

impl GatewayCommand for UpdatePresence {
    const OPCODE: OpCode = OpCode::PresenceUpdate;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identify_json() {
        let identify = Identify {
            token: "secret".into(),
            properties: IdentifyProperties {
                os: "linux".into(),
                browser: "djs-rs".into(),
                device: "djs-rs".into(),
            },
            large_threshold: None,
            shard: Some(ShardId::ONE),
            intents: Intents::GUILDS | Intents::GUILD_MESSAGES,
            presence: None,
            capabilities: GatewayCapabilities::empty(),
        };
        let json = serde_json::to_string(&CommandPayload::new(&identify)).unwrap();
        assert_eq!(
            json,
            r#"{"op":2,"d":{"token":"secret","properties":{"os":"linux","browser":"djs-rs","device":"djs-rs"},"shard":[0,1],"intents":513}}"#
        );
        assert!(!format!("{identify:?}").contains("secret"));
    }

    #[test]
    fn resume_json() {
        let resume = Resume {
            token: "secret".into(),
            session_id: "abc".into(),
            seq: 1337,
        };
        let json = serde_json::to_string(&CommandPayload::new(&resume)).unwrap();
        assert_eq!(
            json,
            r#"{"op":6,"d":{"token":"secret","session_id":"abc","seq":1337}}"#
        );
        assert!(!format!("{resume:?}").contains("secret"));
    }

    #[test]
    fn first_heartbeat_is_null() {
        let json = serde_json::to_string(&CommandPayload::new(&Heartbeat(None))).unwrap();
        assert_eq!(json, r#"{"op":1,"d":null}"#);
    }
}
