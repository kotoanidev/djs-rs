//! The cache's own types, for the resources whose model types do not fit (D-160).

use std::sync::Arc;

use compact_str::CompactString;
use djs_model::{
    application::PartialApplication,
    channel::{Channel, ChannelType},
    component::Component,
    gateway::{
        Activity, ClientStatus, Presence, Status,
        payload::{AvailableGuild, GuildMemberUpdate},
    },
    guild::{
        AgeRestrictionLevel, DefaultMessageNotificationLevel, ExplicitContentFilterLevel, Feature,
        Guild, IncidentsData, Member, MemberFlags, MfaLevel, PartialMember, PremiumTier,
        SystemChannelFlags, VerificationLevel, WelcomeScreen,
    },
    id::{ApplicationId, ChannelId, GuildId, MessageId, RoleId, UserId, WebhookId},
    interaction::{MessageInteraction, Resolved},
    message::{
        Attachment, ChannelMention, Embed, InteractionMetadata, Mention, Message, MessageActivity,
        MessageCall, MessageFlags, MessageReference, MessageSnapshot, MessageType, Nonce, Reaction,
        ReferencedMessage, RoleSubscriptionData, SharedClientTheme,
    },
    permissions::Permissions,
    poll::Poll,
    sticker::{Sticker, StickerItem},
    user::{AvatarDecorationData, Collectibles, PrimaryGuild, User, UserFlags},
    util::{ImageHash, Locale, Timestamp},
};

/// A cached guild: a [`Guild`] without its roles, emojis and stickers, which the cache keeps
/// in their own resources, plus what `GUILD_CREATE` adds.
///
/// Read the guild's roles with [`Cache::roles`](crate::Cache::roles), its emojis with
/// [`Cache::emojis`](crate::Cache::emojis), its stickers with
/// [`Cache::stickers`](crate::Cache::stickers) and its channels with
/// [`CacheView::in_guild`](crate::CacheView::in_guild).
///
/// The fields mean the same as in [`Guild`]; see the
/// [guild object](https://docs.discord.com/developers/resources/guild#guild-object).
///
/// discord.js counterpart: `Guild`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
#[expect(missing_docs, reason = "the fields are documented on `Guild`")]
#[expect(clippy::struct_excessive_bools, reason = "mirrors the guild object")]
pub struct CachedGuild {
    pub id: GuildId,
    pub name: CompactString,
    pub icon: Option<ImageHash>,
    pub icon_hash: Option<ImageHash>,
    pub splash: Option<ImageHash>,
    pub discovery_splash: Option<ImageHash>,
    pub owner: bool,
    pub owner_id: UserId,
    pub permissions: Option<Permissions>,
    pub region: Option<CompactString>,
    pub afk_channel_id: Option<ChannelId>,
    pub afk_timeout: u32,
    pub widget_enabled: bool,
    pub widget_channel_id: Option<ChannelId>,
    pub verification_level: VerificationLevel,
    pub default_message_notifications: DefaultMessageNotificationLevel,
    pub explicit_content_filter: ExplicitContentFilterLevel,
    pub features: Vec<Feature>,
    pub mfa_level: MfaLevel,
    pub application_id: Option<ApplicationId>,
    pub system_channel_id: Option<ChannelId>,
    pub system_channel_flags: SystemChannelFlags,
    pub rules_channel_id: Option<ChannelId>,
    pub max_presences: Option<u32>,
    pub max_members: Option<u32>,
    pub vanity_url_code: Option<CompactString>,
    pub description: Option<CompactString>,
    pub banner: Option<ImageHash>,
    pub premium_tier: PremiumTier,
    pub premium_subscription_count: Option<u32>,
    pub preferred_locale: Locale,
    pub public_updates_channel_id: Option<ChannelId>,
    pub max_video_channel_users: Option<u32>,
    pub max_stage_video_channel_users: Option<u32>,
    pub approximate_member_count: Option<u32>,
    pub approximate_presence_count: Option<u32>,
    pub welcome_screen: Option<Box<WelcomeScreen>>,
    pub nsfw_level: AgeRestrictionLevel,
    pub premium_progress_bar_enabled: bool,
    pub safety_alerts_channel_id: Option<ChannelId>,
    pub incidents_data: Option<Box<IncidentsData>>,
    /// When the bot joined the guild (from `GUILD_CREATE`).
    pub joined_at: Option<Timestamp>,
    /// Whether the guild is large (more members than the identify `large_threshold`).
    pub large: bool,
    /// The number of members, from `GUILD_CREATE`, kept up to date by member joins and
    /// leaves.
    ///
    /// discord.js: `Guild#memberCount`.
    pub member_count: u32,
}

