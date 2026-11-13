//! [`EmojiManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, GuildView};
use djs_collection::Collection;
use djs_model::{
    emoji::Emoji,
    id::{EmojiId, GuildId},
    rest::{CreateGuildEmoji, ModifyGuildEmoji},
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// The custom emojis of one guild. Returned by [`Context::emojis`].
///
/// discord.js counterpart: `GuildEmojiManager` (`guild.emojis`).
#[derive(Debug)]
pub struct EmojiManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    guild_id: GuildId,
}

impl<B: Backend> Clone for EmojiManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for EmojiManager<'_, B> {}

impl<'a, B: Backend> EmojiManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, guild_id: GuildId) -> Self {
        Self { ctx, guild_id }
    }

    /// The guild these emojis belong to.
    #[must_use]
    pub const fn guild_id(self) -> GuildId {
        self.guild_id
    }

    /// The cached emojis of the guild, if the client has a cache.
    ///
    /// discord.js counterpart: `GuildEmojiManager#cache`.
    #[must_use]
    pub fn cache(self) -> Option<GuildView<'a, EmojiId, Emoji, B>> {
        self.ctx.cache().map(|cache| cache.emojis(self.guild_id))
    }

    /// An emoji: the cached one, or [Get Guild Emoji](djs_rest::Rest::get_guild_emoji),
    /// whose result is cached.
    ///
    /// discord.js counterpart: `GuildEmojiManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if Discord was asked and the request failed.
    pub fn fetch(
        self,
        emoji_id: EmojiId,
    ) -> Fetch<Arc<Emoji>, impl Future<Output = Result<Arc<Emoji>, Error>> + Send + 'a> {
        let cached = self.ctx.ops().emoji(self.guild_id, emoji_id);
        let Self { ctx, guild_id } = self;
        Fetch::new(cached, async move {
            let emoji = ctx.rest().get_guild_emoji(guild_id, emoji_id).await?;
            Ok(ctx.store_emoji(guild_id, &emoji))
        })
    }

    /// Every custom emoji of the guild ([List Guild
    /// Emojis](djs_rest::Rest::list_guild_emojis)); the emojis are cached.
    ///
    /// discord.js counterpart: `GuildEmojiManager#fetch()`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list(self) -> Result<Collection<EmojiId, Arc<Emoji>>, Error> {
        let emojis = self.ctx.rest().list_guild_emojis(self.guild_id).await?;
        Ok(emojis
            .iter()
            .map(|emoji| (emoji.id, self.ctx.store_emoji(self.guild_id, emoji)))
            .collect())
    }

    /// Uploads an emoji ([Create Guild Emoji](djs_rest::Rest::create_guild_emoji)); it is
    /// cached.
    ///
    /// discord.js counterpart: `GuildEmojiManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `CREATE_EXPRESSIONS`; the
    /// image must be at most 256 KiB).
    pub async fn create(
        self,
        body: &CreateGuildEmoji,
        reason: Option<&str>,
    ) -> Result<Arc<Emoji>, Error> {
        let emoji = self
            .ctx
            .rest()
            .create_guild_emoji(self.guild_id, body, reason)
            .await?;
        Ok(self.ctx.store_emoji(self.guild_id, &emoji))
    }

    /// Renames an emoji or changes the roles allowed to use it ([Modify Guild
    /// Emoji](djs_rest::Rest::modify_guild_emoji)); the result is cached.
    ///
    /// discord.js counterpart: `GuildEmojiManager#edit` / `GuildEmoji#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn edit(
        self,
        emoji_id: EmojiId,
        body: &ModifyGuildEmoji,
        reason: Option<&str>,
    ) -> Result<Arc<Emoji>, Error> {
        let emoji = self
            .ctx
            .rest()
            .modify_guild_emoji(self.guild_id, emoji_id, body, reason)
            .await?;
        Ok(self.ctx.store_emoji(self.guild_id, &emoji))
    }

    /// Deletes an emoji ([Delete Guild Emoji](djs_rest::Rest::delete_guild_emoji)). The
    /// cache is updated by the `GUILD_EMOJIS_UPDATE` event that follows (D-236).
    ///
    /// discord.js counterpart: `GuildEmojiManager#delete` / `GuildEmoji#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn delete(self, emoji_id: EmojiId, reason: Option<&str>) -> Result<(), Error> {
        self.ctx
            .rest()
            .delete_guild_emoji(self.guild_id, emoji_id, reason)
            .await
    }
}
