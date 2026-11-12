//! What the cache keeps: one [`Limit`] per [`Resource`].

use core::{fmt, time::Duration};

/// A kind of Discord resource the cache can keep.
///
/// Each resource has its own [`Store`](crate::Store) and its own [`Limit`] in the
/// [`CacheConfig`]. A [`Limit::Max`] counts in the resource's scope, the same as the
/// corresponding discord.js manager: over the whole cache, per guild or per channel (D-159).
///
/// discord.js counterpart: the managers passed to `makeCache` (`GuildManager`,
/// `GuildMemberManager`, `MessageManager`, …).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Resource {
    /// Guilds. Limit counted over the whole cache. Cached by default.
    ///
    /// discord.js: `GuildManager` (`client.guilds`).
    Guild,
    /// Guild channels and DM channels, without threads. Limit counted over the whole cache.
    /// Cached by default.
    ///
    /// discord.js: `ChannelManager` (`client.channels`).
    Channel,
    /// Threads. Limit counted per guild.
    ///
    /// discord.js: `GuildTextThreadManager`, `GuildForumThreadManager`.
    Thread,
    /// Roles. Limit counted per guild. Cached by default.
    ///
    /// discord.js: `RoleManager` (`guild.roles`).
    Role,
    /// Custom guild emojis. Limit counted per guild.
    ///
    /// discord.js: `GuildEmojiManager` (`guild.emojis`).
    Emoji,
    /// Guild stickers. Limit counted per guild.
    ///
    /// discord.js: `GuildStickerManager` (`guild.stickers`).
    Sticker,
    /// Guild members. Limit counted per guild.
    ///
    /// discord.js: `GuildMemberManager` (`guild.members`).
    Member,
    /// Users. Limit counted over the whole cache.
    ///
    /// discord.js: `UserManager` (`client.users`).
    User,
    /// Messages. Limit counted per channel.
    ///
    /// discord.js: `MessageManager` (`channel.messages`).
    Message,
    /// Presences (status and activities of members). Limit counted per guild.
    ///
    /// discord.js: `PresenceManager` (`guild.presences`).
    Presence,
    /// Voice states. Limit counted per guild.
    ///
    /// discord.js: `VoiceStateManager` (`guild.voiceStates`).
    VoiceState,
    /// Scheduled events. Limit counted per guild.
    ///
    /// discord.js: `GuildScheduledEventManager` (`guild.scheduledEvents`).
    ScheduledEvent,
    /// Stage instances. Limit counted per guild.
    ///
    /// discord.js: `StageInstanceManager` (`guild.stageInstances`).
    StageInstance,
    /// Guild soundboard sounds. Limit counted per guild.
    ///
    /// discord.js: `GuildSoundboardSoundManager` (`guild.soundboardSounds`).
    SoundboardSound,
}

impl Resource {
    /// Every resource, in declaration order.
    pub const ALL: &'static [Self] = &[
        Self::Guild,
        Self::Channel,
        Self::Thread,
        Self::Role,
        Self::Emoji,
        Self::Sticker,
        Self::Member,
        Self::User,
        Self::Message,
        Self::Presence,
        Self::VoiceState,
        Self::ScheduledEvent,
        Self::StageInstance,
        Self::SoundboardSound,
    ];

    /// The number of resources, the length of [`CacheConfig`]'s limit table.
    const COUNT: usize = Self::ALL.len();

    /// What a [`Limit::Max`] of this resource counts in (D-159).
    pub(crate) const fn scope(self) -> Scope {
        match self {
            Self::Guild | Self::Channel | Self::User => Scope::Cache,
            Self::Message => Scope::Channel,
            Self::Thread
            | Self::Role
            | Self::Emoji
            | Self::Sticker
            | Self::Member
            | Self::Presence
            | Self::VoiceState
            | Self::ScheduledEvent
            | Self::StageInstance
            | Self::SoundboardSound => Scope::Guild,
        }
    }

    /// The position of this resource in [`Self::ALL`].
    const fn index(self) -> usize {
        self as usize
    }
}

/// What a [`Limit::Max`] counts in (D-159).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    /// All entries of the resource.
    Cache,
    /// The entries of one guild.
    Guild,
    /// The entries of one channel.
    Channel,
}

