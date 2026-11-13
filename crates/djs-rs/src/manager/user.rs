//! [`UserManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, CacheView, CachedMessage, CachedUser};
use djs_model::{
    channel::Channel,
    id::UserId,
    rest::{CreateDm, CreateMessage},
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// The users the bot knows. Returned by [`Context::users`].
///
/// discord.js counterpart: `UserManager` (`client.users`).
#[derive(Debug)]
pub struct UserManager<'a, B: Backend> {
    ctx: &'a Context<B>,
}

impl<B: Backend> Clone for UserManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for UserManager<'_, B> {}

impl<'a, B: Backend> UserManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>) -> Self {
        Self { ctx }
    }

    /// The cached users, if the client has a cache.
    ///
    /// discord.js counterpart: `UserManager#cache`.
    #[must_use]
    pub fn cache(self) -> Option<CacheView<'a, UserId, CachedUser, B>> {
        self.ctx.cache().map(djs_cache::Cache::users)
    }

    /// A user: the cached one, or [Get User](djs_rest::Rest::get_user), whose result is
    /// cached.
    ///
    /// discord.js counterpart: `UserManager#fetch`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if Discord was asked and the request failed.
    pub fn fetch(
        self,
        user_id: UserId,
    ) -> Fetch<Arc<CachedUser>, impl Future<Output = Result<Arc<CachedUser>, Error>> + Send + 'a>
    {
        let cached = self.ctx.ops().user(user_id);
        let ctx = self.ctx;
        Fetch::new(cached, async move {
            let user = ctx.rest().get_user(user_id).await?;
            Ok(ctx.store_user(&user))
        })
    }

    /// The DM channel with a user ([Create DM](djs_rest::Rest::create_dm); Discord returns
    /// the existing one).
    ///
    /// discord.js counterpart: `UserManager#createDM`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn create_dm(self, user_id: UserId) -> Result<Arc<Channel>, Error> {
        let channel = self.ctx.rest().create_dm(&CreateDm::new(user_id)).await?;
        Ok(self.ctx.store_channel(&channel))
    }

    /// Sends a direct message: opens the DM channel, then sends `body` to it.
    ///
    /// discord.js counterpart: `UserManager#send`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if a request failed (Discord refuses DMs to users who share no
    /// guild with the bot or closed their DMs).
    pub fn send(
        self,
        user_id: UserId,
        body: impl Into<CreateMessage>,
    ) -> impl Future<Output = Result<Arc<CachedMessage>, Error>> + Send + 'a {
        let body = body.into();
        async move {
            let channel = self.create_dm(user_id).await?;
            self.ctx.messages(channel.id()).send(body).await
        }
    }
}
