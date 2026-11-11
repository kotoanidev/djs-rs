//! Bodies, queries and responses of the [guild endpoints](https://docs.discord.com/developers/resources/guild):
//! the guild, its channels, members, bans, roles, prune, widget, welcome screen, onboarding
//! and incident actions.
//!
//! Deprecated request fields are left out (`region`, role `color`, ban
//! `delete_message_days`, the prune body's `reason`; C-048).

use core::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

use compact_str::CompactString;
use serde::{Deserialize, Serialize, Serializer};

use super::ForumTagData;
use crate::{
    channel::{
        AutoArchiveDuration, Channel, ChannelFlags, ChannelType, DefaultReaction, ForumLayoutType,
        Overwrite, SortOrderType, ThreadMember, VideoQualityMode,
    },
    emoji::PartialEmoji,
    guild::{
        DefaultMessageNotificationLevel, ExplicitContentFilterLevel, Feature, MemberFlags,
        OnboardingMode, OnboardingPrompt, PromptOption, PromptType, RoleColors, SystemChannelFlags,
        VerificationLevel, WelcomeScreenChannel,
    },
    id::{ChannelId, EmojiId, GenericId, RoleId, UserId},
    permissions::Permissions,
    util::{ImageData, Locale, Patch, Timestamp},
};

/// The query of [Get Guild](https://docs.discord.com/developers/resources/guild#get-guild).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetGuild {
    /// Whether to include the approximate member and presence counts.
    pub with_counts: Option<bool>,
}

/// The body of [Modify Guild](https://docs.discord.com/developers/resources/guild#modify-guild).
///
/// Every field is optional; [`Patch`] fields can also be cleared. Adding or removing the
/// `COMMUNITY` feature needs `ADMINISTRATOR`.
///
/// discord.js counterpart: `GuildEditOptions`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyGuild {
    /// The name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// Who can talk in the guild.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub verification_level: Patch<VerificationLevel>,
    /// Which messages notify members by default.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub default_message_notifications: Patch<DefaultMessageNotificationLevel>,
    /// Whose media is scanned for explicit content.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub explicit_content_filter: Patch<ExplicitContentFilterLevel>,
    /// The AFK voice channel.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub afk_channel_id: Patch<ChannelId>,
    /// Seconds of inactivity before members are moved to the AFK channel: 60, 300, 900,
    /// 1800 or 3600.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub afk_timeout: Option<u32>,
    /// The icon (1024×1024; an animated GIF with the `ANIMATED_ICON` feature).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub icon: Patch<ImageData>,
    /// The invite splash (16:9; needs the `INVITE_SPLASH` feature).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub splash: Patch<ImageData>,
    /// The discovery splash (16:9; needs the `DISCOVERABLE` feature).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub discovery_splash: Patch<ImageData>,
    /// The banner (16:9; needs the `BANNER` feature, animated with `ANIMATED_BANNER`).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub banner: Patch<ImageData>,
    /// The channel of welcome and boost messages.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub system_channel_id: Patch<ChannelId>,
    /// Which messages the system channel shows (set flags suppress them).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_channel_flags: Option<SystemChannelFlags>,
    /// The rules or guidelines channel of a community guild.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub rules_channel_id: Patch<ChannelId>,
    /// The channel where community guild moderators get notices from Discord.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub public_updates_channel_id: Patch<ChannelId>,
    /// The language of a community guild (Discord's default: `en-US`).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub preferred_locale: Patch<Locale>,
    /// The enabled features (all of them).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<Feature>>,
    /// The description.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub description: Patch<CompactString>,
    /// Whether the boost progress bar is shown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premium_progress_bar_enabled: Option<bool>,
    /// The channel where community guild moderators get safety alerts.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub safety_alerts_channel_id: Patch<ChannelId>,
}

