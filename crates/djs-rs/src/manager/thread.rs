//! [`ThreadManager`], [`ThreadMemberManager`] and [`FetchedThreads`] (D-242).

use std::{future::Future, sync::Arc};

use djs_cache::Backend;
use djs_collection::Collection;
use djs_model::{
    channel::{Channel, ThreadMember},
    id::{ChannelId, UserId},
    rest::{
        GetThreadMember, ListArchivedThreads, ListJoinedPrivateArchivedThreads, ListThreadMembers,
        StartThreadInForumOrMediaChannel, StartThreadWithoutMessage,
    },
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// A list of threads from Discord: active or archived ones (discord.js: `FetchedThreads`).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FetchedThreads {
    /// The threads, in Discord's order; they are cached.
    pub threads: Collection<ChannelId, Arc<Channel>>,
    /// The bot's own memberships of the threads it joined.
    pub members: Vec<ThreadMember>,
    /// Whether more archived threads follow (pass the last thread's archive timestamp as
    /// `before`); always `false` for active threads.
    pub has_more: bool,
}

impl<B: Backend> Context<B> {
    pub(crate) fn fetched_threads(
        &self,
        threads: &[Channel],
        members: Vec<ThreadMember>,
        has_more: bool,
    ) -> FetchedThreads {
        FetchedThreads {
            threads: threads
                .iter()
                .map(|thread| (thread.id(), self.store_channel(thread)))
                .collect(),
            members,
            has_more,
        }
    }
}

/// The threads of one parent channel: creating them and listing the archived ones.
/// Returned by [`Context::threads`].
///
/// A thread is a channel: edit or delete it through [`Context::channels`]
/// ([`edit_thread`](super::ChannelManager::edit_thread), [`delete`](super::ChannelManager::delete)),
/// and list a guild's active threads with
/// [`GuildChannelManager::list_active_threads`](super::GuildChannelManager::list_active_threads).
///
/// discord.js counterpart: `GuildTextThreadManager` / `GuildForumThreadManager`
/// (`channel.threads`).
#[derive(Debug)]
pub struct ThreadManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    channel_id: ChannelId,
}

impl<B: Backend> Clone for ThreadManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for ThreadManager<'_, B> {}

impl<'a, B: Backend> ThreadManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, channel_id: ChannelId) -> Self {
        Self { ctx, channel_id }
    }

    /// The parent channel.
    #[must_use]
    pub const fn channel_id(self) -> ChannelId {
        self.channel_id
    }

    /// Starts a thread without a message ([Start Thread without
    /// Message](djs_rest::Rest::start_thread_without_message)); it is cached.
    ///
    /// discord.js counterpart: `GuildTextThreadManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `CREATE_PUBLIC_THREADS` or
    /// `CREATE_PRIVATE_THREADS`).
    pub async fn create(
        self,
        body: &StartThreadWithoutMessage,
        reason: Option<&str>,
    ) -> Result<Arc<Channel>, Error> {
        let thread = self
            .ctx
            .rest()
            .start_thread_without_message(self.channel_id, body, reason)
            .await?;
        Ok(self.ctx.store_channel(&thread))
    }

    /// Starts a post in a forum or media channel ([Start Thread in Forum or Media
    /// Channel](djs_rest::Rest::start_thread_in_forum_or_media_channel)). The thread and its
    /// starter message (whose ID is the thread's) are cached.
    ///
    /// discord.js counterpart: `GuildForumThreadManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `SEND_MESSAGES`).
    pub async fn create_in_forum(
        self,
        body: &StartThreadInForumOrMediaChannel,
        reason: Option<&str>,
    ) -> Result<Arc<Channel>, Error> {
        let post = self
            .ctx
            .rest()
            .start_thread_in_forum_or_media_channel(self.channel_id, body, reason)
            .await?;
        if let Some(message) = &post.message {
            self.ctx.store_message(message);
        }
        Ok(self.ctx.store_channel(&post.channel))
    }

    /// Archived public threads, newest first ([List Public Archived
    /// Threads](djs_rest::Rest::list_public_archived_threads)).
    ///
    /// discord.js counterpart: `GuildTextThreadManager#fetchArchived({ type: 'public' })`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `READ_MESSAGE_HISTORY`).
    pub async fn list_public_archived(
        self,
        query: &ListArchivedThreads,
    ) -> Result<FetchedThreads, Error> {
        let list = self
            .ctx
            .rest()
            .list_public_archived_threads(self.channel_id, query)
            .await?;
        Ok(self
            .ctx
            .fetched_threads(&list.threads, list.members, list.has_more))
    }

    /// Archived private threads, newest first ([List Private Archived
    /// Threads](djs_rest::Rest::list_private_archived_threads)).
    ///
    /// discord.js counterpart: `GuildTextThreadManager#fetchArchived({ type: 'private' })`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `READ_MESSAGE_HISTORY` and
    /// `MANAGE_THREADS`).
    pub async fn list_private_archived(
        self,
        query: &ListArchivedThreads,
    ) -> Result<FetchedThreads, Error> {
        let list = self
            .ctx
            .rest()
            .list_private_archived_threads(self.channel_id, query)
            .await?;
        Ok(self
            .ctx
            .fetched_threads(&list.threads, list.members, list.has_more))
    }

    /// Archived private threads the bot joined, newest first ([List Joined Private Archived
    /// Threads](djs_rest::Rest::list_joined_private_archived_threads)).
    ///
    /// discord.js counterpart: `GuildTextThreadManager#fetchArchived({ type: 'private',
    /// fetchAll: false })`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list_joined_private_archived(
        self,
        query: &ListJoinedPrivateArchivedThreads,
    ) -> Result<FetchedThreads, Error> {
        let list = self
            .ctx
            .rest()
            .list_joined_private_archived_threads(self.channel_id, query)
            .await?;
        Ok(self
            .ctx
            .fetched_threads(&list.threads, list.members, list.has_more))
    }
}

