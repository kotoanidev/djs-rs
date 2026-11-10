//! [Guilds](https://docs.discord.com/developers/resources/guild) (servers).

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    emoji::{Emoji, PartialEmoji},
    gateway::Status,
    id::{
        ApplicationId, ChannelId, EmojiId, GenericId, GuildId, IntegrationId, RoleId, SkuId, UserId,
    },
    macros::{int_enum, int_flags, str_enum},
    oauth2::Scope,
    permissions::Permissions,
    sticker::Sticker,
    user::{AvatarDecorationData, Collectibles, User},
    util::{ImageHash, Locale, Timestamp},
};

/// A guild that is not (or not yet) available to the client.
///
/// The [`Ready`](crate::gateway::payload::Ready) event lists every guild the bot is in as
/// unavailable; full guild data then arrives in `GUILD_CREATE` events. A guild also becomes
/// unavailable during a Discord outage.
///
/// See the [unavailable guild object](https://docs.discord.com/developers/resources/guild#unavailable-guild-object).
///
/// discord.js counterpart: guilds with `available === false`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UnavailableGuild {
    /// The guild's ID.
    pub id: GuildId,
    /// Whether the guild is unavailable due to an outage.
    ///
    /// In `GUILD_DELETE` events this is `false` (absent) when the bot was removed from the guild
    /// rather than the guild becoming unavailable.
    #[serde(default)]
    pub unavailable: bool,
}

/// A [guild member](https://docs.discord.com/developers/resources/guild#guild-member-object): a
/// user's membership in one guild, with guild-specific data such as nickname and roles.
///
/// This is the complete shape (it includes the [`user`](Self::user)), as returned by
/// `GET /guilds/{guild.id}/members/{user.id}` and member gateway events. Messages carry a
/// [`PartialMember`] instead (D-051).
///
/// discord.js counterpart: `GuildMember`.
///
/// # Examples
///
/// ```
/// use djs_model::guild::Member;
///
/// let member: Member = serde_json::from_str(r#"{
///     "user": {"id": "80351110224678912", "username": "nelly", "discriminator": "0",
///              "global_name": "Nelly", "avatar": null},
///     "nick": "NOT API SUPPORT", "roles": [], "joined_at": "2015-04-26T06:26:56.936000+00:00",
///     "deaf": false, "mute": false, "flags": 0
/// }"#)?;
/// assert_eq!(member.display_name(), "NOT API SUPPORT");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Member {
    /// The user this membership belongs to.
    pub user: User,
    /// The member's nickname in this guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nick: Option<CompactString>,
    /// The member's [guild avatar hash](https://docs.discord.com/developers/reference#image-formatting),
    /// overriding the user's avatar in this guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<ImageHash>,
    /// The member's [guild banner hash](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner: Option<ImageHash>,
    /// The IDs of the member's roles (the `@everyone` role is implied and not listed).
    pub roles: Vec<RoleId>,
    /// When the user joined the guild. `None` for guest members.
    pub joined_at: Option<Timestamp>,
    /// When the user started boosting the guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium_since: Option<Timestamp>,
    /// Whether the member is deafened in voice channels.
    #[serde(default)]
    pub deaf: bool,
    /// Whether the member is muted in voice channels.
    #[serde(default)]
    pub mute: bool,
    /// The member's [flags](MemberFlags).
    #[serde(default)]
    pub flags: MemberFlags,
    /// Whether the member has not yet passed the guild's membership screening. Such members
    /// cannot use most of the guild until they do.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub pending: bool,
    /// When the member's [timeout](https://support.discord.com/hc/en-us/articles/4413305239191)
    /// ends. A time in the past means the member is no longer timed out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub communication_disabled_until: Option<Timestamp>,
    /// The member's guild-specific avatar decoration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_decoration_data: Option<Box<AvatarDecorationData>>,
    /// The member's guild-specific collectibles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collectibles: Option<Box<Collectibles>>,
}

impl Member {
    /// Returns the name shown for the member in this guild: the nickname, else the user's
    /// display name, else the username.
    ///
    /// discord.js counterpart: `GuildMember#displayName`.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.nick
            .as_deref()
            .unwrap_or_else(|| self.user.display_name())
    }

    /// Returns whether the member is timed out at `now`.
    ///
    /// discord.js counterpart: `GuildMember#isCommunicationDisabled`.
    #[must_use]
    pub fn is_timed_out(&self, now: Timestamp) -> bool {
        self.communication_disabled_until
            .is_some_and(|until| until > now)
    }
}

/// A [guild member](https://docs.discord.com/developers/resources/guild#guild-member-object)
/// without its [`User`], as attached to messages (`MESSAGE_CREATE`, `MESSAGE_UPDATE`) where the
/// user is the message's author (D-051).
///
/// Combine it with the author through [`PartialMember::into_member`].
///
/// discord.js counterpart: the `member` of a `Message` (a `GuildMember`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PartialMember {
    /// The member's nickname in this guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nick: Option<CompactString>,
    /// The member's [guild avatar hash](https://docs.discord.com/developers/reference#image-formatting),
    /// overriding the user's avatar in this guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<ImageHash>,
    /// The member's [guild banner hash](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner: Option<ImageHash>,
    /// The IDs of the member's roles (the `@everyone` role is implied and not listed).
    pub roles: Vec<RoleId>,
    /// When the user joined the guild. `None` for guest members.
    pub joined_at: Option<Timestamp>,
    /// When the user started boosting the guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium_since: Option<Timestamp>,
    /// Whether the member is deafened in voice channels.
    #[serde(default)]
    pub deaf: bool,
    /// Whether the member is muted in voice channels.
    #[serde(default)]
    pub mute: bool,
    /// The member's [flags](MemberFlags).
    #[serde(default)]
    pub flags: MemberFlags,
    /// Whether the member has not yet passed the guild's membership screening. Such members
    /// cannot use most of the guild until they do.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub pending: bool,
    /// When the member's [timeout](https://support.discord.com/hc/en-us/articles/4413305239191)
    /// ends. A time in the past means the member is no longer timed out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub communication_disabled_until: Option<Timestamp>,
    /// The member's guild-specific avatar decoration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_decoration_data: Option<Box<AvatarDecorationData>>,
    /// The member's guild-specific collectibles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collectibles: Option<Box<Collectibles>>,
}