/// The body of [Create Guild Channel](https://docs.discord.com/developers/resources/guild#create-guild-channel).
///
/// Only the name is required; Discord creates a text channel when
/// [`kind`](Self::kind) is `None`. Which fields apply depends on the type.
///
/// discord.js counterpart: `GuildChannelCreateOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::{channel::ChannelType, rest::CreateGuildChannel};
///
/// let mut voice = CreateGuildChannel::new("Lounge");
/// voice.kind = Some(ChannelType::GuildVoice);
/// voice.user_limit = Some(10);
/// assert_eq!(
///     serde_json::to_string(&voice).unwrap(),
///     r#"{"name":"Lounge","type":2,"user_limit":10}"#
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct CreateGuildChannel {
    /// The name (1-100 characters).
    pub name: CompactString,
    /// The type (Discord's default: a text channel).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<ChannelType>,
    /// The topic (0-1024 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<CompactString>,
    /// The bitrate of a voice or stage channel, in bits per second (min 8000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u32>,
    /// The user limit of a voice or stage channel; 0 is no limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_limit: Option<u32>,
    /// Slowmode: seconds a member must wait between messages (0-21600).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<u16>,
    /// The position in the channel list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
    /// The permission overwrites; only permissions the bot has can be allowed or denied.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub permission_overwrites: Vec<Overwrite>,
    /// The parent category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ChannelId>,
    /// Whether the channel is age-restricted.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub nsfw: bool,
    /// The voice region ID (Discord's default: automatic).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtc_region: Option<CompactString>,
    /// The camera video quality of a voice or stage channel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_quality_mode: Option<VideoQualityMode>,
    /// The archive duration that clients preselect for new threads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_auto_archive_duration: Option<AutoArchiveDuration>,
    /// The emoji of the add-reaction button on posts of a forum or media channel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_reaction_emoji: Option<DefaultReaction>,
    /// The tags of a forum or media channel (max 20).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub available_tags: Vec<ForumTagData>,
    /// The default order of posts in a forum or media channel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_sort_order: Option<SortOrderType>,
    /// The default layout of a forum channel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_forum_layout: Option<ForumLayoutType>,
    /// The slowmode copied to new threads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_thread_rate_limit_per_user: Option<u16>,
    /// The channel's flags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<ChannelFlags>,
}

impl CreateGuildChannel {
    /// A channel named `name`, with Discord's defaults for everything else.
    #[must_use]
    pub fn new(name: impl Into<CompactString>) -> Self {
        Self {
            name: name.into(),
            kind: None,
            topic: None,
            bitrate: None,
            user_limit: None,
            rate_limit_per_user: None,
            position: None,
            permission_overwrites: Vec::new(),
            parent_id: None,
            nsfw: false,
            rtc_region: None,
            video_quality_mode: None,
            default_auto_archive_duration: None,
            default_reaction_emoji: None,
            available_tags: Vec::new(),
            default_sort_order: None,
            default_forum_layout: None,
            default_thread_rate_limit_per_user: None,
            flags: None,
        }
    }
}

/// One entry of [Modify Guild Channel Positions](https://docs.discord.com/developers/resources/guild#modify-guild-channel-positions).
///
/// At most one entry per request may change [`parent_id`](Self::parent_id).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyGuildChannelPosition {
    /// The channel.
    pub id: ChannelId,
    /// The new position.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub position: Patch<i32>,
    /// When moving to a new category, whether to sync the permission overwrites with it.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub lock_permissions: Patch<bool>,
    /// The new parent category; [`Patch::Clear`] moves the channel out of its category.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub parent_id: Patch<ChannelId>,
}

impl ModifyGuildChannelPosition {
    /// Moves channel `id` to `position`.
    #[must_use]
    pub const fn new(id: ChannelId, position: i32) -> Self {
        Self {
            id,
            position: Patch::Set(position),
            lock_permissions: Patch::Keep,
            parent_id: Patch::Keep,
        }
    }
}

