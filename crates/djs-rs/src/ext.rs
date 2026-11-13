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
//! | [`WebhookExt`] | [`Webhook`] |
//! | [`InteractionExt`] | [`Interaction`] (and so the [`InteractionCreate`](djs_model::gateway::payload::InteractionCreate) payload), the [router](crate::router)'s wrappers |
//!
//! Import the traits with [`prelude`](crate::prelude).

use std::{future::Future, sync::Arc, time::Duration};

use djs_cache::{Backend, CachedGuild, CachedMember, CachedMessage, CachedUser};
use djs_collection::Collection;
use djs_model::{
    audit_log::AuditLog,
    channel::Channel,
    gateway::payload::{GuildMemberAdd, GuildMemberUpdate, MessageCreate},
    guild::{Guild, Integration},
    id::{
        ApplicationId, ChannelId, GuildId, IntegrationId, InteractionId, MessageId, RoleId, UserId,
        WebhookId,
    },
    interaction::{
        AutocompleteCallbackData, AutocompleteChoices, DeferredCallbackData, Interaction,
        InteractionResponse, MessageCallbackData, ModalCallbackData,
    },
    invite::InviteWithMetadata,
    message::{Message, MessageFlags, MessageReference},
    permissions::Permissions,
    rest::{
        CreateChannelInvite, CreateFollowupMessage, CreateGuildBan, CreateMessage, CreateWebhook,
        EditMessage, EditWebhookMessage, ExecuteWebhook, GetGuildAuditLog, ModifyGuild,
        ModifyGuildChannel, ModifyGuildMember, ModifyWebhook, SendSoundboardSound,
        StartThreadFromMessage,
    },
    user::User,
    util::Timestamp,
    webhook::Webhook,
};
use djs_rest::{Error, ReactionEmoji};