impl PartialMember {
    /// Completes this member with its user, typically the author of the message the member
    /// came with.
    #[must_use]
    pub fn into_member(self, user: User) -> Member {
        Member {
            user,
            nick: self.nick,
            avatar: self.avatar,
            banner: self.banner,
            roles: self.roles,
            joined_at: self.joined_at,
            premium_since: self.premium_since,
            deaf: self.deaf,
            mute: self.mute,
            flags: self.flags,
            pending: self.pending,
            communication_disabled_until: self.communication_disabled_until,
            avatar_decoration_data: self.avatar_decoration_data,
            collectibles: self.collectibles,
        }
    }
}

/// A [role](https://docs.discord.com/developers/topics/permissions#role-object): a named set of
/// permissions (and a color, icon and position in the member list) that members can have.
///
/// The `@everyone` role has the guild's ID.
///
/// discord.js counterpart: `Role`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Role {
    /// The role's ID.
    pub id: RoleId,
    /// The role's name.
    pub name: CompactString,
    /// The role's primary color as an RGB integer. **Deprecated** by Discord in favor of
    /// [`colors`](Self::colors), but still sent.
    pub color: u32,
    /// The role's colors (solid, gradient or holographic).
    pub colors: RoleColors,
    /// Whether members with this role are listed separately in the member list.
    pub hoist: bool,
    /// The [role icon hash](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<ImageHash>,
    /// The role's Unicode emoji, shown instead of an icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unicode_emoji: Option<CompactString>,
    /// The role's position in the role list; roles with equal positions are ordered by ID.
    pub position: i32,
    /// The permissions the role grants.
    pub permissions: Permissions,
    /// Whether the role is managed by an integration (bot role, booster role, …).
    pub managed: bool,
    /// Whether everyone can mention the role.
    pub mentionable: bool,
    /// What the role is tied to (a bot, an integration, boosting, a subscription, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Box<RoleTags>>,
    /// The role's [flags](RoleFlags).
    #[serde(default)]
    pub flags: RoleFlags,
}

/// The [colors of a role](https://docs.discord.com/developers/topics/permissions#role-object-role-colors-object).
///
/// A role without colors has `primary_color == 0` and does not affect the member's color.
/// Secondary and tertiary colors need the guild feature `ENHANCED_ROLE_COLORS`.
///
/// discord.js counterpart: `RoleColors` (`Role#colors`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoleColors {
    /// The primary color as an RGB integer.
    pub primary_color: u32,
    /// The secondary color: makes the role a gradient between the colors.
    pub secondary_color: Option<u32>,
    /// The tertiary color: makes the gradient holographic.
    pub tertiary_color: Option<u32>,
}

impl RoleColors {
    /// The only colors Discord accepts for a holographic role (when a tertiary color is sent).
    ///
    /// discord.js counterpart: `Constants.HolographicStyle`.
    pub const HOLOGRAPHIC: Self = Self {
        primary_color: 11_127_295,
        secondary_color: Some(16_759_788),
        tertiary_color: Some(16_761_760),
    };

    /// A solid color.
    #[must_use]
    pub const fn solid(color: u32) -> Self {
        Self {
            primary_color: color,
            secondary_color: None,
            tertiary_color: None,
        }
    }
}

/// The [tags of a role](https://docs.discord.com/developers/topics/permissions#role-object-role-tags-structure):
/// what the role belongs to.
///
/// On the wire, the boolean tags are keys whose value is `null` when true and that are absent
/// when false; here they are plain `bool`s.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RoleTags {
    /// The bot this role belongs to (the bot's managed role).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot_id: Option<UserId>,
    /// The integration this role belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration_id: Option<IntegrationId>,
    /// Whether this is the guild's Booster role.
    #[serde(
        default,
        with = "null_flag",
        skip_serializing_if = "core::ops::Not::not"
    )]
    pub premium_subscriber: bool,
    /// The SKU and listing of the role's subscription.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_listing_id: Option<SkuId>,
    /// Whether the role can be purchased.
    #[serde(
        default,
        with = "null_flag",
        skip_serializing_if = "core::ops::Not::not"
    )]
    pub available_for_purchase: bool,
    /// Whether the role is a guild's linked role.
    #[serde(
        default,
        with = "null_flag",
        skip_serializing_if = "core::ops::Not::not"
    )]
    pub guild_connections: bool,
}

/// (De)serializes a "null means true" flag: the key is present with a `null` value when the
/// flag is set and absent otherwise (absence is handled by `#[serde(default)]` and
/// `skip_serializing_if`).
mod null_flag {
    use serde::{Deserialize, Deserializer, Serializer};

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "signature required by serde(with)"
    )]
    pub(super) fn serialize<S: Serializer>(_: &bool, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<bool, D::Error> {
        Option::<()>::deserialize(deserializer).map(|_| true)
    }
}

