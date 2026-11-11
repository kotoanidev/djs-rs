//! [Guild templates](https://docs.discord.com/developers/resources/guild-template): codes that
//! create a new guild from a snapshot of an existing one.
//!
//! The snapshot ([`SerializedSourceGuild`]) does not use real IDs: roles and channels get small
//! **placeholder IDs** (`0` is `@everyone`, channels count up from `1`) that other parts of the
//! snapshot refer to (`parent_id`, permission overwrites, `system_channel_id`). Discord replaces
//! them with real snowflakes when a guild is created from the template. They are therefore plain
//! `u64`s here, not [`Id`](crate::id::Id)s, and the snapshot types are separate from
//! [`Role`](crate::guild::Role) and [`Channel`](crate::channel::Channel) (D-059). The same format
//! is used by the (deprecated) "create guild" REST body.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    channel::{
        AutoArchiveDuration, ChannelFlags, ChannelType, DefaultReaction, ForumLayoutType,
        OverwriteType, SortOrderType, VideoQualityMode,
    },
    guild::{
        DefaultMessageNotificationLevel, ExplicitContentFilterLevel, RoleColors,
        SystemChannelFlags, VerificationLevel,
    },
    id::{EmojiId, GuildId, UserId},
    permissions::Permissions,
    user::User,
    util::{ImageHash, Locale, Timestamp},
};

/// A [guild template](https://docs.discord.com/developers/resources/guild-template#guild-template-object).
///
/// discord.js counterpart: `GuildTemplate`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "Template")]
pub struct GuildTemplate {
    /// The template code (its unique ID), as in `discord.new/{code}`.
    pub code: CompactString,
    /// The template's name.
    pub name: CompactString,
    /// The template's description.
    pub description: Option<CompactString>,
    /// How many times the template has been used.
    pub usage_count: u32,
    /// The user who created the template.
    pub creator_id: UserId,
    /// The user who created the template.
    pub creator: User,
    /// When the template was created.
    pub created_at: Timestamp,
    /// When the template was last synced with its source guild.
    pub updated_at: Timestamp,
    /// The guild the template was made from.
    pub source_guild_id: GuildId,
    /// The snapshot of the source guild.
    pub serialized_source_guild: SerializedSourceGuild,
    /// Whether the source guild has changes that are not synced into the template yet.
    pub is_dirty: Option<bool>,
}

/// The guild snapshot of a [`GuildTemplate`] (a partial guild with placeholder IDs, D-059).
///
/// Fields that the "create guild" format treats as optional are [`Option`]s here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SerializedSourceGuild {
    /// The guild's name.
    pub name: CompactString,
    /// The guild's description.
    #[serde(default)]
    pub description: Option<CompactString>,
    /// The voice region. **Deprecated** by Discord: regions are set per voice channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<CompactString>,
    /// The verification level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_level: Option<VerificationLevel>,
    /// The default message notification level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_message_notifications: Option<DefaultMessageNotificationLevel>,
    /// The explicit content filter level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explicit_content_filter: Option<ExplicitContentFilterLevel>,
    /// The preferred locale of a Community guild.
    pub preferred_locale: Locale,
    /// The AFK timeout in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub afk_timeout: Option<u32>,
    /// The roles; the first one (placeholder ID `0`) is `@everyone`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<TemplateRole>,
    /// The channels; categories come before their children.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<TemplateChannel>,
    /// The placeholder ID of the AFK channel.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::serde_util::option_placeholder_id"
    )]
    pub afk_channel_id: Option<u64>,
    /// The placeholder ID of the system channel (welcome messages, boost notices).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::serde_util::option_placeholder_id"
    )]
    pub system_channel_id: Option<u64>,
    /// The system channel flags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_channel_flags: Option<SystemChannelFlags>,
    /// The [icon hash](https://docs.discord.com/developers/reference#image-formatting) of the
    /// source guild.
    #[serde(default)]
    pub icon_hash: Option<ImageHash>,
    /// Whether the boost progress bar is shown.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub premium_progress_bar_enabled: bool,
}

/// A role in a [`SerializedSourceGuild`] (D-059).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TemplateRole {
    /// The placeholder ID (`0` is `@everyone`).
    #[serde(with = "crate::serde_util::placeholder_id")]
    pub id: u64,
    /// The role's name.
    pub name: CompactString,
    /// The role's permissions.
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    pub permissions: Permissions,
    /// The role's primary color as an RGB integer. **Deprecated** by Discord in favor of
    /// [`colors`](Self::colors).
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    pub color: u32,
    /// The role's colors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colors: Option<RoleColors>,
    /// Whether the role's members are listed separately.
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    pub hoist: bool,
    /// The [role icon hash](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<ImageHash>,
    /// The role's Unicode emoji.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unicode_emoji: Option<CompactString>,
    /// Whether everyone can mention the role.
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    pub mentionable: bool,
}

/// A channel in a [`SerializedSourceGuild`] (D-059).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TemplateChannel {
    /// The placeholder ID.
    #[serde(with = "crate::serde_util::placeholder_id")]
    pub id: u64,
    /// The kind of channel.
    #[serde(rename = "type")]
    pub kind: ChannelType,
    /// The channel's name.
    pub name: CompactString,
    /// The sorting position.
    #[serde(default)]
    pub position: i32,
    /// The channel's topic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<CompactString>,
    /// The bitrate of a voice channel, in bits per second.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u32>,
    /// The user limit of a voice channel (`0` = no limit).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_limit: Option<u32>,
    /// Whether the channel is age-restricted.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub nsfw: bool,
    /// The slowmode in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<u16>,
    /// The placeholder ID of the parent category.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::serde_util::option_placeholder_id"
    )]
    pub parent_id: Option<u64>,
    /// The permission overwrites (referring to placeholder role IDs).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permission_overwrites: Vec<TemplateOverwrite>,
    /// The voice region of a voice channel (`None` = automatic).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rtc_region: Option<CompactString>,
    /// The camera video quality of a voice channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_quality_mode: Option<VideoQualityMode>,
    /// The channel's flags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<ChannelFlags>,
    /// The tags of a forum or media channel.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "crate::serde_util::null_as_default"
    )]
    pub available_tags: Vec<TemplateForumTag>,
    /// The default auto-archive duration of new threads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_auto_archive_duration: Option<AutoArchiveDuration>,
    /// The default reaction button of forum posts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_reaction_emoji: Option<DefaultReaction>,
    /// The default slowmode of new threads, in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_thread_rate_limit_per_user: Option<u16>,
    /// The default sort order of forum posts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_sort_order: Option<SortOrderType>,
    /// The default layout of a forum channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_forum_layout: Option<ForumLayoutType>,
}

/// A permission overwrite in a [`TemplateChannel`] (D-059).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TemplateOverwrite {
    /// The placeholder ID of the role (or the ID of the member).
    #[serde(with = "crate::serde_util::placeholder_id")]
    pub id: u64,
    /// Whether [`id`](Self::id) is a role or a member.
    #[serde(rename = "type")]
    pub kind: OverwriteType,
    /// The allowed permissions.
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    pub allow: Permissions,
    /// The denied permissions.
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    pub deny: Permissions,
}

/// A forum tag in a [`TemplateChannel`]: like a [`ForumTag`](crate::channel::ForumTag), but
/// without an ID (D-059).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TemplateForumTag {
    /// The tag's name.
    pub name: CompactString,
    /// Whether only members with `MANAGE_THREADS` can add the tag.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub moderated: bool,
    /// The tag's custom emoji.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji_id: Option<EmojiId>,
    /// The tag's Unicode emoji.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<CompactString>,
}
