//! [Audit logs](https://docs.discord.com/developers/resources/audit-log): the administrative
//! actions taken in a guild, with what they changed.
//!
//! An [`AuditLog`] (returned by `GET /guilds/{guild.id}/audit-logs`) holds
//! [`AuditLogEntry`]s plus the users, webhooks, threads, commands and other objects they
//! refer to. Each entry's [`changes`](AuditLogEntry::changes) keep their "mixed" values as
//! [`JsonValue`]s, which convert to the field's type on demand (D-070, D-072).

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    application_command::ApplicationCommand,
    auto_moderation::{Rule, TriggerType},
    channel::{Channel, OverwriteType},
    guild::{IntegrationType, PartialIntegration},
    guild_scheduled_event::GuildScheduledEvent,
    id::{ApplicationId, AuditLogEntryId, ChannelId, GenericId, GuildId, MessageId, UserId},
    macros::int_enum,
    user::User,
    util::{JsonValue, JsonValueError},
    webhook::Webhook,
};

/// A guild's [audit log](https://docs.discord.com/developers/resources/audit-log#audit-log-object):
/// entries, most recent first, and the objects they refer to.
///
/// Requires the `VIEW_AUDIT_LOG` permission.
///
/// discord.js counterpart: `GuildAuditLogs`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AuditLog {
    /// The application commands referred to by the entries.
    #[serde(default)]
    pub application_commands: Vec<ApplicationCommand>,
    /// The entries, from most to least recent.
    #[serde(default)]
    pub audit_log_entries: Vec<AuditLogEntry>,
    /// The auto moderation rules referred to by the entries.
    #[serde(default)]
    pub auto_moderation_rules: Vec<Rule>,
    /// The scheduled events referred to by the entries.
    #[serde(default)]
    pub guild_scheduled_events: Vec<GuildScheduledEvent>,
    /// The integrations referred to by the entries (partial objects).
    #[serde(default)]
    pub integrations: Vec<PartialIntegration>,
    /// The threads referred to by the entries (archived threads may not be cached elsewhere).
    #[serde(default)]
    pub threads: Vec<Channel>,
    /// The users referred to by the entries.
    #[serde(default)]
    pub users: Vec<User>,
    /// The webhooks referred to by the entries.
    #[serde(default)]
    pub webhooks: Vec<Webhook>,
}

/// An [audit log entry](https://docs.discord.com/developers/resources/audit-log#audit-log-entry-object):
/// one administrative action.
///
/// Some actions done in quick succession by the same user are merged into one entry
/// (message deletes, member moves and disconnects, channel and role reordering); only the
/// first reason is kept.
///
/// Contains floats (through [`JsonValue`]), so it is `PartialEq` but not `Eq` (D-064).
///
/// discord.js counterpart: `GuildAuditLogsEntry`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AuditLogEntry {
    /// The entry's ID.
    pub id: AuditLogEntryId,
    /// What happened.
    pub action_type: AuditLogEvent,
    /// The affected entity (webhook, user, role, …); for
    /// [`ApplicationCommandPermissionUpdate`](AuditLogEvent::ApplicationCommandPermissionUpdate)
    /// the command or the application.
    pub target_id: Option<GenericId>,
    /// The user or app that made the change.
    pub user_id: Option<UserId>,
    /// What was changed on the target.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<AuditLogChange>,
    /// Extra information for some event types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Box<AuditLogEntryOptions>>,
    /// The reason given with the `X-Audit-Log-Reason` header (1-512 characters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<CompactString>,
    /// The guild (only in the `GUILD_AUDIT_LOG_ENTRY_CREATE` gateway event).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
}

