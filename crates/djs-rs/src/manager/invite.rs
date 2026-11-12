//! [`InviteManager`].

use std::future::Future;

use djs_cache::Backend;
use djs_collection::Collection;
use djs_model::{
    id::{ChannelId, GuildId},
    invite::{Invite, InviteWithMetadata},
    rest::{CreateChannelInvite, GetInvite},
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// Invites, by code (D-239). Returned by [`Context::invites`].
///
/// The cache does not keep invites, so every method asks Discord and returns the
/// [`djs-model`](djs_model) types. Lists are keyed by the invite code.
///
/// discord.js counterpart: `GuildInviteManager` (`guild.invites`) and `Client#fetchInvite`.
#[derive(Debug)]
pub struct InviteManager<'a, B: Backend> {
    ctx: &'a Context<B>,
}

impl<B: Backend> Clone for InviteManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for InviteManager<'_, B> {}

impl<'a, B: Backend> InviteManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>) -> Self {
        Self { ctx }
    }

    /// An invite by its code, to any guild ([Get Invite](djs_rest::Rest::get_invite)).
    ///
    /// discord.js counterpart: `Client#fetchInvite`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (Discord answers 404 for an unknown or
    /// expired invite).
    pub fn fetch<'r>(
        self,
        code: &'r str,
        query: &'r GetInvite,
    ) -> Fetch<Invite, impl Future<Output = Result<Invite, Error>> + Send + 'r>
    where
        'a: 'r,
    {
        let ctx = self.ctx;
        Fetch::new(
            None,
            async move { ctx.rest().get_invite(code, query).await },
        )
    }

    /// Deletes an invite ([Delete Invite](djs_rest::Rest::delete_invite)), returning it.
    ///
    /// discord.js counterpart: `GuildInviteManager#delete` / `Invite#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_CHANNELS` on the
    /// channel or `MANAGE_GUILD`).
    pub async fn delete(self, code: &str, reason: Option<&str>) -> Result<Invite, Error> {
        self.ctx.rest().delete_invite(code, reason).await
    }

    /// Creates an invite to a channel ([Create Channel
    /// Invite](djs_rest::Rest::create_channel_invite)).
    ///
    /// discord.js counterpart: `GuildInviteManager#create` / `TextChannel#createInvite`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `CREATE_INSTANT_INVITE`).
    pub async fn create(
        self,
        channel_id: ChannelId,
        body: &CreateChannelInvite,
        reason: Option<&str>,
    ) -> Result<InviteWithMetadata, Error> {
        self.ctx
            .rest()
            .create_channel_invite(channel_id, body, reason)
            .await
    }

    /// The invites to a channel ([Get Channel Invites](djs_rest::Rest::get_channel_invites)).
    ///
    /// discord.js counterpart: `TextChannel#fetchInvites`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_CHANNELS`).
    pub async fn list_channel(
        self,
        channel_id: ChannelId,
    ) -> Result<Collection<String, InviteWithMetadata>, Error> {
        let invites = self.ctx.rest().get_channel_invites(channel_id).await?;
        Ok(by_code(invites))
    }

    /// The invites to a guild ([Get Guild Invites](djs_rest::Rest::get_guild_invites)).
    ///
    /// discord.js counterpart: `GuildInviteManager#fetch()`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_GUILD` or
    /// `VIEW_AUDIT_LOG`).
    pub async fn list_guild(
        self,
        guild_id: GuildId,
    ) -> Result<Collection<String, InviteWithMetadata>, Error> {
        let invites = self.ctx.rest().get_guild_invites(guild_id).await?;
        Ok(by_code(invites))
    }
}

fn by_code(invites: Vec<InviteWithMetadata>) -> Collection<String, InviteWithMetadata> {
    invites
        .into_iter()
        .map(|invite| (invite.code.to_string(), invite))
        .collect()
}