/// The response of [List Active Guild Threads](https://docs.discord.com/developers/resources/guild#list-active-guild-threads).
///
/// discord.js counterpart: `FetchedThreads`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ActiveThreads {
    /// The active threads, highest ID first.
    pub threads: Vec<Channel>,
    /// The current user's membership of each returned thread they joined.
    pub members: Vec<ThreadMember>,
}

/// The query of [List Guild Members](https://docs.discord.com/developers/resources/guild#list-guild-members).
///
/// Needs the `GUILD_MEMBERS` privileged intent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ListGuildMembers {
    /// The page size (1-1000; Discord's default: 1).
    pub limit: Option<u16>,
    /// The highest user ID of the previous page.
    pub after: Option<UserId>,
}

/// The query of [Search Guild Members](https://docs.discord.com/developers/resources/guild#search-guild-members).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SearchGuildMembers {
    /// The start of the usernames and nicknames to find.
    pub query: CompactString,
    /// The maximum number of members (1-1000; Discord's default: 1).
    pub limit: Option<u16>,
}

impl SearchGuildMembers {
    /// Members whose username or nickname starts with `query`.
    #[must_use]
    pub fn new(query: impl Into<CompactString>) -> Self {
        Self {
            query: query.into(),
            limit: None,
        }
    }
}

/// The body of [Add Guild Member](https://docs.discord.com/developers/resources/guild#add-guild-member).
///
/// The access token is redacted in the `Debug` output.
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct AddGuildMember {
    /// The `OAuth2` access token of the user, granted to the bot's application with the
    /// `guilds.join` scope.
    pub access_token: String,
    /// The nickname (needs `MANAGE_NICKNAMES`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick: Option<CompactString>,
    /// The roles (needs `MANAGE_ROLES`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<RoleId>,
    /// Whether the member is muted in voice channels (needs `MUTE_MEMBERS`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute: Option<bool>,
    /// Whether the member is deafened in voice channels (needs `DEAFEN_MEMBERS`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deaf: Option<bool>,
}

impl AddGuildMember {
    /// Adds the user whose access token is `access_token`.
    #[must_use]
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            nick: None,
            roles: Vec::new(),
            mute: None,
            deaf: None,
        }
    }
}

impl fmt::Debug for AddGuildMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AddGuildMember")
            .field("access_token", &"<redacted>")
            .field("nick", &self.nick)
            .field("roles", &self.roles)
            .field("mute", &self.mute)
            .field("deaf", &self.deaf)
            .finish()
    }
}

/// The body of [Modify Guild Member](https://docs.discord.com/developers/resources/guild#modify-guild-member).
///
/// Every field is optional and can be cleared.
///
/// discord.js counterpart: `GuildMemberEditOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::{rest::ModifyGuildMember, util::{Patch, Timestamp}};
///
/// // A timeout until one hour after 2027-01-15 08:00 UTC.
/// let until = Timestamp::from_unix_secs(1_800_000_000 + 3600).unwrap();
/// let mut body = ModifyGuildMember::default();
/// body.communication_disabled_until = Patch::Set(until);
/// assert_eq!(
///     serde_json::to_string(&body).unwrap(),
///     r#"{"communication_disabled_until":"2027-01-15T09:00:00.000000+00:00"}"#
/// );
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyGuildMember {
    /// The nickname (needs `MANAGE_NICKNAMES`).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub nick: Patch<CompactString>,
    /// The roles, all of them (needs `MANAGE_ROLES`).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub roles: Patch<Vec<RoleId>>,
    /// Whether the member is muted in voice channels (needs `MUTE_MEMBERS`; the member must
    /// be connected).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub mute: Patch<bool>,
    /// Whether the member is deafened in voice channels (needs `DEAFEN_MEMBERS`; the member
    /// must be connected).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub deaf: Patch<bool>,
    /// The voice channel to move the member to (needs `MOVE_MEMBERS` and `CONNECT` there);
    /// [`Patch::Clear`] disconnects them.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub channel_id: Patch<ChannelId>,
    /// When the member's [timeout](https://support.discord.com/hc/en-us/articles/4413305239191)
    /// ends (at most 28 days ahead; needs `MODERATE_MEMBERS`); [`Patch::Clear`] ends it.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub communication_disabled_until: Patch<Timestamp>,
    /// The member's flags (only some can be set).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub flags: Patch<MemberFlags>,
}

