//! Methods on Discord structures (discord.js-style "structures" behaviour, D-220).
//!
//! djs-rs keeps the data types of `djs-model` and `djs-cache` plain and adds behaviour through
//! extension traits that take the handler [`Context`] explicitly — `msg.reply(&ctx, "Pong!")` —
//! instead of storing a client reference inside every structure as discord.js does. This keeps
//! structures small, `Clone` cheap and free of reference cycles.
//!
//! Each trait is implemented for the model type *and* the cached type (D-233), and so works on
//! anything that dereferences to one of them, such as event payloads. The methods delegate to
//! the [managers](crate::manager) and return what they return: the cached type in an `Arc`.
//!
//! | Trait | Implemented for |
//! |---|---|
//! | [`MessageExt`] | [`Message`], [`CachedMessage`] |
//! | [`ChannelExt`] | [`Channel`] (also the cached type) |
//! | [`UserExt`] | [`User`], [`CachedUser`] |
//! | [`MemberExt`] | [`CachedMember`], the [`GuildMemberAdd`] and [`GuildMemberUpdate`] payloads (the model [`Member`](djs_model::guild::Member) does not know its guild) |
//! | [`GuildExt`] | [`Guild`], [`CachedGuild`] |
//!
//! Import the traits with [`prelude`](crate::prelude).

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, CachedGuild, CachedMember, CachedMessage, CachedUser};
use djs_model::{
    channel::Channel,
    gateway::payload::{GuildMemberAdd, GuildMemberUpdate},
    guild::Guild,
    id::{ChannelId, GuildId, MessageId, RoleId, UserId},
    message::{Message, MessageReference},
    rest::{CreateGuildBan, CreateMessage, EditMessage, ModifyGuildMember, StartThreadFromMessage},
    user::User,
    util::Timestamp,
};
use djs_rest::{Error, ReactionEmoji};

use crate::{
    Context,
    manager::{BanManager, MemberManager, MessageManager, RoleManager},
};

/// Actions on a message: [`Message`] or [`CachedMessage`].
///
/// This trait is sealed.
///
/// # Examples
///
/// ```no_run
/// # use djs_rs::prelude::*;
/// # let client = Client::builder("token").build();
/// client.on(Events::MessageCreate, |ctx, msg| async move {
///     if msg.content == "!ping" {
///         let reply = msg.reply(&ctx, "Pong!").await?;
///         reply.react(&ctx, "🏓").await?;
///     }
///     Ok(())
/// });
/// ```
pub trait MessageExt: sealed::Message {
    /// Replies to this message in the same channel: the reply references this message
    /// (Discord shows it as a reply). Accepts anything that converts into a
    /// [`CreateMessage`], such as `&str` or `String`. The reply is cached.
    ///
    /// discord.js counterpart: `Message#reply`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the message could not be sent (see
    /// [`Rest::create_message`](djs_rest::Rest::create_message)).
    fn reply<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: impl Into<CreateMessage>,
    ) -> impl Future<Output = Result<Arc<CachedMessage>, Error>> + Send {
        let (channel_id, message_id) = self.ids();
        let mut body = body.into();
        body.message_reference = Some(MessageReference::reply(message_id));
        ctx.messages(channel_id).send(body)
    }

    /// Edits this message ([`MessageManager::edit`]).
    ///
    /// discord.js counterpart: `Message#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn edit<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: &EditMessage,
    ) -> impl Future<Output = Result<Arc<CachedMessage>, Error>> + Send {
        let (channel_id, message_id) = self.ids();
        ctx.messages(channel_id).edit(message_id, body)
    }

    /// Deletes this message ([`MessageManager::delete`]).
    ///
    /// discord.js counterpart: `Message#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn delete<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let (channel_id, message_id) = self.ids();
        ctx.messages(channel_id).delete(message_id)
    }

    /// Reacts to this message ([`MessageManager::react`]): a `&str` for a Unicode emoji, or a
    /// custom [`ReactionEmoji`].
    ///
    /// discord.js counterpart: `Message#react`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn react<'e, B: Backend>(
        &self,
        ctx: &Context<B>,
        emoji: impl Into<ReactionEmoji<'e>>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let (channel_id, message_id) = self.ids();
        ctx.messages(channel_id).react(message_id, emoji)
    }

    /// Pins this message ([`MessageManager::pin`]).
    ///
    /// discord.js counterpart: `Message#pin`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn pin<B: Backend>(
        &self,
        ctx: &Context<B>,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let (channel_id, message_id) = self.ids();
        ctx.messages(channel_id).pin(message_id, reason)
    }

    /// Unpins this message ([`MessageManager::unpin`]).
    ///
    /// discord.js counterpart: `Message#unpin`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn unpin<B: Backend>(
        &self,
        ctx: &Context<B>,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let (channel_id, message_id) = self.ids();
        ctx.messages(channel_id).unpin(message_id, reason)
    }

    /// Starts a thread from this message ([`MessageManager::start_thread`]).
    ///
    /// discord.js counterpart: `Message#startThread`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn start_thread<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: &StartThreadFromMessage,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<Arc<Channel>, Error>> + Send {
        let (channel_id, message_id) = self.ids();
        ctx.messages(channel_id)
            .start_thread(message_id, body, reason)
    }

    /// Publishes this announcement-channel message to the following channels
    /// ([`MessageManager::crosspost`]).
    ///
    /// discord.js counterpart: `Message#crosspost`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn crosspost<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<Arc<CachedMessage>, Error>> + Send {
        let (channel_id, message_id) = self.ids();
        ctx.messages(channel_id).crosspost(message_id)
    }
}