/// A [guild](https://docs.discord.com/developers/resources/guild#guild-object) (called a
/// "server" in the Discord client).
///
/// This is the shape returned by `GET /guilds/{guild.id}` and sent in `GUILD_UPDATE`. The
/// `GUILD_CREATE` event carries additional data (members, channels, …) in its own payload type.
///
/// discord.js counterpart: `Guild`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Guild {
    /// The guild's ID.
    pub id: GuildId,
    /// The guild's name (2-100 characters).
    pub name: CompactString,
    /// The [icon hash](https://docs.discord.com/developers/reference#image-formatting).
    pub icon: Option<ImageHash>,
    /// The icon hash as sent in guild template objects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_hash: Option<ImageHash>,
    /// The [invite splash hash](https://docs.discord.com/developers/reference#image-formatting).
    pub splash: Option<ImageHash>,
    /// The [discovery splash hash](https://docs.discord.com/developers/reference#image-formatting);
    /// only for guilds with the [`DISCOVERABLE`](Feature::Discoverable) feature.
    pub discovery_splash: Option<ImageHash>,
    /// Whether the current user owns the guild (only in `GET /users/@me/guilds`).
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub owner: bool,
    /// The owner's user ID.
    pub owner_id: UserId,
    /// The current user's permissions in the guild, without channel overwrites (only in
    /// `GET /users/@me/guilds`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
    /// The guild's voice region. **Deprecated** by Discord: regions are set per voice channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<CompactString>,
    /// The AFK voice channel.
    pub afk_channel_id: Option<ChannelId>,
    /// How long a member must be idle before being moved to the AFK channel, in seconds.
    pub afk_timeout: u32,
    /// Whether the server widget is enabled.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub widget_enabled: bool,
    /// The channel the widget's invite points to; `None` when the widget has no invite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget_channel_id: Option<ChannelId>,
    /// What members must have before they can talk in the guild.
    pub verification_level: VerificationLevel,
    /// The default notification setting for members.
    pub default_message_notifications: DefaultMessageNotificationLevel,
    /// Whose media Discord scans for explicit content.
    pub explicit_content_filter: ExplicitContentFilterLevel,
    /// The guild's roles.
    pub roles: Vec<Role>,
    /// The guild's custom emojis.
    pub emojis: Vec<Emoji>,
    /// The guild's enabled [features](Feature).
    pub features: Vec<Feature>,
    /// Whether moderators need two-factor authentication.
    pub mfa_level: MfaLevel,
    /// The application that created the guild, for bot-created guilds.
    pub application_id: Option<ApplicationId>,
    /// The channel where welcome messages and boost notices are posted.
    pub system_channel_id: Option<ChannelId>,
    /// Which notices are *not* posted in the system channel.
    pub system_channel_flags: SystemChannelFlags,
    /// The rules channel of a Community guild.
    pub rules_channel_id: Option<ChannelId>,
    /// The maximum number of presences (`None` except for the largest guilds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_presences: Option<u32>,
    /// The maximum number of members.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_members: Option<u32>,
    /// The vanity invite code (`discord.gg/<code>`).
    pub vanity_url_code: Option<CompactString>,
    /// The guild's description.
    pub description: Option<CompactString>,
    /// The [banner hash](https://docs.discord.com/developers/reference#image-formatting).
    pub banner: Option<ImageHash>,
    /// The Server Boost level.
    pub premium_tier: PremiumTier,
    /// The number of boosts the guild currently has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium_subscription_count: Option<u32>,
    /// The preferred locale of a Community guild (used in discovery and Discord notices).
    pub preferred_locale: Locale,
    /// The channel where moderators of a Community guild receive notices from Discord.
    pub public_updates_channel_id: Option<ChannelId>,
    /// The maximum number of users in a video channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_video_channel_users: Option<u32>,
    /// The maximum number of users in a stage video channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_stage_video_channel_users: Option<u32>,
    /// The approximate number of members (`GET /guilds/{guild.id}?with_counts=true`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approximate_member_count: Option<u32>,
    /// The approximate number of online members (`with_counts=true`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approximate_presence_count: Option<u32>,
    /// The welcome screen of a Community guild (sent with invites).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub welcome_screen: Option<Box<WelcomeScreen>>,
    /// The guild's age-restriction (NSFW) level. Defaults to
    /// [`Default`](AgeRestrictionLevel::Default) when absent (older payloads).
    #[serde(default)]
    pub nsfw_level: AgeRestrictionLevel,
    /// The guild's custom stickers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stickers: Vec<Sticker>,
    /// Whether the boost progress bar is shown.
    #[serde(default)]
    pub premium_progress_bar_enabled: bool,
    /// The channel where moderators of a Community guild receive safety alerts.
    pub safety_alerts_channel_id: Option<ChannelId>,
    /// Active safety incidents (paused invites or DMs, detected raids or DM spam).
    pub incidents_data: Option<Box<IncidentsData>>,
}

int_enum! {
    /// The [default notification level](https://docs.discord.com/developers/resources/guild#guild-object-default-message-notification-level)
    /// of a guild's members.
    ///
    /// discord.js counterpart: `GuildDefaultMessageNotifications`.
    pub enum DefaultMessageNotificationLevel: u8 {
        /// Members are notified of every message.
        AllMessages = 0,
        /// Members are only notified of messages that mention them.
        OnlyMentions = 1,
    }
}