use crate::{
    Context,
    collector::{CollectorBuilder, Key, ReactionEvent, mode},
    manager::{
        AutoModerationRuleManager, BanManager, EmojiManager, GuildChannelManager,
        GuildCommandManager, MemberManager, MessageManager, OverwriteTarget, RoleManager,
        ScheduledEventManager, SoundboardSoundManager, StageInstanceManager, StickerManager,
        ThreadManager, ThreadMemberManager,
    },
    router::{ComponentInteraction, ModalInteraction},
    webhook::WebhookError,
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

    /// Removes a reaction from this message: the bot's own with `user: None`, or another
    /// user's ([`MessageManager::remove_reaction`]).
    ///
    /// discord.js counterpart: `ReactionUserManager#remove`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn remove_reaction<'e, B: Backend>(
        &self,
        ctx: &Context<B>,
        emoji: impl Into<ReactionEmoji<'e>>,
        user: Option<UserId>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let (channel_id, message_id) = self.ids();
        ctx.messages(channel_id)
            .remove_reaction(message_id, emoji, user)
    }

    /// Removes every reaction from this message ([`MessageManager::remove_all_reactions`]).
    ///
    /// discord.js counterpart: `ReactionManager#removeAll`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn remove_all_reactions<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let (channel_id, message_id) = self.ids();
        ctx.messages(channel_id).remove_all_reactions(message_id)
    }

    /// A collector of the reactions added to this message (D-263, D-269); with
    /// [`dispose`](CollectorBuilder::dispose) it also yields their removals.
    ///
    /// It ends at its limits, or when the message, its channel, thread or guild is deleted.
    /// Needs the `GUILD_MESSAGE_REACTIONS` or `DIRECT_MESSAGE_REACTIONS` intent.
    ///
    /// discord.js counterpart: `Message#createReactionCollector`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// # use djs_rs::{collector::ReactionEvent, prelude::*};
    /// # async fn example(ctx: Context, poll: djs_rs::model::message::Message) {
    /// let mut votes = poll
    ///     .reaction_collector(&ctx)
    ///     .time(Duration::from_secs(60))
    ///     .build();
    /// while let Some(ReactionEvent::Added(vote)) = votes.next().await {
    ///     println!("{} voted {:?}", vote.user_id, vote.emoji.name());
    /// }
    /// # }
    /// ```
    fn reaction_collector<B: Backend>(&self, ctx: &Context<B>) -> CollectorBuilder<ReactionEvent> {
        let (channel_id, message_id) = self.ids();
        CollectorBuilder::new(
            ctx,
            Key::Message(message_id),
            self.guild(),
            Some(channel_id),
        )
    }

    /// Collects the reactions added to this message until the collector ends, gathered by
    /// emoji ([`reaction_key`](crate::collector::reaction_key)); set a limit (`time`, `max`,
    /// …), or it only ends with the message.
    ///
    /// discord.js counterpart: `Message#awaitReactions`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// # use djs_rs::prelude::*;
    /// # async fn example(ctx: Context, poll: djs_rs::model::message::Message) {
    /// let votes = poll.await_reactions(&ctx).time(Duration::from_secs(60)).await;
    /// let yes = votes.get("👍").map_or(0, |reaction| reaction.count());
    /// println!("{yes} yes votes ({})", votes.reason());
    /// # }
    /// ```
    fn await_reactions<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> CollectorBuilder<ReactionEvent, mode::All> {
        let (channel_id, message_id) = self.ids();
        CollectorBuilder::new(
            ctx,
            Key::Message(message_id),
            self.guild(),
            Some(channel_id),
        )
    }

    /// A collector of the component interactions (buttons, select menus) on this message.
    ///
    /// It ends at its limits, or when the message, its channel, thread or guild is deleted.
    ///
    /// discord.js counterpart: `Message#createMessageComponentCollector`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// # use djs_rs::prelude::*;
    /// # async fn example(ctx: Context, menu: djs_rs::model::message::Message) -> Result<(), HandlerError> {
    /// let mut clicks = menu
    ///     .component_collector(&ctx)
    ///     .idle(Duration::from_secs(120))
    ///     .build();
    /// while let Some(click) = clicks.next().await {
    ///     click.reply(&ctx, format!("You pressed {}", click.custom_id)).await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn component_collector<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> CollectorBuilder<ComponentInteraction> {
        let (channel_id, message_id) = self.ids();
        CollectorBuilder::new(
            ctx,
            Key::Message(message_id),
            self.guild(),
            Some(channel_id),
        )
    }

    /// Waits for one component interaction on this message (D-268); set a
    /// [`time`](CollectorBuilder::time), or it only ends with the message.
    ///
    /// discord.js counterpart: `Message#awaitMessageComponent`.
    ///
    /// # Errors
    ///
    /// [`CollectorError`](crate::collector::CollectorError) with the reason if the collector
    /// ended first ([`EndReason::Time`](crate::collector::EndReason::Time), …).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// # use djs_rs::prelude::*;
    /// # async fn example(ctx: Context, prompt: djs_rs::model::message::Message) -> Result<(), HandlerError> {
    /// match prompt.await_component(&ctx).time(Duration::from_secs(30)).await {
    ///     Ok(click) if click.custom_id == "confirm" => click.update(&ctx, "Confirmed").await?,
    ///     Ok(click) => click.update(&ctx, "Cancelled").await?,
    ///     Err(_) => println!("no answer in 30 seconds"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn await_component<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> CollectorBuilder<ComponentInteraction, mode::First> {
        let (channel_id, message_id) = self.ids();
        CollectorBuilder::new(
            ctx,
            Key::Message(message_id),
            self.guild(),
            Some(channel_id),
        )
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

    /// Edits this guild channel ([`ChannelManager::edit`](crate::manager::ChannelManager::edit)).
    ///
    /// discord.js counterpart: `GuildChannel#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn edit<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: &ModifyGuildChannel,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<Arc<Channel>, Error>> + Send {
        ctx.channels().edit(self.channel_id(), body, reason)
    }

    /// Deletes this channel ([`ChannelManager::delete`](crate::manager::ChannelManager::delete)).
    ///
    /// discord.js counterpart: `GuildChannel#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn delete<B: Backend>(
        &self,
        ctx: &Context<B>,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<Channel, Error>> + Send {
        ctx.channels().delete(self.channel_id(), reason)
    }

    /// Sets this channel's permission overwrite for a role or a member, replacing it (D-235,
    /// [`ChannelManager::set_permissions`](crate::manager::ChannelManager::set_permissions)).
    ///
    /// discord.js counterpart: `PermissionOverwriteManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn set_permissions<B: Backend>(
        &self,
        ctx: &Context<B>,
        target: impl Into<OverwriteTarget>,
        allow: Permissions,
        deny: Permissions,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        ctx.channels()
            .set_permissions(self.channel_id(), target, allow, deny, reason)
    }

    /// Removes this channel's permission overwrite for a role or a member
    /// ([`ChannelManager::delete_permissions`](crate::manager::ChannelManager::delete_permissions)).
    ///
    /// discord.js counterpart: `PermissionOverwriteManager#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn delete_permissions<B: Backend>(
        &self,
        ctx: &Context<B>,
        target: impl Into<OverwriteTarget>,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        ctx.channels()
            .delete_permissions(self.channel_id(), target, reason)
    }

    /// Creates a webhook in this channel ([`WebhookManager::create`](crate::manager::WebhookManager::create)).
    ///
    /// discord.js counterpart: `TextChannel#createWebhook`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn create_webhook<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: &CreateWebhook,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<Webhook, Error>> + Send {
        ctx.webhooks().create(self.channel_id(), body, reason)
    }

    /// The webhooks of this channel ([`WebhookManager::list_channel`](crate::manager::WebhookManager::list_channel)).
    ///
    /// discord.js counterpart: `TextChannel#fetchWebhooks`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn webhooks<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<Collection<WebhookId, Webhook>, Error>> + Send {
        ctx.webhooks().list_channel(self.channel_id())
    }

    /// Creates an invite to this channel ([`InviteManager::create`](crate::manager::InviteManager::create)).
    ///
    /// discord.js counterpart: `TextChannel#createInvite`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn create_invite<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: &CreateChannelInvite,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<InviteWithMetadata, Error>> + Send {
        ctx.invites().create(self.channel_id(), body, reason)
    }

    /// The invites to this channel ([`InviteManager::list_channel`](crate::manager::InviteManager::list_channel)).
    ///
    /// discord.js counterpart: `TextChannel#fetchInvites`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn invites<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<Collection<String, InviteWithMetadata>, Error>> + Send {
        ctx.invites().list_channel(self.channel_id())
    }

    /// The manager of this channel's threads: create, list archived (D-242).
    ///
    /// discord.js counterpart: `TextChannel#threads`.
    fn threads<'a, B: Backend>(&self, ctx: &'a Context<B>) -> ThreadManager<'a, B> {
        ctx.threads(self.channel_id())
    }

    /// The manager of this thread's members: add, remove, join, leave (D-242).
    ///
    /// discord.js counterpart: `ThreadChannel#members`.
    fn thread_members<'a, B: Backend>(&self, ctx: &'a Context<B>) -> ThreadMemberManager<'a, B> {
        ctx.thread_members(self.channel_id())
    }

    /// Plays a soundboard sound in this voice channel
    /// ([`ChannelManager::send_soundboard_sound`](crate::manager::ChannelManager::send_soundboard_sound)).
    ///
    /// discord.js counterpart: `VoiceChannel#sendSoundboardSound`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn send_soundboard_sound<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: &SendSoundboardSound,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        ctx.channels()
            .send_soundboard_sound(self.channel_id(), body)
    }

    /// A collector of the messages sent in this channel
    /// ([`Context::message_collector`]).
    ///
    /// discord.js counterpart: `TextChannel#createMessageCollector`.
    fn message_collector<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> CollectorBuilder<Arc<MessageCreate>> {
        ctx.message_collector(self.channel_id())
    }

    /// Collects the messages sent in this channel until the collector ends
    /// ([`Context::await_messages`]).
    ///
    /// discord.js counterpart: `TextChannel#awaitMessages`.
    fn await_messages<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> CollectorBuilder<Arc<MessageCreate>, mode::All> {
        ctx.await_messages(self.channel_id())
    }

    /// A collector of the component interactions on every message of this channel
    /// ([`Context::component_collector`]).
    ///
    /// discord.js counterpart: `TextChannel#createMessageComponentCollector`.
    fn component_collector<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> CollectorBuilder<ComponentInteraction> {
        ctx.component_collector(self.channel_id())
    }

    /// Waits for one component interaction on any message of this channel
    /// ([`Context::await_component`]).
    ///
    /// discord.js counterpart: `TextChannel#awaitMessageComponent`.
    ///
    /// # Errors
    ///
    /// [`CollectorError`](crate::collector::CollectorError) with the reason if the collector
    /// ended first.
    fn await_component<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> CollectorBuilder<ComponentInteraction, mode::First> {
        ctx.await_component(self.channel_id())
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

    /// The manager of this guild's channels as a whole: create, list, positions (D-234).
    ///
    /// discord.js counterpart: `Guild#channels`.
    fn channels<'a, B: Backend>(&self, ctx: &'a Context<B>) -> GuildChannelManager<'a, B> {
        ctx.guild_channels(self.guild_id())
    }

    /// Edits this guild's settings ([`GuildManager::edit`](crate::manager::GuildManager::edit)).
    ///
    /// discord.js counterpart: `Guild#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn edit<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: &ModifyGuild,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<Arc<CachedGuild>, Error>> + Send {
        ctx.guilds().edit(self.guild_id(), body, reason)
    }

    /// Makes the bot leave this guild ([`GuildManager::leave`](crate::manager::GuildManager::leave)).
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
        ctx.guilds().leave(self.guild_id())
    }

    /// The manager of this guild's custom emojis.
    ///
    /// discord.js counterpart: `Guild#emojis`.
    fn emojis<'a, B: Backend>(&self, ctx: &'a Context<B>) -> EmojiManager<'a, B> {
        ctx.emojis(self.guild_id())
    }

    /// The manager of this guild's stickers.
    ///
    /// discord.js counterpart: `Guild#stickers`.
    fn stickers<'a, B: Backend>(&self, ctx: &'a Context<B>) -> StickerManager<'a, B> {
        ctx.stickers(self.guild_id())
    }

    /// The webhooks of this guild ([`WebhookManager::list_guild`](crate::manager::WebhookManager::list_guild)).
    ///
    /// discord.js counterpart: `Guild#fetchWebhooks`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn webhooks<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<Collection<WebhookId, Webhook>, Error>> + Send {
        ctx.webhooks().list_guild(self.guild_id())
    }

    /// The invites to this guild ([`InviteManager::list_guild`](crate::manager::InviteManager::list_guild)).
    ///
    /// discord.js counterpart: `GuildInviteManager#fetch()`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn invites<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<Collection<String, InviteWithMetadata>, Error>> + Send {
        ctx.invites().list_guild(self.guild_id())
    }

    /// The manager of this guild's scheduled events.
    ///
    /// discord.js counterpart: `Guild#scheduledEvents`.
    fn scheduled_events<'a, B: Backend>(
        &self,
        ctx: &'a Context<B>,
    ) -> ScheduledEventManager<'a, B> {
        ctx.scheduled_events(self.guild_id())
    }

    /// The manager of this guild's live stages.
    ///
    /// discord.js counterpart: `Guild#stageInstances`.
    fn stage_instances<'a, B: Backend>(&self, ctx: &'a Context<B>) -> StageInstanceManager<'a, B> {
        ctx.stage_instances(self.guild_id())
    }

    /// The manager of this guild's soundboard sounds.
    ///
    /// discord.js counterpart: `Guild#soundboardSounds`.
    fn soundboard_sounds<'a, B: Backend>(
        &self,
        ctx: &'a Context<B>,
    ) -> SoundboardSoundManager<'a, B> {
        ctx.soundboard_sounds(self.guild_id())
    }

    /// The manager of the bot's application commands in this guild (D-244).
    ///
    /// discord.js counterpart: `Guild#commands`.
    ///
    /// # Panics
    ///
    /// Panics before the first `READY` (D-247), like [`Context::commands`].
    #[track_caller]
    fn commands<'a, B: Backend>(&self, ctx: &'a Context<B>) -> GuildCommandManager<'a, B> {
        ctx.guild_commands(self.guild_id())
    }

    /// The manager of this guild's auto moderation rules.
    ///
    /// discord.js counterpart: `Guild#autoModerationRules`.
    fn auto_moderation_rules<'a, B: Backend>(
        &self,
        ctx: &'a Context<B>,
    ) -> AutoModerationRuleManager<'a, B> {
        ctx.auto_moderation_rules(self.guild_id())
    }

    /// A page of this guild's audit log ([`GuildManager::audit_log`](crate::manager::GuildManager::audit_log)).
    ///
    /// discord.js counterpart: `Guild#fetchAuditLogs`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn audit_log<B: Backend>(
        &self,
        ctx: &Context<B>,
        query: &GetGuildAuditLog,
    ) -> impl Future<Output = Result<AuditLog, Error>> + Send {
        ctx.guilds().audit_log(self.guild_id(), query)
    }

    /// This guild's integrations ([`GuildManager::integrations`](crate::manager::GuildManager::integrations)).
    ///
    /// discord.js counterpart: `Guild#fetchIntegrations`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn integrations<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<Collection<IntegrationId, Integration>, Error>> + Send {
        ctx.guilds().integrations(self.guild_id())
    }
}

