//! [`BanManager`].

use std::future::Future;

use djs_cache::Backend;
use djs_collection::Collection;
use djs_model::{
    guild::Ban,
    id::{GuildId, UserId},
    rest::{CreateGuildBan, GetGuildBans},
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// The bans of one guild. Returned by [`Context::bans`].
///
/// The cache does not keep bans, so every `fetch` asks Discord and returns the
/// [`djs-model`](djs_model) [`Ban`].
///
/// discord.js counterpart: `GuildBanManager` (`guild.bans`).
#[derive(Debug)]
pub struct BanManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    guild_id: GuildId,
}

impl<B: Backend> Clone for BanManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for BanManager<'_, B> {}

impl<'a, B: Backend> BanManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, guild_id: GuildId) -> Self {
        Self { ctx, guild_id }
    }

    /// The guild these bans belong to.
    #[must_use]
    pub const fn guild_id(self) -> GuildId {
        self.guild_id
    }

    /// A user's ban ([Get Guild Ban](djs_rest::Rest::get_guild_ban)).
    ///
    /// discord.js counterpart: `GuildBanManager#fetch(user)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (Discord answers 404 when the user is not
    /// banned).
    pub fn fetch(
        self,
        user_id: UserId,
    ) -> Fetch<Ban, impl Future<Output = Result<Ban, Error>> + Send + 'a> {
        let Self { ctx, guild_id } = self;
        Fetch::new(None, async move {
            ctx.rest().get_guild_ban(guild_id, user_id).await
        })
    }

    /// A page of bans, sorted by user ID ([Get Guild Bans](djs_rest::Rest::get_guild_bans)).
    ///
    /// discord.js counterpart: `GuildBanManager#fetch()`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `BAN_MEMBERS`).
    pub async fn list(self, query: &GetGuildBans) -> Result<Collection<UserId, Ban>, Error> {
        let bans = self.ctx.rest().get_guild_bans(self.guild_id, query).await?;
        Ok(bans.into_iter().map(|ban| (ban.user.id, ban)).collect())
    }

    /// Bans a user, member or not ([Create Guild Ban](djs_rest::Rest::create_guild_ban)).
    ///
    /// discord.js counterpart: `GuildBanManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `BAN_MEMBERS`).
    pub async fn create(
        self,
        user_id: UserId,
        body: &CreateGuildBan,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.ctx
            .rest()
            .create_guild_ban(self.guild_id, user_id, body, reason)
            .await
    }

    /// Lifts a ban ([Remove Guild Ban](djs_rest::Rest::remove_guild_ban)).
    ///
    /// discord.js counterpart: `GuildBanManager#remove`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn remove(self, user_id: UserId, reason: Option<&str>) -> Result<(), Error> {
        self.ctx
            .rest()
            .remove_guild_ban(self.guild_id, user_id, reason)
            .await
    }
}