/// The body of [Modify Current Member](https://docs.discord.com/developers/resources/guild#modify-current-member):
/// the bot's own guild profile.
///
/// discord.js counterpart: `GuildMemberManager#editMe`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyCurrentMember {
    /// The nickname (needs `CHANGE_NICKNAME`).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub nick: Patch<CompactString>,
    /// The guild banner.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub banner: Patch<ImageData>,
    /// The guild avatar.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub avatar: Patch<ImageData>,
    /// The guild bio.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub bio: Patch<CompactString>,
}

/// The query of [Get Guild Bans](https://docs.discord.com/developers/resources/guild#get-guild-bans).
///
/// Bans are sorted by user ID; with both `before` and `after`, only `before` is used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetGuildBans {
    /// The page size (max 1000, the default).
    pub limit: Option<u16>,
    /// Bans of users with a lower ID.
    pub before: Option<UserId>,
    /// Bans of users with a higher ID.
    pub after: Option<UserId>,
}

/// The body of [Create Guild Ban](https://docs.discord.com/developers/resources/guild#create-guild-ban).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct CreateGuildBan {
    /// How many seconds of the user's recent messages to delete (0-604800, 7 days).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_message_seconds: Option<u32>,
}

/// The body of [Bulk Guild Ban](https://docs.discord.com/developers/resources/guild#bulk-guild-ban).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct BulkGuildBan {
    /// The users to ban (max 200).
    pub user_ids: Vec<UserId>,
    /// How many seconds of their recent messages to delete (0-604800, 7 days).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_message_seconds: Option<u32>,
}

impl BulkGuildBan {
    /// Bans `user_ids`.
    #[must_use]
    pub const fn new(user_ids: Vec<UserId>) -> Self {
        Self {
            user_ids,
            delete_message_seconds: None,
        }
    }
}

/// The [response](https://docs.discord.com/developers/resources/guild#bulk-guild-ban-bulk-ban-response)
/// of Bulk Guild Ban.
///
/// discord.js counterpart: `BulkBanResult`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BulkBanResponse {
    /// The users that were banned.
    pub banned_users: Vec<UserId>,
    /// The users that could not be banned.
    pub failed_users: Vec<UserId>,
}

/// The body of [Create Guild Role](https://docs.discord.com/developers/resources/guild#create-guild-role).
///
/// Every field is optional (Discord's defaults: `new role`, the `@everyone` permissions, no
/// color).
///
/// discord.js counterpart: `RoleCreateOptions`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct CreateGuildRole {
    /// The name (max 100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The permissions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
    /// The colors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<RoleColors>,
    /// Whether members with the role are shown separately in the member list.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub hoist: bool,
    /// The icon (needs the `ROLE_ICONS` feature).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<ImageData>,
    /// A Unicode emoji as the icon (needs the `ROLE_ICONS` feature).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unicode_emoji: Option<CompactString>,
    /// Whether everyone can mention the role.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub mentionable: bool,
}

/// One entry of [Modify Guild Role Positions](https://docs.discord.com/developers/resources/guild#modify-guild-role-positions).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyGuildRolePosition {
    /// The role.
    pub id: RoleId,
    /// The new position.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub position: Patch<i32>,
}

impl ModifyGuildRolePosition {
    /// Moves role `id` to `position`.
    #[must_use]
    pub const fn new(id: RoleId, position: i32) -> Self {
        Self {
            id,
            position: Patch::Set(position),
        }
    }
}