impl MessageExt for Message {}
impl MessageExt for CachedMessage {}

/// Actions on a [`Channel`] (the model type, which is also the type the cache keeps).
///
/// This trait is sealed.
///
/// # Examples
///
/// ```no_run
/// # use djs_rs::prelude::*;
/// # let client = Client::builder("token").build();
/// client.on(Events::ThreadCreate, |ctx, thread| async move {
///     thread.send(&ctx, "Welcome to the thread!").await?;
///     Ok(())
/// });
/// ```
pub trait ChannelExt: sealed::Channel {
    /// The manager of this channel's messages.
    ///
    /// discord.js counterpart: `TextChannel#messages`.
    fn messages<'a, B: Backend>(&self, ctx: &'a Context<B>) -> MessageManager<'a, B> {
        ctx.messages(self.channel_id())
    }

    /// Sends a message to this channel ([`MessageManager::send`]).
    ///
    /// discord.js counterpart: `TextChannel#send`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the message could not be sent.
    fn send<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: impl Into<CreateMessage>,
    ) -> impl Future<Output = Result<Arc<CachedMessage>, Error>> + Send {
        ctx.messages(self.channel_id()).send(body)
    }

    /// Shows the bot as typing in this channel ([`ChannelManager::send_typing`](crate::manager::ChannelManager::send_typing)).
    ///
    /// discord.js counterpart: `TextChannel#sendTyping`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn send_typing<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        ctx.channels().send_typing(self.channel_id())
    }

    /// Deletes 2 to 100 recent messages of this channel ([`MessageManager::bulk_delete`]).
    ///
    /// discord.js counterpart: `TextChannel#bulkDelete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn bulk_delete<B: Backend>(
        &self,
        ctx: &Context<B>,
        message_ids: Vec<MessageId>,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        ctx.messages(self.channel_id())
            .bulk_delete(message_ids, reason)
    }
}

impl ChannelExt for Channel {}

/// Actions on a user: [`User`] or [`CachedUser`].
///
/// This trait is sealed.
///
/// # Examples
///
/// ```no_run
/// # use djs_rs::prelude::*;
/// # let client = Client::builder("token").build();
/// client.on(Events::GuildMemberAdd, |ctx, join| async move {
///     join.user.send(&ctx, "Welcome!").await?;
///     Ok(())
/// });
/// ```
pub trait UserExt: sealed::User {
    /// Sends a direct message to this user ([`UserManager::send`](crate::manager::UserManager::send)).
    ///
    /// discord.js counterpart: `User#send`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if a request failed.
    fn send<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: impl Into<CreateMessage>,
    ) -> impl Future<Output = Result<Arc<CachedMessage>, Error>> + Send {
        ctx.users().send(self.user_id(), body)
    }

    /// The DM channel with this user ([`UserManager::create_dm`](crate::manager::UserManager::create_dm)).
    ///
    /// discord.js counterpart: `User#createDM`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn create_dm<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<Arc<Channel>, Error>> + Send {
        ctx.users().create_dm(self.user_id())
    }
}