impl GuildExt for Guild {}
impl GuildExt for CachedGuild {}

/// Actions on a [`Webhook`] (D-237, D-240).
///
/// This trait is sealed.
///
/// # Examples
///
/// ```no_run
/// # use djs_rs::prelude::*;
/// # use djs_rs::model::id::ChannelId;
/// # async fn run(ctx: Context, channel: ChannelId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// let webhooks = ctx.webhooks().list_channel(channel).await?;
/// if let Some(webhook) = webhooks.values().find(|webhook| webhook.token.is_some()) {
///     webhook.send(&ctx, "Hello from a webhook").await?;
/// }
/// # Ok(())
/// # }
/// ```
pub trait WebhookExt: sealed::Webhook {
    /// Sends a message through this webhook with its token
    /// ([`WebhookManager::send`](crate::manager::WebhookManager::send)).
    ///
    /// discord.js counterpart: `Webhook#send`.
    ///
    /// # Errors
    ///
    /// Returns [`WebhookError::MissingToken`] for a webhook without a token (channel-follower
    /// and application webhooks), or the REST error if the request failed.
    fn send<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: impl Into<ExecuteWebhook>,
    ) -> impl Future<Output = Result<Arc<CachedMessage>, WebhookError>> + Send {
        let (webhook_id, token) = self.webhook_parts();
        let body = body.into();
        let webhooks = ctx.webhooks();
        async move {
            let token = token.ok_or(WebhookError::MissingToken)?;
            Ok(webhooks.send(webhook_id, token, body).await?)
        }
    }

    /// Edits this webhook ([`WebhookManager::edit`](crate::manager::WebhookManager::edit)).
    ///
    /// discord.js counterpart: `Webhook#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn edit<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: &ModifyWebhook,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<Webhook, Error>> + Send {
        ctx.webhooks().edit(self.webhook_parts().0, body, reason)
    }

    /// Deletes this webhook ([`WebhookManager::delete`](crate::manager::WebhookManager::delete)).
    ///
    /// discord.js counterpart: `Webhook#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn delete<B: Backend>(
        &self,
        ctx: &Context<B>,
        reason: Option<&str>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        ctx.webhooks().delete(self.webhook_parts().0, reason)
    }
}

