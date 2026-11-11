//! Typed gateway events and their fast deserialization.

use core::fmt;

use compact_str::CompactString;
use serde::de::{DeserializeSeed, Deserializer, Error as DeError, IgnoredAny, MapAccess, Visitor};

use crate::gateway::{
    OpCode,
    payload::{self, Hello},
};

/// A payload received from the gateway, decoded according to its opcode.
///
/// Decode payloads with [`GatewayEventDeserializer`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum GatewayEvent {
    /// An event was dispatched (opcode 0), with its sequence number.
    Dispatch(u64, DispatchEvent),
    /// Discord requests an immediate heartbeat (opcode 1).
    Heartbeat,
    /// The client should reconnect and resume (opcode 7).
    Reconnect,
    /// The session was invalidated (opcode 9). The value says whether the session can be
    /// resumed.
    InvalidSession(bool),
    /// Sent right after connecting, with the heartbeat interval (opcode 10).
    Hello(Hello),
    /// A heartbeat was acknowledged (opcode 11).
    HeartbeatAck,
    /// A payload with an opcode this version of the library does not handle. Its data was
    /// skipped.
    Unknown(OpCode),
}

/// Defines [`DispatchEvent`] (variants, [`name`](DispatchEvent::name)) and the routing of
/// `t` to the payload types from one list, so they cannot drift apart.
macro_rules! dispatch_events {
    ($(#[doc = $doc:literal] $name:literal => $variant:ident($ty:ty),)*) => {
        /// The event carried by a [`GatewayEvent::Dispatch`]: every documented dispatch event,
        /// with its own payload type (D-073) from [`payload`](crate::gateway::payload).
        ///
        /// Large payloads are boxed to keep this enum small; small `Copy` payloads are inline.
        /// Events this version does not know are skipped efficiently (validated, not stored)
        /// and reported as [`Unknown`](Self::Unknown) with their name.
        #[derive(Clone, Debug, PartialEq)]
        #[non_exhaustive]
        pub enum DispatchEvent {
            $(#[doc = $doc] $variant($ty),)*
            /// `RESUMED`: a resume completed and all missed events were replayed.
            Resumed,
            /// An event this version of the library does not decode, with its name.
            Unknown(CompactString),
        }

        impl DispatchEvent {
            /// Returns the event name as sent by Discord, such as `"MESSAGE_CREATE"`.
            #[must_use]
            pub fn name(&self) -> &str {
                match self {
                    $(Self::$variant(_) => $name,)*
                    Self::Resumed => "RESUMED",
                    Self::Unknown(name) => name,
                }
            }

            /// Decodes the data of the event named `name` from the next map value.
            fn decode<'de, A: MapAccess<'de>>(name: &str, map: &mut A) -> Result<Self, A::Error> {
                Ok(match name {
                    $($name => Self::$variant(map.next_value()?),)*
                    "RESUMED" => {
                        map.next_value::<IgnoredAny>()?;
                        Self::Resumed
                    }
                    other => {
                        map.next_value::<IgnoredAny>()?;
                        Self::Unknown(CompactString::from(other))
                    }
                })
            }
        }

        /// Serializes the event's data (the `d` field of the payload): the payload type's own
        /// JSON shape; `null` for [`Resumed`](Self::Resumed) and [`Unknown`](Self::Unknown)
        /// events (whose data is not kept).
        impl serde::Serialize for DispatchEvent {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                match self {
                    $(Self::$variant(data) => data.serialize(serializer),)*
                    Self::Resumed | Self::Unknown(_) => serializer.serialize_unit(),
                }
            }
        }
    };
}

dispatch_events! {
    /// `READY`: the session was established.
    "READY" => Ready(Box<payload::Ready>),
    /// `RATE_LIMITED`: a gateway command was rate limited.
    "RATE_LIMITED" => RateLimited(Box<payload::RateLimited>),
    /// `APPLICATION_COMMAND_PERMISSIONS_UPDATE`: a command's permissions were updated.
    "APPLICATION_COMMAND_PERMISSIONS_UPDATE" => ApplicationCommandPermissionsUpdate(Box<payload::ApplicationCommandPermissionsUpdate>),
    /// `AUTO_MODERATION_RULE_CREATE`: an auto moderation rule was created.
    "AUTO_MODERATION_RULE_CREATE" => AutoModerationRuleCreate(Box<payload::AutoModerationRuleCreate>),
    /// `AUTO_MODERATION_RULE_UPDATE`: an auto moderation rule was updated.
    "AUTO_MODERATION_RULE_UPDATE" => AutoModerationRuleUpdate(Box<payload::AutoModerationRuleUpdate>),
    /// `AUTO_MODERATION_RULE_DELETE`: an auto moderation rule was deleted.
    "AUTO_MODERATION_RULE_DELETE" => AutoModerationRuleDelete(Box<payload::AutoModerationRuleDelete>),
    /// `AUTO_MODERATION_ACTION_EXECUTION`: an auto moderation action ran.
    "AUTO_MODERATION_ACTION_EXECUTION" => AutoModerationActionExecution(Box<payload::AutoModerationActionExecution>),
    /// `CHANNEL_CREATE`: a guild channel was created.
    "CHANNEL_CREATE" => ChannelCreate(Box<payload::ChannelCreate>),
    /// `CHANNEL_UPDATE`: a channel was updated.
    "CHANNEL_UPDATE" => ChannelUpdate(Box<payload::ChannelUpdate>),
    /// `CHANNEL_DELETE`: a channel was deleted.
    "CHANNEL_DELETE" => ChannelDelete(Box<payload::ChannelDelete>),
    /// `CHANNEL_INFO`: ephemeral channel data (answer to a channel info request).
    "CHANNEL_INFO" => ChannelInfo(Box<payload::ChannelInfo>),
    /// `CHANNEL_PINS_UPDATE`: a message was pinned or unpinned.
    "CHANNEL_PINS_UPDATE" => ChannelPinsUpdate(Box<payload::ChannelPinsUpdate>),
    /// `THREAD_CREATE`: a thread was created or the bot was added to one.
    "THREAD_CREATE" => ThreadCreate(Box<payload::ThreadCreate>),
    /// `THREAD_UPDATE`: a thread was updated.
    "THREAD_UPDATE" => ThreadUpdate(Box<payload::ThreadUpdate>),
    /// `THREAD_DELETE`: a thread was deleted.
    "THREAD_DELETE" => ThreadDelete(Box<payload::ThreadDelete>),
    /// `THREAD_LIST_SYNC`: the active threads of channels the bot gained access to.
    "THREAD_LIST_SYNC" => ThreadListSync(Box<payload::ThreadListSync>),
    /// `THREAD_MEMBER_UPDATE`: the bot's thread member was updated.
    "THREAD_MEMBER_UPDATE" => ThreadMemberUpdate(Box<payload::ThreadMemberUpdate>),
    /// `THREAD_MEMBERS_UPDATE`: users were added to or removed from a thread.
    "THREAD_MEMBERS_UPDATE" => ThreadMembersUpdate(Box<payload::ThreadMembersUpdate>),
    /// `ENTITLEMENT_CREATE`: an entitlement was created.
    "ENTITLEMENT_CREATE" => EntitlementCreate(Box<payload::EntitlementCreate>),
    /// `ENTITLEMENT_UPDATE`: an entitlement was updated or ended.
    "ENTITLEMENT_UPDATE" => EntitlementUpdate(Box<payload::EntitlementUpdate>),
    /// `ENTITLEMENT_DELETE`: an entitlement was deleted.
    "ENTITLEMENT_DELETE" => EntitlementDelete(Box<payload::EntitlementDelete>),
    /// `GUILD_CREATE`: a guild was loaded, became available or was joined.
    "GUILD_CREATE" => GuildCreate(Box<payload::GuildCreate>),
    /// `GUILD_UPDATE`: a guild was updated.
    "GUILD_UPDATE" => GuildUpdate(Box<payload::GuildUpdate>),
    /// `GUILD_DELETE`: a guild became unavailable or the bot left it.
    "GUILD_DELETE" => GuildDelete(payload::GuildDelete),
    /// `GUILD_AUDIT_LOG_ENTRY_CREATE`: an audit log entry was created.
    "GUILD_AUDIT_LOG_ENTRY_CREATE" => GuildAuditLogEntryCreate(Box<payload::GuildAuditLogEntryCreate>),
    /// `GUILD_BAN_ADD`: a user was banned.
    "GUILD_BAN_ADD" => GuildBanAdd(Box<payload::GuildBanAdd>),
    /// `GUILD_BAN_REMOVE`: a user was unbanned.
    "GUILD_BAN_REMOVE" => GuildBanRemove(Box<payload::GuildBanRemove>),
    /// `GUILD_EMOJIS_UPDATE`: a guild's emojis changed.
    "GUILD_EMOJIS_UPDATE" => GuildEmojisUpdate(Box<payload::GuildEmojisUpdate>),
    /// `GUILD_STICKERS_UPDATE`: a guild's stickers changed.
    "GUILD_STICKERS_UPDATE" => GuildStickersUpdate(Box<payload::GuildStickersUpdate>),
    /// `GUILD_INTEGRATIONS_UPDATE`: a guild's integrations changed.
    "GUILD_INTEGRATIONS_UPDATE" => GuildIntegrationsUpdate(payload::GuildIntegrationsUpdate),
    /// `GUILD_MEMBER_ADD`: a user joined a guild.
    "GUILD_MEMBER_ADD" => GuildMemberAdd(Box<payload::GuildMemberAdd>),
    /// `GUILD_MEMBER_REMOVE`: a user left or was removed from a guild.
    "GUILD_MEMBER_REMOVE" => GuildMemberRemove(Box<payload::GuildMemberRemove>),
    /// `GUILD_MEMBER_UPDATE`: a member was updated.
    "GUILD_MEMBER_UPDATE" => GuildMemberUpdate(Box<payload::GuildMemberUpdate>),
    /// `GUILD_MEMBERS_CHUNK`: members requested with a guild members request.
    "GUILD_MEMBERS_CHUNK" => GuildMembersChunk(Box<payload::GuildMembersChunk>),
    /// `GUILD_ROLE_CREATE`: a role was created.
    "GUILD_ROLE_CREATE" => GuildRoleCreate(Box<payload::GuildRoleCreate>),
    /// `GUILD_ROLE_UPDATE`: a role was updated.
    "GUILD_ROLE_UPDATE" => GuildRoleUpdate(Box<payload::GuildRoleUpdate>),
    /// `GUILD_ROLE_DELETE`: a role was deleted.
    "GUILD_ROLE_DELETE" => GuildRoleDelete(payload::GuildRoleDelete),
    /// `GUILD_SCHEDULED_EVENT_CREATE`: a scheduled event was created.
    "GUILD_SCHEDULED_EVENT_CREATE" => GuildScheduledEventCreate(Box<payload::GuildScheduledEventCreate>),
    /// `GUILD_SCHEDULED_EVENT_UPDATE`: a scheduled event was updated.
    "GUILD_SCHEDULED_EVENT_UPDATE" => GuildScheduledEventUpdate(Box<payload::GuildScheduledEventUpdate>),
    /// `GUILD_SCHEDULED_EVENT_DELETE`: a scheduled event was deleted.
    "GUILD_SCHEDULED_EVENT_DELETE" => GuildScheduledEventDelete(Box<payload::GuildScheduledEventDelete>),
    /// `GUILD_SCHEDULED_EVENT_USER_ADD`: a user subscribed to a scheduled event.
    "GUILD_SCHEDULED_EVENT_USER_ADD" => GuildScheduledEventUserAdd(payload::GuildScheduledEventUserAdd),
    /// `GUILD_SCHEDULED_EVENT_USER_REMOVE`: a user unsubscribed from a scheduled event.
    "GUILD_SCHEDULED_EVENT_USER_REMOVE" => GuildScheduledEventUserRemove(payload::GuildScheduledEventUserRemove),
    /// `GUILD_SOUNDBOARD_SOUND_CREATE`: a soundboard sound was created.
    "GUILD_SOUNDBOARD_SOUND_CREATE" => GuildSoundboardSoundCreate(Box<payload::GuildSoundboardSoundCreate>),
    /// `GUILD_SOUNDBOARD_SOUND_UPDATE`: a soundboard sound was updated.
    "GUILD_SOUNDBOARD_SOUND_UPDATE" => GuildSoundboardSoundUpdate(Box<payload::GuildSoundboardSoundUpdate>),
    /// `GUILD_SOUNDBOARD_SOUND_DELETE`: a soundboard sound was deleted.
    "GUILD_SOUNDBOARD_SOUND_DELETE" => GuildSoundboardSoundDelete(payload::GuildSoundboardSoundDelete),
    /// `GUILD_SOUNDBOARD_SOUNDS_UPDATE`: several soundboard sounds were updated.
    "GUILD_SOUNDBOARD_SOUNDS_UPDATE" => GuildSoundboardSoundsUpdate(Box<payload::GuildSoundboardSoundsUpdate>),
    /// `SOUNDBOARD_SOUNDS`: a guild's soundboard sounds (answer to a soundboard sounds request).
    "SOUNDBOARD_SOUNDS" => SoundboardSounds(Box<payload::SoundboardSounds>),
    /// `INTEGRATION_CREATE`: an integration was created.
    "INTEGRATION_CREATE" => IntegrationCreate(Box<payload::IntegrationCreate>),
    /// `INTEGRATION_UPDATE`: an integration was updated.
    "INTEGRATION_UPDATE" => IntegrationUpdate(Box<payload::IntegrationUpdate>),
    /// `INTEGRATION_DELETE`: an integration was deleted.
    "INTEGRATION_DELETE" => IntegrationDelete(payload::IntegrationDelete),
    /// `INTERACTION_CREATE`: a user used a command, component or modal.
    "INTERACTION_CREATE" => InteractionCreate(Box<payload::InteractionCreate>),
    /// `INVITE_CREATE`: an invite was created.
    "INVITE_CREATE" => InviteCreate(Box<payload::InviteCreate>),
    /// `INVITE_DELETE`: an invite was deleted.
    "INVITE_DELETE" => InviteDelete(Box<payload::InviteDelete>),
    /// `MESSAGE_CREATE`: a message was sent.
    "MESSAGE_CREATE" => MessageCreate(Box<payload::MessageCreate>),
    /// `MESSAGE_UPDATE`: a message was edited.
    "MESSAGE_UPDATE" => MessageUpdate(Box<payload::MessageUpdate>),
    /// `MESSAGE_DELETE`: a message was deleted.
    "MESSAGE_DELETE" => MessageDelete(Box<payload::MessageDelete>),
    /// `MESSAGE_DELETE_BULK`: several messages were deleted.
    "MESSAGE_DELETE_BULK" => MessageDeleteBulk(Box<payload::MessageDeleteBulk>),
    /// `MESSAGE_REACTION_ADD`: a user reacted to a message.
    "MESSAGE_REACTION_ADD" => MessageReactionAdd(Box<payload::MessageReactionAdd>),
    /// `MESSAGE_REACTION_REMOVE`: a reaction was removed.
    "MESSAGE_REACTION_REMOVE" => MessageReactionRemove(Box<payload::MessageReactionRemove>),
    /// `MESSAGE_REACTION_REMOVE_ALL`: all reactions were removed from a message.
    "MESSAGE_REACTION_REMOVE_ALL" => MessageReactionRemoveAll(payload::MessageReactionRemoveAll),
    /// `MESSAGE_REACTION_REMOVE_EMOJI`: all reactions with one emoji were removed.
    "MESSAGE_REACTION_REMOVE_EMOJI" => MessageReactionRemoveEmoji(Box<payload::MessageReactionRemoveEmoji>),
    /// `MESSAGE_POLL_VOTE_ADD`: a user voted on a poll.
    "MESSAGE_POLL_VOTE_ADD" => MessagePollVoteAdd(Box<payload::MessagePollVoteAdd>),
    /// `MESSAGE_POLL_VOTE_REMOVE`: a user removed a poll vote.
    "MESSAGE_POLL_VOTE_REMOVE" => MessagePollVoteRemove(Box<payload::MessagePollVoteRemove>),
    /// `PRESENCE_UPDATE`: a user's presence or user data changed.
    "PRESENCE_UPDATE" => PresenceUpdate(Box<payload::PresenceUpdate>),
    /// `STAGE_INSTANCE_CREATE`: a stage went live.
    "STAGE_INSTANCE_CREATE" => StageInstanceCreate(Box<payload::StageInstanceCreate>),
    /// `STAGE_INSTANCE_UPDATE`: a stage instance was updated.
    "STAGE_INSTANCE_UPDATE" => StageInstanceUpdate(Box<payload::StageInstanceUpdate>),
    /// `STAGE_INSTANCE_DELETE`: a stage ended.
    "STAGE_INSTANCE_DELETE" => StageInstanceDelete(Box<payload::StageInstanceDelete>),
    /// `SUBSCRIPTION_CREATE`: an app subscription was created.
    "SUBSCRIPTION_CREATE" => SubscriptionCreate(Box<payload::SubscriptionCreate>),
    /// `SUBSCRIPTION_UPDATE`: an app subscription was updated.
    "SUBSCRIPTION_UPDATE" => SubscriptionUpdate(Box<payload::SubscriptionUpdate>),
    /// `SUBSCRIPTION_DELETE`: an app subscription was deleted.
    "SUBSCRIPTION_DELETE" => SubscriptionDelete(Box<payload::SubscriptionDelete>),
    /// `TYPING_START`: a user started typing.
    "TYPING_START" => TypingStart(Box<payload::TypingStart>),
    /// `USER_UPDATE`: the bot's user changed.
    "USER_UPDATE" => UserUpdate(Box<payload::UserUpdate>),
    /// `VOICE_CHANNEL_EFFECT_SEND`: someone sent an effect in the bot's voice channel.
    "VOICE_CHANNEL_EFFECT_SEND" => VoiceChannelEffectSend(Box<payload::VoiceChannelEffectSend>),
    /// `VOICE_CHANNEL_START_TIME_UPDATE`: a voice channel's start time changed.
    "VOICE_CHANNEL_START_TIME_UPDATE" => VoiceChannelStartTimeUpdate(Box<payload::VoiceChannelStartTimeUpdate>),
    /// `VOICE_CHANNEL_STATUS_UPDATE`: a voice channel's status changed.
    "VOICE_CHANNEL_STATUS_UPDATE" => VoiceChannelStatusUpdate(Box<payload::VoiceChannelStatusUpdate>),
    /// `VOICE_STATE_UPDATE`: someone joined, left or moved between voice channels.
    "VOICE_STATE_UPDATE" => VoiceStateUpdate(Box<payload::VoiceStateUpdate>),
    /// `VOICE_SERVER_UPDATE`: where to connect for voice.
    "VOICE_SERVER_UPDATE" => VoiceServerUpdate(Box<payload::VoiceServerUpdate>),
    /// `WEBHOOKS_UPDATE`: a channel's webhooks changed.
    "WEBHOOKS_UPDATE" => WebhooksUpdate(payload::WebhooksUpdate),
}

/// A [`DeserializeSeed`] that decodes a gateway payload into a [`GatewayEvent`].
///
/// Gateway payloads are envelopes — `{"op": 0, "s": 42, "t": "MESSAGE_CREATE", "d": {…}}` — whose
/// `d` type depends on `op` and `t`. Deserializing them naively requires buffering `d` in a
/// generic value first. Instead, this type is constructed with `op`, `s` and `t` known up front
/// (usually via [`from_json`](Self::from_json), which scans only the top-level keys), so `d` is
/// decoded in a single pass directly into its concrete type, and unhandled events are skipped
/// without allocating.
///
/// It works with any serde data format; [`from_json`](Self::from_json) is a JSON-specific
/// helper.
///
/// # Examples
///
/// ```
/// use djs_model::gateway::{GatewayEvent, GatewayEventDeserializer, OpCode};
/// use serde::de::DeserializeSeed;
///
/// let json = r#"{"t":null,"s":null,"op":10,"d":{"heartbeat_interval":41250}}"#;
///
/// let seed = GatewayEventDeserializer::from_json(json).unwrap();
/// assert_eq!(seed.op(), OpCode::Hello);
///
/// let mut deserializer = serde_json::Deserializer::from_str(json);
/// let event = seed.deserialize(&mut deserializer).unwrap();
/// assert!(matches!(event, GatewayEvent::Hello(hello) if hello.heartbeat_interval == 41250));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GatewayEventDeserializer<'a> {
    op: OpCode,
    sequence: Option<u64>,
    event_type: Option<&'a str>,
}