int_enum! {
    /// The [action of an audit log entry](https://docs.discord.com/developers/resources/audit-log#audit-log-entry-object-audit-log-events).
    ///
    /// discord.js counterpart: `AuditLogEvent`.
    pub enum AuditLogEvent: u8 {
        /// Server settings were updated (changes: guild fields).
        GuildUpdate = 1,
        /// A channel was created (changes: channel fields).
        ChannelCreate = 10,
        /// A channel's settings were updated (changes: channel fields).
        ChannelUpdate = 11,
        /// A channel was deleted (changes: channel fields).
        ChannelDelete = 12,
        /// A permission overwrite was added to a channel (changes: overwrite fields; options:
        /// `id`, `type`, `role_name`).
        ChannelOverwriteCreate = 13,
        /// A permission overwrite was updated (changes: overwrite fields; options: `id`, `type`,
        /// `role_name`).
        ChannelOverwriteUpdate = 14,
        /// A permission overwrite was deleted (changes: overwrite fields; options: `id`, `type`,
        /// `role_name`).
        ChannelOverwriteDelete = 15,
        /// Channels were reordered (options: `count`).
        ChannelPositionUpdate = 16,
        /// A member was kicked (options: `integration_type`).
        MemberKick = 20,
        /// Members were pruned (options: `delete_member_days`, `members_removed`).
        MemberPrune = 21,
        /// A member was banned.
        MemberBanAdd = 22,
        /// A ban was lifted.
        MemberBanRemove = 23,
        /// A member was updated (changes: member fields).
        MemberUpdate = 24,
        /// A member was added to or removed from roles (changes: `$add` / `$remove` with partial
        /// roles; options: `integration_type`).
        MemberRoleUpdate = 25,
        /// A member was moved to another voice channel (options: `channel_id`, `count`).
        MemberMove = 26,
        /// A member was disconnected from a voice channel (options: `count`).
        MemberDisconnect = 27,
        /// A bot was added.
        BotAdd = 28,
        /// A role was created (changes: role fields).
        RoleCreate = 30,
        /// A role was edited (changes: role fields).
        RoleUpdate = 31,
        /// A role was deleted (changes: role fields).
        RoleDelete = 32,
        /// Roles were reordered (options: `count`).
        RolePositionUpdate = 33,
        /// An invite was created (changes: invite fields, `channel_id` instead of `channel`).
        InviteCreate = 40,
        /// An invite was updated (changes: invite fields).
        InviteUpdate = 41,
        /// An invite was deleted (changes: invite fields).
        InviteDelete = 42,
        /// A webhook was created (changes: webhook fields, `avatar_hash` instead of `avatar`).
        WebhookCreate = 50,
        /// A webhook was updated (changes: webhook fields).
        WebhookUpdate = 51,
        /// A webhook was deleted (changes: webhook fields).
        WebhookDelete = 52,
        /// An emoji was created (changes: emoji fields).
        EmojiCreate = 60,
        /// An emoji was renamed (changes: emoji fields).
        EmojiUpdate = 61,
        /// An emoji was deleted (changes: emoji fields).
        EmojiDelete = 62,
        /// A message was deleted (options: `channel_id`, `count`).
        MessageDelete = 72,
        /// Messages were bulk-deleted (options: `count`).
        MessageBulkDelete = 73,
        /// A message was pinned (options: `channel_id`, `message_id`).
        MessagePin = 74,
        /// A message was unpinned (options: `channel_id`, `message_id`).
        MessageUnpin = 75,
        /// An app was added (changes: integration fields).
        IntegrationCreate = 80,
        /// An app was updated, e.g. its scopes (changes: integration fields).
        IntegrationUpdate = 81,
        /// An app was removed (changes: integration fields).
        IntegrationDelete = 82,
        /// A stage went live (changes: stage instance fields; options: `channel_id`).
        StageInstanceCreate = 83,
        /// A stage instance was updated (changes: stage instance fields; options: `channel_id`).
        StageInstanceUpdate = 84,
        /// A stage ended (changes: stage instance fields; options: `channel_id`).
        StageInstanceDelete = 85,
        /// A sticker was created (changes: sticker fields).
        StickerCreate = 90,
        /// A sticker was updated (changes: sticker fields).
        StickerUpdate = 91,
        /// A sticker was deleted (changes: sticker fields).
        StickerDelete = 92,
        /// A scheduled event was created (changes: scheduled event fields).
        GuildScheduledEventCreate = 100,
        /// A scheduled event was updated (changes: scheduled event fields).
        GuildScheduledEventUpdate = 101,
        /// A scheduled event was cancelled (changes: scheduled event fields).
        GuildScheduledEventDelete = 102,
        /// A thread was created (changes: thread fields).
        ThreadCreate = 110,
        /// A thread was updated (changes: thread fields).
        ThreadUpdate = 111,
        /// A thread was deleted (changes: thread fields).
        ThreadDelete = 112,
        /// A command's permissions were updated (changes keyed by the role, channel or user ID
        /// with command permission objects; options: `application_id`).
        ApplicationCommandPermissionUpdate = 121,
        /// A soundboard sound was created (changes: soundboard sound fields).
        SoundboardSoundCreate = 130,
        /// A soundboard sound was updated (changes: soundboard sound fields).
        SoundboardSoundUpdate = 131,
        /// A soundboard sound was deleted (changes: soundboard sound fields).
        SoundboardSoundDelete = 132,
        /// An auto moderation rule was created (changes: rule fields).
        AutoModerationRuleCreate = 140,
        /// An auto moderation rule was updated (changes: rule fields).
        AutoModerationRuleUpdate = 141,
        /// An auto moderation rule was deleted (changes: rule fields).
        AutoModerationRuleDelete = 142,
        /// Auto moderation blocked a message (options: rule name and trigger type,
        /// `channel_id`).
        AutoModerationBlockMessage = 143,
        /// Auto moderation flagged a message to a channel (options: rule name and trigger type,
        /// `channel_id`).
        AutoModerationFlagToChannel = 144,
        /// Auto moderation timed a member out (options: rule name and trigger type,
        /// `channel_id`).
        AutoModerationUserCommunicationDisabled = 145,
        /// Auto moderation quarantined a member (options: rule name and trigger type,
        /// `channel_id`).
        AutoModerationQuarantineUser = 146,
        /// A creator monetization request was created.
        CreatorMonetizationRequestCreated = 150,
        /// The creator monetization terms were accepted.
        CreatorMonetizationTermsAccepted = 151,
        /// An onboarding question was created (changes: onboarding prompt fields).
        OnboardingPromptCreate = 163,
        /// An onboarding question was updated (changes: onboarding prompt fields).
        OnboardingPromptUpdate = 164,
        /// An onboarding question was deleted (changes: onboarding prompt fields).
        OnboardingPromptDelete = 165,
        /// Onboarding was created (changes: onboarding fields).
        OnboardingCreate = 166,
        /// Onboarding was updated (changes: onboarding fields).
        OnboardingUpdate = 167,
        /// The server guide was created.
        HomeSettingsCreate = 190,
        /// The server guide was updated.
        HomeSettingsUpdate = 191,
        /// A user set a voice channel status (options: `channel_id`, `status`).
        VoiceChannelStatusCreate = 192,
        /// A user deleted a voice channel status (options: `channel_id`).
        VoiceChannelStatusDelete = 193,
    }
}

