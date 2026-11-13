//! [Lobby endpoints](https://docs.discord.com/developers/resources/lobby): groups of players
//! of an app's game (Discord Social SDK), their members, messages and linked channels.
//!
//! Most endpoints take the bot token. Those acting as a player — creating or joining a lobby by
//! its secret, leaving it, linking a channel, sending and reading messages and inviting
//! oneself — need a client built with [`bearer_token`](crate::RestBuilder::bearer_token) and
//! the player's access token (with the `sdk.social_layer` scope where noted).

use djs_model::{
    id::{ChannelId, LobbyId, MessageId, UserId},
    lobby::{Lobby, LobbyInvite, LobbyMember, LobbyMessage},
    rest::{
        AddLobbyMember, BulkLobbyMember, CreateLobby, CreateOrJoinLobby, GetLobbyMessages,
        LobbyMetadata, ModifyLobby, SendLobbyMessage,
    },
};
use serde::Serialize;

use crate::client::boxed;
use crate::{Error, Request, Rest, Route};

/// The body of Link Channel to Lobby; without a channel it unlinks.
#[derive(Serialize)]
struct ChannelLinking {
    #[serde(skip_serializing_if = "Option::is_none")]
    channel_id: Option<ChannelId>,
}

/// Lobbies.
impl Rest {
    /// [Create Lobby](https://docs.discord.com/developers/resources/lobby#create-lobby):
    /// creates a lobby with the given members. Social SDK clients cannot join or leave it
    /// themselves.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, players: Vec<djs_model::id::UserId>) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::{CreateLobby, LobbyMemberData};
    ///
    /// let mut body = CreateLobby::default();
    /// body.members = players.into_iter().map(LobbyMemberData::new).collect();
    /// let lobby = rest.create_lobby(&body).await?;
    /// println!("Lobby {} has {} members", lobby.id, lobby.members.len());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn create_lobby<'a>(
        &'a self,
        body: &'a CreateLobby,
    ) -> impl Future<Output = Result<Lobby, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::CreateLobby).json(body)?)
                .await
        })
    }

    /// [Create or Join Lobby](https://docs.discord.com/developers/resources/lobby#create-or-join-lobby):
    /// joins the application's lobby with the body's secret, creating it if there is none.
    /// Needs the player's access token with the `sdk.social_layer` scope.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn create_or_join_lobby<'a>(
        &'a self,
        body: &'a CreateOrJoinLobby,
    ) -> impl Future<Output = Result<Lobby, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::CreateOrJoinLobby).json(body)?)
                .await
        })
    }

    /// [Get Lobby](https://docs.discord.com/developers/resources/lobby#get-lobby): returns a
    /// lobby with its members.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_lobby(
        &self,
        lobby_id: LobbyId,
    ) -> impl Future<Output = Result<Lobby, Error>> + Send + '_ {
        boxed(async move { self.call(Request::new(Route::GetLobby { lobby_id })).await })
    }

    /// [Modify Lobby](https://docs.discord.com/developers/resources/lobby#modify-lobby):
    /// changes a lobby's metadata, members or idle timeout and returns it.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn modify_lobby<'a>(
        &'a self,
        lobby_id: LobbyId,
        body: &'a ModifyLobby,
    ) -> impl Future<Output = Result<Lobby, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::ModifyLobby { lobby_id }).json(body)?)
                .await
        })
    }

    /// [Delete Lobby](https://docs.discord.com/developers/resources/lobby#delete-lobby):
    /// deletes a lobby; deleting one that is already gone succeeds too.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn delete_lobby(
        &self,
        lobby_id: LobbyId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.request_ignoring_body(Request::new(Route::DeleteLobby { lobby_id }))
                .await
        })
    }
}