impl<'a> GatewayEventDeserializer<'a> {
    /// Creates a deserializer from an already known opcode, sequence number and event name.
    ///
    /// The `sequence` is used when the payload being deserialized has no `s` field; an `s` field
    /// in the payload takes precedence.
    #[must_use]
    pub const fn new(op: OpCode, sequence: Option<u64>, event_type: Option<&'a str>) -> Self {
        Self {
            op,
            sequence,
            event_type,
        }
    }

    /// Reads `op`, `s` and `t` from the top level of a JSON gateway payload.
    ///
    /// Only the top-level object is scanned; nested values are skipped without being parsed.
    /// Discord sends `t`, `s` and `op` before `d`, in which case scanning stops before reaching
    /// `d` at all. Keys in any order are still handled correctly.
    ///
    /// Returns `None` if the input is not a JSON object with an integer `op` field, or if `t`
    /// contains escape sequences (event names never do).
    #[must_use]
    pub fn from_json(json: &'a str) -> Option<Self> {
        scan::top_level(json)
    }

    /// Returns the opcode.
    #[must_use]
    pub const fn op(&self) -> OpCode {
        self.op
    }

    /// Returns the sequence number, if known.
    #[must_use]
    pub const fn sequence(&self) -> Option<u64> {
        self.sequence
    }