/// The members of one thread. Returned by [`Context::thread_members`].
///
/// The cache does not keep thread memberships, so the [`djs-model`](djs_model)
/// [`ThreadMember`] is returned.
///
/// discord.js counterpart: `ThreadMemberManager` (`thread.members`).
#[derive(Debug)]
pub struct ThreadMemberManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    thread_id: ChannelId,
}

impl<B: Backend> Clone for ThreadMemberManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for ThreadMemberManager<'_, B> {}

impl<'a, B: Backend> ThreadMemberManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, thread_id: ChannelId) -> Self {
        Self { ctx, thread_id }
    }

    /// The thread.
    #[must_use]
    pub const fn thread_id(self) -> ChannelId {
        self.thread_id
    }

    /// A user's membership of the thread ([Get Thread
    /// Member](djs_rest::Rest::get_thread_member)).
    ///
    /// discord.js counterpart: `ThreadMemberManager#fetch(member)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (404 when the user is not a member).
    pub fn fetch<'r>(
        self,
        user_id: UserId,
        query: &'r GetThreadMember,
    ) -> Fetch<ThreadMember, impl Future<Output = Result<ThreadMember, Error>> + Send + 'r>
    where
        'a: 'r,
    {
        let Self { ctx, thread_id } = self;
        Fetch::new(None, async move {
            ctx.rest()
                .get_thread_member(thread_id, user_id, query)
                .await
        })
    }

    /// A page of the thread's members, by user ID ([List Thread
    /// Members](djs_rest::Rest::list_thread_members); needs the `GUILD_MEMBERS` intent).
    ///
    /// discord.js counterpart: `ThreadMemberManager#fetch()`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list(
        self,
        query: &ListThreadMembers,
    ) -> Result<Collection<UserId, ThreadMember>, Error> {
        let members = self
            .ctx
            .rest()
            .list_thread_members(self.thread_id, query)
            .await?;
        Ok(members
            .into_iter()
            .filter_map(|member| Some((member.user_id?, member)))
            .collect())
    }

    /// Adds a user to the thread ([Add Thread Member](djs_rest::Rest::add_thread_member)).
    ///
    /// discord.js counterpart: `ThreadMemberManager#add`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (the thread must not be archived).
    pub async fn add(self, user_id: UserId) -> Result<(), Error> {
        self.ctx
            .rest()
            .add_thread_member(self.thread_id, user_id)
            .await
    }

    /// Removes a user from the thread ([Remove Thread
    /// Member](djs_rest::Rest::remove_thread_member)).
    ///
    /// discord.js counterpart: `ThreadMemberManager#remove`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_THREADS`, or to be the
    /// creator of a private thread).
    pub async fn remove(self, user_id: UserId) -> Result<(), Error> {
        self.ctx
            .rest()
            .remove_thread_member(self.thread_id, user_id)
            .await
    }

    /// Makes the bot join the thread ([Join Thread](djs_rest::Rest::join_thread)).
    ///
    /// discord.js counterpart: `ThreadChannel#join`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn join(self) -> Result<(), Error> {
        self.ctx.rest().join_thread(self.thread_id).await
    }

    /// Makes the bot leave the thread ([Leave Thread](djs_rest::Rest::leave_thread)).
    ///
    /// discord.js counterpart: `ThreadChannel#leave`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn leave(self) -> Result<(), Error> {
        self.ctx.rest().leave_thread(self.thread_id).await
    }
}
