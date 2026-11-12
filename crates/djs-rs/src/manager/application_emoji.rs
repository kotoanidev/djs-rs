//! [`ApplicationEmojiManager`].

use std::future::Future;

use djs_cache::Backend;
use djs_collection::Collection;
use djs_model::{
    emoji::Emoji,
    id::{ApplicationId, EmojiId},
    rest::{CreateApplicationEmoji, ModifyApplicationEmoji},
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// The emojis of the bot's application, usable by the bot in every guild. Returned by
/// [`Context::application_emojis`].
///
/// The cache does not keep application emojis, so the [`djs-model`](djs_model) [`Emoji`] is
/// returned.
///
/// discord.js counterpart: `ApplicationEmojiManager` (`client.application.emojis`).
#[derive(Debug)]
pub struct ApplicationEmojiManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    application_id: ApplicationId,
}

impl<B: Backend> Clone for ApplicationEmojiManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for ApplicationEmojiManager<'_, B> {}

impl<'a, B: Backend> ApplicationEmojiManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, application_id: ApplicationId) -> Self {
        Self {
            ctx,
            application_id,
        }
    }

    /// An emoji ([Get Application Emoji](djs_rest::Rest::get_application_emoji)).
    ///
    /// discord.js counterpart: `ApplicationEmojiManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub fn fetch(
        self,
        emoji_id: EmojiId,
    ) -> Fetch<Emoji, impl Future<Output = Result<Emoji, Error>> + Send + 'a> {
        let Self {
            ctx,
            application_id,
        } = self;
        Fetch::new(None, async move {
            ctx.rest()
                .get_application_emoji(application_id, emoji_id)
                .await
        })
    }

    /// Every emoji of the application ([List Application
    /// Emojis](djs_rest::Rest::list_application_emojis)).
    ///
    /// discord.js counterpart: `ApplicationEmojiManager#fetch()`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list(self) -> Result<Collection<EmojiId, Emoji>, Error> {
        let emojis = self
            .ctx
            .rest()
            .list_application_emojis(self.application_id)
            .await?;
        Ok(emojis.into_iter().map(|emoji| (emoji.id, emoji)).collect())
    }

    /// Uploads an emoji ([Create Application Emoji](djs_rest::Rest::create_application_emoji)).
    ///
    /// discord.js counterpart: `ApplicationEmojiManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (at most 2000 emojis).
    pub async fn create(self, body: &CreateApplicationEmoji) -> Result<Emoji, Error> {
        self.ctx
            .rest()
            .create_application_emoji(self.application_id, body)
            .await
    }

    /// Renames an emoji ([Modify Application Emoji](djs_rest::Rest::modify_application_emoji)).
    ///
    /// discord.js counterpart: `ApplicationEmojiManager#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn edit(
        self,
        emoji_id: EmojiId,
        body: &ModifyApplicationEmoji,
    ) -> Result<Emoji, Error> {
        self.ctx
            .rest()
            .modify_application_emoji(self.application_id, emoji_id, body)
            .await
    }

    /// Deletes an emoji ([Delete Application Emoji](djs_rest::Rest::delete_application_emoji)).
    ///
    /// discord.js counterpart: `ApplicationEmojiManager#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn delete(self, emoji_id: EmojiId) -> Result<(), Error> {
        self.ctx
            .rest()
            .delete_application_emoji(self.application_id, emoji_id)
            .await
    }
}