    /// Returns the dispatch event name, if any.
    #[must_use]
    pub const fn event_type(&self) -> Option<&'a str> {
        self.event_type
    }
}

impl<'de> DeserializeSeed<'de> for GatewayEventDeserializer<'_> {
    type Value = GatewayEvent;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<GatewayEvent, D::Error> {
        deserializer.deserialize_map(EnvelopeVisitor(self))
    }
}

struct EnvelopeVisitor<'a>(GatewayEventDeserializer<'a>);

/// What was decoded from the `d` field.
enum Data {
    Dispatch(DispatchEvent),
    Event(GatewayEvent),
}

impl EnvelopeVisitor<'_> {
    fn data<'de, A: MapAccess<'de>>(&self, map: &mut A) -> Result<Data, A::Error> {
        let event = match self.0.op {
            OpCode::Dispatch => {
                let name = self.0.event_type.ok_or_else(|| {
                    A::Error::custom("dispatch payload without an event name (`t`)")
                })?;
                return Ok(Data::Dispatch(DispatchEvent::decode(name, map)?));
            }
            OpCode::InvalidSession => GatewayEvent::InvalidSession(map.next_value()?),
            OpCode::Hello => GatewayEvent::Hello(map.next_value()?),
            op => {
                map.next_value::<IgnoredAny>()?;
                Self::without_data(op)
            }
        };
        Ok(Data::Event(event))
    }

    fn without_data(op: OpCode) -> GatewayEvent {
        match op {
            OpCode::Heartbeat => GatewayEvent::Heartbeat,
            OpCode::Reconnect => GatewayEvent::Reconnect,
            OpCode::HeartbeatAck => GatewayEvent::HeartbeatAck,
            op => GatewayEvent::Unknown(op),
        }
    }
}

