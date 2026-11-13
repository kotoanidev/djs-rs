//! [`RoleManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, GuildView};
use djs_collection::Collection;
use djs_model::{
    guild::Role,
    id::{GuildId, RoleId},
    rest::{CreateGuildRole, ModifyGuildRole, ModifyGuildRolePosition},
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// The roles of one guild. Returned by [`Context::roles`].
///
/// discord.js counterpart: `RoleManager` (`guild.roles`).
#[derive(Debug)]
pub struct RoleManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    guild_id: GuildId,
}

impl<B: Backend> Clone for RoleManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for RoleManager<'_, B> {}

impl<'a, B: Backend> RoleManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, guild_id: GuildId) -> Self {
        Self { ctx, guild_id }
    }

    /// The guild these roles belong to.
    #[must_use]
    pub const fn guild_id(self) -> GuildId {
        self.guild_id
    }

    /// The cached roles of the guild, if the client has a cache.
    ///
    /// discord.js counterpart: `RoleManager#cache`.
    #[must_use]
    pub fn cache(self) -> Option<GuildView<'a, RoleId, Role, B>> {
        self.ctx.cache().map(|cache| cache.roles(self.guild_id))
    }

    /// A role: the cached one, or [Get Guild Role](djs_rest::Rest::get_guild_role), whose
    /// result is cached.
    ///
    /// discord.js counterpart: `RoleManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if Discord was asked and the request failed.
    pub fn fetch(
        self,
        role_id: RoleId,
    ) -> Fetch<Arc<Role>, impl Future<Output = Result<Arc<Role>, Error>> + Send + 'a> {
        let cached = self.ctx.ops().role(self.guild_id, role_id);
        let Self { ctx, guild_id } = self;
        Fetch::new(cached, async move {
            let role = ctx.rest().get_guild_role(guild_id, role_id).await?;
            Ok(ctx.store_role(guild_id, &role))
        })
    }

    /// Every role of the guild, in Discord's order ([Get Guild
    /// Roles](djs_rest::Rest::get_guild_roles)); the roles are cached.
    ///
    /// discord.js counterpart: `RoleManager#fetch()`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list(self) -> Result<Collection<RoleId, Arc<Role>>, Error> {
        let roles = self.ctx.rest().get_guild_roles(self.guild_id).await?;
        Ok(roles
            .iter()
            .map(|role| (role.id, self.ctx.store_role(self.guild_id, role)))
            .collect())
    }
    /// Creates a role ([Create Guild Role](djs_rest::Rest::create_guild_role)); it is cached.
    ///
    /// discord.js counterpart: `RoleManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_ROLES`).
    pub async fn create(
        self,
        body: &CreateGuildRole,
        reason: Option<&str>,
    ) -> Result<Arc<Role>, Error> {
        let role = self
            .ctx
            .rest()
            .create_guild_role(self.guild_id, body, reason)
            .await?;
        Ok(self.ctx.store_role(self.guild_id, &role))
    }

    /// Edits a role ([Modify Guild Role](djs_rest::Rest::modify_guild_role)); the result is
    /// cached.
    ///
    /// discord.js counterpart: `RoleManager#edit` / `Role#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (the role must be below the bot's highest
    /// role).
    pub async fn edit(
        self,
        role_id: RoleId,
        body: &ModifyGuildRole,
        reason: Option<&str>,
    ) -> Result<Arc<Role>, Error> {
        let role = self
            .ctx
            .rest()
            .modify_guild_role(self.guild_id, role_id, body, reason)
            .await?;
        Ok(self.ctx.store_role(self.guild_id, &role))
    }

    /// Deletes a role ([Delete Guild Role](djs_rest::Rest::delete_guild_role)). The cache is
    /// updated by the `GUILD_ROLE_DELETE` event that follows (D-236).
    ///
    /// discord.js counterpart: `RoleManager#delete` / `Role#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn delete(self, role_id: RoleId, reason: Option<&str>) -> Result<(), Error> {
        self.ctx
            .rest()
            .delete_guild_role(self.guild_id, role_id, reason)
            .await
    }

    /// Moves roles ([Modify Guild Role
    /// Positions](djs_rest::Rest::modify_guild_role_positions)), returning every role of the
    /// guild in Discord's order; the roles are cached.
    ///
    /// discord.js counterpart: `RoleManager#setPositions`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn set_positions(
        self,
        positions: &[ModifyGuildRolePosition],
        reason: Option<&str>,
    ) -> Result<Collection<RoleId, Arc<Role>>, Error> {
        let roles = self
            .ctx
            .rest()
            .modify_guild_role_positions(self.guild_id, positions, reason)
            .await?;
        Ok(roles
            .iter()
            .map(|role| (role.id, self.ctx.store_role(self.guild_id, role)))
            .collect())
    }
}