impl UserExt for User {}
impl UserExt for CachedUser {}

/// Actions on a guild member: [`CachedMember`], or the payload of
/// [`Events::GuildMemberAdd`](crate::Events::GuildMemberAdd) or
/// [`Events::GuildMemberUpdate`](crate::Events::GuildMemberUpdate).
///
/// This trait is sealed.
///
/// # Examples
///
/// ```no_run
/// # use djs_rs::prelude::*;
/// # use djs_rs::model::id::RoleId;
/// # let client = Client::builder("token").build();
/// const NEWCOMER: RoleId = RoleId::new(1_300_000_000_000_000_000);
/// client.on(Events::GuildMemberAdd, |ctx, member| async move {
///     member.add_role(&ctx, NEWCOMER, Some("new member")).await?;
///     Ok(())
/// });
/// ```
pub trait MemberExt: sealed::Member {
    /// Edits this member ([`MemberManager::edit`]).
    ///
    /// discord.js counterpart: `GuildMember#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn edit<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: &ModifyGuildMember,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<Arc<CachedMember>, Error>> + Send {
        let (guild_id, user_id) = self.ids();
        ctx.members(guild_id).edit(user_id, body, reason)
    }

    /// Times this member out until `until`, or ends the timeout with `None`
    /// ([`MemberManager::timeout`]).
    ///
    /// discord.js counterpart: `GuildMember#timeout`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn timeout<B: Backend>(
        &self,
        ctx: &Context<B>,
        until: Option<Timestamp>,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<Arc<CachedMember>, Error>> + Send {
        let (guild_id, user_id) = self.ids();
        ctx.members(guild_id).timeout(user_id, until, reason)
    }

    /// Sets this member's nickname, or removes it with `None`
    /// ([`MemberManager::set_nickname`]).
    ///
    /// discord.js counterpart: `GuildMember#setNickname`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn set_nickname<B: Backend>(
        &self,
        ctx: &Context<B>,
        nickname: Option<&str>,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<Arc<CachedMember>, Error>> + Send {
        let (guild_id, user_id) = self.ids();
        ctx.members(guild_id)
            .set_nickname(user_id, nickname, reason)
    }

    /// Gives this member a role ([`MemberManager::add_role`]).
    ///
    /// discord.js counterpart: `GuildMemberRoleManager#add`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn add_role<B: Backend>(
        &self,
        ctx: &Context<B>,
        role_id: RoleId,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let (guild_id, user_id) = self.ids();
        ctx.members(guild_id).add_role(user_id, role_id, reason)
    }

    /// Takes a role from this member ([`MemberManager::remove_role`]).
    ///
    /// discord.js counterpart: `GuildMemberRoleManager#remove`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn remove_role<B: Backend>(
        &self,
        ctx: &Context<B>,
        role_id: RoleId,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let (guild_id, user_id) = self.ids();
        ctx.members(guild_id).remove_role(user_id, role_id, reason)
    }

    /// Kicks this member ([`MemberManager::kick`]).
    ///
    /// discord.js counterpart: `GuildMember#kick`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn kick<B: Backend>(
        &self,
        ctx: &Context<B>,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let (guild_id, user_id) = self.ids();
        ctx.members(guild_id).kick(user_id, reason)
    }

    /// Bans this member ([`MemberManager::ban`]).
    ///
    /// discord.js counterpart: `GuildMember#ban`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn ban<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: &CreateGuildBan,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let (guild_id, user_id) = self.ids();
        ctx.members(guild_id).ban(user_id, body, reason)
    }
}