impl<'de> Visitor<'de> for EnvelopeVisitor<'_> {
    type Value = GatewayEvent;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a gateway payload object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<GatewayEvent, A::Error> {
        let mut data = None;
        let mut sequence = None;

        while let Some(field) = map.next_key::<Field>()? {
            match field {
                Field::Data => {
                    if data.is_some() {
                        return Err(A::Error::duplicate_field("d"));
                    }
                    data = Some(self.data(&mut map)?);
                }
                Field::Sequence => sequence = map.next_value::<Option<u64>>()?,
                Field::Other => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        match (self.0.op, data) {
            (OpCode::Dispatch, Some(Data::Dispatch(event))) => {
                let sequence = sequence
                    .or(self.0.sequence)
                    .ok_or_else(|| A::Error::missing_field("s"))?;
                Ok(GatewayEvent::Dispatch(sequence, event))
            }
            (_, Some(Data::Event(event))) => Ok(event),
            (OpCode::Dispatch | OpCode::InvalidSession | OpCode::Hello, _) => {
                Err(A::Error::missing_field("d"))
            }
            (op, _) => Ok(Self::without_data(op)),
        }
    }
}

/// The envelope keys the visitor cares about.
enum Field {
    Data,
    Sequence,
    Other,
}

impl<'de> serde::Deserialize<'de> for Field {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FieldVisitor;

        impl Visitor<'_> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a gateway payload key")
            }

            fn visit_str<E: DeError>(self, value: &str) -> Result<Field, E> {
                Ok(match value {
                    "d" => Field::Data,
                    "s" => Field::Sequence,
                    _ => Field::Other,
                })
            }
        }

        deserializer.deserialize_identifier(FieldVisitor)
    }
}

