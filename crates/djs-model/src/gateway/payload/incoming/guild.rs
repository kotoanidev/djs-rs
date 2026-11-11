//! Guild, member, role, scheduled event, soundboard, integration, auto moderation and command
//! permission events.

use compact_str::CompactString;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as DeError, MapAccess},
};

use super::{guild_id_event, newtype_event};
use crate::{
    application_command::GuildCommandPermissions,
    audit_log::AuditLogEntry,
    auto_moderation::{Action, Rule, TriggerType},
    channel::Channel,
    emoji::Emoji,
    gateway::Presence,
    guild::{Guild, Integration, Member, MemberFlags, Role, UnavailableGuild},
    guild_scheduled_event::GuildScheduledEvent,
    id::{
        ApplicationId, AutoModerationRuleId, ChannelId, GuildId, IntegrationId, MessageId, RoleId,
        ScheduledEventId, SoundboardSoundId, UserId,
    },
    serde_util::extras::{self, Extras, Visit},
    soundboard::SoundboardSound,
    stage_instance::StageInstance,
    sticker::Sticker,
    user::{AvatarDecorationData, Collectibles, User},
    util::{ImageHash, Timestamp},
    voice::VoiceState,
};

/// The data of the [Guild Create](https://docs.discord.com/developers/events/gateway-events#guild-create)
/// dispatch event: a guild from [`Ready`](super::Ready) was loaded, became available again, or
/// the bot joined a new guild.
///
/// discord.js counterpart: the `GuildCreate` event (and the guilds becoming available).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum GuildCreate {
    /// The guild with its members, channels, presences, ….
    Available(Box<AvailableGuild>),
    /// The guild is unavailable due to an outage.
    Unavailable(UnavailableGuild),
}

impl GuildCreate {
    /// Returns the guild's ID.
    #[must_use]
    pub const fn id(&self) -> GuildId {
        match self {
            Self::Available(guild) => guild.guild.id,
            Self::Unavailable(guild) => guild.id,
        }
    }
}

/// An available guild in a [`GuildCreate`]: the [`Guild`] plus the state the gateway sends with
/// it (D-073).
///
/// Without the `GUILD_PRESENCES` intent, or in guilds with over 75 000 members, `members` and
/// `presences` only hold the bot and users in voice channels.
///
/// Dereferences to the contained [`Guild`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct AvailableGuild {
    /// The guild.
    pub guild: Guild,
    /// When the bot joined the guild (always sent by Discord).
    pub joined_at: Option<Timestamp>,
    /// Whether the guild counts as large (offline members are not sent).
    pub large: bool,
    /// The total number of members.
    pub member_count: u32,
    /// The voice states of members in voice channels (without `guild_id`).
    pub voice_states: Vec<VoiceState>,
    /// The members sent with the guild.
    pub members: Vec<Member>,
    /// The channels.
    pub channels: Vec<Channel>,
    /// The active threads the bot can see.
    pub threads: Vec<Channel>,
    /// The presences of the members sent (without `guild_id`).
    pub presences: Vec<Presence>,
    /// The stage instances.
    pub stage_instances: Vec<StageInstance>,
    /// The scheduled events.
    pub guild_scheduled_events: Vec<GuildScheduledEvent>,
    /// The soundboard sounds.
    pub soundboard_sounds: Vec<SoundboardSound>,
}

impl core::ops::Deref for AvailableGuild {
    type Target = Guild;

    fn deref(&self) -> &Guild {
        &self.guild
    }
}

impl core::ops::DerefMut for AvailableGuild {
    fn deref_mut(&mut self) -> &mut Guild {
        &mut self.guild
    }
}

/// The `GUILD_CREATE` extras, plus the `id` and `unavailable` needed to recognize an unavailable
/// guild (whose missing fields make the [`Guild`] fail).
#[derive(Default)]
struct GuildCreateExtra {
    id: Option<GuildId>,
    unavailable: Option<bool>,
    joined_at: Option<Timestamp>,
    large: bool,
    member_count: u32,
    voice_states: Vec<VoiceState>,
    members: Vec<Member>,
    channels: Vec<Channel>,
    threads: Vec<Channel>,
    presences: Vec<Presence>,
    stage_instances: Vec<StageInstance>,
    guild_scheduled_events: Vec<GuildScheduledEvent>,
    soundboard_sounds: Vec<SoundboardSound>,
}