/// The [optional entry info](https://docs.discord.com/developers/resources/audit-log#audit-log-entry-object-optional-audit-entry-info)
/// of some event types (D-071): see each [`AuditLogEvent`] for the fields it has.
///
/// Discord sends the counts and types as strings; they are decoded as numbers and enums and
/// written back as strings.
///
/// discord.js counterpart: `GuildAuditLogsEntry#extra`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "OptionalAuditEntryInfo")]
pub struct AuditLogEntryOptions {
    /// The application whose command permissions were updated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_id: Option<ApplicationId>,
    /// The name of the auto moderation rule that triggered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_moderation_rule_name: Option<CompactString>,
    /// The trigger type of the auto moderation rule that triggered.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "option_string_number"
    )]
    pub auto_moderation_rule_trigger_type: Option<TriggerType>,
    /// The channel the entities were in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,
    /// The number of entities targeted, or of position updates.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "option_string_number"
    )]
    pub count: Option<u32>,
    /// The inactivity in days after which members were pruned.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "option_string_number"
    )]
    pub delete_member_days: Option<u32>,
    /// The overwritten role or member.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<GenericId>,
    /// The number of members removed by a prune.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "option_string_number"
    )]
    pub members_removed: Option<u32>,
    /// The pinned or unpinned message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<MessageId>,
    /// The overwritten role's name (role overwrites only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_name: Option<CompactString>,
    /// Whether the overwritten entity is a role or a member.
    #[serde(
        default,
        rename = "type",
        skip_serializing_if = "Option::is_none",
        with = "option_string_number"
    )]
    pub kind: Option<OverwriteType>,
    /// The type of integration that performed the action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration_type: Option<IntegrationType>,
    /// The new voice channel status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<CompactString>,
}

/// A number Discord sends as a decimal string.
trait StringNumber: Sized + Copy {
    fn from_u64(value: u64) -> Option<Self>;
    fn to_u64(self) -> u64;
}

impl StringNumber for u32 {
    fn from_u64(value: u64) -> Option<Self> {
        Self::try_from(value).ok()
    }

    fn to_u64(self) -> u64 {
        self.into()
    }
}