/// The body of [Modify Guild Role](https://docs.discord.com/developers/resources/guild#modify-guild-role).
///
/// Every field is optional; [`Patch`] fields can also be cleared.
///
/// discord.js counterpart: `RoleEditOptions`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyGuildRole {
    /// The name (max 100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The permissions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
    /// The colors; [`Patch::Clear`] removes them.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub colors: Patch<RoleColors>,
    /// Whether members with the role are shown separately in the member list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hoist: Option<bool>,
    /// The icon (needs the `ROLE_ICONS` feature).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub icon: Patch<ImageData>,
    /// A Unicode emoji as the icon (needs the `ROLE_ICONS` feature).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub unicode_emoji: Patch<CompactString>,
    /// Whether everyone can mention the role.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentionable: Option<bool>,
}

/// The query of [Get Guild Prune Count](https://docs.discord.com/developers/resources/guild#get-guild-prune-count).
///
/// By default members with roles are never pruned; members whose roles are all in
/// [`include_roles`](Self::include_roles) are.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetGuildPruneCount {
    /// Days of inactivity (1-30; Discord's default: 7).
    pub days: Option<u8>,
    /// Roles whose members may be pruned too (sent comma-separated).
    pub include_roles: Vec<RoleId>,
}

/// The body of [Begin Guild Prune](https://docs.discord.com/developers/resources/guild#begin-guild-prune).
///
/// The reason is the `reason` parameter of the method (the body field is deprecated).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct BeginGuildPrune {
    /// Days of inactivity (1-30; Discord's default: 7).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<u8>,
    /// Whether to return the number of pruned members (Discord's default: yes;
    /// discouraged for large guilds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_prune_count: Option<bool>,
    /// Roles whose members may be pruned too.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include_roles: Vec<RoleId>,
}

/// The body of [Modify Guild Widget](https://docs.discord.com/developers/resources/guild#modify-guild-widget).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyGuildWidget {
    /// Whether the widget is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// The channel the widget's invite leads to.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub channel_id: Patch<ChannelId>,
}

/// The response of [Get Guild Vanity URL](https://docs.discord.com/developers/resources/guild#get-guild-vanity-url).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VanityUrl {
    /// The vanity invite code (`discord.gg/<code>`), `None` when not set.
    pub code: Option<CompactString>,
    /// How many times it was used.
    pub uses: u32,
}

/// The body of [Modify Guild Welcome Screen](https://docs.discord.com/developers/resources/guild#modify-guild-welcome-screen).
///
/// Every field is optional and can be cleared.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyGuildWelcomeScreen {
    /// Whether the welcome screen is enabled.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub enabled: Patch<bool>,
    /// The suggested channels (all of them).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub welcome_channels: Patch<Vec<WelcomeScreenChannel>>,
    /// The guild description shown on the welcome screen.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub description: Patch<CompactString>,
}

/// The body of [Modify Guild Onboarding](https://docs.discord.com/developers/resources/guild#modify-guild-onboarding).
///
/// When enabled, onboarding needs at least 7 default channels, 5 of which `@everyone` can
/// send messages in (with the default [`mode`](Self::mode)).
///
/// discord.js counterpart: `GuildOnboardingEditOptions`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyGuildOnboarding {
    /// The prompts (all of them).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Vec<OnboardingPromptData>>,
    /// The channels new members get automatically.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_channel_ids: Option<Vec<ChannelId>>,
    /// Whether onboarding is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Which channels count towards the requirements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<OnboardingMode>,
}

/// An [onboarding prompt](https://docs.discord.com/developers/resources/guild#guild-onboarding-object-onboarding-prompt-structure)
/// in a request (D-118).
///
/// Discord requires an ID even for new prompts: a prompt without one gets an ID generated
/// from the current time when it is serialized, like discord.js does.
///
/// discord.js counterpart: `GuildOnboardingPromptData`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct OnboardingPromptData {
    /// The ID of an existing prompt; `None` for a new one.
    pub id: Option<GenericId>,
    /// How the options are shown.
    pub kind: PromptType,
    /// The options.
    pub options: Vec<PromptOptionData>,
    /// The title.
    pub title: CompactString,
    /// Whether members can only pick one option.
    pub single_select: bool,
    /// Whether members must answer before finishing onboarding.
    pub required: bool,
    /// Whether the prompt is part of onboarding (otherwise only in Channels & Roles).
    pub in_onboarding: bool,
}