/// Lobby members.
impl Rest {
    /// [Add a Member to a Lobby](https://docs.discord.com/developers/resources/lobby#add-a-member-to-a-lobby):
    /// adds a user to a lobby, or updates the member if they are in it already.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn add_lobby_member<'a>(
        &'a self,
        lobby_id: LobbyId,
        user_id: UserId,
        body: &'a AddLobbyMember,
    ) -> impl Future<Output = Result<LobbyMember, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::AddLobbyMember { lobby_id, user_id }).json(body)?)
                .await
        })
    }

    /// [Bulk Update Lobby Members](https://docs.discord.com/developers/resources/lobby#bulk-update-lobby-members):
    /// adds, updates or removes up to 25 members in one request (D-134) and returns the added
    /// and updated ones. Users that fail permission checks or are in too many lobbies are
    /// silently left out.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, lobby_id: djs_model::id::LobbyId, joined: djs_model::id::UserId, left: djs_model::id::UserId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::{BulkLobbyMember, LobbyMemberData};
    ///
    /// rest.bulk_update_lobby_members(
    ///     lobby_id,
    ///     &[
    ///         BulkLobbyMember::upsert(LobbyMemberData::new(joined)),
    ///         BulkLobbyMember::remove(left),
    ///     ],
    /// )
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10013` Unknown User for a user Discord does not know.
    pub fn bulk_update_lobby_members<'a>(
        &'a self,
        lobby_id: LobbyId,
        members: &'a [BulkLobbyMember],
    ) -> impl Future<Output = Result<Vec<LobbyMember>, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::BulkUpdateLobbyMembers { lobby_id }).json(members)?)
                .await
        })
    }

    /// [Remove a Member from a Lobby](https://docs.discord.com/developers/resources/lobby#remove-a-member-from-a-lobby):
    /// removes a user from a lobby; removing one who already left succeeds too.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; an error for a lobby that does not exist.
    pub fn remove_lobby_member(
        &self,
        lobby_id: LobbyId,
        user_id: UserId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.request_ignoring_body(Request::new(Route::RemoveLobbyMember { lobby_id, user_id }))
                .await
        })
    }

    /// [Leave Lobby](https://docs.discord.com/developers/resources/lobby#leave-lobby): removes
    /// the player from a lobby. Needs the player's access token.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; an error for a lobby that does not exist.
    pub fn leave_lobby(
        &self,
        lobby_id: LobbyId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.request_ignoring_body(Request::new(Route::LeaveLobby { lobby_id }))
                .await
        })
    }
}

/// Linked channels and lobby invites.
impl Rest {
    /// [Link Channel to Lobby](https://docs.discord.com/developers/resources/lobby#link-channel-to-lobby):
    /// links a guild text channel to the lobby, so their messages are shared. Needs the access
    /// token of a member with [`CAN_LINK_LOBBY`](djs_model::lobby::LobbyMemberFlags::CAN_LINK_LOBBY).
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn link_channel_to_lobby(
        &self,
        lobby_id: LobbyId,
        channel_id: ChannelId,
    ) -> impl Future<Output = Result<Lobby, Error>> + Send + '_ {
        boxed(async move {
            let body = ChannelLinking {
                channel_id: Some(channel_id),
            };
            self.call(Request::new(Route::LinkChannelToLobby { lobby_id }).json(&body)?)
                .await
        })
    }

    /// [Unlink Channel from Lobby](https://docs.discord.com/developers/resources/lobby#unlink-channel-from-lobby):
    /// removes the lobby's linked channel. Needs the access token of a member with
    /// [`CAN_LINK_LOBBY`](djs_model::lobby::LobbyMemberFlags::CAN_LINK_LOBBY).
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn unlink_channel_from_lobby(
        &self,
        lobby_id: LobbyId,
    ) -> impl Future<Output = Result<Lobby, Error>> + Send + '_ {
        boxed(async move {
            let body = ChannelLinking { channel_id: None };
            self.call(Request::new(Route::UnlinkChannelFromLobby { lobby_id }).json(&body)?)
                .await
        })
    }

    /// [Create Lobby Channel Invite for Self](https://docs.discord.com/developers/resources/lobby#create-lobby-channel-invite-for-self):
    /// a single-use invite to the guild of the lobby's linked channel for the player, valid
    /// for one hour. Needs the player's access token with the `sdk.social_layer` scope.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; an error when the lobby has no linked channel.
    pub fn create_lobby_channel_invite_for_self(
        &self,
        lobby_id: LobbyId,
    ) -> impl Future<Output = Result<LobbyInvite, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::CreateLobbyChannelInviteForSelf {
                lobby_id,
            }))
            .await
        })
    }

    /// [Create Lobby Channel Invite for User](https://docs.discord.com/developers/resources/lobby#create-lobby-channel-invite-for-user):
    /// a single-use invite to the guild of the lobby's linked channel for a member, valid for
    /// one hour.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; an error when the lobby has no linked channel.
    pub fn create_lobby_channel_invite_for_user(
        &self,
        lobby_id: LobbyId,
        user_id: UserId,
    ) -> impl Future<Output = Result<LobbyInvite, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::CreateLobbyChannelInviteForUser {
                lobby_id,
                user_id,
            }))
            .await
        })
    }
}

/// Lobby messages.
impl Rest {
    /// [Send Lobby Message](https://docs.discord.com/developers/resources/lobby#send-lobby-message):
    /// sends a message to the lobby (and its linked channel). Needs the access token of a
    /// member with the `sdk.social_layer` scope.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn send_lobby_message<'a>(
        &'a self,
        lobby_id: LobbyId,
        body: &'a SendLobbyMessage,
    ) -> impl Future<Output = Result<LobbyMessage, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::SendLobbyMessage { lobby_id }).json(body)?)
                .await
        })
    }

    /// [Get Lobby Messages](https://docs.discord.com/developers/resources/lobby#get-lobby-messages):
    /// returns the lobby's most recent messages (50 by default). Needs the access token of a
    /// member with the `sdk.social_layer` scope.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_lobby_messages<'a>(
        &'a self,
        lobby_id: LobbyId,
        query: &'a GetLobbyMessages,
    ) -> impl Future<Output = Result<Vec<LobbyMessage>, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::GetLobbyMessages { lobby_id }).query(query))
                .await
        })
    }

    /// [Update Lobby Message Moderation Metadata](https://docs.discord.com/developers/resources/lobby#update-lobby-message-moderation-metadata):
    /// sets the app's moderation decision for a message (up to 5 keys of at most 1024
    /// characters, values of at most 2000), delivered to Social SDK clients as an update.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn update_lobby_message_moderation_metadata<'a>(
        &'a self,
        lobby_id: LobbyId,
        message_id: MessageId,
        metadata: &'a LobbyMetadata,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.request_ignoring_body(
                Request::new(Route::UpdateLobbyMessageModerationMetadata {
                    lobby_id,
                    message_id,
                })
                .json(metadata)?,
            )
            .await
        })
    }
}