impl MemberExt for CachedMember {}
impl MemberExt for GuildMemberAdd {}
impl MemberExt for GuildMemberUpdate {}

/// Actions on a guild: [`Guild`] or [`CachedGuild`].
///
/// This trait is sealed.
///
/// # Examples
///
/// ```no_run
/// # use djs_rs::prelude::*;
/// # use djs_rs::model::id::UserId;
/// # async fn run(ctx: Context, guild: std::sync::Arc<djs_rs::cache::CachedGuild>) -> Result<(), djs_rs::rest::Error> {
/// let owner = guild.members(&ctx).fetch(guild.owner_id).await?;
/// println!("{} owns {}", owner.user.username, guild.name);
/// # Ok(())
/// # }
/// ```
pub trait GuildExt: sealed::Guild {
    /// The manager of this guild's members.
    ///
    /// discord.js counterpart: `Guild#members`.
    fn members<'a, B: Backend>(&self, ctx: &'a Context<B>) -> MemberManager<'a, B> {
        ctx.members(self.guild_id())
    }

    /// The manager of this guild's roles.
    ///
    /// discord.js counterpart: `Guild#roles`.
    fn roles<'a, B: Backend>(&self, ctx: &'a Context<B>) -> RoleManager<'a, B> {
        ctx.roles(self.guild_id())
    }

    /// The manager of this guild's bans.
    ///
    /// discord.js counterpart: `Guild#bans`.
    fn bans<'a, B: Backend>(&self, ctx: &'a Context<B>) -> BanManager<'a, B> {
        ctx.bans(self.guild_id())
    }

    /// Makes the bot leave this guild ([Leave Guild](djs_rest::Rest::leave_guild)).
    ///
    /// discord.js counterpart: `Guild#leave`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (the owner cannot leave its guild).
    fn leave<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        ctx.rest().leave_guild(self.guild_id())
    }
}

impl GuildExt for Guild {}
impl GuildExt for CachedGuild {}

/// The IDs the extension traits need; sealed so the traits stay implementable only here.
mod sealed {
    use super::{ChannelId, GuildId, MessageId, UserId};

    pub trait Message {
        /// The channel and the message.
        fn ids(&self) -> (ChannelId, MessageId);
    }

    impl Message for djs_model::message::Message {
        fn ids(&self) -> (ChannelId, MessageId) {
            (self.channel_id, self.id)
        }
    }

    impl Message for djs_cache::CachedMessage {
        fn ids(&self) -> (ChannelId, MessageId) {
            (self.channel_id, self.id)
        }
    }

    pub trait Channel {
        fn channel_id(&self) -> ChannelId;
    }

    impl Channel for djs_model::channel::Channel {
        fn channel_id(&self) -> ChannelId {
            self.id()
        }
    }

    pub trait User {
        fn user_id(&self) -> UserId;
    }

    impl User for djs_model::user::User {
        fn user_id(&self) -> UserId {
            self.id
        }
    }

    impl User for djs_cache::CachedUser {
        fn user_id(&self) -> UserId {
            self.id
        }
    }

    pub trait Member {
        /// The guild and the user.
        fn ids(&self) -> (GuildId, UserId);
    }

    impl Member for djs_cache::CachedMember {
        fn ids(&self) -> (GuildId, UserId) {
            (self.guild_id, self.user.id)
        }
    }

    impl Member for djs_model::gateway::payload::GuildMemberAdd {
        fn ids(&self) -> (GuildId, UserId) {
            (self.guild_id, self.member.user.id)
        }
    }

    impl Member for djs_model::gateway::payload::GuildMemberUpdate {
        fn ids(&self) -> (GuildId, UserId) {
            (self.guild_id, self.user.id)
        }
    }

    pub trait Guild {
        fn guild_id(&self) -> GuildId;
    }

    impl Guild for djs_model::guild::Guild {
        fn guild_id(&self) -> GuildId {
            self.id
        }
    }

    impl Guild for djs_cache::CachedGuild {
        fn guild_id(&self) -> GuildId {
            self.id
        }
    }
}