impl<'de> Extras<'de> for GuildCreateExtra {
    fn visit<A: MapAccess<'de>>(&mut self, key: &str, map: &mut A) -> Result<Visit, A::Error> {
        match key {
            "id" => {
                let id: GuildId = map.next_value()?;
                self.id = Some(id);
                return Ok(Visit::Replay(id.get()));
            }
            "unavailable" => self.unavailable = map.next_value()?,
            "joined_at" => self.joined_at = map.next_value()?,
            "large" => self.large = map.next_value()?,
            "member_count" => self.member_count = map.next_value()?,
            "voice_states" => self.voice_states = map.next_value()?,
            "members" => self.members = map.next_value()?,
            "channels" => self.channels = map.next_value()?,
            "threads" => self.threads = map.next_value()?,
            "presences" => self.presences = map.next_value()?,
            "stage_instances" => self.stage_instances = map.next_value()?,
            "guild_scheduled_events" => self.guild_scheduled_events = map.next_value()?,
            "soundboard_sounds" => self.soundboard_sounds = map.next_value()?,
            _ => return Ok(Visit::Pass),
        }
        Ok(Visit::Consumed)
    }

    fn recover(&self) -> bool {
        self.unavailable == Some(true) && self.id.is_some()
    }
}

impl<'de> Deserialize<'de> for GuildCreate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (guild, extra) = extras::deserialize::<Guild, GuildCreateExtra, D>(deserializer)?;
        let Some(guild) = guild else {
            let id = extra.id.ok_or_else(|| D::Error::missing_field("id"))?;
            return Ok(Self::Unavailable(UnavailableGuild::new(id, true)));
        };
        Ok(Self::Available(Box::new(AvailableGuild {
            guild,
            joined_at: extra.joined_at,
            large: extra.large,
            member_count: extra.member_count,
            voice_states: extra.voice_states,
            members: extra.members,
            channels: extra.channels,
            threads: extra.threads,
            presences: extra.presences,
            stage_instances: extra.stage_instances,
            guild_scheduled_events: extra.guild_scheduled_events,
            soundboard_sounds: extra.soundboard_sounds,
        })))
    }
}

impl Serialize for GuildCreate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Flat<'a> {
            #[serde(flatten)]
            guild: &'a Guild,
            #[serde(skip_serializing_if = "Option::is_none")]
            joined_at: Option<Timestamp>,
            large: bool,
            member_count: u32,
            voice_states: &'a [VoiceState],
            members: &'a [Member],
            channels: &'a [Channel],
            threads: &'a [Channel],
            presences: &'a [Presence],
            stage_instances: &'a [StageInstance],
            guild_scheduled_events: &'a [GuildScheduledEvent],
            soundboard_sounds: &'a [SoundboardSound],
        }

        match self {
            Self::Available(guild) => Flat {
                guild: &guild.guild,
                joined_at: guild.joined_at,
                large: guild.large,
                member_count: guild.member_count,
                voice_states: &guild.voice_states,
                members: &guild.members,
                channels: &guild.channels,
                threads: &guild.threads,
                presences: &guild.presences,
                stage_instances: &guild.stage_instances,
                guild_scheduled_events: &guild.guild_scheduled_events,
                soundboard_sounds: &guild.soundboard_sounds,
            }
            .serialize(serializer),
            Self::Unavailable(guild) => guild.serialize(serializer),
        }
    }
}

newtype_event! {
    /// The data of the [Guild Update](https://docs.discord.com/developers/events/gateway-events#guild-update)
    /// dispatch event.
    ///
    /// discord.js counterpart: the `GuildUpdate` event.
    GuildUpdate(Guild)
}

newtype_event! {
    /// The data of the [Guild Delete](https://docs.discord.com/developers/events/gateway-events#guild-delete)
    /// dispatch event: the guild became unavailable ([`unavailable`](UnavailableGuild::unavailable)
    /// is `true`) or the bot left or was removed (`false`).
    ///
    /// discord.js counterpart: the `GuildDelete` / `GuildUnavailable` events.
    GuildDelete(UnavailableGuild)
}

newtype_event! {
    /// The data of the [Guild Audit Log Entry Create](https://docs.discord.com/developers/events/gateway-events#guild-audit-log-entry-create)
    /// dispatch event (requires `VIEW_AUDIT_LOG`); the entry's `guild_id` is set.
    ///
    /// discord.js counterpart: the `GuildAuditLogEntryCreate` event.
    GuildAuditLogEntryCreate(AuditLogEntry)
}

/// The data of the [Guild Ban Add](https://docs.discord.com/developers/events/gateway-events#guild-ban-add)
/// dispatch event (requires `BAN_MEMBERS` or `VIEW_AUDIT_LOG`).
///
/// discord.js counterpart: the `GuildBanAdd` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildBanAdd {
    /// The guild.
    pub guild_id: GuildId,
    /// The banned user.
    pub user: User,
}