impl OnboardingPromptData {
    /// A new multiple-choice prompt titled `title` with `options`, shown during onboarding.
    #[must_use]
    pub fn new(title: impl Into<CompactString>, options: Vec<PromptOptionData>) -> Self {
        Self {
            id: None,
            kind: PromptType::MultipleChoice,
            options,
            title: title.into(),
            single_select: false,
            required: false,
            in_onboarding: true,
        }
    }
}

impl From<OnboardingPrompt> for OnboardingPromptData {
    fn from(prompt: OnboardingPrompt) -> Self {
        let OnboardingPrompt {
            id,
            kind,
            options,
            title,
            single_select,
            required,
            in_onboarding,
            ..
        } = prompt;
        Self {
            id: Some(id),
            kind,
            options: options.into_iter().map(Into::into).collect(),
            title,
            single_select,
            required,
            in_onboarding,
        }
    }
}

impl Serialize for OnboardingPromptData {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            id: GenericId,
            #[serde(rename = "type")]
            kind: PromptType,
            options: &'a [PromptOptionData],
            title: &'a str,
            single_select: bool,
            required: bool,
            in_onboarding: bool,
        }

        // Destructured without `..`, so a new field cannot be forgotten here.
        let Self {
            id,
            kind,
            options,
            title,
            single_select,
            required,
            in_onboarding,
        } = self;
        Wire {
            id: id.unwrap_or_else(generated_id),
            kind: *kind,
            options,
            title,
            single_select: *single_select,
            required: *required,
            in_onboarding: *in_onboarding,
        }
        .serialize(serializer)
    }
}

/// A fresh, unique snowflake for a new onboarding prompt: the current time, plus a
/// process-wide counter in the increment bits so prompts serialized in the same millisecond
/// still differ.
fn generated_id() -> GenericId {
    static INCREMENT: AtomicU64 = AtomicU64::new(0);
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        });
    let base = GenericId::from_timestamp_ms(now_ms).map_or(1 << 22, GenericId::get);
    GenericId::new(base | (INCREMENT.fetch_add(1, Ordering::Relaxed) & 0xfff))
}

/// A [prompt option](https://docs.discord.com/developers/resources/guild#guild-onboarding-object-prompt-option-structure)
/// in a request (D-118): the emoji is given by its parts, as Discord requires here.
///
/// discord.js counterpart: `GuildOnboardingPromptOptionData`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct PromptOptionData {
    /// The ID of an existing option; `None` for a new one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<GenericId>,
    /// The channels a member is added to when they pick the option.
    pub channel_ids: Vec<ChannelId>,
    /// The roles a member gets when they pick the option.
    pub role_ids: Vec<RoleId>,
    /// The custom emoji's ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_id: Option<EmojiId>,
    /// The custom emoji's name, or the Unicode emoji.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<CompactString>,
    /// Whether the custom emoji is animated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_animated: Option<bool>,
    /// The title.
    pub title: CompactString,
    /// The description.
    pub description: Option<CompactString>,
}

impl PromptOptionData {
    /// A new option titled `title` that gives nothing yet.
    #[must_use]
    pub fn new(title: impl Into<CompactString>) -> Self {
        Self {
            id: None,
            channel_ids: Vec::new(),
            role_ids: Vec::new(),
            emoji_id: None,
            emoji_name: None,
            emoji_animated: None,
            title: title.into(),
            description: None,
        }
    }
}

impl From<PromptOption> for PromptOptionData {
    fn from(option: PromptOption) -> Self {
        let PromptOption {
            id,
            channel_ids,
            role_ids,
            emoji,
            title,
            description,
            ..
        } = option;
        let (emoji_id, emoji_name, emoji_animated) = match emoji {
            Some(PartialEmoji::Custom { id, name, animated }) => (Some(id), name, Some(animated)),
            Some(PartialEmoji::Unicode { name }) => (None, Some(name), None),
            None => (None, None, None),
        };
        Self {
            id: Some(id),
            channel_ids,
            role_ids,
            emoji_id,
            emoji_name,
            emoji_animated,
            title,
            description,
        }
    }
}