/// Whether a [`Resource`] is cached, and how many of its entries are kept.
///
/// A limit counts in the resource's scope (see [`Resource`]): `Limit::Max(100)` for
/// [`Resource::Member`] keeps up to 100 members in every guild, for [`Resource::Message`] up
/// to 100 messages in every channel. When a scope is full, inserting a new entry evicts the
/// one inserted first (D-157); updating an entry that is already cached does not change its
/// place.
///
/// discord.js counterpart: the numbers passed to `Options.cacheWithLimits` (`0` disables a
/// manager's cache, no entry means unlimited).
///
/// # Examples
///
/// ```
/// use djs_cache::Limit;
///
/// assert!(Limit::Max(200).is_enabled());
/// assert_eq!(Limit::Max(200).max(), Some(200));
/// assert!(!Limit::Disabled.is_enabled());
/// // A maximum of zero keeps nothing; the configuration stores it as `Disabled`.
/// assert!(!Limit::Max(0).is_enabled());
/// assert_eq!(Limit::Unlimited.max(), None);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Limit {
    /// The resource is not cached.
    Disabled,
    /// At most this many entries per scope. `Max(0)` is the same as [`Limit::Disabled`].
    Max(usize),
    /// Every entry is kept.
    Unlimited,
}

impl Limit {
    /// Returns whether the resource is cached at all.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::Disabled | Self::Max(0))
    }

    /// Returns the maximum number of entries per scope, or `None` if there is no maximum
    /// ([`Limit::Unlimited`]) or nothing is cached ([`Limit::Disabled`]).
    #[must_use]
    pub const fn max(self) -> Option<usize> {
        match self {
            Self::Max(0) | Self::Disabled | Self::Unlimited => None,
            Self::Max(max) => Some(max),
        }
    }

    /// `Max(0)` becomes `Disabled`, so equal settings compare equal.
    const fn normalized(self) -> Self {
        match self {
            Self::Max(0) => Self::Disabled,
            other => other,
        }
    }
}

/// Which resources the cache keeps, and how many entries of each.
///
/// [`CacheConfig::default`] keeps only guilds, channels and roles (and always the current
/// user), which covers most bots at the lowest cost (D-152). [`CacheConfig::discord_js`]
/// keeps what discord.js keeps by default. Customise either with [`CacheConfig::builder`] or
/// [`CacheConfigBuilder::from`].
///
/// discord.js counterpart: the `makeCache` client option.
///
/// # Examples
///
/// ```
/// use djs_cache::{CacheConfig, Limit, Resource};
///
/// let config = CacheConfig::builder()
///     .members(Limit::Unlimited)
///     .messages(Limit::Max(50))
///     .build();
/// assert_eq!(config.limit(Resource::Member), Limit::Unlimited);
/// assert_eq!(config.limit(Resource::Message), Limit::Max(50));
/// // Not mentioned: the default.
/// assert_eq!(config.limit(Resource::Guild), Limit::Unlimited);
/// assert_eq!(config.limit(Resource::Presence), Limit::Disabled);
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CacheConfig {
    limits: [Limit; Resource::COUNT],
    sweepers: SweepConfig,
}

impl CacheConfig {
    /// Returns a builder that starts from the [default](Self::default) configuration.
    pub fn builder() -> CacheConfigBuilder {
        CacheConfigBuilder {
            config: Self::default(),
        }
    }

    /// Returns discord.js' default cache: every resource without a limit, except messages,
    /// which keep the latest 200 per channel.
    ///
    /// discord.js counterpart: `Options.cacheWithLimits(Options.DefaultMakeCacheSettings)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_cache::{CacheConfig, Limit, Resource};
    ///
    /// let config = CacheConfig::discord_js();
    /// assert_eq!(config.limit(Resource::Message), Limit::Max(200));
    /// assert_eq!(config.limit(Resource::Presence), Limit::Unlimited);
    /// ```
    #[must_use]
    pub fn discord_js() -> Self {
        let mut config = Self::uniform(Limit::Unlimited);
        config.set(Resource::Message, Limit::Max(200));
        config
    }

    /// Returns the limit of `resource`.
    #[must_use]
    pub const fn limit(&self, resource: Resource) -> Limit {
        self.limits[resource.index()]
    }

