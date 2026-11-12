//! [`ChannelManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, CacheView};
use djs_model::{channel::Channel, id::ChannelId};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// The channels the bot can see. Returned by [`Context::channels`].
///
/// Messages are sent through [`Context::messages`].
///
/// discord.js counterpart: `ChannelManager` (`client.channels`).
#[derive(Debug)]
pub struct ChannelManager<'a, B: Backend> {
    ctx: &'a Context<B>,
}

impl<B: Backend> Clone for ChannelManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for ChannelManager<'_, B> {}

impl<'a, B: Backend> ChannelManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>) -> Self {
        Self { ctx }
    }

    /// The cached guild and DM channels (threads are cached per guild,
    /// [`Cache::threads`](djs_cache::Cache::threads)), if the client has a cache.
    ///
    /// discord.js counterpart: `ChannelManager#cache`.
    #[must_use]
    pub fn cache(self) -> Option<CacheView<'a, ChannelId, Channel, B>> {
        self.ctx.cache().map(djs_cache::Cache::channels)
    }

    /// A channel or thread: the cached channel, or [Get
    /// Channel](djs_rest::Rest::get_channel), whose result is cached (a thread among the
    /// guild's threads).
    ///
    /// discord.js counterpart: `ChannelManager#fetch`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if Discord was asked and the request failed.
    pub fn fetch(
        self,
        channel_id: ChannelId,
    ) -> Fetch<Arc<Channel>, impl Future<Output = Result<Arc<Channel>, Error>> + Send + 'a> {
        let cached = self.cache().and_then(|channels| channels.get(channel_id));
        let ctx = self.ctx;
        Fetch::new(cached, async move {
            let channel = ctx.rest().get_channel(channel_id).await?;
            Ok(ctx.store_channel(&channel))
        })
    }

    /// Shows the bot as typing in a channel for about 10 seconds, or until it sends a message.
    ///
    /// discord.js counterpart: `TextChannel#sendTyping`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn send_typing(self, channel_id: ChannelId) -> Result<(), Error> {
        self.ctx.rest().trigger_typing_indicator(channel_id).await
    }
}
