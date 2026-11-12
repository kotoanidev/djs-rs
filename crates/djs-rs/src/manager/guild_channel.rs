//! [`GuildChannelManager`].

use std::sync::Arc;

use djs_cache::Backend;
use djs_collection::Collection;
use djs_model::{
    channel::Channel,
    id::{ChannelId, GuildId},
    rest::{CreateGuildChannel, ModifyGuildChannelPosition},
};
use djs_rest::Error;

use super::FetchedThreads;
use crate::Context;

/// The channels of one guild, as a whole (D-234). Returned by [`Context::guild_channels`].
///
/// Operations on one channel by its ID (fetch, edit, delete, permission overwrites) are on
/// [`Context::channels`]. The cached channels of a guild are listed by
/// [`CacheView::in_guild`](djs_cache::CacheView::in_guild) on `ctx.channels().cache()`.
///
/// discord.js counterpart: `GuildChannelManager` (`guild.channels`).
#[derive(Debug)]
pub struct GuildChannelManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    guild_id: GuildId,
}

impl<B: Backend> Clone for GuildChannelManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for GuildChannelManager<'_, B> {}

impl<'a, B: Backend> GuildChannelManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, guild_id: GuildId) -> Self {
        Self { ctx, guild_id }
    }

    /// The guild these channels belong to.
    #[must_use]
    pub const fn guild_id(self) -> GuildId {
        self.guild_id
    }

    /// Every channel of the guild except threads, in Discord's order ([Get Guild
    /// Channels](djs_rest::Rest::get_guild_channels)); the channels are cached.
    ///
    /// discord.js counterpart: `GuildChannelManager#fetch()`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list(self) -> Result<Collection<ChannelId, Arc<Channel>>, Error> {
        let channels = self.ctx.rest().get_guild_channels(self.guild_id).await?;
        Ok(channels
            .iter()
            .map(|channel| (channel.id(), self.ctx.store_channel(channel)))
            .collect())
    }

    /// Creates a channel ([Create Guild Channel](djs_rest::Rest::create_guild_channel)); it is
    /// cached.
    ///
    /// discord.js counterpart: `GuildChannelManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_CHANNELS`).
    pub async fn create(
        self,
        body: &CreateGuildChannel,
        reason: Option<&str>,
    ) -> Result<Arc<Channel>, Error> {
        let channel = self
            .ctx
            .rest()
            .create_guild_channel(self.guild_id, body, reason)
            .await?;
        Ok(self.ctx.store_channel(&channel))
    }

    /// Moves channels ([Modify Guild Channel
    /// Positions](djs_rest::Rest::modify_guild_channel_positions)); the cache follows the
    /// `CHANNEL_UPDATE` events Discord sends.
    ///
    /// discord.js counterpart: `GuildChannelManager#setPositions`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_CHANNELS`).
    pub async fn set_positions(
        self,
        positions: &[ModifyGuildChannelPosition],
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.ctx
            .rest()
            .modify_guild_channel_positions(self.guild_id, positions, reason)
            .await
    }
    /// The guild's active threads, with the bot's memberships ([List Active Guild
    /// Threads](djs_rest::Rest::list_active_guild_threads)); the threads are cached.
    ///
    /// discord.js counterpart: `GuildChannelManager#fetchActiveThreads`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list_active_threads(self) -> Result<FetchedThreads, Error> {
        let active = self
            .ctx
            .rest()
            .list_active_guild_threads(self.guild_id)
            .await?;
        Ok(self
            .ctx
            .fetched_threads(&active.threads, active.members, false))
    }
}