int_enum! {
    /// The [explicit content filter level](https://docs.discord.com/developers/resources/guild#guild-object-explicit-content-filter-level):
    /// whose media is scanned.
    ///
    /// discord.js counterpart: `GuildExplicitContentFilter`.
    pub enum ExplicitContentFilterLevel: u8 {
        /// No scanning.
        Disabled = 0,
        /// Media sent by members without roles is scanned.
        MembersWithoutRoles = 1,
        /// Media sent by all members is scanned.
        AllMembers = 2,
    }
}

int_enum! {
    /// The [MFA level](https://docs.discord.com/developers/resources/guild#guild-object-mfa-level)
    /// required for moderation actions.
    ///
    /// discord.js counterpart: `GuildMFALevel`.
    pub enum MfaLevel: u8 {
        /// No two-factor authentication requirement.
        None = 0,
        /// Moderators need two-factor authentication.
        Elevated = 1,
    }
}

int_enum! {
    /// The [verification level](https://docs.discord.com/developers/resources/guild#guild-object-verification-level)
    /// members must meet before they can talk.
    ///
    /// discord.js counterpart: `GuildVerificationLevel`.
    pub enum VerificationLevel: u8 {
        /// Unrestricted.
        None = 0,
        /// A verified email address.
        Low = 1,
        /// Registered on Discord for more than 5 minutes.
        Medium = 2,
        /// Member of the guild for more than 10 minutes.
        High = 3,
        /// A verified phone number.
        VeryHigh = 4,
    }
}

int_enum! {
    /// The [age-restriction level](https://docs.discord.com/developers/resources/guild#guild-object-guild-nsfw-level)
    /// of a guild (`nsfw_level`).
    ///
    /// discord.js counterpart: `GuildNSFWLevel`.
    #[doc(alias = "NsfwLevel")]
    #[derive(Default)]
    pub enum AgeRestrictionLevel: u8 {
        /// Not rated.
        #[default]
        Default = 0,
        /// Explicit content.
        Explicit = 1,
        /// Safe for everyone.
        Safe = 2,
        /// Age restricted.
        AgeRestricted = 3,
    }
}

int_enum! {
    /// The [premium tier](https://docs.discord.com/developers/resources/guild#guild-object-premium-tier)
    /// (Server Boost level) of a guild.
    ///
    /// discord.js counterpart: `GuildPremiumTier`.
    pub enum PremiumTier: u8 {
        /// No Server Boost perks.
        None = 0,
        /// Server Boost level 1.
        Tier1 = 1,
        /// Server Boost level 2.
        Tier2 = 2,
        /// Server Boost level 3.
        Tier3 = 3,
    }
}

str_enum! {
    /// A [guild feature](https://docs.discord.com/developers/resources/guild#guild-object-guild-features):
    /// something a guild has access to or has enabled.
    ///
    /// Bots can toggle `COMMUNITY`, `DISCOVERABLE`, `INVITES_DISABLED` and
    /// `RAID_ALERTS_DISABLED` when modifying a guild (with the required permissions).
    ///
    /// discord.js counterpart: `GuildFeature`.
    #[doc(alias = "GuildFeature")]
    pub enum Feature {
        /// Can set an animated banner.
        AnimatedBanner = "ANIMATED_BANNER",
        /// Can set an animated icon.
        AnimatedIcon = "ANIMATED_ICON",
        /// Uses the old application command permissions behavior.
        ApplicationCommandPermissionsV2 = "APPLICATION_COMMAND_PERMISSIONS_V2",
        /// Has auto moderation rules set up.
        AutoModeration = "AUTO_MODERATION",
        /// Can set a banner.
        Banner = "BANNER",
        /// A Community guild: welcome screen, membership screening, stage channels, discovery
        /// and community updates.
        Community = "COMMUNITY",
        /// Has enabled monetization.
        CreatorMonetizableProvisional = "CREATOR_MONETIZABLE_PROVISIONAL",
        /// Has enabled the role subscription promo page.
        CreatorStorePage = "CREATOR_STORE_PAGE",
        /// Is the support server of an app in the App Directory.
        DeveloperSupportServer = "DEVELOPER_SUPPORT_SERVER",
        /// Can be discovered in the directory.
        Discoverable = "DISCOVERABLE",
        /// Can give roles gradient colors.
        EnhancedRoleColors = "ENHANCED_ROLE_COLORS",
        /// Can be featured in the directory.
        Featurable = "FEATURABLE",
        /// Has access to guest invites.
        GuestsEnabled = "GUESTS_ENABLED",
        /// Can set guild (server) tags.
        GuildTags = "GUILD_TAGS",
        /// Is listed in a directory channel.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may change
        /// without notice.
        HasDirectoryEntry = "HAS_DIRECTORY_ENTRY",
        /// Is a Student Hub.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may change
        /// without notice.
        Hub = "HUB",
        /// Has paused invites; no new users can join.
        InvitesDisabled = "INVITES_DISABLED",
        /// Can set an invite splash background.
        InviteSplash = "INVITE_SPLASH",
        /// Is part of a Student Hub.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may change
        /// without notice.
        LinkedToHub = "LINKED_TO_HUB",
        /// Has enabled membership screening.
        MemberVerificationGateEnabled = "MEMBER_VERIFICATION_GATE_ENABLED",
        /// Has enabled monetization.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may change
        /// without notice.
        MonetizationEnabled = "MONETIZATION_ENABLED",
        /// Has more custom soundboard sound slots.
        MoreSoundboard = "MORE_SOUNDBOARD",
        /// Has more custom sticker slots.
        MoreStickers = "MORE_STICKERS",
        /// Can create announcement channels.
        News = "NEWS",
        /// Is partnered.
        Partnered = "PARTNERED",
        /// Has completed the migration to the separate pin-messages permission.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may change
        /// without notice.
        PinPermissionMigrationComplete = "PIN_PERMISSION_MIGRATION_COMPLETE",
        /// Can be previewed before joining (membership screening or the directory).
        PreviewEnabled = "PREVIEW_ENABLED",
        /// Can create private threads.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may change
        /// without notice.
        PrivateThreads = "PRIVATE_THREADS",
        /// Pruning members requires the administrator permission.
        PruneRequiresAdmin = "PRUNE_REQUIRES_ADMIN",
        /// Has disabled raid alerts in the safety alerts channel.
        RaidAlertsDisabled = "RAID_ALERTS_DISABLED",
        /// Uses Discord's relay (sharded gateway) infrastructure.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may change
        /// without notice.
        RelayEnabled = "RELAY_ENABLED",
        /// Can set role icons.
        RoleIcons = "ROLE_ICONS",
        /// Has role subscriptions that can be purchased.
        RoleSubscriptionsAvailableForPurchase = "ROLE_SUBSCRIPTIONS_AVAILABLE_FOR_PURCHASE",
        /// Has enabled role subscriptions.
        RoleSubscriptionsEnabled = "ROLE_SUBSCRIPTIONS_ENABLED",
        /// Has created soundboard sounds.
        Soundboard = "SOUNDBOARD",
        /// Has enabled ticketed events.
        TicketedEventsEnabled = "TICKETED_EVENTS_ENABLED",
        /// Can set a vanity URL.
        VanityUrl = "VANITY_URL",
        /// Is verified.
        Verified = "VERIFIED",
        /// Can use 384 kbps voice bitrate (formerly VIP voice servers).
        VipRegions = "VIP_REGIONS",
        /// Has enabled the welcome screen.
        WelcomeScreenEnabled = "WELCOME_SCREEN_ENABLED",
    }
}