impl WebhookExt for Webhook {}

/// Responses to an [`Interaction`] (D-248): one method per Discord callback, plus the original
/// response and follow-up messages. Works on the
/// [`InteractionCreate`](djs_model::gateway::payload::InteractionCreate) payload too, which
/// dereferences to the interaction.
///
/// This trait is sealed.
///
/// # The order of responses
///
/// Nothing is tracked locally (D-248): the rules are Discord's, and Discord rejects a request
/// that breaks them.
///
/// 1. **Within 3 seconds**, exactly one *initial response*:
///    - [`reply`](Self::reply) — a message (commands, components, modals);
///    - [`defer_reply`](Self::defer_reply) — "thinking…", the message comes later with
///      [`edit_reply`](Self::edit_reply);
///    - [`update`](Self::update) / [`defer_update`](Self::defer_update) — edit the message the
///      component is on, now or later (components, modals opened from one);
///    - [`show_modal`](Self::show_modal) — open a modal (commands, components);
///    - [`respond`](Self::respond) — autocomplete suggestions (autocomplete only);
///    - [`launch_activity`](Self::launch_activity) — start the app's Activity.
///
///    A second initial response fails with error `40060` ("Interaction has already been
///    acknowledged"); a late one with `10062` ("Unknown interaction").
/// 2. **For 15 minutes** after it, the interaction token allows
///    [`fetch_reply`](Self::fetch_reply), [`edit_reply`](Self::edit_reply),
///    [`delete_reply`](Self::delete_reply) and the follow-up messages
///    ([`follow_up`](Self::follow_up) and the `*_follow_up` methods).
///
/// These requests are authorized by the interaction token, not the bot token, and do not
/// count against the global rate limit. Messages they return are cached (D-233).
///
/// # Examples
///
/// ```no_run
/// # use djs_rs::prelude::*;
/// # let client = Client::builder("token").build();
/// client.on(Events::InteractionCreate, |ctx, interaction| async move {
///     let Some(command) = interaction.command() else {
///         return Ok(());
///     };
///     match command.name.as_str() {
///         "ping" => interaction.reply(&ctx, "Pong!").await?,
///         "slow" => {
///             interaction.defer_reply(&ctx, Default::default()).await?;
///             // … up to 15 minutes of work …
///             interaction.edit_reply(&ctx, "Done").await?;
///         }
///         _ => {}
///     }
///     Ok(())
/// });
/// ```
pub trait InteractionExt: sealed::Interaction {
    /// Responds with a message ([`ChannelMessageWithSource`](InteractionResponse::ChannelMessageWithSource));
    /// accepts anything that converts into [`MessageCallbackData`], such as `&str`. Set
    /// [`MessageFlags::EPHEMERAL`] in its `flags` for a message only the user sees.
    ///
    /// An initial response (see [the order of responses](InteractionExt#the-order-of-responses)).
    ///
    /// discord.js counterpart: `reply`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn reply<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: impl Into<MessageCallbackData>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let response = InteractionResponse::ChannelMessageWithSource(Box::new(body.into()));
        callback(ctx, self.interaction_parts(), response)
    }

    /// Acknowledges the interaction and shows "thinking…"
    /// ([`DeferredChannelMessageWithSource`](InteractionResponse::DeferredChannelMessageWithSource));
    /// the message follows with [`edit_reply`](Self::edit_reply). `flags` can only be
    /// [`MessageFlags::EPHEMERAL`] (for an ephemeral message) or empty.
    ///
    /// An initial response (see [the order of responses](InteractionExt#the-order-of-responses)).
    ///
    /// discord.js counterpart: `deferReply`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn defer_reply<B: Backend>(
        &self,
        ctx: &Context<B>,
        flags: MessageFlags,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let response =
            InteractionResponse::DeferredChannelMessageWithSource(DeferredCallbackData::new(flags));
        callback(ctx, self.interaction_parts(), response)
    }

    /// Edits the message the component is on
    /// ([`UpdateMessage`](InteractionResponse::UpdateMessage)): fields left `None` keep their
    /// value. Component interactions and modals opened from a component.
    ///
    /// An initial response (see [the order of responses](InteractionExt#the-order-of-responses)).
    ///
    /// discord.js counterpart: `update`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn update<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: impl Into<MessageCallbackData>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let response = InteractionResponse::UpdateMessage(Box::new(body.into()));
        callback(ctx, self.interaction_parts(), response)
    }

    /// Acknowledges a component interaction without a loading state
    /// ([`DeferredUpdateMessage`](InteractionResponse::DeferredUpdateMessage)); the message
    /// can be edited later with [`edit_reply`](Self::edit_reply).
    ///
    /// An initial response (see [the order of responses](InteractionExt#the-order-of-responses)).
    ///
    /// discord.js counterpart: `deferUpdate`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn defer_update<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        callback(
            ctx,
            self.interaction_parts(),
            InteractionResponse::DeferredUpdateMessage,
        )
    }

    /// Opens a modal ([`Modal`](InteractionResponse::Modal)), for example one built with
    /// `djs_builders::ModalBuilder`. Its submission arrives as a new interaction. Not for
    /// modal-submit and autocomplete interactions.
    ///
    /// An initial response (see [the order of responses](InteractionExt#the-order-of-responses)).
    ///
    /// discord.js counterpart: `showModal`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn show_modal<B: Backend>(
        &self,
        ctx: &Context<B>,
        modal: ModalCallbackData,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let response = InteractionResponse::Modal(Box::new(modal));
        callback(ctx, self.interaction_parts(), response)
    }

    /// Suggests choices for the option being typed in an autocomplete interaction
    /// ([`AutocompleteResult`](InteractionResponse::AutocompleteResult)); up to 25, typed like
    /// the option. An empty list shows "no suggestions".
    ///
    /// An initial response (see [the order of responses](InteractionExt#the-order-of-responses)).
    ///
    /// discord.js counterpart: `AutocompleteInteraction#respond`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn respond<B: Backend>(
        &self,
        ctx: &Context<B>,
        choices: AutocompleteChoices,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let response =
            InteractionResponse::AutocompleteResult(AutocompleteCallbackData::new(choices));
        callback(ctx, self.interaction_parts(), response)
    }

    /// Launches the app's Activity ([`LaunchActivity`](InteractionResponse::LaunchActivity);
    /// apps with Activities enabled).
    ///
    /// An initial response (see [the order of responses](InteractionExt#the-order-of-responses)).
    ///
    /// discord.js counterpart: `launchActivity`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn launch_activity<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        callback(
            ctx,
            self.interaction_parts(),
            InteractionResponse::LaunchActivity,
        )
    }

    /// The message of the initial response
    /// ([Get Original Interaction Response](djs_rest::Rest::get_original_interaction_response)).
    ///
    /// discord.js counterpart: `fetchReply`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (for example before an initial response).
    fn fetch_reply<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<Arc<CachedMessage>, Error>> + Send {
        let (_, application_id, token) = self.interaction_parts();
        async move {
            let message = ctx
                .rest()
                .get_original_interaction_response(application_id, token)
                .await?;
            Ok(ctx.store_message(&message))
        }
    }

    /// Edits the message of the initial response — also how the message of a
    /// [`defer_reply`](Self::defer_reply) is sent, and how a
    /// [`defer_update`](Self::defer_update)d message is edited
    /// ([Edit Original Interaction Response](djs_rest::Rest::edit_original_interaction_response)).
    /// Accepts anything that converts into an [`EditWebhookMessage`], such as `&str`.
    ///
    /// discord.js counterpart: `editReply`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn edit_reply<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: impl Into<EditWebhookMessage>,
    ) -> impl Future<Output = Result<Arc<CachedMessage>, Error>> + Send {
        let (_, application_id, token) = self.interaction_parts();
        let body = body.into();
        async move {
            let message = ctx
                .rest()
                .edit_original_interaction_response(application_id, token, &body)
                .await?;
            Ok(ctx.store_message(&message))
        }
    }

    /// Deletes the message of the initial response
    /// ([Delete Original Interaction Response](djs_rest::Rest::delete_original_interaction_response)).
    ///
    /// discord.js counterpart: `deleteReply`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn delete_reply<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let (_, application_id, token) = self.interaction_parts();
        async move {
            ctx.rest()
                .delete_original_interaction_response(application_id, token)
                .await
        }
    }

    /// Sends another message for the interaction
    /// ([Create Followup Message](djs_rest::Rest::create_followup_message)); accepts anything
    /// that converts into a [`CreateFollowupMessage`], such as `&str`. After a
    /// [`defer_reply`](Self::defer_reply), the first follow-up replaces "thinking…".
    ///
    /// discord.js counterpart: `followUp`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (`40094` after the 5 follow-ups an app
    /// installed only for the user may send).
    fn follow_up<B: Backend>(
        &self,
        ctx: &Context<B>,
        body: impl Into<CreateFollowupMessage>,
    ) -> impl Future<Output = Result<Arc<CachedMessage>, Error>> + Send {
        let (_, application_id, token) = self.interaction_parts();
        let body = body.into();
        async move {
            let message = ctx
                .rest()
                .create_followup_message(application_id, token, &body)
                .await?;
            Ok(ctx.store_message(&message))
        }
    }

    /// A follow-up message ([Get Followup Message](djs_rest::Rest::get_followup_message)).
    ///
    /// discord.js counterpart: `fetchReply` with a message ID.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn fetch_follow_up<B: Backend>(
        &self,
        ctx: &Context<B>,
        message_id: MessageId,
    ) -> impl Future<Output = Result<Arc<CachedMessage>, Error>> + Send {
        let (_, application_id, token) = self.interaction_parts();
        async move {
            let message = ctx
                .rest()
                .get_followup_message(application_id, token, message_id)
                .await?;
            Ok(ctx.store_message(&message))
        }
    }

    /// Edits a follow-up message ([Edit Followup Message](djs_rest::Rest::edit_followup_message));
    /// accepts anything that converts into an [`EditWebhookMessage`], such as `&str`.
    ///
    /// discord.js counterpart: `editReply` with a message ID.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn edit_follow_up<B: Backend>(
        &self,
        ctx: &Context<B>,
        message_id: MessageId,
        body: impl Into<EditWebhookMessage>,
    ) -> impl Future<Output = Result<Arc<CachedMessage>, Error>> + Send {
        let (_, application_id, token) = self.interaction_parts();
        let body = body.into();
        async move {
            let message = ctx
                .rest()
                .edit_followup_message(application_id, token, message_id, &body)
                .await?;
            Ok(ctx.store_message(&message))
        }
    }

    /// Deletes a follow-up message
    /// ([Delete Followup Message](djs_rest::Rest::delete_followup_message)).
    ///
    /// discord.js counterpart: `deleteReply` with a message ID.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    fn delete_follow_up<B: Backend>(
        &self,
        ctx: &Context<B>,
        message_id: MessageId,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let (_, application_id, token) = self.interaction_parts();
        async move {
            ctx.rest()
                .delete_followup_message(application_id, token, message_id)
                .await
        }
    }

    /// A collector of the component interactions on this interaction's reply (and its
    /// follow-ups), matched by the interaction they belong to: no need to fetch the reply.
    /// For a component interaction (whose response edits its message), the components of
    /// that message.
    ///
    /// It ends at its limits, or when the reply, its channel, thread or guild is deleted.
    ///
    /// discord.js counterpart: `InteractionResponse#createMessageComponentCollector`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// # use djs_rs::prelude::*;
    /// # let client = Client::builder("token").build();
    /// client.command("counter", |ctx, cmd| async move {
    ///     cmd.reply(&ctx, "Count: 0").await?; // with a "+1" button
    ///     let mut clicks = cmd.component_collector(&ctx).time(Duration::from_secs(300)).build();
    ///     let mut count = 0;
    ///     while let Some(click) = clicks.next().await {
    ///         count += 1;
    ///         click.update(&ctx, format!("Count: {count}")).await?;
    ///     }
    ///     Ok(())
    /// });
    /// ```
    fn component_collector<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> CollectorBuilder<ComponentInteraction> {
        let interaction = self.interaction();
        CollectorBuilder::new(
            ctx,
            reply_key(interaction),
            interaction.guild_id,
            interaction.channel_id,
        )
    }

    /// Waits for one component interaction on this interaction's reply (D-268); set a
    /// [`time`](CollectorBuilder::time).
    ///
    /// discord.js counterpart: `InteractionResponse#awaitMessageComponent`.
    ///
    /// # Errors
    ///
    /// [`CollectorError`](crate::collector::CollectorError) with the reason if the collector
    /// ended first.
    fn await_component<B: Backend>(
        &self,
        ctx: &Context<B>,
    ) -> CollectorBuilder<ComponentInteraction, mode::First> {
        let interaction = self.interaction();
        CollectorBuilder::new(
            ctx,
            reply_key(interaction),
            interaction.guild_id,
            interaction.channel_id,
        )
    }

    /// Waits up to `time` for the user of this interaction to submit a modal (D-268), usually
    /// the one shown with [`show_modal`](Self::show_modal). Add a
    /// [`filter`](CollectorBuilder::filter) on the modal's `custom_id` when several can be
    /// open. `time` is required, as in discord.js: a closed modal sends nothing.
    ///
    /// discord.js counterpart: `awaitModalSubmit`.
    ///
    /// # Errors
    ///
    /// [`CollectorError`](crate::collector::CollectorError) with
    /// [`EndReason::Time`](crate::collector::EndReason::Time) if nothing was submitted in time.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// # use djs_rs::prelude::*;
    /// # let client = Client::builder("token").build();
    /// client.command("feedback", |ctx, cmd| async move {
    ///     // cmd.show_modal(&ctx, …) with a text input "text" in the modal "feedback"
    ///     let submitted = cmd
    ///         .await_modal_submit(&ctx, Duration::from_secs(300))
    ///         .filter(|modal| modal.custom_id == "feedback")
    ///         .await?;
    ///     let text = submitted.text_value("text").unwrap_or_default();
    ///     submitted.reply(&ctx, format!("Thanks for: {text}")).await?;
    ///     Ok(())
    /// });
    /// ```
    fn await_modal_submit<B: Backend>(
        &self,
        ctx: &Context<B>,
        time: Duration,
    ) -> CollectorBuilder<ModalInteraction, mode::First> {
        let interaction = self.interaction();
        // Every interaction a bot receives has a user; the fallback matches no one.
        let user = interaction
            .author()
            .map_or(UserId::new(u64::MAX), |user| user.id);
        CollectorBuilder::new(ctx, Key::User(user), interaction.guild_id, None).time(time)
    }
}