/// A minimal scanner for the top level of a JSON object.
mod scan {
    use super::GatewayEventDeserializer;
    use crate::gateway::OpCode;

    struct Cursor<'a> {
        bytes: &'a [u8],
        at: usize,
    }

    impl<'a> Cursor<'a> {
        fn peek(&self) -> Option<u8> {
            self.bytes.get(self.at).copied()
        }

        fn skip_whitespace(&mut self) {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.at += 1;
            }
        }

        fn eat(&mut self, byte: u8) -> Option<()> {
            self.skip_whitespace();
            (self.peek()? == byte).then(|| self.at += 1)
        }

        /// Reads a string starting at the opening quote. Returns the raw contents (without
        /// quotes) and whether they contain escape sequences.
        fn string(&mut self) -> Option<(&'a [u8], bool)> {
            self.eat(b'"')?;
            let start = self.at;
            let mut escaped = false;
            loop {
                match *self.bytes.get(self.at)? {
                    b'"' => break,
                    b'\\' => {
                        escaped = true;
                        self.at += 2;
                    }
                    _ => self.at += 1,
                }
            }
            let contents = self.bytes.get(start..self.at)?;
            self.at += 1;
            Some((contents, escaped))
        }

        fn literal(&mut self, literal: &[u8]) -> bool {
            self.skip_whitespace();
            let matches = self
                .bytes
                .get(self.at..)
                .is_some_and(|rest| rest.starts_with(literal));
            if matches {
                self.at += literal.len();
            }
            matches
        }

        fn unsigned(&mut self) -> Option<u64> {
            self.skip_whitespace();
            let start = self.at;
            let mut value = 0_u64;
            while let Some(digit @ b'0'..=b'9') = self.peek() {
                value = value
                    .checked_mul(10)?
                    .checked_add(u64::from(digit - b'0'))?;
                self.at += 1;
            }
            (self.at > start).then_some(value)
        }

        /// Skips any JSON value without interpreting it.
        fn skip_value(&mut self) -> Option<()> {
            self.skip_whitespace();
            match self.peek()? {
                b'"' => {
                    self.string()?;
                }
                b'{' | b'[' => {
                    let mut depth = 0_usize;
                    loop {
                        match *self.bytes.get(self.at)? {
                            b'{' | b'[' => {
                                depth += 1;
                                self.at += 1;
                            }
                            b'}' | b']' => {
                                depth -= 1;
                                self.at += 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            b'"' => {
                                self.string()?;
                            }
                            _ => self.at += 1,
                        }
                    }
                }
                _ => {
                    // Number, `true`, `false` or `null`.
                    while !matches!(
                        self.peek()?,
                        b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r'
                    ) {
                        self.at += 1;
                    }
                }
            }
            Some(())
        }
    }

    pub(super) fn top_level(json: &str) -> Option<GatewayEventDeserializer<'_>> {
        let mut cursor = Cursor {
            bytes: json.as_bytes(),
            at: 0,
        };
        let mut op = None;
        let mut sequence = None;
        let mut event_type = None;
        let (mut seen_sequence, mut seen_event_type) = (false, false);

        cursor.eat(b'{')?;
        cursor.skip_whitespace();
        if cursor.peek() != Some(b'}') {
            loop {
                let (key, _) = cursor.string()?;
                cursor.eat(b':')?;
                match key {
                    b"op" => {
                        op = Some(u8::try_from(cursor.unsigned()?).ok()?);
                    }
                    b"s" => {
                        seen_sequence = true;
                        if !cursor.literal(b"null") {
                            sequence = Some(cursor.unsigned()?);
                        }
                    }
                    b"t" => {
                        seen_event_type = true;
                        if !cursor.literal(b"null") {
                            let (name, escaped) = cursor.string()?;
                            if escaped {
                                return None;
                            }
                            event_type = Some(core::str::from_utf8(name).ok()?);
                        }
                    }
                    _ => cursor.skip_value()?,
                }

                // Stop as soon as everything needed is known.
                if let Some(op) = op
                    && (op != OpCode::Dispatch.value() || (seen_sequence && seen_event_type))
                {
                    break;
                }

                cursor.skip_whitespace();
                match cursor.peek()? {
                    b',' => cursor.at += 1,
                    b'}' => break,
                    _ => return None,
                }
            }
        }

        Some(GatewayEventDeserializer::new(
            OpCode::from_value(op?),
            sequence,
            event_type,
        ))
    }
}

