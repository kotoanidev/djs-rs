//! Borrowed read-only views of one resource (D-162), like a discord.js manager's `cache`.

use core::{fmt, hash::Hash};
use std::sync::Arc;

use djs_model::{
    channel::Channel,
    id::{ChannelId, GuildId},
};

use crate::{
    Backend,
    limits::{Global, GlobalOf, Index, ScopedOf},
};

/// A resource kept over the whole cache (guilds, channels, users), as returned by
/// [`Cache::guilds`](crate::Cache::guilds), [`Cache::channels`](crate::Cache::channels) and
/// [`Cache::users`](crate::Cache::users).
///
/// A view of a disabled resource is empty. Every read is a snapshot: the cache may change
/// right after it.
///
/// discord.js counterpart: `client.guilds.cache`, `client.channels.cache`, `client.users.cache`.
///
/// # Examples
///
/// ```
/// use djs_cache::Cache;
/// use djs_model::id::GuildId;
///
/// let cache = Cache::new();
/// let guilds = cache.guilds();
/// assert!(guilds.is_enabled());
/// assert!(guilds.get(GuildId::new(1)).is_none());
/// assert_eq!(guilds.len(), 0);
/// ```
pub struct CacheView<'a, K, V, B>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + Sync + 'static,
    B: Backend,
{
    entries: Option<&'a GlobalOf<K, V, B>>,
    guild_index: Option<&'a Index<GuildId, K>>,
}

impl<'a, K, V, B> CacheView<'a, K, V, B>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + Sync + 'static,
    B: Backend,
{
    pub(crate) const fn new(
        entries: Option<&'a GlobalOf<K, V, B>>,
        guild_index: Option<&'a Index<GuildId, K>>,
    ) -> Self {
        Self {
            entries,
            guild_index,
        }
    }

    /// Returns whether the resource is cached at all.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.entries.is_some()
    }

    /// Returns the entry with this ID.
    ///
    /// discord.js counterpart: `Collection#get`.
    #[must_use]
    pub fn get(&self, id: K) -> Option<Arc<V>> {
        self.entries?.get(&id)
    }

    /// Returns whether an entry with this ID is cached.
    ///
    /// discord.js counterpart: `Collection#has`.
    #[must_use]
    pub fn contains(&self, id: K) -> bool {
        self.entries.is_some_and(|entries| entries.contains(&id))
    }

    /// Returns the number of cached entries.
    ///
    /// discord.js counterpart: `Collection#size`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.map_or(0, Global::len)
    }

    /// Returns whether nothing is cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the IDs of every cached entry, in no particular order.
    #[must_use]
    pub fn keys(&self) -> Vec<K> {
        self.entries.map(Global::keys).unwrap_or_default()
    }

    /// Returns every cached entry, in no particular order.
    #[must_use]
    pub fn values(&self) -> Vec<Arc<V>> {
        self.entries.map(Global::values).unwrap_or_default()
    }
}

impl<B: Backend> CacheView<'_, ChannelId, Channel, B> {
    /// Returns the IDs of the cached channels of a guild, in no particular order.
    ///
    /// discord.js counterpart: `guild.channels.cache`.
    #[must_use]
    pub fn in_guild(&self, guild_id: GuildId) -> Vec<ChannelId> {
        self.guild_index
            .map(|index| index.keys_of(guild_id))
            .unwrap_or_default()
    }
}

impl<K, V, B> fmt::Debug for CacheView<'_, K, V, B>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + Sync + 'static,
    B: Backend,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CacheView")
            .field("enabled", &self.is_enabled())
            .field("len", &self.len())
            .finish()
    }
}

/// A resource kept per guild (roles, emojis, stickers, members, …), for one guild, as
/// returned by [`Cache::roles`](crate::Cache::roles), [`Cache::members`](crate::Cache::members)
/// and the like.
///
/// A view of a disabled resource is empty. Every read is a snapshot: the cache may change
/// right after it.
///
/// discord.js counterpart: `guild.roles.cache`, `guild.members.cache`, ….
///
/// # Examples
///
/// ```
/// use djs_cache::Cache;
/// use djs_model::id::{GuildId, RoleId};
///
/// let cache = Cache::new();
/// let roles = cache.roles(GuildId::new(1));
/// assert!(roles.is_enabled());
/// assert!(roles.get(RoleId::new(2)).is_none());
/// // Members are not cached by default.
/// assert!(!cache.members(GuildId::new(1)).is_enabled());
/// ```
pub struct GuildView<'a, K, V, B>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + Sync + 'static,
    B: Backend,
{
    entries: Option<&'a ScopedOf<K, V, B>>,
    guild_id: GuildId,
}

impl<'a, K, V, B> GuildView<'a, K, V, B>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + Sync + 'static,
    B: Backend,
{
    pub(crate) const fn new(entries: Option<&'a ScopedOf<K, V, B>>, guild_id: GuildId) -> Self {
        Self { entries, guild_id }
    }

    /// Returns the guild this view is about.
    #[must_use]
    pub const fn guild_id(&self) -> GuildId {
        self.guild_id
    }

    /// Returns whether the resource is cached at all.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.entries.is_some()
    }

    /// Returns the guild's entry with this ID.
    ///
    /// discord.js counterpart: `Collection#get`.
    #[must_use]
    pub fn get(&self, id: K) -> Option<Arc<V>> {
        self.entries?.get(self.guild_id, id)
    }

    /// Returns whether the guild has a cached entry with this ID.
    ///
    /// discord.js counterpart: `Collection#has`.
    #[must_use]
    pub fn contains(&self, id: K) -> bool {
        self.get(id).is_some()
    }

    /// Returns the number of the guild's cached entries.
    ///
    /// discord.js counterpart: `Collection#size`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries
            .map_or(0, |entries| entries.len_of(self.guild_id))
    }

    /// Returns whether nothing of the guild is cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the IDs of the guild's cached entries, in no particular order.
    #[must_use]
    pub fn keys(&self) -> Vec<K> {
        self.entries
            .map(|entries| entries.keys_of(self.guild_id))
            .unwrap_or_default()
    }

    /// Returns the guild's cached entries, in no particular order.
    #[must_use]
    pub fn values(&self) -> Vec<Arc<V>> {
        self.entries
            .map(|entries| entries.values_of(self.guild_id))
            .unwrap_or_default()
    }
}

impl<K, V, B> fmt::Debug for GuildView<'_, K, V, B>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + Sync + 'static,
    B: Backend,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GuildView")
            .field("guild_id", &self.guild_id)
            .field("enabled", &self.is_enabled())
            .field("len", &self.len())
            .finish()
    }
}