/// The [preview of a guild](https://docs.discord.com/developers/resources/guild#guild-preview-object),
/// visible to non-members of discoverable guilds.
///
/// discord.js counterpart: `GuildPreview`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "GuildPreview")]
pub struct Preview {
    /// The guild's ID.
    pub id: GuildId,
    /// The guild's name.
    pub name: CompactString,
    /// The [icon hash](https://docs.discord.com/developers/reference#image-formatting).
    pub icon: Option<ImageHash>,
    /// The [invite splash hash](https://docs.discord.com/developers/reference#image-formatting).
    pub splash: Option<ImageHash>,
    /// The [discovery splash hash](https://docs.discord.com/developers/reference#image-formatting).
    pub discovery_splash: Option<ImageHash>,
    /// The guild's custom emojis.
    pub emojis: Vec<Emoji>,
    /// The guild's enabled features.
    pub features: Vec<Feature>,
    /// The approximate number of members.
    pub approximate_member_count: u32,
    /// The approximate number of online members.
    pub approximate_presence_count: u32,
    /// The guild's description.
    pub description: Option<CompactString>,
    /// The guild's custom stickers.
    pub stickers: Vec<Sticker>,
}

/// The [widget settings](https://docs.discord.com/developers/resources/guild#guild-widget-settings-object)
/// of a guild.
///
/// discord.js counterpart: `GuildWidgetSettings`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WidgetSettings {
    /// Whether the widget is enabled.
    pub enabled: bool,
    /// The channel the widget's invite points to.
    pub channel_id: Option<ChannelId>,
}

/// The public [widget](https://docs.discord.com/developers/resources/guild#guild-widget-object)
/// of a guild (`GET /guilds/{guild.id}/widget.json`).
///
/// discord.js counterpart: `Widget`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Widget {
    /// The guild's ID.
    pub id: GuildId,
    /// The guild's name.
    pub name: CompactString,
    /// An instant invite to the widget's channel.
    pub instant_invite: Option<CompactString>,
    /// The voice and stage channels everyone can access.
    pub channels: Vec<WidgetChannel>,
    /// Up to 100 online members, anonymized.
    pub members: Vec<WidgetMember>,
    /// The number of online members.
    pub presence_count: u32,
}

/// A channel listed in a [`Widget`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WidgetChannel {
    /// The channel's ID.
    pub id: ChannelId,
    /// The channel's name.
    pub name: CompactString,
    /// The channel's position.
    pub position: i32,
}

/// An online member listed in a [`Widget`]. Members are anonymized: the ID is an index, not a
/// user ID.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WidgetMember {
    /// An anonymous index (`"0"`, `"1"`, …).
    pub id: CompactString,
    /// The member's username.
    pub username: CompactString,
    /// The member's discriminator (anonymized as `"0000"`).
    pub discriminator: CompactString,
    /// The member's avatar hash (anonymized as `None`).
    pub avatar: Option<ImageHash>,
    /// The member's online status.
    pub status: Status,
    /// What the member is doing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<WidgetActivity>,
    /// A URL of the member's avatar image.
    pub avatar_url: CompactString,
}

/// The activity of a [`WidgetMember`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WidgetActivity {
    /// The activity's name.
    pub name: CompactString,
}

str_enum! {
    /// The [style of a widget image](https://docs.discord.com/developers/resources/guild#get-guild-widget-image-widget-style-options)
    /// (`GET /guilds/{guild.id}/widget.png?style=`).
    ///
    /// discord.js counterpart: `GuildWidgetStyle`.
    pub enum WidgetStyle {
        /// A shield with the Discord icon and the online member count.
        Shield = "shield",
        /// A large image with the guild icon, name and online count, "POWERED BY DISCORD" below.
        Banner1 = "banner1",
        /// A smaller image with the guild icon, name and online count, the Discord logo on the
        /// right.
        Banner2 = "banner2",
        /// A large image with the guild icon, name and online count, with a "Chat Now" button.
        Banner3 = "banner3",
        /// A large Discord logo with the guild icon, name and online count, and a large
        /// "JOIN MY SERVER" button.
        Banner4 = "banner4",
    }
}