impl InteractionExt for Interaction {}
impl InteractionExt for crate::router::CommandInteraction {}
impl InteractionExt for crate::router::AutocompleteInteraction {}
impl InteractionExt for crate::router::ComponentInteraction {}
impl InteractionExt for crate::router::ModalInteraction {}

/// What identifies the components of an interaction's reply: the interaction itself, or for a
/// component interaction its message (discord.js does the same in `update`).
fn reply_key(interaction: &Interaction) -> Key {
    match &interaction.message {
        Some(message) => Key::Message(message.id),
        None => Key::Interaction(interaction.id),
    }
}

/// Sends the initial response `response` (Create Interaction Response).
async fn callback<B: Backend>(
    ctx: &Context<B>,
    (interaction_id, _, token): (InteractionId, ApplicationId, &str),
    response: InteractionResponse,
) -> Result<(), Error> {
    ctx.rest()
        .create_interaction_response(interaction_id, token, &response)
        .await
}

/// The IDs the extension traits need; sealed so the traits stay implementable only here.
mod sealed {
    use super::{ApplicationId, ChannelId, GuildId, InteractionId, MessageId, UserId, WebhookId};

    pub trait Interaction {
        /// The interaction's ID, its application and its token.
        fn interaction_parts(&self) -> (InteractionId, ApplicationId, &str);

