//! [`GuildManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, CacheView, CachedGuild};
use djs_collection::Collection;
use djs_model::{
    audit_log::AuditLog,
    guild::Integration,
    id::{GuildId, IntegrationId},
    rest::{GetGuild, GetGuildAuditLog, ModifyGuild},
};
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
    /// Edits a guild's settings ([Modify Guild](djs_rest::Rest::modify_guild)); the result is
    /// cached.
    ///
    /// discord.js counterpart: `GuildManager#edit` / `Guild#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_GUILD`).
    pub async fn edit(
        self,
        guild_id: GuildId,
        body: &ModifyGuild,
        reason: Option<&str>,
    ) -> Result<Arc<CachedGuild>, Error> {
        let guild = self.ctx.rest().modify_guild(guild_id, body, reason).await?;
        Ok(self.ctx.store_guild(&guild))
    }

    /// Makes the bot leave a guild ([Leave Guild](djs_rest::Rest::leave_guild)). The cache is
    /// updated by the `GUILD_DELETE` event that follows (D-236).
    ///
    /// discord.js counterpart: `Guild#leave`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (an owner cannot leave its guild).
    pub async fn leave(self, guild_id: GuildId) -> Result<(), Error> {
        self.ctx.rest().leave_guild(guild_id).await
    }
    /// A page of the guild's audit log, newest first ([Get Guild Audit
    /// Log](djs_rest::Rest::get_guild_audit_log)), with the users, webhooks, threads, … its
    /// entries mention.
    ///
    /// discord.js counterpart: `Guild#fetchAuditLogs`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `VIEW_AUDIT_LOG`).
    pub async fn audit_log(
        self,
        guild_id: GuildId,
        query: &GetGuildAuditLog,
    ) -> Result<AuditLog, Error> {
        self.ctx.rest().get_guild_audit_log(guild_id, query).await
    }

    /// The guild's integrations: bots, Twitch, YouTube, … ([Get Guild
    /// Integrations](djs_rest::Rest::get_guild_integrations)).
    ///
    /// discord.js counterpart: `Guild#fetchIntegrations`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_GUILD`).
    pub async fn integrations(
        self,
        guild_id: GuildId,
    ) -> Result<Collection<IntegrationId, Integration>, Error> {
        let integrations = self.ctx.rest().get_guild_integrations(guild_id).await?;
        Ok(integrations
            .into_iter()
            .map(|integration| (integration.id, integration))
            .collect())
    }

    /// Removes an integration, kicking its bot if it has one ([Delete Guild
    /// Integration](djs_rest::Rest::delete_guild_integration)).
    ///
    /// discord.js counterpart: `Integration#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_GUILD`).
    pub async fn delete_integration(
        self,
        guild_id: GuildId,
        integration_id: IntegrationId,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.ctx
            .rest()
            .delete_guild_integration(guild_id, integration_id, reason)
            .await
    }
}