impl CachedGuild {
    /// Builds a cached guild from a `GUILD_CREATE`.
    pub(crate) fn from_available(guild: &AvailableGuild) -> Self {
        Self::new(
            &guild.guild,
            guild.joined_at,
            guild.large,
            guild.member_count,
        )
    }

    /// Builds the cached guild after a `GUILD_UPDATE`, which lacks the `GUILD_CREATE` extras:
    /// they are kept from `previous`.
    pub(crate) fn updated(guild: &Guild, previous: &Self) -> Self {
        Self::new(
            guild,
            previous.joined_at,
            previous.large,
            previous.member_count,
        )
    }

    pub(crate) fn new(
        guild: &Guild,
        joined_at: Option<Timestamp>,
        large: bool,
        member_count: u32,
    ) -> Self {
        Self {
            id: guild.id,
            name: guild.name.clone(),
            icon: guild.icon,
            icon_hash: guild.icon_hash,
            splash: guild.splash,
            discovery_splash: guild.discovery_splash,
            owner: guild.owner,
            owner_id: guild.owner_id,
            permissions: guild.permissions,
            region: guild.region.clone(),
            afk_channel_id: guild.afk_channel_id,
            afk_timeout: guild.afk_timeout,
            widget_enabled: guild.widget_enabled,
            widget_channel_id: guild.widget_channel_id,
            verification_level: guild.verification_level,
            default_message_notifications: guild.default_message_notifications,
            explicit_content_filter: guild.explicit_content_filter,
            features: guild.features.clone(),
            mfa_level: guild.mfa_level,
            application_id: guild.application_id,
            system_channel_id: guild.system_channel_id,
            system_channel_flags: guild.system_channel_flags,
            rules_channel_id: guild.rules_channel_id,
            max_presences: guild.max_presences,
            max_members: guild.max_members,
            vanity_url_code: guild.vanity_url_code.clone(),
            description: guild.description.clone(),
            banner: guild.banner,
            premium_tier: guild.premium_tier,
            premium_subscription_count: guild.premium_subscription_count,
            preferred_locale: guild.preferred_locale.clone(),
            public_updates_channel_id: guild.public_updates_channel_id,
            max_video_channel_users: guild.max_video_channel_users,
            max_stage_video_channel_users: guild.max_stage_video_channel_users,
            approximate_member_count: guild.approximate_member_count,
            approximate_presence_count: guild.approximate_presence_count,
            welcome_screen: guild.welcome_screen.clone(),
            nsfw_level: guild.nsfw_level,
            premium_progress_bar_enabled: guild.premium_progress_bar_enabled,
            safety_alerts_channel_id: guild.safety_alerts_channel_id,
            incidents_data: guild.incidents_data.clone(),
            joined_at,
            large,
            member_count,
        }
    }
}