/// The data of the [Guild Ban Remove](https://docs.discord.com/developers/events/gateway-events#guild-ban-remove)
/// dispatch event (requires `BAN_MEMBERS` or `VIEW_AUDIT_LOG`).
///
/// discord.js counterpart: the `GuildBanRemove` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildBanRemove {
    /// The guild.
    pub guild_id: GuildId,
    /// The unbanned user.
    pub user: User,
}

/// The data of the [Guild Emojis Update](https://docs.discord.com/developers/events/gateway-events#guild-emojis-update)
/// dispatch event: the guild's full emoji list.
///
/// discord.js counterpart: the `GuildEmojiCreate` / `Update` / `Delete` events (derived from it).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildEmojisUpdate {
    /// The guild.
    pub guild_id: GuildId,
    /// The guild's emojis.
    pub emojis: Vec<Emoji>,
}

/// The data of the [Guild Stickers Update](https://docs.discord.com/developers/events/gateway-events#guild-stickers-update)
/// dispatch event: the guild's full sticker list.
///
/// discord.js counterpart: the `GuildStickerCreate` / `Update` / `Delete` events (derived from
/// it).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildStickersUpdate {
    /// The guild.
    pub guild_id: GuildId,
    /// The guild's stickers.
    pub stickers: Vec<Sticker>,
}

/// The data of the [Guild Integrations Update](https://docs.discord.com/developers/events/gateway-events#guild-integrations-update)
/// dispatch event.
///
/// discord.js counterpart: the `GuildIntegrationsUpdate` event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildIntegrationsUpdate {
    /// The guild.
    pub guild_id: GuildId,
}

guild_id_event! {
    /// The data of the [Guild Member Add](https://docs.discord.com/developers/events/gateway-events#guild-member-add)
    /// dispatch event (requires the `GUILD_MEMBERS` intent; may repeat for existing members).
    ///
    /// discord.js counterpart: the `GuildMemberAdd` event.
    GuildMemberAdd {
        /// The member.
        member: Member
    }
}

/// The data of the [Guild Member Remove](https://docs.discord.com/developers/events/gateway-events#guild-member-remove)
/// dispatch event: a user left, was kicked or banned (requires the `GUILD_MEMBERS` intent).
///
/// discord.js counterpart: the `GuildMemberRemove` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildMemberRemove {
    /// The guild.
    pub guild_id: GuildId,
    /// The removed user.
    pub user: User,
}

/// The data of the [Guild Member Update](https://docs.discord.com/developers/events/gateway-events#guild-member-update)
/// dispatch event: a member or their user changed (requires the `GUILD_MEMBERS` intent).
///
/// Optional fields that were not sent are `None`.
///
/// discord.js counterpart: the `GuildMemberUpdate` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildMemberUpdate {
    /// The guild.
    pub guild_id: GuildId,
    /// The member's roles.
    pub roles: Vec<RoleId>,
    /// The member's user.
    pub user: User,
    /// The nickname.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nick: Option<CompactString>,
    /// The guild avatar hash.
    #[serde(default)]
    pub avatar: Option<ImageHash>,
    /// The guild banner hash.
    #[serde(default)]
    pub banner: Option<ImageHash>,
    /// When the user joined (`None` for some guest members).
    #[serde(default)]
    pub joined_at: Option<Timestamp>,
    /// When the member started boosting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium_since: Option<Timestamp>,
    /// Whether the member is server-deafened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deaf: Option<bool>,
    /// Whether the member is server-muted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mute: Option<bool>,
    /// Whether the member has not passed membership screening yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
    /// When the member's timeout ends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub communication_disabled_until: Option<Timestamp>,
    /// The member's flags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<MemberFlags>,
    /// The guild avatar decoration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_decoration_data: Option<Box<AvatarDecorationData>>,
    /// The guild collectibles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collectibles: Option<Box<Collectibles>>,
}

/// The data of the [Guild Members Chunk](https://docs.discord.com/developers/events/gateway-events#guild-members-chunk)
/// dispatch event: one part of the answer to a
/// [`RequestGuildMembers`](crate::gateway::payload::RequestGuildMembers) command.
///
/// discord.js counterpart: the `GuildMembersChunk` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildMembersChunk {
    /// The guild.
    pub guild_id: GuildId,
    /// The members in this chunk.
    pub members: Vec<Member>,
    /// This chunk's index (`0 ≤ chunk_index < chunk_count`).
    pub chunk_index: u32,
    /// The number of chunks of the answer.
    pub chunk_count: u32,
    /// The requested IDs that are not members.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_found: Vec<UserId>,
    /// The members' presences (if requested).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub presences: Vec<Presence>,
    /// The request's nonce.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<CompactString>,
}

