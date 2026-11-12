//! The cache itself.

use core::{fmt, hash::Hash};
use std::sync::{Arc, PoisonError, RwLock};

use dashmap::DashMap;
use djs_model::{
    channel::Channel,
    emoji::Emoji,
    gateway::{
        DispatchEvent,
        payload::{AvailableGuild, GuildMemberUpdate},
    },
    guild::{Guild, Member, Role},
    id::{ChannelId, EmojiId, GuildId, RoleId, StickerId, UserId},
    sticker::Sticker,
    user::{CurrentUser, User},
    util::Timestamp,
};

use crate::{
    Backend, CacheConfig, CacheView, CachedGuild, CachedMember, DashMapBackend, GuildView,
    Previous, Resource, Store, UpdateCache,
    config::Scope,
    dashmap_store::Hasher,
    limits::{Global, GlobalOf, Index, Scoped, ScopedOf},
};

/// An in-memory view of Discord state, kept up to date from gateway events.
///
/// What it keeps is set by its [`CacheConfig`] (by default guilds, channels, roles and the
/// current user, D-152); where it keeps it by its [`Backend`] (by default
/// [`DashMapBackend`]). Feed it every dispatch event with [`update`](Self::update) or
/// [`update_dispatch`](Self::update_dispatch), and read it through the per-resource views
/// ([`guilds`](Self::guilds), [`members`](Self::members), …).
///
/// Reads return [`Arc`]s, which hold no lock and can be kept across `.await` (D-151). The
/// cache is `Send + Sync`: share one between all shards, for example in an `Arc`.
///
/// discord.js counterpart: the `cache` collections of the client's managers
/// (`client.guilds.cache`, `guild.members.cache`, …), filled by the client's event handlers.
///
/// # Examples
///
/// ```
/// use djs_cache::{Cache, CacheConfig, Limit};
/// use djs_model::{gateway::payload::GuildRoleCreate, id::GuildId};
/// # use djs_model::guild::Role;
/// # let role: Role = serde_json::from_str(r#"{"id":"2","name":"mods","color":0,"hoist":false,
/// #   "position":1,"permissions":"0","managed":false,"mentionable":false,"flags":0,
/// #   "colors":{"primary_color":0,"secondary_color":null,"tertiary_color":null}}"#).unwrap();
/// # let event: GuildRoleCreate = serde_json::from_value(serde_json::json!({
/// #     "guild_id": "1", "role": role })).unwrap();
///
/// let cache = Cache::with_config(CacheConfig::builder().members(Limit::Max(1_000)).build());
///
/// // `event` is a `GUILD_ROLE_CREATE` received from the gateway.
/// let previous = cache.update(&event);
/// assert!(previous.is_none());
///
/// let role = cache.roles(GuildId::new(1)).get(event.role.id).unwrap();
/// assert_eq!(role.name, "mods");
/// ```
pub struct Cache<B: Backend = DashMapBackend> {
    config: CacheConfig,
    backend: B,
    current_user: RwLock<Option<Arc<CurrentUser>>>,
    unavailable_guilds: DashMap<GuildId, (), Hasher>,
    guilds: Option<GlobalOf<GuildId, CachedGuild, B>>,
    channels: Option<GlobalOf<ChannelId, Channel, B>>,
    /// The channels of each guild, for listing and for dropping a deleted guild's channels.
    guild_channels: Index<GuildId, ChannelId>,
    roles: Option<ScopedOf<RoleId, Role, B>>,
    emojis: Option<ScopedOf<EmojiId, Emoji, B>>,
    stickers: Option<ScopedOf<StickerId, Sticker, B>>,
    members: Option<ScopedOf<UserId, CachedMember, B>>,
    users: Option<GlobalOf<UserId, User, B>>,
}

// The fields above match the scopes the resources declare (D-159).
const _: () = {
    assert!(matches!(Resource::Guild.scope(), Scope::Cache));
    assert!(matches!(Resource::Channel.scope(), Scope::Cache));
    assert!(matches!(Resource::User.scope(), Scope::Cache));
    assert!(matches!(Resource::Role.scope(), Scope::Guild));
    assert!(matches!(Resource::Emoji.scope(), Scope::Guild));
    assert!(matches!(Resource::Sticker.scope(), Scope::Guild));
    assert!(matches!(Resource::Member.scope(), Scope::Guild));
};