/// A cached user (D-177): a [`User`] with its rarely set fields behind one box.
///
/// Every user has an ID, a username and usually a display name and an avatar; few have a
/// banner, an accent color, an avatar decoration, collectibles or a primary guild. Those are
/// kept in one allocation that only exists when one of them is set, and read through methods
/// ([`banner`](Self::banner), …). That keeps the user a user in many guilds shares with their
/// members and messages at about two thirds of a [`User`].
///
/// The fields mean the same as in [`User`]; see the
/// [user object](https://docs.discord.com/developers/resources/user#user-object).
///
/// discord.js counterpart: `User`.
///
/// # Examples
///
/// ```
/// use djs_cache::{Cache, CacheConfig, Limit};
/// use djs_model::{gateway::payload::GuildMemberAdd, id::UserId};
///
/// let cache = Cache::with_config(CacheConfig::builder().users(Limit::Unlimited).build());
/// let member: GuildMemberAdd = serde_json::from_str(
///     r#"{"guild_id":"1","user":{"id":"2","username":"nelly","discriminator":"0",
///         "global_name":"Nelly","avatar":null},"roles":[],"joined_at":null,
///         "deaf":false,"mute":false,"flags":0}"#,
/// )?;
/// cache.update(&member);
/// let user = cache.users().get(UserId::new(2)).unwrap();
/// assert_eq!(user.display_name(), "Nelly");
/// assert_eq!(user.tag(), "nelly");
/// assert_eq!(user.banner(), None);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
#[expect(missing_docs, reason = "the fields are documented on `User`")]
pub struct CachedUser {
    pub id: UserId,
    pub username: CompactString,
    pub discriminator: u16,
    pub global_name: Option<CompactString>,
    pub avatar: Option<ImageHash>,
    pub bot: bool,
    pub system: bool,
    pub public_flags: Option<UserFlags>,
    extras: Option<Box<UserExtras>>,
}

/// The rarely set fields of a [`CachedUser`].
#[derive(Clone, Debug, PartialEq, Eq)]
struct UserExtras {
    banner: Option<ImageHash>,
    accent_color: Option<u32>,
    avatar_decoration_data: Option<AvatarDecorationData>,
    collectibles: Option<Collectibles>,
    primary_guild: Option<PrimaryGuild>,
}

impl CachedUser {
    pub(crate) fn new(user: &User) -> Self {
        let rare = user.banner.is_some()
            || user.accent_color.is_some()
            || user.avatar_decoration_data.is_some()
            || user.collectibles.is_some()
            || user.primary_guild.is_some();
        Self {
            id: user.id,
            username: user.username.clone(),
            discriminator: user.discriminator,
            global_name: user.global_name.clone(),
            avatar: user.avatar,
            bot: user.bot,
            system: user.system,
            public_flags: user.public_flags,
            extras: rare.then(|| {
                Box::new(UserExtras {
                    banner: user.banner,
                    accent_color: user.accent_color,
                    avatar_decoration_data: user.avatar_decoration_data.as_deref().cloned(),
                    collectibles: user.collectibles.as_deref().cloned(),
                    primary_guild: user.primary_guild.as_deref().cloned(),
                })
            }),
        }
    }

    /// Whether this holds the same data as `user` (so an equal user is not stored again).
    pub(crate) fn is(&self, user: &User) -> bool {
        self.id == user.id
            && self.username == user.username
            && self.discriminator == user.discriminator
            && self.global_name == user.global_name
            && self.avatar == user.avatar
            && self.bot == user.bot
            && self.system == user.system
            && self.public_flags == user.public_flags
            && self.banner() == user.banner
            && self.accent_color() == user.accent_color
            && self.avatar_decoration_data() == user.avatar_decoration_data.as_deref()
            && self.collectibles() == user.collectibles.as_deref()
            && self.primary_guild() == user.primary_guild.as_deref()
    }

    /// The user's banner (Nitro).
    #[must_use]
    pub fn banner(&self) -> Option<ImageHash> {
        self.extras.as_ref().and_then(|extras| extras.banner)
    }

    /// The user's banner color, as an RGB integer.
    #[must_use]
    pub fn accent_color(&self) -> Option<u32> {
        self.extras.as_ref().and_then(|extras| extras.accent_color)
    }

    /// The user's avatar decoration.
    #[must_use]
    pub fn avatar_decoration_data(&self) -> Option<&AvatarDecorationData> {
        self.extras.as_ref()?.avatar_decoration_data.as_ref()
    }

    /// The user's collectibles (such as a nameplate).
    #[must_use]
    pub fn collectibles(&self) -> Option<&Collectibles> {
        self.extras.as_ref()?.collectibles.as_ref()
    }

    /// The guild whose tag the user displays.
    #[must_use]
    pub fn primary_guild(&self) -> Option<&PrimaryGuild> {
        self.extras.as_ref()?.primary_guild.as_ref()
    }