/// The body of [Modify Guild Incident Actions](https://docs.discord.com/developers/resources/guild#modify-guild-incident-actions).
///
/// Both actions last at most 24 hours; [`Patch::Clear`] ends one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyGuildIncidentActions {
    /// Until when invites are disabled.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub invites_disabled_until: Patch<Timestamp>,
    /// Until when direct messages between members are disabled.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub dms_disabled_until: Patch<Timestamp>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modify_guild_and_member() {
        let guild = ModifyGuild {
            name: Some("Rustaceans".into()),
            icon: Patch::Clear,
            preferred_locale: Patch::Set(Locale::Polish),
            ..ModifyGuild::default()
        };
        assert_eq!(
            serde_json::to_string(&guild).unwrap(),
            r#"{"name":"Rustaceans","icon":null,"preferred_locale":"pl"}"#
        );

        let member = ModifyGuildMember {
            nick: Patch::Clear,
            roles: Patch::Set(vec![RoleId::new(1)]),
            channel_id: Patch::Clear,
            ..ModifyGuildMember::default()
        };
        assert_eq!(
            serde_json::to_string(&member).unwrap(),
            r#"{"nick":null,"roles":["1"],"channel_id":null}"#
        );

        let add = AddGuildMember::new("super-secret");
        assert!(!format!("{add:?}").contains("super-secret"));
    }

    #[test]
    fn positions_and_roles() {
        let mut moved = ModifyGuildChannelPosition::new(ChannelId::new(2), 0);
        moved.parent_id = Patch::Clear;
        moved.lock_permissions = Patch::Set(true);
        assert_eq!(
            serde_json::to_string(&[moved]).unwrap(),
            r#"[{"id":"2","position":0,"lock_permissions":true,"parent_id":null}]"#
        );

        let role = CreateGuildRole {
            name: Some("mods".into()),
            colors: Some(RoleColors::solid(0x0000_ff00)),
            hoist: true,
            ..CreateGuildRole::default()
        };
        assert_eq!(
            serde_json::to_string(&role).unwrap(),
            concat!(
                r#"{"name":"mods","colors":{"primary_color":65280,"secondary_color":null,"#,
                r#""tertiary_color":null},"hoist":true}"#
            )
        );
        let modify = ModifyGuildRole {
            unicode_emoji: Patch::Clear,
            ..ModifyGuildRole::default()
        };
        assert_eq!(
            serde_json::to_string(&modify).unwrap(),
            r#"{"unicode_emoji":null}"#
        );
    }

    #[test]
    fn new_prompts_get_unique_generated_ids() {
        let mut option = PromptOptionData::new("Rust");
        option.emoji_name = Some("🦀".into());
        let prompt = OnboardingPromptData::new("Languages", vec![option]);
        let first: serde_json::Value = serde_json::to_value(&prompt).unwrap();
        let second: serde_json::Value = serde_json::to_value(&prompt).unwrap();
        assert_ne!(first["id"], second["id"]);
        let id: u64 = first["id"].as_str().unwrap().parse().unwrap();
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let id_ms = u128::from(GenericId::new(id).timestamp_ms());
        assert!(now_ms.abs_diff(id_ms) < 60_000, "{id_ms} vs {now_ms}");
        assert_eq!(
            first["options"],
            serde_json::json!([{
                "channel_ids": [], "role_ids": [], "emoji_name": "🦀", "title": "Rust",
                "description": null
            }])
        );

        let existing = OnboardingPromptData {
            id: Some(GenericId::new(42)),
            ..prompt
        };
        assert_eq!(serde_json::to_value(&existing).unwrap()["id"], "42");
    }
}