impl Cache {
    /// Creates a cache with the [default configuration](CacheConfig::default) and the
    /// [`DashMapBackend`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Creates a cache with `config` and the [`DashMapBackend`].
    #[must_use]
    pub fn with_config(config: CacheConfig) -> Self {
        Self::with_backend(config, DashMapBackend::new())
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

impl<B: Backend> Cache<B> {
    /// Creates a cache with `config`, keeping its resources in stores created by `backend`.
    ///
    /// [`Backend::store`] is called once for each enabled resource.
    #[must_use]
    pub fn with_backend(config: CacheConfig, backend: B) -> Self {
        Self {
            current_user: RwLock::new(None),
            unavailable_guilds: DashMap::with_hasher(Hasher::default()),
            guilds: global(&backend, &config, Resource::Guild),
            channels: global(&backend, &config, Resource::Channel),
            guild_channels: Index::new(),
            roles: scoped(&backend, &config, Resource::Role),
            emojis: scoped(&backend, &config, Resource::Emoji),
            stickers: scoped(&backend, &config, Resource::Sticker),
            members: scoped(&backend, &config, Resource::Member),
            users: global(&backend, &config, Resource::User),
            config,
            backend,
        }
    }

    /// Returns the configuration.
    #[must_use]
    pub const fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Returns the backend.
    #[must_use]
    pub const fn backend(&self) -> &B {
        &self.backend
    }

    /// Applies a dispatch event's payload to the cache, returning the previous state of what
    /// it changed (D-153, D-163). See [`UpdateCache`] for the events and what each returns.
    ///
    /// discord.js counterpart: the client's internal event handlers (`client.actions`).
    pub fn update<E: UpdateCache>(&self, event: &E) -> E::Previous {
        event.update_cache(self)
    }

    /// Applies any dispatch event to the cache, returning the previous state as a
    /// [`Previous`]. Events the cache does not use return [`Previous::None`].
    pub fn update_dispatch(&self, event: &DispatchEvent) -> Previous {
        crate::update::dispatch(self, event)
    }

    /// Returns the bot's own user, from `READY` and `USER_UPDATE`. Always kept.
    ///
    /// discord.js counterpart: `client.user`.
    #[must_use]
    pub fn current_user(&self) -> Option<Arc<CurrentUser>> {
        self.current_user
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Returns the guilds that are unavailable: listed in `READY` but not loaded yet, or
    /// unreachable because of a Discord outage. Their cached data, if any, is kept.
    ///
    /// discord.js counterpart: guilds with `Guild#available === false`.
    #[must_use]
    pub fn unavailable_guilds(&self) -> Vec<GuildId> {
        self.unavailable_guilds
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }

    /// Returns the guilds ([`Resource::Guild`]).
    ///
    /// discord.js counterpart: `client.guilds.cache`.
    #[must_use]
    pub fn guilds(&self) -> CacheView<'_, GuildId, CachedGuild, B> {
        CacheView::new(self.guilds.as_ref(), None)
    }

    /// Returns the channels ([`Resource::Channel`]); [`CacheView::in_guild`] lists a guild's.
    ///
    /// discord.js counterpart: `client.channels.cache`.
    #[must_use]
    pub fn channels(&self) -> CacheView<'_, ChannelId, Channel, B> {
        CacheView::new(self.channels.as_ref(), Some(&self.guild_channels))
    }

    /// Returns the users ([`Resource::User`]).
    ///
    /// discord.js counterpart: `client.users.cache`.
    #[must_use]
    pub fn users(&self) -> CacheView<'_, UserId, User, B> {
        CacheView::new(self.users.as_ref(), None)
    }

    /// Returns a guild's roles ([`Resource::Role`]).
    ///
    /// discord.js counterpart: `guild.roles.cache`.
    #[must_use]
    pub fn roles(&self, guild_id: GuildId) -> GuildView<'_, RoleId, Role, B> {
        GuildView::new(self.roles.as_ref(), guild_id)
    }

    /// Returns a guild's custom emojis ([`Resource::Emoji`]).
    ///
    /// discord.js counterpart: `guild.emojis.cache`.
    #[must_use]
    pub fn emojis(&self, guild_id: GuildId) -> GuildView<'_, EmojiId, Emoji, B> {
        GuildView::new(self.emojis.as_ref(), guild_id)
    }

    /// Returns a guild's stickers ([`Resource::Sticker`]).
    ///
    /// discord.js counterpart: `guild.stickers.cache`.
    #[must_use]
    pub fn stickers(&self, guild_id: GuildId) -> GuildView<'_, StickerId, Sticker, B> {
        GuildView::new(self.stickers.as_ref(), guild_id)
    }

    /// Returns a guild's members ([`Resource::Member`]), by user ID.
    ///
    /// discord.js counterpart: `guild.members.cache`.
    #[must_use]
    pub fn members(&self, guild_id: GuildId) -> GuildView<'_, UserId, CachedMember, B> {
        GuildView::new(self.members.as_ref(), guild_id)
    }
}