/// An [integration](https://docs.discord.com/developers/resources/guild#integration-object)
/// in a guild: a bot or app, or a Twitch/YouTube subscription.
///
/// Fields marked "not for Discord bots" are absent for `discord` integrations.
///
/// discord.js counterpart: `Integration`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Integration {
    /// The integration's ID.
    pub id: IntegrationId,
    /// The integration's name.
    pub name: CompactString,
    /// The kind of integration.
    #[serde(rename = "type")]
    pub kind: IntegrationType,
    /// Whether the integration is enabled.
    pub enabled: bool,
    /// Whether the integration is syncing (not for Discord bots).
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub syncing: bool,
    /// The role given to subscribers (not for Discord bots).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_id: Option<RoleId>,
    /// Whether emoticons are synced (Twitch only).
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub enable_emoticons: bool,
    /// What happens to expired subscribers (not for Discord bots).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expire_behavior: Option<IntegrationExpireBehavior>,
    /// The grace period before subscribers expire, in days (not for Discord bots).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expire_grace_period: Option<u32>,
    /// The user who added the integration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<Box<User>>,
    /// The integration's account.
    pub account: IntegrationAccount,
    /// When the integration last synced (not for Discord bots).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synced_at: Option<Timestamp>,
    /// The number of subscribers (not for Discord bots).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscriber_count: Option<u32>,
    /// Whether the integration was revoked (not for Discord bots).
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub revoked: bool,
    /// The bot or `OAuth2` application of a Discord integration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application: Option<Box<IntegrationApplication>>,
    /// The scopes the application was authorized for.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<Scope>,
}

str_enum! {
    /// The kind of an [`Integration`].
    pub enum IntegrationType {
        /// A Twitch subscription.
        Twitch = "twitch",
        /// A YouTube membership.
        Youtube = "youtube",
        /// A Discord bot or `OAuth2` app.
        Discord = "discord",
        /// A guild (role) subscription.
        GuildSubscription = "guild_subscription",
    }
}

int_enum! {
    /// [What happens to expired subscribers](https://docs.discord.com/developers/resources/guild#integration-object-integration-expire-behaviors)
    /// of an integration.
    ///
    /// discord.js counterpart: `IntegrationExpireBehavior`.
    pub enum IntegrationExpireBehavior: u8 {
        /// The subscriber role is removed.
        RemoveRole = 0,
        /// The member is kicked.
        Kick = 1,
    }
}

/// The [account of an integration](https://docs.discord.com/developers/resources/guild#integration-account-object)
/// (e.g. the Twitch channel).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IntegrationAccount {
    /// The account's ID on the platform.
    pub id: CompactString,
    /// The account's name.
    pub name: CompactString,
}

/// The [application of a Discord integration](https://docs.discord.com/developers/resources/guild#integration-application-object).
///
/// discord.js counterpart: `IntegrationApplication`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IntegrationApplication {
    /// The application's ID.
    pub id: ApplicationId,
    /// The application's name.
    pub name: CompactString,
    /// The [icon hash](https://docs.discord.com/developers/reference#image-formatting).
    pub icon: Option<ImageHash>,
    /// The application's description.
    pub description: CompactString,
    /// The application's bot user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot: Option<User>,
}

/// A [ban](https://docs.discord.com/developers/resources/guild#ban-object).
///
/// discord.js counterpart: `GuildBan`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Ban {
    /// The reason given for the ban.
    pub reason: Option<CompactString>,
    /// The banned user.
    pub user: User,
}

/// The [welcome screen](https://docs.discord.com/developers/resources/guild#welcome-screen-object)
/// of a Community guild, shown to new members.
///
/// discord.js counterpart: `WelcomeScreen`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WelcomeScreen {
    /// The guild description shown on the welcome screen.
    pub description: Option<CompactString>,
    /// Up to 5 suggested channels.
    pub welcome_channels: Vec<WelcomeScreenChannel>,
}

/// A [channel suggested on the welcome screen](https://docs.discord.com/developers/resources/guild#welcome-screen-object-welcome-screen-channel-structure).
///
/// discord.js counterpart: `WelcomeChannel`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WelcomeScreenChannel {
    /// The channel's ID.
    pub channel_id: ChannelId,
    /// The description shown for the channel.
    pub description: CompactString,
    /// The custom emoji's ID, for a custom emoji.
    pub emoji_id: Option<EmojiId>,
    /// The custom emoji's name, the Unicode emoji itself, or `None` without an emoji.
    pub emoji_name: Option<CompactString>,
}

/// The [onboarding](https://docs.discord.com/developers/resources/guild#guild-onboarding-object)
/// flow of a guild: questions that give new members channels and roles.
///
/// discord.js counterpart: `GuildOnboarding`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "GuildOnboarding")]
pub struct Onboarding {
    /// The guild's ID.
    pub guild_id: GuildId,
    /// The questions shown in onboarding and in "Channels & Roles".
    pub prompts: Vec<OnboardingPrompt>,
    /// Channels members are opted into automatically.
    pub default_channel_ids: Vec<ChannelId>,
    /// Whether onboarding is enabled.
    pub enabled: bool,
    /// Which channels count towards onboarding constraints. Defaults to
    /// [`Default`](OnboardingMode::Default) when absent (older payloads).
    #[serde(default)]
    pub mode: OnboardingMode,
}