impl StringNumber for OverwriteType {
    fn from_u64(value: u64) -> Option<Self> {
        u8::try_from(value).ok().map(Self::from)
    }

    fn to_u64(self) -> u64 {
        u8::from(self).into()
    }
}

impl StringNumber for TriggerType {
    fn from_u64(value: u64) -> Option<Self> {
        u8::try_from(value).ok().map(Self::from)
    }

    fn to_u64(self) -> u64 {
        u8::from(self).into()
    }
}

/// (De)serializes an optional number written as a decimal string (numbers are accepted too).
mod option_string_number {
    use core::marker::PhantomData;

    use serde::{
        Deserializer, Serializer,
        de::{Error, Unexpected, Visitor},
    };

    use super::StringNumber;

    #[expect(clippy::ref_option, reason = "signature required by serde(with)")]
    pub(super) fn serialize<T: StringNumber, S: Serializer>(
        value: &Option<T>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(value) => serializer.collect_str(&value.to_u64()),
            None => serializer.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, T: StringNumber, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<T>, D::Error> {
        struct NumberVisitor<T>(PhantomData<T>);

        impl<'de, T: StringNumber> Visitor<'de> for NumberVisitor<T> {
            type Value = Option<T>;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("a number as a decimal string")
            }

            fn visit_none<E: Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_some<D: Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                deserializer.deserialize_any(self)
            }

            fn visit_u64<E: Error>(self, value: u64) -> Result<Self::Value, E> {
                T::from_u64(value)
                    .map(Some)
                    .ok_or_else(|| E::invalid_value(Unexpected::Unsigned(value), &self))
            }

            fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
                value
                    .parse()
                    .ok()
                    .and_then(T::from_u64)
                    .map(Some)
                    .ok_or_else(|| E::invalid_value(Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_option(NumberVisitor(PhantomData))
    }
}

/// A [change](https://docs.discord.com/developers/resources/audit-log#audit-log-change-object)
/// in an audit log entry: one field of the target, with its old and new value (D-070).
///
/// The key is usually a field of the changed object, with these exceptions:
/// - member role updates use `$add` / `$remove`, with arrays of partial roles
///   (`{"id", "name"}`) as the new value;
/// - command permission updates use the role, channel or user ID, with command permission
///   objects as values;
/// - invites use `channel_id` instead of `channel`, webhooks `avatar_hash` instead of `avatar`.
///
/// An absent [`new_value`](Self::new_value) means the field was reset or set to `null`; an
/// absent [`old_value`](Self::old_value) means it was `null` before.
///
/// discord.js counterpart: `AuditLogChange`.
///
/// # Examples
///
/// ```
/// use djs_model::{audit_log::AuditLogChange, permissions::Permissions};
///
/// let change: AuditLogChange =
///     serde_json::from_str(r#"{"key":"permissions","old_value":"0","new_value":"8"}"#).unwrap();
/// assert_eq!(change.key, "permissions");
/// let new: Option<Permissions> = change.new_value_as().unwrap();
/// assert_eq!(new, Some(Permissions::ADMINISTRATOR));
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AuditLogChange {
    /// The changed field (see the exceptions above).
    pub key: CompactString,
    /// The value before the change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_value: Option<JsonValue>,
    /// The value after the change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_value: Option<JsonValue>,
}

impl AuditLogChange {
    /// Converts the [`old_value`](Self::old_value) to the field's type (`Ok(None)` if absent).
    ///
    /// # Errors
    ///
    /// Returns an error if the value does not have the shape of `T`.
    pub fn old_value_as<'a, T: Deserialize<'a>>(&'a self) -> Result<Option<T>, JsonValueError> {
        self.old_value.as_ref().map(JsonValue::to_type).transpose()
    }

    /// Converts the [`new_value`](Self::new_value) to the field's type (`Ok(None)` if absent).
    ///
    /// # Errors
    ///
    /// Returns an error if the value does not have the shape of `T`.
    pub fn new_value_as<'a, T: Deserialize<'a>>(&'a self) -> Result<Option<T>, JsonValueError> {
        self.new_value.as_ref().map(JsonValue::to_type).transpose()
    }
}

#[cfg(test)]
mod tests {
    use core::mem::size_of;

    use super::*;

    #[test]
    fn sizes() {
        assert!(
            size_of::<AuditLogEntry>() <= 128,
            "{}",
            size_of::<AuditLogEntry>()
        );
    }
}
