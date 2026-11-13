//! [Channel endpoints](https://docs.discord.com/developers/resources/channel): channels,
//! permission overwrites, invites, group DMs and threads.
//!
//! Creating guild channels and listing a guild's channels or active threads are guild
//! endpoints.

use djs_model::{
    channel::{Channel, FollowedChannel, ThreadMember},
    id::{ChannelId, GenericId, MessageId, RoleId, UserId},
    invite::InviteWithMetadata,
    permissions::Permissions,
    rest::{
        ArchivedThreads, CreateChannelInvite, EditChannelPermissions, FollowAnnouncementChannel,
        ForumThread, GetChannelPermissions, GetThreadMember, GroupDmAddRecipient,
        ListArchivedThreads, ListJoinedPrivateArchivedThreads, ListThreadMembers, ModifyGroupDm,
        ModifyGuildChannel, ModifyThread, SetVoiceChannelStatus, StartThreadFromMessage,
        StartThreadInForumOrMediaChannel, StartThreadWithoutMessage,
    },
};

use super::PermissionsResponse;
use crate::client::boxed;
use crate::{Error, Request, Rest, Route};

impl Rest {
    /// [Get Channel](https://docs.discord.com/developers/resources/channel#get-channel):
    /// returns a channel; for a thread the current user joined, with their
    /// [`ThreadMember`].
    ///
    /// discord.js counterpart: `ChannelManager#fetch`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10003` Unknown Channel, `50001` Missing Access.
    pub fn get_channel(
        &self,
        channel_id: ChannelId,
    ) -> impl Future<Output = Result<Channel, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetChannel { channel_id }))
                .await
        })
    }

    /// [Modify Channel](https://docs.discord.com/developers/resources/channel#modify-channel)
    /// of a guild channel (D-109): changes the fields set in `body`. Needs
    /// `MANAGE_CHANNELS`, and `MANAGE_ROLES` to change permission overwrites.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildChannel#edit`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, channel_id: djs_model::id::ChannelId) -> Result<(), djs_rest::Error> {
    /// use djs_model::{rest::ModifyGuildChannel, util::Patch};
    ///
    /// let mut body = ModifyGuildChannel::default();
    /// body.topic = Patch::Set("Rust bots only".into());
    /// body.rate_limit_per_user = Patch::Set(30);
    /// rest.modify_channel(channel_id, &body, Some("Calming things down"))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50035` for an invalid field (see
    /// [`ApiError::field_errors`](crate::ApiError::field_errors)), `50013` Missing
    /// Permissions.
    pub fn modify_channel<'a>(
        &'a self,
        channel_id: ChannelId,
        body: &'a ModifyGuildChannel,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Channel, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ModifyChannel { channel_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Modify Channel](https://docs.discord.com/developers/resources/channel#modify-channel)
    /// of a thread (D-109): changes the fields set in `body`. Needs `MANAGE_THREADS`, except
    /// for unarchiving an unlocked thread; an archived thread must be unarchived by the same
    /// request.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `ThreadChannel#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50083` when the thread is archived.
    pub fn modify_thread<'a>(
        &'a self,
        channel_id: ChannelId,
        body: &'a ModifyThread,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Channel, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ModifyChannel { channel_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Modify Channel](https://docs.discord.com/developers/resources/channel#modify-channel)
    /// of a group DM (D-109): changes its name or icon.
    ///
    /// discord.js counterpart: `PartialGroupDMChannel#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn modify_group_dm<'a>(
        &'a self,
        channel_id: ChannelId,
        body: &'a ModifyGroupDm,
    ) -> impl Future<Output = Result<Channel, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::ModifyChannel { channel_id }).json(body)?)
                .await
        })
    }

    /// [Delete/Close Channel](https://docs.discord.com/developers/resources/channel#deleteclose-channel):
    /// deletes a guild channel or thread (for good), or closes a DM; returns the channel.
    /// Needs `MANAGE_CHANNELS`, or `MANAGE_THREADS` for a thread. The channels of a deleted
    /// category are kept, without a parent.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildChannel#delete`, `DMChannel#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50074` for the rules, guidelines or community updates
    /// channel of a community guild.
    pub fn delete_channel<'a>(
        &'a self,
        channel_id: ChannelId,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Channel, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::DeleteChannel { channel_id }).reason(reason))
                .await
        })
    }

    /// [Get Channel Member Permissions](https://docs.discord.com/developers/resources/channel#get-channel-member-permissions):
    /// returns a member's permissions in a channel, computed from their roles and the
    /// channel's overwrites (a thread uses its parent's). Needs `VIEW_CHANNEL` (and `CONNECT`
    /// in voice and stage channels).
    ///
    /// discord.js counterpart: `GuildChannel#permissionsFor(member)` (computed locally).
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10007` Unknown Member.
    pub fn get_channel_member_permissions<'a>(
        &'a self,
        channel_id: ChannelId,
        user_id: UserId,
        query: &'a GetChannelPermissions,
    ) -> impl Future<Output = Result<Permissions, Error>> + Send + 'a {
        boxed(async move {
            let response: PermissionsResponse = self
                .request(
                    Request::new(Route::GetChannelMemberPermissions {
                        channel_id,
                        user_id,
                    })
                    .query(query),
                )
                .await?;
            Ok(response.permissions)
        })
    }

    /// [Get Channel Role Permissions](https://docs.discord.com/developers/resources/channel#get-channel-role-permissions):
    /// returns the permissions of a role together with `@everyone` in a channel, with their
    /// overwrites applied (other roles and member overwrites are ignored). Needs
    /// `VIEW_CHANNEL` (and `CONNECT` in voice and stage channels).
    ///
    /// discord.js counterpart: `GuildChannel#permissionsFor(role)` (computed locally).
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10011` Unknown Role.
    pub fn get_channel_role_permissions<'a>(
        &'a self,
        channel_id: ChannelId,
        role_id: RoleId,
        query: &'a GetChannelPermissions,
    ) -> impl Future<Output = Result<Permissions, Error>> + Send + 'a {
        boxed(async move {
            let response: PermissionsResponse = self
                .request(
                    Request::new(Route::GetChannelRolePermissions {
                        channel_id,
                        role_id,
                    })
                    .query(query),
                )
                .await?;
            Ok(response.permissions)
        })
    }

    /// [Set Voice Channel Status](https://docs.discord.com/developers/resources/channel#set-voice-channel-status):
    /// sets or removes the status of a voice channel. Needs `SET_VOICE_CHANNEL_STATUS`, and
    /// `MANAGE_CHANNELS` when the current user is not connected to the channel.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `VoiceChannel#setStatus`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn set_voice_channel_status<'a>(
        &'a self,
        channel_id: ChannelId,
        body: &'a SetVoiceChannelStatus,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.request_ignoring_body(
                Request::new(Route::SetVoiceChannelStatus { channel_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Edit Channel Permissions](https://docs.discord.com/developers/resources/channel#edit-channel-permissions):
    /// creates or replaces the permission overwrite of a role or member (`overwrite_id`) in
    /// a guild channel. Needs `MANAGE_ROLES`; only permissions the bot has can be allowed or
    /// denied, unless it has a `MANAGE_ROLES` overwrite there.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `PermissionOverwriteManager#edit` / `#create`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, channel_id: djs_model::id::ChannelId, role_id: djs_model::id::RoleId) -> Result<(), djs_rest::Error> {
    /// use djs_model::{channel::OverwriteType, permissions::Permissions, rest::EditChannelPermissions};
    ///
    /// let mut muted = EditChannelPermissions::new(OverwriteType::Role);
    /// muted.deny = Some(Permissions::SEND_MESSAGES | Permissions::ADD_REACTIONS);
    /// rest.edit_channel_permissions(channel_id, role_id.cast(), &muted, None)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50013` Missing Permissions.
    pub fn edit_channel_permissions<'a>(
        &'a self,
        channel_id: ChannelId,
        overwrite_id: GenericId,
        body: &'a EditChannelPermissions,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::EditChannelPermissions {
                    channel_id,
                    overwrite_id,
                })
                .json(body)?
                .reason(reason),
            )
            .await
        })
    }

    /// [Delete Channel Permission](https://docs.discord.com/developers/resources/channel#delete-channel-permission):
    /// removes the permission overwrite of a role or member (`overwrite_id`). Needs
    /// `MANAGE_ROLES`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `PermissionOverwriteManager#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn delete_channel_permission<'a>(
        &'a self,
        channel_id: ChannelId,
        overwrite_id: GenericId,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::DeleteChannelPermission {
                    channel_id,
                    overwrite_id,
                })
                .reason(reason),
            )
            .await
        })
    }

    /// [Get Channel Invites](https://docs.discord.com/developers/resources/channel#get-channel-invites):
    /// returns the invites of a guild channel. Needs `MANAGE_CHANNELS`.
    ///
    /// discord.js counterpart: `GuildChannel#fetchInvites`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_channel_invites(
        &self,
        channel_id: ChannelId,
    ) -> impl Future<Output = Result<Vec<InviteWithMetadata>, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetChannelInvites { channel_id }))
                .await
        })
    }

    /// [Create Channel Invite](https://docs.discord.com/developers/resources/channel#create-channel-invite):
    /// creates an invite to a guild channel. Needs `CREATE_INSTANT_INVITE` (and
    /// `MANAGE_ROLES` for [`role_ids`](CreateChannelInvite::role_ids)). With a
    /// [`target_users_file`](CreateChannelInvite::target_users_file) the request is sent as
    /// `multipart/form-data` (D-111).
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildChannel#createInvite`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, channel_id: djs_model::id::ChannelId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::CreateChannelInvite;
    ///
    /// let mut body = CreateChannelInvite::default();
    /// body.max_uses = Some(1);
    /// body.unique = true;
    /// let invite = rest.create_channel_invite(channel_id, &body, None).await?;
    /// println!("https://discord.gg/{}", invite.code);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30016` Maximum number of invites reached.
    pub fn create_channel_invite<'a>(
        &'a self,
        channel_id: ChannelId,
        body: &'a CreateChannelInvite,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<InviteWithMetadata, Error>> + Send + 'a {
        boxed(async move {
            let mut request = Request::new(Route::CreateChannelInvite { channel_id })
                .json(body)?
                .reason(reason);
            if let Some(file) = &body.target_users_file {
                request = request.form_file("target_users_file", file);
            }
            self.call(request).await
        })
    }

    /// [Follow Announcement Channel](https://docs.discord.com/developers/resources/channel#follow-announcement-channel):
    /// publishes the messages of an announcement channel into another channel, through a
    /// webhook created there. Needs `MANAGE_WEBHOOKS` in the target channel.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `NewsChannel#addFollower`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30007` Maximum number of webhooks reached.
    pub fn follow_announcement_channel<'a>(
        &'a self,
        channel_id: ChannelId,
        body: &'a FollowAnnouncementChannel,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<FollowedChannel, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::FollowAnnouncementChannel { channel_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Trigger Typing Indicator](https://docs.discord.com/developers/resources/channel#trigger-typing-indicator):
    /// shows "… is typing" for 10 seconds (or until the bot sends a message). Meant for
    /// commands that take a few seconds to answer.
    ///
    /// discord.js counterpart: `TextBasedChannel#sendTyping`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn trigger_typing_indicator(
        &self,
        channel_id: ChannelId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.request_ignoring_body(Request::new(Route::TriggerTypingIndicator { channel_id }))
                .await
        })
    }

    /// [Group DM Add Recipient](https://docs.discord.com/developers/resources/channel#group-dm-add-recipient):
    /// adds a user to a group DM with the access token they gave the application
    /// (`gdm.join` scope).
    ///
    /// discord.js counterpart: `PartialGroupDMChannel#addRecipient`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn group_dm_add_recipient<'a>(
        &'a self,
        channel_id: ChannelId,
        user_id: UserId,
        body: &'a GroupDmAddRecipient,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.request_ignoring_body(
                Request::new(Route::GroupDmAddRecipient {
                    channel_id,
                    user_id,
                })
                .json(body)?,
            )
            .await
        })
    }

    /// [Group DM Remove Recipient](https://docs.discord.com/developers/resources/channel#group-dm-remove-recipient):
    /// removes a user from a group DM.
    ///
    /// discord.js counterpart: `PartialGroupDMChannel#removeRecipient`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn group_dm_remove_recipient(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.request_ignoring_body(Request::new(Route::GroupDmRemoveRecipient {
                channel_id,
                user_id,
            }))
            .await
        })
    }

    /// [Start Thread from Message](https://docs.discord.com/developers/resources/channel#start-thread-from-message):
    /// creates a thread on an existing message (a public thread in a text channel, an
    /// announcement thread in an announcement channel); the thread gets the message's ID.
    /// Needs `CREATE_PUBLIC_THREADS`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Message#startThread`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `160004` when the message already has a thread.
    pub fn start_thread_from_message<'a>(
        &'a self,
        channel_id: ChannelId,
        message_id: MessageId,
        body: &'a StartThreadFromMessage,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Channel, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::StartThreadFromMessage {
                    channel_id,
                    message_id,
                })
                .json(body)?
                .reason(reason),
            )
            .await
        })
    }

    /// [Start Thread without Message](https://docs.discord.com/developers/resources/channel#start-thread-without-message):
    /// creates a thread that does not start from a message. Needs `CREATE_PUBLIC_THREADS` or
    /// `CREATE_PRIVATE_THREADS`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildTextThreadManager#create`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, channel_id: djs_model::id::ChannelId) -> Result<(), djs_rest::Error> {
    /// use djs_model::{channel::ChannelType, rest::StartThreadWithoutMessage};
    ///
    /// let mut body = StartThreadWithoutMessage::new("Ticket #42", ChannelType::PrivateThread);
    /// body.invitable = Some(false);
    /// let thread = rest
    ///     .start_thread_without_message(channel_id, &body, None)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `160006` Maximum number of active threads reached.
    pub fn start_thread_without_message<'a>(
        &'a self,
        channel_id: ChannelId,
        body: &'a StartThreadWithoutMessage,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Channel, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::StartThreadWithoutMessage { channel_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Start Thread in Forum or Media Channel](https://docs.discord.com/developers/resources/channel#start-thread-in-forum-or-media-channel):
    /// creates a post with its first message, and returns both (D-115). The files of the
    /// message are uploaded with it. Needs `SEND_MESSAGES`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildForumThreadManager#create`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `40067` when the channel requires a tag and none is applied.
    pub fn start_thread_in_forum_or_media_channel<'a>(
        &'a self,
        channel_id: ChannelId,
        body: &'a StartThreadInForumOrMediaChannel,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<ForumThread, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::StartThreadInForumOrMediaChannel { channel_id })
                    .json(body)?
                    .files(&body.message.files)
                    .reason(reason),
            )
            .await
        })
    }

    /// [Join Thread](https://docs.discord.com/developers/resources/channel#join-thread):
    /// adds the current user to a thread that is not archived.
    ///
    /// discord.js counterpart: `ThreadMemberManager#add('@me')` / `ThreadChannel#join`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn join_thread(
        &self,
        channel_id: ChannelId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::JoinThread { channel_id }))
                .await
        })
    }

    /// [Add Thread Member](https://docs.discord.com/developers/resources/channel#add-thread-member):
    /// adds a member to a thread that is not archived (also succeeds if they already are
    /// one). Needs permission to send messages in the thread.
    ///
    /// discord.js counterpart: `ThreadMemberManager#add`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn add_thread_member(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::AddThreadMember {
                channel_id,
                user_id,
            }))
            .await
        })
    }

    /// [Leave Thread](https://docs.discord.com/developers/resources/channel#leave-thread):
    /// removes the current user from a thread that is not archived.
    ///
    /// discord.js counterpart: `ThreadChannel#leave`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn leave_thread(
        &self,
        channel_id: ChannelId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::LeaveThread { channel_id }))
                .await
        })
    }

    /// [Remove Thread Member](https://docs.discord.com/developers/resources/channel#remove-thread-member):
    /// removes a member from a thread that is not archived. Needs `MANAGE_THREADS` (or to
    /// be the creator of a private thread).
    ///
    /// discord.js counterpart: `ThreadMemberManager#remove`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn remove_thread_member(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::RemoveThreadMember {
                channel_id,
                user_id,
            }))
            .await
        })
    }

    /// [Get Thread Member](https://docs.discord.com/developers/resources/channel#get-thread-member):
    /// returns a member of a thread.
    ///
    /// discord.js counterpart: `ThreadMemberManager#fetch`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10007` Unknown Member when the user is not in the thread.
    pub fn get_thread_member<'a>(
        &'a self,
        channel_id: ChannelId,
        user_id: UserId,
        query: &'a GetThreadMember,
    ) -> impl Future<Output = Result<ThreadMember, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::GetThreadMember {
                    channel_id,
                    user_id,
                })
                .query(query),
            )
            .await
        })
    }

    /// [List Thread Members](https://docs.discord.com/developers/resources/channel#list-thread-members):
    /// returns the members of a thread; with
    /// [`with_member`](ListThreadMembers::with_member) one page of them, each with their
    /// guild member. Needs the `GUILD_MEMBERS` privileged intent.
    ///
    /// discord.js counterpart: `ThreadMemberManager#fetch()`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn list_thread_members<'a>(
        &'a self,
        channel_id: ChannelId,
        query: &'a ListThreadMembers,
    ) -> impl Future<Output = Result<Vec<ThreadMember>, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::ListThreadMembers { channel_id }).query(query))
                .await
        })
    }

    /// [List Public Archived Threads](https://docs.discord.com/developers/resources/channel#list-public-archived-threads):
    /// returns archived public (or announcement) threads of a channel, most recently
    /// archived first. Needs `READ_MESSAGE_HISTORY`.
    ///
    /// discord.js counterpart: `GuildTextThreadManager#fetchArchived({ type: 'public' })`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn list_public_archived_threads<'a>(
        &'a self,
        channel_id: ChannelId,
        query: &'a ListArchivedThreads,
    ) -> impl Future<Output = Result<ArchivedThreads, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::ListPublicArchivedThreads { channel_id }).query(query))
                .await
        })
    }

    /// [List Private Archived Threads](https://docs.discord.com/developers/resources/channel#list-private-archived-threads):
    /// returns archived private threads of a channel, most recently archived first. Needs
    /// `READ_MESSAGE_HISTORY` and `MANAGE_THREADS`.
    ///
    /// discord.js counterpart: `GuildTextThreadManager#fetchArchived({ type: 'private' })`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn list_private_archived_threads<'a>(
        &'a self,
        channel_id: ChannelId,
        query: &'a ListArchivedThreads,
    ) -> impl Future<Output = Result<ArchivedThreads, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::ListPrivateArchivedThreads { channel_id }).query(query))
                .await
        })
    }

    /// [List Joined Private Archived Threads](https://docs.discord.com/developers/resources/channel#list-joined-private-archived-threads):
    /// returns the archived private threads of a channel that the current user joined,
    /// highest ID first. Needs `READ_MESSAGE_HISTORY`.
    ///
    /// discord.js counterpart:
    /// `GuildTextThreadManager#fetchArchived({ type: 'private', fetchAll: false })`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn list_joined_private_archived_threads<'a>(
        &'a self,
        channel_id: ChannelId,
        query: &'a ListJoinedPrivateArchivedThreads,
    ) -> impl Future<Output = Result<ArchivedThreads, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ListJoinedPrivateArchivedThreads { channel_id }).query(query),
            )
            .await
        })
    }
}