    /// The name to display: the global display name if set, otherwise the username.
    ///
    /// discord.js counterpart: `User#displayName`.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.global_name.as_deref().unwrap_or(&self.username)
    }

    /// The user's tag: `username#1234` for a legacy user, the username for a user with a
    /// unique username (discriminator `0`).
    ///
    /// discord.js counterpart: `User#tag`.
    #[must_use]
    pub fn tag(&self) -> CompactString {
        if self.discriminator == 0 {
            self.username.clone()
        } else {
            compact_str::format_compact!("{}#{:04}", self.username, self.discriminator)
        }
    }
}

/// A cached guild member (D-177): a [`Member`] with its guild, its user shared with the user
/// resource, and its rarely set fields behind one box.
///
/// When users are cached ([`Resource::User`](crate::Resource::User)), `user` is the same
/// [`Arc`] the user resource holds, so a user in many guilds is stored once. A guild avatar
/// or banner, a boost or timeout time, an avatar decoration and collectibles are kept in one
/// allocation that only exists when one of them is set, and read through methods
/// ([`avatar`](Self::avatar), …).
///
/// The fields mean the same as in [`Member`]; see the
/// [guild member object](https://docs.discord.com/developers/resources/guild#guild-member-object).
///
/// discord.js counterpart: `GuildMember`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
#[expect(missing_docs, reason = "the fields are documented on `Member`")]
pub struct CachedMember {
    /// The guild the member belongs to.
    pub guild_id: GuildId,
    pub user: Arc<CachedUser>,
    pub nick: Option<CompactString>,
    pub roles: Box<[RoleId]>,
    pub joined_at: Option<Timestamp>,
    pub deaf: bool,
    pub mute: bool,
    pub pending: bool,
    pub flags: MemberFlags,
    extras: Option<Box<MemberExtras>>,
}

/// The rarely set fields of a [`CachedMember`].
#[derive(Clone, Debug, PartialEq, Eq)]
struct MemberExtras {
    avatar: Option<ImageHash>,
    banner: Option<ImageHash>,
    premium_since: Option<Timestamp>,
    communication_disabled_until: Option<Timestamp>,
    avatar_decoration_data: Option<AvatarDecorationData>,
    collectibles: Option<Collectibles>,
}

impl MemberExtras {
    /// The extras, or `None` when none of them is set.
    fn new(
        avatar: Option<ImageHash>,
        banner: Option<ImageHash>,
        premium_since: Option<Timestamp>,
        communication_disabled_until: Option<Timestamp>,
        avatar_decoration_data: Option<&AvatarDecorationData>,
        collectibles: Option<&Collectibles>,
    ) -> Option<Box<Self>> {
        let rare = avatar.is_some()
            || banner.is_some()
            || premium_since.is_some()
            || communication_disabled_until.is_some()
            || avatar_decoration_data.is_some()
            || collectibles.is_some();
        rare.then(|| {
            Box::new(Self {
                avatar,
                banner,
                premium_since,
                communication_disabled_until,
                avatar_decoration_data: avatar_decoration_data.cloned(),
                collectibles: collectibles.cloned(),
            })
        })
    }
}

impl CachedMember {
    pub(crate) fn new(guild_id: GuildId, member: &Member, user: Arc<CachedUser>) -> Self {
        Self {
            guild_id,
            user,
            nick: member.nick.clone(),
            roles: member.roles.as_slice().into(),
            joined_at: member.joined_at,
            deaf: member.deaf,
            mute: member.mute,
            pending: member.pending,
            flags: member.flags,
            extras: MemberExtras::new(
                member.avatar,
                member.banner,
                member.premium_since,
                member.communication_disabled_until,
                member.avatar_decoration_data.as_deref(),
                member.collectibles.as_deref(),
            ),
        }
    }