/// A [question in onboarding](https://docs.discord.com/developers/resources/guild#guild-onboarding-object-onboarding-prompt-structure).
///
/// discord.js counterpart: `GuildOnboardingPrompt`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OnboardingPrompt {
    /// The prompt's ID.
    pub id: GenericId,
    /// How the prompt is displayed.
    #[serde(rename = "type")]
    pub kind: PromptType,
    /// The answers to choose from.
    pub options: Vec<PromptOption>,
    /// The question.
    pub title: CompactString,
    /// Whether only one answer can be chosen.
    pub single_select: bool,
    /// Whether the prompt must be answered to finish onboarding.
    pub required: bool,
    /// Whether the prompt is part of onboarding; if not, it only appears in "Channels & Roles".
    pub in_onboarding: bool,
}

/// An [answer of an onboarding prompt](https://docs.discord.com/developers/resources/guild#guild-onboarding-object-prompt-option-structure).
///
/// When creating or editing an option, Discord expects `emoji_id`/`emoji_name`/`emoji_animated`
/// instead of the [`emoji`](Self::emoji) object received here.
///
/// discord.js counterpart: `GuildOnboardingPromptOption`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PromptOption {
    /// The option's ID.
    pub id: GenericId,
    /// Channels the member is added to when choosing this option.
    pub channel_ids: Vec<ChannelId>,
    /// Roles the member gets when choosing this option.
    pub role_ids: Vec<RoleId>,
    /// The option's emoji.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<PartialEmoji>,
    /// The option's title.
    pub title: CompactString,
    /// The option's description.
    pub description: Option<CompactString>,
}

int_enum! {
    /// The [onboarding mode](https://docs.discord.com/developers/resources/guild#guild-onboarding-object-onboarding-mode):
    /// what counts towards the onboarding constraints.
    ///
    /// discord.js counterpart: `GuildOnboardingMode`.
    #[derive(Default)]
    pub enum OnboardingMode: u8 {
        /// Only default channels count.
        #[default]
        Default = 0,
        /// Default channels and questions count.
        Advanced = 1,
    }
}

int_enum! {
    /// How an [`OnboardingPrompt`] is displayed.
    ///
    /// discord.js counterpart: `GuildOnboardingPromptType`.
    pub enum PromptType: u8 {
        /// Multiple choice buttons.
        MultipleChoice = 0,
        /// A dropdown.
        Dropdown = 1,
    }
}

/// [Safety incidents](https://docs.discord.com/developers/resources/guild#incidents-data-object)
/// of a guild: paused invites or DMs and detected raids or DM spam.
///
/// discord.js counterpart: `IncidentActions`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IncidentsData {
    /// When invites are enabled again.
    pub invites_disabled_until: Option<Timestamp>,
    /// When direct messages between members are enabled again.
    pub dms_disabled_until: Option<Timestamp>,
    /// When DM spam was detected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dm_spam_detected_at: Option<Timestamp>,
    /// When a raid was detected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raid_detected_at: Option<Timestamp>,
}

int_flags! {
    /// [Flags of a guild's system channel](https://docs.discord.com/developers/resources/guild#guild-object-system-channel-flags):
    /// which automatic messages Discord does **not** post there.
    ///
    /// discord.js counterpart: `GuildSystemChannelFlags` / `SystemChannelFlagsBitField`.
    pub struct SystemChannelFlags: u64 {
        /// No message when a member joins.
        const SUPPRESS_JOIN_NOTIFICATIONS = 1 << 0;
        /// No message when the guild is boosted.
        const SUPPRESS_PREMIUM_SUBSCRIPTIONS = 1 << 1;
        /// No server setup tips.
        const SUPPRESS_GUILD_REMINDER_NOTIFICATIONS = 1 << 2;
        /// No sticker reply button on join messages.
        const SUPPRESS_JOIN_NOTIFICATION_REPLIES = 1 << 3;
        /// No message when a role subscription is purchased or renewed.
        const SUPPRESS_ROLE_SUBSCRIPTION_PURCHASE_NOTIFICATIONS = 1 << 4;
        /// No sticker reply button on role subscription messages.
        const SUPPRESS_ROLE_SUBSCRIPTION_PURCHASE_NOTIFICATION_REPLIES = 1 << 5;
    }
}

int_flags! {
    /// [Flags of a guild member](https://docs.discord.com/developers/resources/guild#guild-member-object-guild-member-flags).
    ///
    /// Only [`BYPASSES_VERIFICATION`](Self::BYPASSES_VERIFICATION) can be changed by bots
    /// (when modifying a member); the others are set by Discord.
    ///
    /// discord.js counterpart: `GuildMemberFlags` / `GuildMemberFlagsBitField`.
    pub struct MemberFlags: u64 {
        /// The member left the guild and joined again.
        const DID_REJOIN = 1 << 0;
        /// The member completed onboarding.
        const COMPLETED_ONBOARDING = 1 << 1;
        /// The member is exempt from the guild's verification requirements. Editable.
        const BYPASSES_VERIFICATION = 1 << 2;
        /// The member started onboarding.
        const STARTED_ONBOARDING = 1 << 3;
        /// The member is a guest who can only access the voice channel they were invited to.
        const IS_GUEST = 1 << 4;
        /// The member started the Server Guide new-member actions.
        const STARTED_HOME_ACTIONS = 1 << 5;
        /// The member completed the Server Guide new-member actions.
        const COMPLETED_HOME_ACTIONS = 1 << 6;
        /// AutoMod blocks the member's username, display name or nickname.
        const AUTOMOD_QUARANTINED_USERNAME = 1 << 7;
        /// AutoMod blocks the member's bio.
        ///
        /// **Deprecated** by Discord.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const AUTOMOD_QUARANTINED_BIO = 1 << 8;
        /// The member dismissed the DM settings upsell.
        const DM_SETTINGS_UPSELL_ACKNOWLEDGED = 1 << 9;
        /// AutoMod blocks the member's guild tag.
        const AUTOMOD_QUARANTINED_GUILD_TAG = 1 << 10;
    }
}

