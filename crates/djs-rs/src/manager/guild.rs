//! [`GuildManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, CacheView, CachedGuild};
use djs_model::{id::GuildId, rest::GetGuild};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// The guilds of the bot. Returned by [`Context::guilds`].
///
/// discord.js counterpart: `GuildManager` (`client.guilds`).
#[derive(Debug)]
pub struct GuildManager<'a, B: Backend> {
    ctx: &'a Context<B>,
}

impl<B: Backend> Clone for GuildManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for GuildManager<'_, B> {}

impl<'a, B: Backend> GuildManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>) -> Self {
        Self { ctx }
    }

    /// The cached guilds, if the client has a cache.
    ///
    /// discord.js counterpart: `GuildManager#cache`.
    #[must_use]
    pub fn cache(self) -> Option<CacheView<'a, GuildId, CachedGuild, B>> {
        self.ctx.cache().map(djs_cache::Cache::guilds)
    }

    /// A guild: the cached one, or [Get Guild](djs_rest::Rest::get_guild) (with the
    /// approximate member and presence counts), whose result is cached.
    ///
    /// discord.js counterpart: `GuildManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if Discord was asked and the request failed (for example, the
    /// bot is not in the guild).
    pub fn fetch(
        self,
        guild_id: GuildId,
    ) -> Fetch<Arc<CachedGuild>, impl Future<Output = Result<Arc<CachedGuild>, Error>> + Send + 'a>
    {
        let cached = self.cache().and_then(|guilds| guilds.get(guild_id));
        let ctx = self.ctx;
        Fetch::new(cached, async move {
            let mut query = GetGuild::default();
            query.with_counts = Some(true);
            let guild = ctx.rest().get_guild(guild_id, &query).await?;
            Ok(ctx.store_guild(&guild))
        })
    }
}