    /// Builds the member after a `GUILD_MEMBER_UPDATE`. Fields the update may omit (`deaf`,
    /// `mute`, `pending`, `flags`) are kept from `previous`, or default when the member was
    /// not cached.
    pub(crate) fn updated(
        update: &GuildMemberUpdate,
        user: Arc<CachedUser>,
        previous: Option<&Self>,
    ) -> Self {
        Self {
            guild_id: update.guild_id,
            user,
            nick: update.nick.clone(),
            roles: update.roles.as_slice().into(),
            joined_at: update
                .joined_at
                .or_else(|| previous.and_then(|member| member.joined_at)),
            deaf: update
                .deaf
                .unwrap_or_else(|| previous.is_some_and(|member| member.deaf)),
            mute: update
                .mute
                .unwrap_or_else(|| previous.is_some_and(|member| member.mute)),
            pending: update
                .pending
                .unwrap_or_else(|| previous.is_some_and(|member| member.pending)),
            flags: update
                .flags
                .unwrap_or_else(|| previous.map_or_else(MemberFlags::empty, |member| member.flags)),
            extras: MemberExtras::new(
                update.avatar,
                update.banner,
                update.premium_since,
                update.communication_disabled_until,
                update.avatar_decoration_data.as_deref(),
                update.collectibles.as_deref(),
            ),
        }
    }

    /// Builds a member from the partial member attached to a message (or an interaction),
    /// whose user is the message's author.
    pub(crate) fn from_partial(
        guild_id: GuildId,
        member: &PartialMember,
        user: Arc<CachedUser>,
    ) -> Self {
        Self {
            guild_id,
            user,
            nick: member.nick.clone(),
            roles: member.roles.as_slice().into(),
            joined_at: member.joined_at,
            deaf: member.deaf,
            mute: member.mute,
            pending: member.pending,
            flags: member.flags,
            extras: MemberExtras::new(
                member.avatar,
                member.banner,
                member.premium_since,
                member.communication_disabled_until,
                member.avatar_decoration_data.as_deref(),
                member.collectibles.as_deref(),
            ),
        }
    }

    /// The member's ID (their user's ID).
    #[must_use]
    pub fn id(&self) -> UserId {
        self.user.id
    }

    /// The name shown in the guild: the nickname, else the user's display name, else the
    /// username.
    ///
    /// discord.js counterpart: `GuildMember#displayName`.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.nick
            .as_deref()
            .unwrap_or_else(|| self.user.display_name())
    }

    /// The member's guild avatar (overriding their user's avatar in the guild).
    #[must_use]
    pub fn avatar(&self) -> Option<ImageHash> {
        self.extras.as_ref().and_then(|extras| extras.avatar)
    }

    /// The member's guild banner.
    #[must_use]
    pub fn banner(&self) -> Option<ImageHash> {
        self.extras.as_ref().and_then(|extras| extras.banner)
    }

    /// When the member started boosting the guild.
    ///
    /// discord.js counterpart: `GuildMember#premiumSince`.
    #[must_use]
    pub fn premium_since(&self) -> Option<Timestamp> {
        self.extras.as_ref().and_then(|extras| extras.premium_since)
    }

    /// When the member's timeout ends (`None` or a past time: not timed out).
    ///
    /// discord.js counterpart: `GuildMember#communicationDisabledUntil`.
    #[must_use]
    pub fn communication_disabled_until(&self) -> Option<Timestamp> {
        self.extras
            .as_ref()
            .and_then(|extras| extras.communication_disabled_until)
    }

    /// The member's guild avatar decoration.
    #[must_use]
    pub fn avatar_decoration_data(&self) -> Option<&AvatarDecorationData> {
        self.extras.as_ref()?.avatar_decoration_data.as_ref()
    }

    /// The member's guild collectibles.
    #[must_use]
    pub fn collectibles(&self) -> Option<&Collectibles> {
        self.extras.as_ref()?.collectibles.as_ref()
    }
}