/// The data of the [Guild Role Create](https://docs.discord.com/developers/events/gateway-events#guild-role-create)
/// dispatch event.
///
/// discord.js counterpart: the `GuildRoleCreate` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildRoleCreate {
    /// The guild.
    pub guild_id: GuildId,
    /// The created role.
    pub role: Role,
}

/// The data of the [Guild Role Update](https://docs.discord.com/developers/events/gateway-events#guild-role-update)
/// dispatch event.
///
/// discord.js counterpart: the `GuildRoleUpdate` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildRoleUpdate {
    /// The guild.
    pub guild_id: GuildId,
    /// The updated role.
    pub role: Role,
}

/// The data of the [Guild Role Delete](https://docs.discord.com/developers/events/gateway-events#guild-role-delete)
/// dispatch event.
///
/// discord.js counterpart: the `GuildRoleDelete` event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildRoleDelete {
    /// The guild.
    pub guild_id: GuildId,
    /// The deleted role.
    pub role_id: RoleId,
}

newtype_event! {
    /// The data of the [Guild Scheduled Event Create](https://docs.discord.com/developers/events/gateway-events#guild-scheduled-event-create)
    /// dispatch event.
    ///
    /// discord.js counterpart: the `GuildScheduledEventCreate` event.
    GuildScheduledEventCreate(GuildScheduledEvent)
}

newtype_event! {
    /// The data of the [Guild Scheduled Event Update](https://docs.discord.com/developers/events/gateway-events#guild-scheduled-event-update)
    /// dispatch event.
    ///
    /// discord.js counterpart: the `GuildScheduledEventUpdate` event.
    GuildScheduledEventUpdate(GuildScheduledEvent)
}

newtype_event! {
    /// The data of the [Guild Scheduled Event Delete](https://docs.discord.com/developers/events/gateway-events#guild-scheduled-event-delete)
    /// dispatch event.
    ///
    /// discord.js counterpart: the `GuildScheduledEventDelete` event.
    GuildScheduledEventDelete(GuildScheduledEvent)
}

/// The data of the [Guild Scheduled Event User Add](https://docs.discord.com/developers/events/gateway-events#guild-scheduled-event-user-add)
/// dispatch event: a user subscribed to an event.
///
/// discord.js counterpart: the `GuildScheduledEventUserAdd` event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildScheduledEventUserAdd {
    /// The scheduled event.
    pub guild_scheduled_event_id: ScheduledEventId,
    /// The user.
    pub user_id: UserId,
    /// The guild.
    pub guild_id: GuildId,
}

/// The data of the [Guild Scheduled Event User Remove](https://docs.discord.com/developers/events/gateway-events#guild-scheduled-event-user-remove)
/// dispatch event: a user unsubscribed from an event.
///
/// discord.js counterpart: the `GuildScheduledEventUserRemove` event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildScheduledEventUserRemove {
    /// The scheduled event.
    pub guild_scheduled_event_id: ScheduledEventId,
    /// The user.
    pub user_id: UserId,
    /// The guild.
    pub guild_id: GuildId,
}

newtype_event! {
    /// The data of the [Guild Soundboard Sound Create](https://docs.discord.com/developers/events/gateway-events#guild-soundboard-sound-create)
    /// dispatch event.
    ///
    /// discord.js counterpart: the `GuildSoundboardSoundCreate` event.
    GuildSoundboardSoundCreate(SoundboardSound)
}

newtype_event! {
    /// The data of the [Guild Soundboard Sound Update](https://docs.discord.com/developers/events/gateway-events#guild-soundboard-sound-update)
    /// dispatch event.
    ///
    /// discord.js counterpart: the `GuildSoundboardSoundUpdate` event.
    GuildSoundboardSoundUpdate(SoundboardSound)
}

/// The data of the [Guild Soundboard Sound Delete](https://docs.discord.com/developers/events/gateway-events#guild-soundboard-sound-delete)
/// dispatch event.
///
/// discord.js counterpart: the `GuildSoundboardSoundDelete` event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildSoundboardSoundDelete {
    /// The deleted sound.
    pub sound_id: SoundboardSoundId,
    /// The guild.
    pub guild_id: GuildId,
}

/// The data of the [Guild Soundboard Sounds Update](https://docs.discord.com/developers/events/gateway-events#guild-soundboard-sounds-update)
/// dispatch event: several sounds changed.
///
/// discord.js counterpart: the `GuildSoundboardSoundsUpdate` event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildSoundboardSoundsUpdate {
    /// The guild's sounds.
    pub soundboard_sounds: Vec<SoundboardSound>,
    /// The guild.
    pub guild_id: GuildId,
}

