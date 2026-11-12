//! [`MessageManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, CachedMessage, ChannelView};
use djs_collection::Collection;
use djs_model::{
    channel::Channel,
    id::{ChannelId, MessageId},
    rest::{
        BulkDeleteMessages, CreateMessage, EditMessage, GetChannelMessages, StartThreadFromMessage,
    },
};
use djs_rest::{Error, ReactionEmoji};

use super::Fetch;
use crate::Context;

/// The messages of one channel. Returned by [`Context::messages`].
///
/// discord.js counterpart: `MessageManager` (`channel.messages`), plus `TextChannel#send`
/// and `TextChannel#bulkDelete`.
#[derive(Debug)]
pub struct MessageManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    channel_id: ChannelId,
}

impl<B: Backend> Clone for MessageManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for MessageManager<'_, B> {}

impl<'a, B: Backend> MessageManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, channel_id: ChannelId) -> Self {
        Self { ctx, channel_id }
    }

    /// The channel these messages belong to.
    #[must_use]
    pub const fn channel_id(self) -> ChannelId {
        self.channel_id
    }

    /// The cached messages of the channel, if the client has a cache.
    ///
    /// discord.js counterpart: `MessageManager#cache`.
    #[must_use]
    pub fn cache(self) -> Option<ChannelView<'a, MessageId, CachedMessage, B>> {
        self.ctx
            .cache()
            .map(|cache| cache.messages(self.channel_id))
    }

    /// A message: the cached one, or [Get Channel
    /// Message](djs_rest::Rest::get_channel_message), whose result is cached.
    ///
    /// discord.js counterpart: `MessageManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if Discord was asked and the request failed (it needs
    /// `READ_MESSAGE_HISTORY` in guild channels).
    pub fn fetch(
        self,
        message_id: MessageId,
    ) -> Fetch<
        Arc<CachedMessage>,
        impl Future<Output = Result<Arc<CachedMessage>, Error>> + Send + 'a,
    > {
        let cached = self.cache().and_then(|messages| messages.get(message_id));
        let Self { ctx, channel_id } = self;
        Fetch::new(cached, async move {
            let message = ctx
                .rest()
                .get_channel_message(channel_id, message_id)
                .await?;
            Ok(ctx.store_message(&message))
        })
    }

    /// Up to 100 messages, newest first ([Get Channel
    /// Messages](djs_rest::Rest::get_channel_messages)); the messages are cached.
    ///
    /// discord.js counterpart: `MessageManager#fetch(options)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list(
        self,
        query: &GetChannelMessages,
    ) -> Result<Collection<MessageId, Arc<CachedMessage>>, Error> {
        let messages = self
            .ctx
            .rest()
            .get_channel_messages(self.channel_id, query)
            .await?;
        Ok(messages
            .iter()
            .map(|message| (message.id, self.ctx.store_message(message)))
            .collect())
    }

    /// Sends a message ([Create Message](djs_rest::Rest::create_message)); accepts anything
    /// that converts into a [`CreateMessage`], such as `&str`. The sent message is cached.
    ///
    /// discord.js counterpart: `TextChannel#send`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the message could not be sent.
    pub fn send(
        self,
        body: impl Into<CreateMessage>,
    ) -> impl Future<Output = Result<Arc<CachedMessage>, Error>> + Send + 'a {
        let body = body.into();
        let Self { ctx, channel_id } = self;
        async move {
            let message = ctx.rest().create_message(channel_id, &body).await?;
            Ok(ctx.store_message(&message))
        }
    }

    /// Edits a message ([Edit Message](djs_rest::Rest::edit_message)); the result is cached.
    ///
    /// discord.js counterpart: `MessageManager#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (only the author can edit the content).
    pub async fn edit(
        self,
        message_id: MessageId,
        body: &EditMessage,
    ) -> Result<Arc<CachedMessage>, Error> {
        let message = self
            .ctx
            .rest()
            .edit_message(self.channel_id, message_id, body)
            .await?;
        Ok(self.ctx.store_message(&message))
    }

    /// Deletes a message ([Delete Message](djs_rest::Rest::delete_message)).
    ///
    /// discord.js counterpart: `MessageManager#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (other users' messages need
    /// `MANAGE_MESSAGES`).
    pub async fn delete(self, message_id: MessageId) -> Result<(), Error> {
        self.ctx
            .rest()
            .delete_message(self.channel_id, message_id)
            .await
    }

    /// Deletes 2 to 100 messages at once, none older than two weeks ([Bulk Delete
    /// Messages](djs_rest::Rest::bulk_delete_messages)).
    ///
    /// discord.js counterpart: `TextChannel#bulkDelete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_MESSAGES`).
    pub async fn bulk_delete(
        self,
        message_ids: Vec<MessageId>,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.ctx
            .rest()
            .bulk_delete_messages(
                self.channel_id,
                &BulkDeleteMessages::new(message_ids),
                reason,
            )
            .await
    }

    /// Reacts to a message with an emoji: a `&str` for a Unicode emoji, or a custom
    /// [`ReactionEmoji`].
    ///
    /// discord.js counterpart: `Message#react`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (`10014` Unknown Emoji for an emoji the
    /// bot cannot use).
    pub fn react<'e>(
        self,
        message_id: MessageId,
        emoji: impl Into<ReactionEmoji<'e>>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let emoji = emoji.into();
        let Self { ctx, channel_id } = self;
        async move {
            ctx.rest()
                .create_reaction(channel_id, message_id, emoji)
                .await
        }
    }

    /// Pins a message ([Pin Message](djs_rest::Rest::pin_message)).
    ///
    /// discord.js counterpart: `MessageManager#pin`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `PIN_MESSAGES`).
    pub async fn pin(self, message_id: MessageId, reason: Option<&str>) -> Result<(), Error> {
        self.ctx
            .rest()
            .pin_message(self.channel_id, message_id, reason)
            .await
    }

    /// Unpins a message ([Unpin Message](djs_rest::Rest::unpin_message)).
    ///
    /// discord.js counterpart: `MessageManager#unpin`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn unpin(self, message_id: MessageId, reason: Option<&str>) -> Result<(), Error> {
        self.ctx
            .rest()
            .unpin_message(self.channel_id, message_id, reason)
            .await
    }

    /// Starts a thread from a message ([Start Thread from
    /// Message](djs_rest::Rest::start_thread_from_message)); the thread is cached.
    ///
    /// discord.js counterpart: `Message#startThread`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn start_thread(
        self,
        message_id: MessageId,
        body: &StartThreadFromMessage,
        reason: Option<&str>,
    ) -> Result<Arc<Channel>, Error> {
        let thread = self
            .ctx
            .rest()
            .start_thread_from_message(self.channel_id, message_id, body, reason)
            .await?;
        Ok(self.ctx.store_channel(&thread))
    }

    /// Publishes a message of an announcement channel to the channels following it
    /// ([Crosspost Message](djs_rest::Rest::crosspost_message)).
    ///
    /// discord.js counterpart: `MessageManager#crosspost`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn crosspost(self, message_id: MessageId) -> Result<Arc<CachedMessage>, Error> {
        let message = self
            .ctx
            .rest()
            .crosspost_message(self.channel_id, message_id)
            .await?;
        Ok(self.ctx.store_message(&message))
    }
}