/// A cached message: a [`Message`] whose author is shared with the user resource, without the
/// `member` field (members are a resource of their own, D-165).
///
/// When users are cached ([`Resource::User`](crate::Resource::User)), `author` is the same
/// [`Arc`] the user resource holds, so an author of many messages is stored once. The author's
/// guild member, when the message came with one, is cached as a member.
///
/// The fields mean the same as in [`Message`]; see the
/// [message object](https://docs.discord.com/developers/resources/message#message-object).
///
/// discord.js counterpart: `Message`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
#[expect(missing_docs, reason = "the fields are documented on `Message`")]
pub struct CachedMessage {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub guild_id: Option<GuildId>,
    pub author: Arc<CachedUser>,
    pub content: CompactString,
    pub timestamp: Timestamp,
    pub edited_timestamp: Option<Timestamp>,
    pub tts: bool,
    pub mention_everyone: bool,
    pub mentions: Vec<Mention>,
    pub mention_roles: Vec<RoleId>,
    pub mention_channels: Vec<ChannelMention>,
    pub attachments: Vec<Attachment>,
    pub embeds: Vec<Embed>,
    pub reactions: Vec<Reaction>,
    pub nonce: Option<Nonce>,
    pub pinned: bool,
    pub webhook_id: Option<WebhookId>,
    pub kind: MessageType,
    pub activity: Option<Box<MessageActivity>>,
    pub application: Option<Box<PartialApplication>>,
    pub application_id: Option<ApplicationId>,
    pub flags: MessageFlags,
    pub message_reference: Option<MessageReference>,
    pub message_snapshots: Vec<MessageSnapshot>,
    pub referenced_message: ReferencedMessage,
    pub interaction_metadata: Option<Box<InteractionMetadata>>,
    pub interaction: Option<Box<MessageInteraction>>,
    pub components: Vec<Component>,
    pub thread: Option<Box<Channel>>,
    pub resolved: Option<Box<Resolved>>,
    pub sticker_items: Vec<StickerItem>,
    pub stickers: Vec<Sticker>,
    pub position: Option<u32>,
    pub role_subscription_data: Option<Box<RoleSubscriptionData>>,
    pub poll: Option<Box<Poll>>,
    pub call: Option<Box<MessageCall>>,
    pub shared_client_theme: Option<Box<SharedClientTheme>>,
    pub channel_type: Option<ChannelType>,
}

impl CachedMessage {
    pub(crate) fn new(message: &Message, author: Arc<CachedUser>) -> Self {
        Self {
            id: message.id,
            channel_id: message.channel_id,
            guild_id: message.guild_id,
            author,
            content: message.content.clone(),
            timestamp: message.timestamp,
            edited_timestamp: message.edited_timestamp,
            tts: message.tts,
            mention_everyone: message.mention_everyone,
            mentions: message.mentions.clone(),
            mention_roles: message.mention_roles.clone(),
            mention_channels: message.mention_channels.clone(),
            attachments: message.attachments.clone(),
            embeds: message.embeds.clone(),
            reactions: message.reactions.clone(),
            nonce: message.nonce.clone(),
            pinned: message.pinned,
            webhook_id: message.webhook_id,
            kind: message.kind,
            activity: message.activity.clone(),
            application: message.application.clone(),
            application_id: message.application_id,
            flags: message.flags,
            message_reference: message.message_reference,
            message_snapshots: message.message_snapshots.clone(),
            referenced_message: message.referenced_message.clone(),
            interaction_metadata: message.interaction_metadata.clone(),
            interaction: message.interaction.clone(),
            components: message.components.clone(),
            thread: message.thread.clone(),
            resolved: message.resolved.clone(),
            sticker_items: message.sticker_items.clone(),
            stickers: message.stickers.clone(),
            position: message.position,
            role_subscription_data: message.role_subscription_data.clone(),
            poll: message.poll.clone(),
            call: message.call.clone(),
            shared_client_theme: message.shared_client_theme.clone(),
            channel_type: message.channel_type,
        }
    }
}

/// A cached presence: a member's status and activities (D-164).
///
/// It keeps its user's ID only; the user's data lives in the user resource. The activities are
/// the model's, complete, in a boxed slice. Offline members have no cached presence.
///
/// See the [presence update](https://docs.discord.com/developers/events/gateway-events#presence-update).
///
/// discord.js counterpart: `Presence`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CachedPresence {
    /// The guild the presence is in.
    pub guild_id: GuildId,
    /// The user the presence is of.
    pub user_id: UserId,
    /// The overall status.
    pub status: Status,
    /// The status on each platform.
    pub client_status: ClientStatus,
    /// The current activities.
    pub activities: Box<[Activity]>,
}

impl CachedPresence {
    pub(crate) fn new(guild_id: GuildId, presence: &Presence) -> Self {
        Self {
            guild_id,
            user_id: presence.user.id,
            status: presence.status.clone(),
            client_status: presence.client_status.clone(),
            activities: presence.activities.clone().into_boxed_slice(),
        }
    }
}