    /// Returns whether `resource` is cached.
    #[must_use]
    pub const fn is_enabled(&self, resource: Resource) -> bool {
        self.limit(resource).is_enabled()
    }

    /// Returns which sweeps run periodically (D-167). The cache itself runs nothing: the
    /// djs-rs `Client` runs them, or call the [`Cache`](crate::Cache) sweep methods yourself.
    #[must_use]
    pub const fn sweepers(&self) -> &SweepConfig {
        &self.sweepers
    }

    const fn uniform(limit: Limit) -> Self {
        Self {
            limits: [limit; Resource::COUNT],
            sweepers: SweepConfig::DEFAULT,
        }
    }

    const fn set(&mut self, resource: Resource, limit: Limit) {
        self.limits[resource.index()] = limit.normalized();
    }
}

impl Default for CacheConfig {
    /// Guilds, channels and roles without a limit; everything else disabled (D-152).
    fn default() -> Self {
        let mut config = Self::uniform(Limit::Disabled);
        for resource in [Resource::Guild, Resource::Channel, Resource::Role] {
            config.set(resource, Limit::Unlimited);
        }
        config
    }
}

impl fmt::Debug for CacheConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Limits<'a>(&'a CacheConfig);

        impl fmt::Debug for Limits<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_map()
                    .entries(
                        Resource::ALL
                            .iter()
                            .map(|&resource| (resource, self.0.limit(resource))),
                    )
                    .finish()
            }
        }

        f.debug_struct("CacheConfig")
            .field("limits", &Limits(self))
            .field("sweepers", &self.sweepers)
            .finish()
    }
}

/// Builds a [`CacheConfig`]. Created by [`CacheConfig::builder`], or from an existing
/// configuration with [`CacheConfigBuilder::from`].
///
/// Every method sets the [`Limit`] of one [`Resource`]; resources not mentioned keep the
/// starting configuration's limit.
///
/// # Examples
///
/// Start from discord.js' defaults and turn presences off:
///
/// ```
/// use djs_cache::{CacheConfig, CacheConfigBuilder, Limit, Resource};
///
/// let config = CacheConfigBuilder::from(CacheConfig::discord_js())
///     .presences(Limit::Disabled)
///     .build();
/// assert!(!config.is_enabled(Resource::Presence));
/// assert!(config.is_enabled(Resource::Member));
/// ```
#[derive(Clone, Debug)]
#[must_use = "a builder does nothing until `build` is called"]
pub struct CacheConfigBuilder {
    config: CacheConfig,
}

impl CacheConfigBuilder {
    /// Sets the limit of any resource.
    pub const fn limit(mut self, resource: Resource, limit: Limit) -> Self {
        self.config.set(resource, limit);
        self
    }

    /// Sets the limit of [`Resource::Guild`] (default [`Limit::Unlimited`]).
    pub const fn guilds(self, limit: Limit) -> Self {
        self.limit(Resource::Guild, limit)
    }

    /// Sets the limit of [`Resource::Channel`] (default [`Limit::Unlimited`]).
    pub const fn channels(self, limit: Limit) -> Self {
        self.limit(Resource::Channel, limit)
    }

    /// Sets the limit of [`Resource::Thread`], per guild (default [`Limit::Disabled`]).
    pub const fn threads(self, limit: Limit) -> Self {
        self.limit(Resource::Thread, limit)
    }

    /// Sets the limit of [`Resource::Role`], per guild (default [`Limit::Unlimited`]).
    pub const fn roles(self, limit: Limit) -> Self {
        self.limit(Resource::Role, limit)
    }

    /// Sets the limit of [`Resource::Emoji`], per guild (default [`Limit::Disabled`]).
    pub const fn emojis(self, limit: Limit) -> Self {
        self.limit(Resource::Emoji, limit)
    }

    /// Sets the limit of [`Resource::Sticker`], per guild (default [`Limit::Disabled`]).
    pub const fn stickers(self, limit: Limit) -> Self {
        self.limit(Resource::Sticker, limit)
    }

    /// Sets the limit of [`Resource::Member`], per guild (default [`Limit::Disabled`]).
    pub const fn members(self, limit: Limit) -> Self {
        self.limit(Resource::Member, limit)
    }

    /// Sets the limit of [`Resource::User`] (default [`Limit::Disabled`]).
    pub const fn users(self, limit: Limit) -> Self {
        self.limit(Resource::User, limit)
    }