        /// The whole interaction.
        fn interaction(&self) -> &djs_model::interaction::Interaction;
    }

    impl Interaction for djs_model::interaction::Interaction {
        fn interaction_parts(&self) -> (InteractionId, ApplicationId, &str) {
            (self.id, self.application_id, &self.token)
        }

        fn interaction(&self) -> &Self {
            self
        }
    }

    /// The router's wrappers (D-252) answer for their interaction.
    macro_rules! wrapped {
        ($($wrapper:ident),*) => {$(
            impl Interaction for crate::router::$wrapper {
                fn interaction_parts(&self) -> (InteractionId, ApplicationId, &str) {
                    crate::router::$wrapper::interaction(self).interaction_parts()
                }

                fn interaction(&self) -> &djs_model::interaction::Interaction {
                    crate::router::$wrapper::interaction(self)
                }
            }
        )*};
    }

    wrapped!(
        CommandInteraction,
        AutocompleteInteraction,
        ComponentInteraction,
        ModalInteraction
    );

    pub trait Message {
        /// The channel and the message.
        fn ids(&self) -> (ChannelId, MessageId);

        /// The guild, if the message is in one and its data says so.
        fn guild(&self) -> Option<GuildId>;
    }

    impl Message for djs_model::message::Message {
        fn ids(&self) -> (ChannelId, MessageId) {
            (self.channel_id, self.id)
        }

        fn guild(&self) -> Option<GuildId> {
            self.guild_id
        }
    }

    impl Message for djs_cache::CachedMessage {
        fn ids(&self) -> (ChannelId, MessageId) {
            (self.channel_id, self.id)
        }

        fn guild(&self) -> Option<GuildId> {
            self.guild_id
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

    pub trait Webhook {
        /// The ID and the token, if the webhook has one.
        fn webhook_parts(&self) -> (WebhookId, Option<&str>);
    }

    impl Webhook for djs_model::webhook::Webhook {
        fn webhook_parts(&self) -> (WebhookId, Option<&str>) {
            (self.id, self.token.as_deref())
        }
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