int_flags! {
    /// [Flags of a role](https://docs.discord.com/developers/topics/permissions#role-object-role-flags).
    ///
    /// discord.js counterpart: `RoleFlags` / `RoleFlagsBitField`.
    pub struct RoleFlags: u64 {
        /// Members can pick the role in an onboarding prompt.
        const IN_PROMPT = 1 << 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const USER: &str = r#"{"id":"80351110224678912","username":"nelly","discriminator":"0","global_name":"Nelly","avatar":null}"#;

    #[test]
    fn member_from_documented_example() {
        // The docs example with its empty `user` filled in.
        let json = format!(
            r#"{{"user":{USER},"nick":"NOT API SUPPORT","avatar":null,"banner":null,"roles":[],
                "joined_at":"2015-04-26T06:26:56.936000+00:00","deaf":false,"mute":false}}"#
        );
        let member: Member = serde_json::from_str(&json).unwrap();
        assert_eq!(member.user.id.get(), 80_351_110_224_678_912);
        assert_eq!(member.nick.as_deref(), Some("NOT API SUPPORT"));
        assert_eq!(member.avatar, None);
        assert_eq!(
            member.joined_at.unwrap().to_string(),
            "2015-04-26T06:26:56.936000+00:00"
        );
        assert_eq!(member.flags, MemberFlags::empty());
        assert!(!member.pending);
        assert_eq!(member.display_name(), "NOT API SUPPORT");
    }

    #[test]
    fn partial_member_from_message_create() {
        // The `member` of a real MESSAGE_CREATE payload: no `user`.
        let json = r#"{"roles":["1100000000000000001"],"premium_since":null,"pending":false,
            "nick":null,"mute":false,"joined_at":"2024-05-01T12:00:00.000000+00:00","flags":2,
            "deaf":false,"communication_disabled_until":"2026-11-10T20:00:00.000000+00:00",
            "banner":null,"avatar":null}"#;
        let partial: PartialMember = serde_json::from_str(json).unwrap();
        assert_eq!(partial.roles.len(), 1);
        assert_eq!(partial.flags, MemberFlags::COMPLETED_ONBOARDING);
        let user: User = serde_json::from_str(USER).unwrap();
        let member = partial.into_member(user);
        assert_eq!(member.display_name(), "Nelly");
        let before: Timestamp = "2026-11-10T19:00:00.000000+00:00".parse().unwrap();
        let after: Timestamp = "2026-11-10T21:00:00.000000+00:00".parse().unwrap();
        assert!(member.is_timed_out(before));
        assert!(!member.is_timed_out(after));
    }

    #[test]
    fn role_from_documented_example() {
        let json = r#"{"id":"41771983423143936","name":"WE DEM BOYZZ!!!!!!","color":3447003,
            "colors":{"primary_color":3447003,"secondary_color":null,"tertiary_color":null},
            "hoist":true,"icon":"cf3ced8600b777c9486c6d8d84fb4327","unicode_emoji":null,
            "position":1,"permissions":"66321471","managed":false,"mentionable":false,"flags":0}"#;
        let role: Role = serde_json::from_str(json).unwrap();
        assert_eq!(role.colors, RoleColors::solid(3_447_003));
        assert_eq!(role.permissions.bits(), 66_321_471);
        assert!(role.permissions.contains(Permissions::SEND_MESSAGES));
        assert_eq!(role.position, 1);
        assert!(role.tags.is_none());
        assert_eq!(
            role.icon.unwrap().to_string(),
            "cf3ced8600b777c9486c6d8d84fb4327"
        );
    }

    #[test]
    fn role_tags_null_flags() {
        let json = r#"{"bot_id":"1","premium_subscriber":null,"guild_connections":null}"#;
        let tags: RoleTags = serde_json::from_str(json).unwrap();
        assert_eq!(tags.bot_id.map(crate::id::Id::get), Some(1));
        assert!(tags.premium_subscriber);
        assert!(tags.guild_connections);
        assert!(!tags.available_for_purchase);
        // Set flags are written back as `null`, unset flags are omitted.
        assert_eq!(serde_json::to_string(&tags).unwrap(), json);
    }

    #[test]
    fn holographic_colors_serialize_like_the_docs() {
        assert_eq!(
            serde_json::to_string(&RoleColors::HOLOGRAPHIC).unwrap(),
            r#"{"primary_color":11127295,"secondary_color":16759788,"tertiary_color":16761760}"#
        );
    }

    #[test]
    fn hot_type_sizes() {
        assert!(
            size_of::<Member>() <= 304,
            "Member is {} bytes",
            size_of::<Member>()
        );
        assert!(
            size_of::<PartialMember>() <= 160,
            "PartialMember is {} bytes",
            size_of::<PartialMember>()
        );
        assert!(
            size_of::<Role>() <= 128,
            "Role is {} bytes",
            size_of::<Role>()
        );
    }
}