/// The changes the event handlers in `update.rs` make.
impl<B: Backend> Cache<B> {
    pub(crate) fn set_current_user(&self, user: &CurrentUser) -> Option<Arc<CurrentUser>> {
        self.current_user
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .replace(Arc::new(user.clone()))
    }

    pub(crate) fn mark_unavailable(&self, guild_id: GuildId) {
        self.unavailable_guilds.insert(guild_id, ());
    }

    /// `GUILD_CREATE` of an available guild.
    pub(crate) fn insert_guild(&self, guild: &AvailableGuild) -> Option<Arc<CachedGuild>> {
        let id = guild.guild.id;
        self.unavailable_guilds.remove(&id);
        for channel in &guild.channels {
            self.insert_channel(channel, Some(id));
        }
        replace(self.roles.as_ref(), id, &guild.guild.roles, |role| role.id);
        replace(self.emojis.as_ref(), id, &guild.guild.emojis, |emoji| {
            emoji.id
        });
        replace(
            self.stickers.as_ref(),
            id,
            &guild.guild.stickers,
            |sticker| sticker.id,
        );
        for member in &guild.members {
            self.upsert_member(id, member);
        }
        self.put_guild(CachedGuild::from_available(guild))
    }

    /// `GUILD_UPDATE`. The guild's roles, emojis and stickers are replaced by the update's,
    /// unless a list is empty (Discord may omit them; their own events cover removals).
    pub(crate) fn update_guild(&self, guild: &Guild) -> Option<Arc<CachedGuild>> {
        if !guild.roles.is_empty() {
            replace(self.roles.as_ref(), guild.id, &guild.roles, |role| role.id);
        }
        if !guild.emojis.is_empty() {
            replace(self.emojis.as_ref(), guild.id, &guild.emojis, |emoji| {
                emoji.id
            });
        }
        if !guild.stickers.is_empty() {
            replace(
                self.stickers.as_ref(),
                guild.id,
                &guild.stickers,
                |sticker| sticker.id,
            );
        }
        let guilds = self.guilds.as_ref()?;
        let previous = guilds.get(&guild.id)?;
        let updated = CachedGuild::updated(guild, &previous);
        guilds.insert(guild.id, Arc::new(updated)).previous
    }

    /// `GUILD_DELETE`: the bot left the guild (`unavailable == false`), or the guild is in an
    /// outage, in which case its data stays.
    pub(crate) fn remove_guild(&self, id: GuildId, unavailable: bool) -> Option<Arc<CachedGuild>> {
        if unavailable {
            self.mark_unavailable(id);
            return None;
        }
        self.unavailable_guilds.remove(&id);
        self.remove_guild_children(id);
        self.guilds.as_ref()?.remove(&id)
    }

    fn put_guild(&self, guild: CachedGuild) -> Option<Arc<CachedGuild>> {
        let guilds = self.guilds.as_ref()?;
        let inserted = guilds.insert(guild.id, Arc::new(guild));
        if let Some((evicted, _)) = inserted.evicted {
            self.remove_guild_children(evicted);
        }
        inserted.previous
    }

    /// Drops everything cached for a guild (it was deleted, or evicted by the guild limit).
    fn remove_guild_children(&self, id: GuildId) {
        if let Some(channels) = &self.channels {
            for channel in self.guild_channels.remove_parent(id) {
                channels.remove(&channel);
            }
        }
        remove_all(self.roles.as_ref(), id);
        remove_all(self.emojis.as_ref(), id);
        remove_all(self.stickers.as_ref(), id);
        remove_all(self.members.as_ref(), id);
    }

    /// Adds a guild member to the guild's member count.
    pub(crate) fn count_member(&self, guild_id: GuildId, joined: bool) {
        let Some(guilds) = &self.guilds else { return };
        let Some(guild) = guilds.get(&guild_id) else {
            return;
        };
        let mut updated = CachedGuild::clone(&guild);
        updated.member_count = if joined {
            updated.member_count.saturating_add(1)
        } else {
            updated.member_count.saturating_sub(1)
        };
        guilds.insert(guild_id, Arc::new(updated));
    }