    /// Sets the limit of [`Resource::Message`], per channel (default [`Limit::Disabled`]).
    pub const fn messages(self, limit: Limit) -> Self {
        self.limit(Resource::Message, limit)
    }

    /// Sets the limit of [`Resource::Presence`], per guild (default [`Limit::Disabled`]).
    pub const fn presences(self, limit: Limit) -> Self {
        self.limit(Resource::Presence, limit)
    }

    /// Sets the limit of [`Resource::VoiceState`], per guild (default [`Limit::Disabled`]).
    pub const fn voice_states(self, limit: Limit) -> Self {
        self.limit(Resource::VoiceState, limit)
    }

    /// Sets the limit of [`Resource::ScheduledEvent`], per guild (default
    /// [`Limit::Disabled`]).
    pub const fn scheduled_events(self, limit: Limit) -> Self {
        self.limit(Resource::ScheduledEvent, limit)
    }

    /// Sets the limit of [`Resource::StageInstance`], per guild (default
    /// [`Limit::Disabled`]).
    pub const fn stage_instances(self, limit: Limit) -> Self {
        self.limit(Resource::StageInstance, limit)
    }

    /// Sets the limit of [`Resource::SoundboardSound`], per guild (default
    /// [`Limit::Disabled`]).
    pub const fn soundboard_sounds(self, limit: Limit) -> Self {
        self.limit(Resource::SoundboardSound, limit)
    }

    /// Sets the periodic sweep of old messages (default none: messages are bounded by their
    /// [limit](Self::messages)). See [`Cache::sweep_messages_older_than`](crate::Cache::sweep_messages_older_than).
    ///
    /// discord.js counterpart: the `messages` key of the `sweepers` client option.
    pub const fn sweep_messages(mut self, sweep: Option<Sweep>) -> Self {
        self.config.sweepers.messages = sweep;
        self
    }

    /// Sets the periodic sweep of long-archived threads (default every hour, archived more
    /// than 4 hours ago). See [`Cache::sweep_archived_threads`](crate::Cache::sweep_archived_threads).
    ///
    /// discord.js counterpart: the `threads` key of the `sweepers` client option.
    pub const fn sweep_threads(mut self, sweep: Option<Sweep>) -> Self {
        self.config.sweepers.threads = sweep;
        self
    }

    /// Builds the configuration.
    #[must_use]
    pub fn build(self) -> CacheConfig {
        self.config
    }
}

/// A periodic sweep: every `interval`, remove the entries older than `lifetime` (D-167).
///
/// discord.js counterpart: `SweepOptions` with `interval` and `filter:
/// Sweepers.filterByLifetime({ lifetime })`.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use djs_cache::{CacheConfig, Sweep};
///
/// // Every 10 minutes, sweep messages older than 30 minutes.
/// let config = CacheConfig::builder()
///     .sweep_messages(Some(Sweep::new(
///         Duration::from_secs(600),
///         Duration::from_secs(1_800),
///     )))
///     .build();
/// assert_eq!(config.sweepers().messages.unwrap().lifetime, Duration::from_secs(1_800));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Sweep {
    /// How often the sweep runs.
    pub interval: Duration,
    /// How old an entry must be to be swept.
    pub lifetime: Duration,
}

impl Sweep {
    /// Creates a sweep running every `interval` over entries older than `lifetime`.
    #[must_use]
    pub const fn new(interval: Duration, lifetime: Duration) -> Self {
        Self { interval, lifetime }
    }
}

/// The periodic sweeps of a [`CacheConfig`] (D-167, D-169). Set them with
/// [`CacheConfigBuilder::sweep_messages`] and [`CacheConfigBuilder::sweep_threads`].
///
/// discord.js counterpart: the `sweepers` client option.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SweepConfig {
    /// Sweeps messages by age (their last edit, else their creation). Default: none.
    pub messages: Option<Sweep>,
    /// Sweeps threads archived for longer than the lifetime. Default: every hour, archived
    /// more than 4 hours ago, like discord.js.
    pub threads: Option<Sweep>,
}

impl SweepConfig {
    const DEFAULT: Self = Self {
        messages: None,
        threads: Some(Sweep::new(
            Duration::from_secs(60 * 60),
            Duration::from_secs(4 * 60 * 60),
        )),
    };
}