#[cfg(test)]
mod tests {
    use serde::de::DeserializeSeed;

    use super::*;

    fn decode(json: &str) -> GatewayEvent {
        let seed = GatewayEventDeserializer::from_json(json).expect("scan failed");
        let mut deserializer = serde_json::Deserializer::from_str(json);
        let event = seed
            .deserialize(&mut deserializer)
            .expect("deserialize failed");
        deserializer.end().expect("trailing data");
        event
    }

    #[test]
    fn scan_discord_order() {
        let seed = GatewayEventDeserializer::from_json(
            r#"{"t":"MESSAGE_CREATE","s":3,"op":0,"d":{"never":"read"}}"#,
        )
        .unwrap();
        assert_eq!(seed.op(), OpCode::Dispatch);
        assert_eq!(seed.sequence(), Some(3));
        assert_eq!(seed.event_type(), Some("MESSAGE_CREATE"));
    }

    #[test]
    fn scan_any_order_and_nested_decoys() {
        let json = r#"{ "d" : {"op": 9, "t": "DECOY", "s": 1, "x": [{"op":1}, "\"op\":2"]},
                        "op" : 0 , "s" : 77, "t" : "READY" }"#;
        let seed = GatewayEventDeserializer::from_json(json).unwrap();
        assert_eq!(seed.op(), OpCode::Dispatch);
        assert_eq!(seed.sequence(), Some(77));
        assert_eq!(seed.event_type(), Some("READY"));
    }

    #[test]
    fn scan_rejects_garbage() {
        for json in [
            "",
            "[]",
            "{}",
            r#"{"op":"1"}"#,
            r#"{"op":300}"#,
            "{\"t\":\"A\\u0042\",\"op\":0}",
        ] {
            assert!(
                GatewayEventDeserializer::from_json(json).is_none(),
                "{json}"
            );
        }
    }

    #[test]
    fn hello_and_control_events() {
        assert_eq!(
            decode(r#"{"t":null,"s":null,"op":10,"d":{"heartbeat_interval":45000}}"#),
            GatewayEvent::Hello(Hello {
                heartbeat_interval: 45000
            })
        );
        assert_eq!(decode(r#"{"op":11}"#), GatewayEvent::HeartbeatAck);
        assert_eq!(decode(r#"{"op":1,"d":null}"#), GatewayEvent::Heartbeat);
        assert_eq!(
            decode(r#"{"t":null,"s":null,"op":7,"d":null}"#),
            GatewayEvent::Reconnect
        );
        assert_eq!(
            decode(r#"{"op":9,"d":false}"#),
            GatewayEvent::InvalidSession(false)
        );
        assert_eq!(
            decode(r#"{"op":42,"d":{"x":1}}"#),
            GatewayEvent::Unknown(OpCode::Unknown(42))
        );
    }

    #[test]
    fn unknown_dispatch_is_skipped() {
        let event = decode(r#"{"t":"FUTURE_EVENT","s":2,"op":0,"d":{"id":"1","big":[1,2,3]}}"#);
        assert_eq!(
            event,
            GatewayEvent::Dispatch(2, DispatchEvent::Unknown("FUTURE_EVENT".into()))
        );
    }

    #[test]
    fn resumed() {
        let event = decode(r#"{"t":"RESUMED","s":10,"op":0,"d":{}}"#);
        assert_eq!(event, GatewayEvent::Dispatch(10, DispatchEvent::Resumed));
    }

    #[test]
    fn dispatch_without_sequence_fails() {
        let json = r#"{"t":"RESUMED","op":0,"d":{}}"#;
        let seed = GatewayEventDeserializer::from_json(json).unwrap();
        let mut deserializer = serde_json::Deserializer::from_str(json);
        assert!(seed.deserialize(&mut deserializer).is_err());
    }
}