    /// `CHANNEL_CREATE` / `CHANNEL_UPDATE`, and the channels of a `GUILD_CREATE` (which lack
    /// their `guild_id`; the cached copy gets it). Threads are a resource of their own and are
    /// skipped here.
    pub(crate) fn insert_channel(
        &self,
        channel: &Channel,
        guild_id: Option<GuildId>,
    ) -> Option<Arc<Channel>> {
        if channel.as_thread().is_some() {
            return None;
        }
        let channels = self.channels.as_ref()?;
        let mut channel = channel.clone();
        if let Some(guild_id) = guild_id.or_else(|| channel.guild_id()) {
            set_guild_id(&mut channel, guild_id);
            self.guild_channels.insert(guild_id, channel.id());
        }
        let inserted = channels.insert(channel.id(), Arc::new(channel));
        if let Some((id, evicted)) = &inserted.evicted
            && let Some(guild_id) = evicted.guild_id()
        {
            self.guild_channels.remove(guild_id, *id);
        }
        inserted.previous
    }

    /// `CHANNEL_DELETE`.
    pub(crate) fn remove_channel(&self, channel: &Channel) -> Option<Arc<Channel>> {
        let removed = self.channels.as_ref()?.remove(&channel.id());
        let guild_id = channel
            .guild_id()
            .or_else(|| removed.as_ref().and_then(|removed| removed.guild_id()));
        if let Some(guild_id) = guild_id {
            self.guild_channels.remove(guild_id, channel.id());
        }
        removed
    }

    /// `CHANNEL_PINS_UPDATE`.
    pub(crate) fn update_pins(
        &self,
        channel_id: ChannelId,
        last_pin_timestamp: Option<Timestamp>,
    ) -> Option<Arc<Channel>> {
        let channels = self.channels.as_ref()?;
        let mut channel = Channel::clone(&*channels.get(&channel_id)?);
        if !set_last_pin_timestamp(&mut channel, last_pin_timestamp) {
            return None;
        }
        channels.insert(channel_id, Arc::new(channel)).previous
    }

    pub(crate) fn insert_role(&self, guild_id: GuildId, role: &Role) -> Option<Arc<Role>> {
        self.roles
            .as_ref()?
            .insert(guild_id, role.id, Arc::new(role.clone()))
            .previous
    }

    pub(crate) fn remove_role(&self, guild_id: GuildId, role_id: RoleId) -> Option<Arc<Role>> {
        self.roles.as_ref()?.remove(guild_id, role_id)
    }

    pub(crate) fn replace_emojis(&self, guild_id: GuildId, emojis: &[Emoji]) -> Vec<Arc<Emoji>> {
        replace(self.emojis.as_ref(), guild_id, emojis, |emoji| emoji.id)
    }

    pub(crate) fn replace_stickers(
        &self,
        guild_id: GuildId,
        stickers: &[Sticker],
    ) -> Vec<Arc<Sticker>> {
        replace(self.stickers.as_ref(), guild_id, stickers, |sticker| {
            sticker.id
        })
    }

    /// `GUILD_MEMBER_ADD`, the members of `GUILD_CREATE` and `GUILD_MEMBERS_CHUNK`.
    pub(crate) fn upsert_member(
        &self,
        guild_id: GuildId,
        member: &Member,
    ) -> Option<Arc<CachedMember>> {
        let Some(members) = &self.members else {
            self.cache_user(&member.user);
            return None;
        };
        let user = self.share_user(&member.user);
        let id = user.id;
        members
            .insert(
                guild_id,
                id,
                Arc::new(CachedMember::new(guild_id, member, user)),
            )
            .previous
    }

    /// `GUILD_MEMBER_UPDATE`. A member that was not cached is added, like discord.js does.
    pub(crate) fn update_member(&self, update: &GuildMemberUpdate) -> Option<Arc<CachedMember>> {
        let Some(members) = &self.members else {
            self.cache_user(&update.user);
            return None;
        };
        let user = self.share_user(&update.user);
        let id = user.id;
        let previous = members.get(update.guild_id, id);
        let updated = CachedMember::updated(update, user, previous.as_deref());
        members
            .insert(update.guild_id, id, Arc::new(updated))
            .previous
    }

    /// `GUILD_MEMBER_REMOVE`. The user stays cached, like in discord.js.
    pub(crate) fn remove_member(
        &self,
        guild_id: GuildId,
        user: &User,
    ) -> Option<Arc<CachedMember>> {
        self.cache_user(user);
        self.members.as_ref()?.remove(guild_id, user.id)
    }

    /// Caches a user seen in an event, if users are cached.
    fn cache_user(&self, user: &User) {
        if self.users.is_some() {
            self.share_user(user);
        }
    }

