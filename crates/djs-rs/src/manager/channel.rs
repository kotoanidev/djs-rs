//! [`ChannelManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, CacheView};
use djs_model::{
    channel::{Channel, OverwriteType},
    id::{ChannelId, GenericId, RoleId, UserId},
    permissions::Permissions,
    rest::{EditChannelPermissions, ModifyGuildChannel, ModifyThread},
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// The channels the bot can see, by ID. Returned by [`Context::channels`].
///
/// Messages are sent through [`Context::messages`]; a guild's channels are created, listed
/// and reordered through [`Context::guild_channels`] (D-234).
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
    /// Edits a guild channel ([Modify Channel](djs_rest::Rest::modify_channel)); the result
    /// is cached.
    ///
    /// discord.js counterpart: `GuildChannelManager#edit` / `GuildChannel#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_CHANNELS`).
    pub async fn edit(
        self,
        channel_id: ChannelId,
        body: &ModifyGuildChannel,
        reason: Option<&str>,
    ) -> Result<Arc<Channel>, Error> {
        let channel = self
            .ctx
            .rest()
            .modify_channel(channel_id, body, reason)
            .await?;
        Ok(self.ctx.store_channel(&channel))
    }

    /// Edits a thread ([Modify Thread](djs_rest::Rest::modify_thread)): its name, archiving,
    /// locking, slowmode, tags; the result is cached.
    ///
    /// discord.js counterpart: `ThreadChannel#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn edit_thread(
        self,
        thread_id: ChannelId,
        body: &ModifyThread,
        reason: Option<&str>,
    ) -> Result<Arc<Channel>, Error> {
        let thread = self
            .ctx
            .rest()
            .modify_thread(thread_id, body, reason)
            .await?;
        Ok(self.ctx.store_channel(&thread))
    }

    /// Deletes a channel, or closes a DM ([Delete Channel](djs_rest::Rest::delete_channel)),
    /// returning what Discord sent back. The cache is updated by the `CHANNEL_DELETE` event
    /// that follows, whose handlers get the channel as it was (D-236).
    ///
    /// discord.js counterpart: `GuildChannelManager#delete` / `GuildChannel#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_CHANNELS`, or
    /// `MANAGE_THREADS` for a thread).
    pub async fn delete(
        self,
        channel_id: ChannelId,
        reason: Option<&str>,
    ) -> Result<Channel, Error> {
        self.ctx.rest().delete_channel(channel_id, reason).await
    }

    /// Sets a channel's permission overwrite for a role or a member (D-235): `allow` and
    /// `deny` replace the whole overwrite, as Discord does ([Edit Channel
    /// Permissions](djs_rest::Rest::edit_channel_permissions)).
    ///
    /// The target is a [`RoleId`] or a [`UserId`] (the `@everyone` role has the guild's ID:
    /// `guild_id.cast()`).
    ///
    /// discord.js counterpart: `PermissionOverwriteManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_ROLES`).
    pub fn set_permissions(
        self,
        channel_id: ChannelId,
        target: impl Into<OverwriteTarget>,
        allow: Permissions,
        deny: Permissions,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let (id, kind) = target.into().parts();
        let mut body = EditChannelPermissions::new(kind);
        body.allow = Some(allow);
        body.deny = Some(deny);
        let ctx = self.ctx;
        async move {
            ctx.rest()
                .edit_channel_permissions(channel_id, id, &body, reason)
                .await
        }
    }

    /// Removes a channel's permission overwrite for a role or a member ([Delete Channel
    /// Permission](djs_rest::Rest::delete_channel_permission)).
    ///
    /// discord.js counterpart: `PermissionOverwriteManager#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_ROLES`).
    pub fn delete_permissions(
        self,
        channel_id: ChannelId,
        target: impl Into<OverwriteTarget>,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let (id, _) = target.into().parts();
        let ctx = self.ctx;
        async move {
            ctx.rest()
                .delete_channel_permission(channel_id, id, reason)
                .await
        }
    }
}

/// Whose permissions a channel overwrite changes (D-235): built from a [`RoleId`] or a
/// [`UserId`].
///
/// discord.js counterpart: the `UserResolvable | RoleResolvable` of
/// `PermissionOverwriteManager`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OverwriteTarget {
    /// A role (the guild's ID for `@everyone`).
    Role(RoleId),
    /// A member.
    Member(UserId),
}

impl OverwriteTarget {
    const fn parts(self) -> (GenericId, OverwriteType) {
        match self {
            Self::Role(id) => (id.cast(), OverwriteType::Role),
            Self::Member(id) => (id.cast(), OverwriteType::Member),
        }
    }
}

impl From<RoleId> for OverwriteTarget {
    fn from(id: RoleId) -> Self {
        Self::Role(id)
    }
}

impl From<UserId> for OverwriteTarget {
    fn from(id: UserId) -> Self {
        Self::Member(id)
    }
}