impl Default for SweepConfig {
    /// discord.js' defaults (`Options.DefaultSweeperSettings`): archived threads only.
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<CacheConfig> for CacheConfigBuilder {
    /// Starts a builder from an existing configuration.
    fn from(config: CacheConfig) -> Self {
        Self { config }
    }
}

#[cfg(test)]
mod tests {
    use super::{CacheConfig, Limit, Resource, Scope};

    #[test]
    fn resource_indexes_match_all() {
        for (position, resource) in Resource::ALL.iter().enumerate() {
            assert_eq!(resource.index(), position, "{resource:?}");
        }
    }

    #[test]
    fn the_default_is_minimal() {
        let config = CacheConfig::default();
        for &resource in Resource::ALL {
            let expected = matches!(
                resource,
                Resource::Guild | Resource::Channel | Resource::Role
            );
            assert_eq!(config.is_enabled(resource), expected, "{resource:?}");
        }
        assert_eq!(CacheConfig::builder().build(), config);
    }

    #[test]
    fn the_discord_js_preset_limits_only_messages() {
        let config = CacheConfig::discord_js();
        for &resource in Resource::ALL {
            let expected = if resource == Resource::Message {
                Limit::Max(200)
            } else {
                Limit::Unlimited
            };
            assert_eq!(config.limit(resource), expected, "{resource:?}");
        }
    }

    #[test]
    fn a_zero_maximum_is_stored_as_disabled() {
        let config = CacheConfig::builder().guilds(Limit::Max(0)).build();
        assert_eq!(config.limit(Resource::Guild), Limit::Disabled);
        assert_eq!(
            config,
            CacheConfig::builder().guilds(Limit::Disabled).build()
        );
    }

    #[test]
    fn every_builder_method_sets_its_resource() {
        let config = CacheConfig::builder()
            .guilds(Limit::Max(1))
            .channels(Limit::Max(2))
            .threads(Limit::Max(3))
            .roles(Limit::Max(4))
            .emojis(Limit::Max(5))
            .stickers(Limit::Max(6))
            .members(Limit::Max(7))
            .users(Limit::Max(8))
            .messages(Limit::Max(9))
            .presences(Limit::Max(10))
            .voice_states(Limit::Max(11))
            .scheduled_events(Limit::Max(12))
            .stage_instances(Limit::Max(13))
            .soundboard_sounds(Limit::Max(14))
            .build();
        for (position, &resource) in Resource::ALL.iter().enumerate() {
            assert_eq!(
                config.limit(resource),
                Limit::Max(position + 1),
                "{resource:?}"
            );
        }
    }

    #[test]
    fn scopes_follow_the_discord_js_managers() {
        assert_eq!(Resource::Guild.scope(), Scope::Cache);
        assert_eq!(Resource::User.scope(), Scope::Cache);
        assert_eq!(Resource::Channel.scope(), Scope::Cache);
        assert_eq!(Resource::Message.scope(), Scope::Channel);
        assert_eq!(Resource::Member.scope(), Scope::Guild);
        assert_eq!(Resource::Presence.scope(), Scope::Guild);
    }

    #[test]
    fn debug_lists_every_resource() {
        let debug = format!("{:?}", CacheConfig::default());
        assert!(
            debug.starts_with(
                "CacheConfig { limits: {Guild: Unlimited, Channel: Unlimited, Thread: Disabled"
            ),
            "{debug}"
        );
        assert!(debug.contains("SoundboardSound: Disabled}, sweepers: SweepConfig {"));
    }

    #[test]
    fn sweeps_default_to_discord_js() {
        use core::time::Duration;

        use super::{Sweep, SweepConfig};

        let sweepers = CacheConfig::default().sweepers().clone();
        assert_eq!(sweepers, SweepConfig::default());
        assert_eq!(sweepers.messages, None);
        assert_eq!(
            sweepers.threads,
            Some(Sweep::new(
                Duration::from_secs(3_600),
                Duration::from_secs(14_400)
            ))
        );
        assert_eq!(CacheConfig::discord_js().sweepers(), &sweepers);
        let off = CacheConfig::builder().sweep_threads(None).build();
        assert_eq!(off.sweepers().threads, None);
    }
}