    /// Returns an `Arc` of `user`: the user resource's own when it holds an equal user (so a
    /// user in many guilds is stored once), else a new one, which the user resource then
    /// keeps.
    fn share_user(&self, user: &User) -> Arc<User> {
        let Some(users) = &self.users else {
            return Arc::new(user.clone());
        };
        if let Some(cached) = users.get(&user.id)
            && *cached == *user
        {
            return cached;
        }
        let shared = Arc::new(user.clone());
        users.insert(user.id, Arc::clone(&shared));
        shared
    }
}

impl<B: Backend> fmt::Debug for Cache<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cache")
            .field("config", &self.config)
            .field("backend", &self.backend)
            .field("guilds", &self.guilds().len())
            .field("channels", &self.channels().len())
            .field("users", &self.users().len())
            .field("members", &self.members.as_ref().map_or(0, Scoped::len))
            .finish_non_exhaustive()
    }
}

/// Creates the store of a resource kept over the whole cache, if it is enabled.
fn global<K, V, B>(
    backend: &B,
    config: &CacheConfig,
    resource: Resource,
) -> Option<GlobalOf<K, V, B>>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + Sync + 'static,
    B: Backend,
{
    let limit = config.limit(resource);
    limit
        .is_enabled()
        .then(|| Global::new(backend.store(resource, limit), limit))
}

/// Creates the store of a resource kept per guild, if it is enabled.
fn scoped<K, V, B>(
    backend: &B,
    config: &CacheConfig,
    resource: Resource,
) -> Option<ScopedOf<K, V, B>>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + Sync + 'static,
    B: Backend,
{
    let limit = config.limit(resource);
    limit
        .is_enabled()
        .then(|| Scoped::new(backend.store(resource, limit), limit))
}

/// Replaces a guild's entries of a resource (the roles of a `GUILD_CREATE`, the emojis of a
/// `GUILD_EMOJIS_UPDATE`, …), returning the previous ones.
fn replace<K, V, S>(
    entries: Option<&Scoped<GuildId, K, V, S>>,
    guild_id: GuildId,
    items: &[V],
    id: impl Fn(&V) -> K,
) -> Vec<Arc<V>>
where
    K: Eq + Hash + Copy,
    V: Clone,
    S: Store<(GuildId, K), V>,
{
    let Some(entries) = entries else {
        return Vec::new();
    };
    let previous = entries.remove_parent(guild_id);
    for item in items {
        entries.insert(guild_id, id(item), Arc::new(item.clone()));
    }
    previous
}

fn remove_all<K, V, S>(entries: Option<&Scoped<GuildId, K, V, S>>, guild_id: GuildId)
where
    K: Eq + Hash + Copy,
    S: Store<(GuildId, K), V>,
{
    if let Some(entries) = entries {
        entries.remove_parent(guild_id);
    }
}

/// Sets the guild of a guild channel (Discord omits it in `GUILD_CREATE`).
const fn set_guild_id(channel: &mut Channel, guild_id: GuildId) {
    let slot = match channel {
        Channel::Text(channel) | Channel::Announcement(channel) => &mut channel.guild_id,
        Channel::Voice(channel) | Channel::Stage(channel) => &mut channel.guild_id,
        Channel::Category(channel) => &mut channel.guild_id,
        Channel::AnnouncementThread(channel)
        | Channel::PublicThread(channel)
        | Channel::PrivateThread(channel) => &mut channel.guild_id,
        Channel::Directory(channel) => &mut channel.guild_id,
        Channel::Forum(channel) | Channel::Media(channel) => &mut channel.guild_id,
        Channel::Unknown(channel) => &mut channel.guild_id,
        _ => return,
    };
    *slot = Some(guild_id);
}

/// Sets the last pin time of a channel that has one; returns whether it has.
const fn set_last_pin_timestamp(channel: &mut Channel, timestamp: Option<Timestamp>) -> bool {
    let slot = match channel {
        Channel::Text(channel) | Channel::Announcement(channel) => &mut channel.last_pin_timestamp,
        Channel::Dm(channel) => &mut channel.last_pin_timestamp,
        Channel::GroupDm(channel) => &mut channel.last_pin_timestamp,
        Channel::AnnouncementThread(channel)
        | Channel::PublicThread(channel)
        | Channel::PrivateThread(channel) => &mut channel.last_pin_timestamp,
        _ => return false,
    };
    *slot = timestamp;
    true
}

/// The cache is shared between shards.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Cache>();
};
