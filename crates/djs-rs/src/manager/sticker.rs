//! [`StickerManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, GuildView};
use djs_collection::Collection;
use djs_model::{
    id::{GuildId, StickerId},
    rest::{CreateGuildSticker, ModifyGuildSticker},
    sticker::Sticker,
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// The stickers of one guild. Returned by [`Context::stickers`].
///
/// discord.js counterpart: `GuildStickerManager` (`guild.stickers`).
#[derive(Debug)]
pub struct StickerManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    guild_id: GuildId,
}

impl<B: Backend> Clone for StickerManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for StickerManager<'_, B> {}

impl<'a, B: Backend> StickerManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, guild_id: GuildId) -> Self {
        Self { ctx, guild_id }
    }

    /// The guild these stickers belong to.
    #[must_use]
    pub const fn guild_id(self) -> GuildId {
        self.guild_id
    }

    /// The cached stickers of the guild, if the client has a cache.
    ///
    /// discord.js counterpart: `GuildStickerManager#cache`.
    #[must_use]
    pub fn cache(self) -> Option<GuildView<'a, StickerId, Sticker, B>> {
        self.ctx.cache().map(|cache| cache.stickers(self.guild_id))
    }

    /// A sticker: the cached one, or [Get Guild Sticker](djs_rest::Rest::get_guild_sticker),
    /// whose result is cached.
    ///
    /// discord.js counterpart: `GuildStickerManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if Discord was asked and the request failed.
    pub fn fetch(
        self,
        sticker_id: StickerId,
    ) -> Fetch<Arc<Sticker>, impl Future<Output = Result<Arc<Sticker>, Error>> + Send + 'a> {
        let cached = self.cache().and_then(|stickers| stickers.get(sticker_id));
        let Self { ctx, guild_id } = self;
        Fetch::new(cached, async move {
            let sticker = ctx.rest().get_guild_sticker(guild_id, sticker_id).await?;
            Ok(ctx.store_sticker(guild_id, &sticker))
        })
    }

    /// Every sticker of the guild ([List Guild
    /// Stickers](djs_rest::Rest::list_guild_stickers)); the stickers are cached.
    ///
    /// discord.js counterpart: `GuildStickerManager#fetch()`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list(self) -> Result<Collection<StickerId, Arc<Sticker>>, Error> {
        let stickers = self.ctx.rest().list_guild_stickers(self.guild_id).await?;
        Ok(stickers
            .iter()
            .map(|sticker| (sticker.id, self.ctx.store_sticker(self.guild_id, sticker)))
            .collect())
    }

    /// Uploads a sticker ([Create Guild Sticker](djs_rest::Rest::create_guild_sticker)); it
    /// is cached.
    ///
    /// discord.js counterpart: `GuildStickerManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `CREATE_EXPRESSIONS`; the file
    /// must be a PNG, APNG, GIF or Lottie JSON of at most 512 KiB).
    pub async fn create(
        self,
        body: &CreateGuildSticker,
        reason: Option<&str>,
    ) -> Result<Arc<Sticker>, Error> {
        let sticker = self
            .ctx
            .rest()
            .create_guild_sticker(self.guild_id, body, reason)
            .await?;
        Ok(self.ctx.store_sticker(self.guild_id, &sticker))
    }

    /// Changes a sticker's name, description or tags ([Modify Guild
    /// Sticker](djs_rest::Rest::modify_guild_sticker)); the result is cached.
    ///
    /// discord.js counterpart: `GuildStickerManager#edit` / `Sticker#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn edit(
        self,
        sticker_id: StickerId,
        body: &ModifyGuildSticker,
        reason: Option<&str>,
    ) -> Result<Arc<Sticker>, Error> {
        let sticker = self
            .ctx
            .rest()
            .modify_guild_sticker(self.guild_id, sticker_id, body, reason)
            .await?;
        Ok(self.ctx.store_sticker(self.guild_id, &sticker))
    }

    /// Deletes a sticker ([Delete Guild Sticker](djs_rest::Rest::delete_guild_sticker)). The
    /// cache is updated by the `GUILD_STICKERS_UPDATE` event that follows (D-236).
    ///
    /// discord.js counterpart: `GuildStickerManager#delete` / `Sticker#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn delete(self, sticker_id: StickerId, reason: Option<&str>) -> Result<(), Error> {
        self.ctx
            .rest()
            .delete_guild_sticker(self.guild_id, sticker_id, reason)
            .await
    }
}