/// The data of the [Soundboard Sounds](https://docs.discord.com/developers/events/gateway-events#soundboard-sounds)
/// dispatch event: a guild's sounds, sent in response to a
/// [`RequestSoundboardSounds`](crate::gateway::payload::RequestSoundboardSounds) command.
///
/// discord.js counterpart: the `SoundboardSounds` event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SoundboardSounds {
    /// The guild's sounds.
    pub soundboard_sounds: Vec<SoundboardSound>,
    /// The guild.
    pub guild_id: GuildId,
}

guild_id_event! {
    /// The data of the [Integration Create](https://docs.discord.com/developers/events/gateway-events#integration-create)
    /// dispatch event (the integration has no `user`).
    ///
    /// discord.js counterpart: the integration data behind `GuildIntegrationsUpdate`.
    IntegrationCreate {
        /// The created integration.
        integration: Integration
    }
}

guild_id_event! {
    /// The data of the [Integration Update](https://docs.discord.com/developers/events/gateway-events#integration-update)
    /// dispatch event (the integration has no `user`).
    ///
    /// discord.js counterpart: the integration data behind `GuildIntegrationsUpdate`.
    IntegrationUpdate {
        /// The updated integration.
        integration: Integration
    }
}

/// The data of the [Integration Delete](https://docs.discord.com/developers/events/gateway-events#integration-delete)
/// dispatch event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IntegrationDelete {
    /// The deleted integration.
    pub id: IntegrationId,
    /// The guild.
    pub guild_id: GuildId,
    /// The bot or `OAuth2` application of a Discord integration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_id: Option<ApplicationId>,
}

newtype_event! {
    /// The data of the [Auto Moderation Rule Create](https://docs.discord.com/developers/events/gateway-events#auto-moderation-rule-create)
    /// dispatch event (requires `MANAGE_GUILD`).
    ///
    /// discord.js counterpart: the `AutoModerationRuleCreate` event.
    AutoModerationRuleCreate(Rule)
}

newtype_event! {
    /// The data of the [Auto Moderation Rule Update](https://docs.discord.com/developers/events/gateway-events#auto-moderation-rule-update)
    /// dispatch event (requires `MANAGE_GUILD`).
    ///
    /// discord.js counterpart: the `AutoModerationRuleUpdate` event.
    AutoModerationRuleUpdate(Rule)
}

newtype_event! {
    /// The data of the [Auto Moderation Rule Delete](https://docs.discord.com/developers/events/gateway-events#auto-moderation-rule-delete)
    /// dispatch event (requires `MANAGE_GUILD`).
    ///
    /// discord.js counterpart: the `AutoModerationRuleDelete` event.
    AutoModerationRuleDelete(Rule)
}

/// The data of the [Auto Moderation Action Execution](https://docs.discord.com/developers/events/gateway-events#auto-moderation-action-execution)
/// dispatch event: a rule triggered and one of its actions ran (requires `MANAGE_GUILD`; the
/// content fields require the `MESSAGE_CONTENT` intent).
///
/// discord.js counterpart: the `AutoModerationActionExecution` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AutoModerationActionExecution {
    /// The guild.
    pub guild_id: GuildId,
    /// The action that ran.
    pub action: Action,
    /// The rule.
    pub rule_id: AutoModerationRuleId,
    /// The rule's trigger type.
    pub rule_trigger_type: TriggerType,
    /// The user whose content triggered the rule.
    pub user_id: UserId,
    /// The channel the content was posted in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,
    /// The user's message (absent if it was blocked or the content was not a message).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<MessageId>,
    /// The alert message posted by a `SEND_ALERT_MESSAGE` action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alert_system_message_id: Option<MessageId>,
    /// The user's content (empty without the `MESSAGE_CONTENT` intent).
    #[serde(default)]
    pub content: CompactString,
    /// The keyword or phrase of the rule that matched.
    #[serde(default)]
    pub matched_keyword: Option<CompactString>,
    /// The part of the content that matched (`None` without the `MESSAGE_CONTENT` intent).
    #[serde(default)]
    pub matched_content: Option<CompactString>,
}

newtype_event! {
    /// The data of the [Application Command Permissions Update](https://docs.discord.com/developers/events/gateway-events#application-command-permissions-update)
    /// dispatch event.
    ///
    /// discord.js counterpart: the `ApplicationCommandPermissionsUpdate` event.
    ApplicationCommandPermissionsUpdate(GuildCommandPermissions)
}
