//! The cache itself.

use core::fmt;

use crate::{Backend, CacheConfig, DashMapBackend};

/// An in-memory view of Discord state, kept up to date from gateway events.
///
/// What it keeps is set by its [`CacheConfig`] (by default guilds, channels, roles and the
/// current user, D-152); where it keeps it by its [`Backend`] (by default
/// [`DashMapBackend`]). Reads return [`Arc`](std::sync::Arc)s, which hold no lock and can be
/// kept across `.await` (D-151). The cache is `Send + Sync`: share one between all shards,
/// for example in an `Arc`.
///
/// discord.js counterpart: the `cache` collections of the client's managers
/// (`client.guilds.cache`, `guild.members.cache`, …), filled by the client's event handlers.
///
/// > **Status:** Phase 5, step 1 — configuration and storage. The cached resources and the
/// > event handling follow in the next steps (D-154).
///
/// # Examples
///
/// ```
/// use djs_cache::{Cache, CacheConfig, Limit, Resource};
///
/// let cache = Cache::with_config(CacheConfig::builder().members(Limit::Max(1_000)).build());
/// assert_eq!(cache.config().limit(Resource::Member), Limit::Max(1_000));
/// ```
pub struct Cache<B: Backend = DashMapBackend> {
    config: CacheConfig,
    backend: B,
}

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
    #[must_use]
    pub const fn with_backend(config: CacheConfig, backend: B) -> Self {
        Self { config, backend }
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
}

impl<B: Backend> fmt::Debug for Cache<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cache")
            .field("config", &self.config)
            .field("backend", &self.backend)
            .finish_non_exhaustive()
    }
}

/// The cache is shared between shards.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Cache>();
};

#[cfg(test)]
mod tests {
    use super::Cache;
    use crate::{CacheConfig, Limit, Resource};

    #[test]
    fn new_uses_the_default_configuration() {
        assert_eq!(Cache::new().config(), &CacheConfig::default());
        assert_eq!(Cache::default().config(), &CacheConfig::default());
    }

    #[test]
    fn with_config_keeps_the_configuration() {
        let cache = Cache::with_config(CacheConfig::discord_js());
        assert_eq!(cache.config().limit(Resource::Message), Limit::Max(200));
    }

    #[test]
    fn debug_shows_configuration_and_backend() {
        let debug = format!("{:?}", Cache::new());
        assert!(
            debug.starts_with("Cache { config: {Guild: Unlimited"),
            "{debug}"
        );
        assert!(debug.ends_with("backend: DashMapBackend, .. }"), "{debug}");
    }
}
