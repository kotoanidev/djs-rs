//! [Message endpoints](https://docs.discord.com/developers/resources/message#get-channel-messages):
//! messages, reactions and pins.

use djs_model::{
    id::{ChannelId, GuildId, MessageId, UserId},
    message::Message,
    rest::{
        BulkDeleteMessages, ChannelPins, CreateMessage, EditMessage, GetChannelMessages,
        GetChannelPins, GetReactions, MessageSearchResults, SearchGuildMessages,
    },
    user::User,
};
use http::StatusCode;

use crate::{
    Error, ErrorKind, ReactionEmoji, Request, Rest, Route, client::boxed, error::not_yet_indexed,
};

impl Rest {
    /// [Get Channel Messages](https://docs.discord.com/developers/resources/message#get-channel-messages):
    /// returns messages of a channel, newest first.
    ///
    /// Needs `VIEW_CHANNEL` (and `CONNECT` in voice channels); without
    /// `READ_MESSAGE_HISTORY` the list is empty.
    ///
    /// discord.js counterpart: `MessageManager#fetch` with options.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, channel_id: djs_model::id::ChannelId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::GetChannelMessages;
    ///
    /// let mut query = GetChannelMessages::default();
    /// query.limit = Some(10);
    /// for message in rest.get_channel_messages(channel_id, &query).await? {
    ///     println!("{}: {}", message.author.username, message.content);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_channel_messages<'a>(
        &'a self,
        channel_id: ChannelId,
        query: &'a GetChannelMessages,
    ) -> impl Future<Output = Result<Vec<Message>, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::GetChannelMessages { channel_id }).query(query))
                .await
        })
    }

    /// [Search Guild Messages](https://docs.discord.com/developers/resources/message#search-guild-messages):
    /// searches the messages of a guild (without their reactions).
    ///
    /// Needs `READ_MESSAGE_HISTORY`, and depends on the `MESSAGE_CONTENT` privileged intent.
    /// The search may return fewer messages than the limit, so page with `offset` or the IDs
    /// rather than by counting.
    ///
    /// discord.js counterpart: none (discord.js does not wrap this endpoint).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::SearchGuildMessages;
    /// use djs_rest::ErrorKind;
    ///
    /// let mut query = SearchGuildMessages::default();
    /// query.content = Some("release notes".into());
    /// let results = loop {
    ///     match rest.search_guild_messages(guild_id, &query).await {
    ///         Err(error) => match error.kind() {
    ///             ErrorKind::NotYetIndexed { retry_after, .. } => {
    ///                 tokio::time::sleep((*retry_after).max(std::time::Duration::from_secs(1))).await;
    ///             }
    ///             _ => return Err(error),
    ///         },
    ///         Ok(results) => break results,
    ///     }
    /// };
    /// println!("{} matching messages", results.total_results);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// [`ErrorKind::NotYetIndexed`] while the guild is not indexed yet (D-104); otherwise see
    /// [`Rest::request`].
    pub fn search_guild_messages<'a>(
        &'a self,
        guild_id: GuildId,
        query: &'a SearchGuildMessages,
    ) -> impl Future<Output = Result<MessageSearchResults, Error>> + Send + 'a {
        boxed(async move {
            let request = Request::new(Route::SearchGuildMessages { guild_id }).query(query);
            let (status, body) = self.execute(request).await?;
            if status == StatusCode::ACCEPTED {
                return Err(not_yet_indexed(&body));
            }
            serde_json::from_slice(&body)
                .map_err(|source| Error::with_source(ErrorKind::Deserialize, source))
        })
    }

    /// [Get Channel Message](https://docs.discord.com/developers/resources/message#get-channel-message):
    /// returns one message.
    ///
    /// Needs `VIEW_CHANNEL` and `READ_MESSAGE_HISTORY` (and `CONNECT` in voice channels).
    ///
    /// discord.js counterpart: `MessageManager#fetch` with an ID.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10008` Unknown Message if it does not exist.
    pub fn get_channel_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> impl Future<Output = Result<Message, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetChannelMessage {
                channel_id,
                message_id,
            }))
            .await
        })
    }

    /// [Create Message](https://docs.discord.com/developers/resources/message#create-message):
    /// sends a message to a channel and returns it. With files, the request is sent as
    /// `multipart/form-data`.
    ///
    /// Requires the `SEND_MESSAGES` permission in guild channels (and
    /// `SEND_MESSAGES_IN_THREADS` in threads); replies need `READ_MESSAGE_HISTORY`, TTS
    /// messages `SEND_TTS_MESSAGES`.
    ///
    /// discord.js counterpart: `TextChannel#send`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]. Common API errors: `50001` Missing Access, `50013` Missing
    /// Permissions, `50035` Invalid Form Body.
    pub fn create_message<'a>(
        &'a self,
        channel_id: ChannelId,
        body: &'a CreateMessage,
    ) -> impl Future<Output = Result<Message, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::CreateMessage { channel_id })
                    .json(body)?
                    .files(&body.files),
            )
            .await
        })
    }

    /// [Crosspost Message](https://docs.discord.com/developers/resources/message#crosspost-message):
    /// publishes a message of an announcement channel to the channels following it.
    ///
    /// Needs `SEND_MESSAGES` for the bot's own messages, plus `MANAGE_MESSAGES` for others'.
    ///
    /// discord.js counterpart: `Message#crosspost`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `40033` if the message was already published.
    pub fn crosspost_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> impl Future<Output = Result<Message, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::CrosspostMessage {
                channel_id,
                message_id,
            }))
            .await
        })
    }

    /// [Create Reaction](https://docs.discord.com/developers/resources/message#create-reaction):
    /// reacts to a message as the bot.
    ///
    /// Needs `READ_MESSAGE_HISTORY`, and `ADD_REACTIONS` when nobody has reacted with this
    /// emoji yet.
    ///
    /// discord.js counterpart: `Message#react`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, channel_id: djs_model::id::ChannelId, message_id: djs_model::id::MessageId) -> Result<(), djs_rest::Error> {
    /// use djs_rest::ReactionEmoji;
    ///
    /// rest.create_reaction(channel_id, message_id, ReactionEmoji::Unicode("👍"))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10014` Unknown Emoji for an emoji the bot cannot use.
    pub fn create_reaction<'a>(
        &'a self,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: ReactionEmoji<'a>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::CreateReaction {
                channel_id,
                message_id,
                emoji,
            }))
            .await
        })
    }

    /// [Delete Own Reaction](https://docs.discord.com/developers/resources/message#delete-own-reaction):
    /// removes the bot's reaction.
    ///
    /// discord.js counterpart: `ReactionUserManager#remove` for the client user.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn delete_own_reaction<'a>(
        &'a self,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: ReactionEmoji<'a>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::DeleteOwnReaction {
                channel_id,
                message_id,
                emoji,
            }))
            .await
        })
    }

    /// [Delete User Reaction](https://docs.discord.com/developers/resources/message#delete-user-reaction):
    /// removes another user's reaction. Needs `MANAGE_MESSAGES`.
    ///
    /// discord.js counterpart: `ReactionUserManager#remove`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn delete_user_reaction<'a>(
        &'a self,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: ReactionEmoji<'a>,
        user_id: UserId,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::DeleteUserReaction {
                channel_id,
                message_id,
                emoji,
                user_id,
            }))
            .await
        })
    }

    /// [Get Reactions](https://docs.discord.com/developers/resources/message#get-reactions):
    /// returns the users who reacted with an emoji.
    ///
    /// discord.js counterpart: `ReactionUserManager#fetch`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_reactions<'a>(
        &'a self,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: ReactionEmoji<'a>,
        query: &'a GetReactions,
    ) -> impl Future<Output = Result<Vec<User>, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::GetReactions {
                    channel_id,
                    message_id,
                    emoji,
                })
                .query(query),
            )
            .await
        })
    }

    /// [Delete All Reactions](https://docs.discord.com/developers/resources/message#delete-all-reactions):
    /// removes every reaction of a message. Needs `MANAGE_MESSAGES`.
    ///
    /// discord.js counterpart: `ReactionManager#removeAll`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn delete_all_reactions(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::DeleteAllReactions {
                channel_id,
                message_id,
            }))
            .await
        })
    }

    /// [Delete All Reactions for Emoji](https://docs.discord.com/developers/resources/message#delete-all-reactions-for-emoji):
    /// removes every reaction with one emoji. Needs `MANAGE_MESSAGES`.
    ///
    /// discord.js counterpart: `MessageReaction#remove`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn delete_all_reactions_for_emoji<'a>(
        &'a self,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: ReactionEmoji<'a>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::DeleteAllReactionsForEmoji {
                channel_id,
                message_id,
                emoji,
            }))
            .await
        })
    }

    /// [Edit Message](https://docs.discord.com/developers/resources/message#edit-message):
    /// changes a message and returns it. With files, the request is sent as
    /// `multipart/form-data`; see [`EditMessage`] for how attachments are kept.
    ///
    /// The author can edit everything; others with `MANAGE_MESSAGES` only the flags.
    ///
    /// discord.js counterpart: `Message#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50005` when editing someone else's content.
    pub fn edit_message<'a>(
        &'a self,
        channel_id: ChannelId,
        message_id: MessageId,
        body: &'a EditMessage,
    ) -> impl Future<Output = Result<Message, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::EditMessage {
                    channel_id,
                    message_id,
                })
                .json(body)?
                .files(&body.files),
            )
            .await
        })
    }

    /// [Delete Message](https://docs.discord.com/developers/resources/message#delete-message):
    /// deletes a message; others' messages need `MANAGE_MESSAGES` in guild channels.
    ///
    /// Messages older than two weeks are deleted under a separate, stricter rate limit, which
    /// the client tracks on its own.
    ///
    /// discord.js counterpart: `Message#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn delete_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::DeleteMessage {
                channel_id,
                message_id,
            }))
            .await
        })
    }

    /// [Bulk Delete Messages](https://docs.discord.com/developers/resources/message#bulk-delete-messages):
    /// deletes 2 to 100 messages at once (guild channels only, none older than two weeks).
    /// Needs `MANAGE_MESSAGES`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `TextChannel#bulkDelete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50034` if a message is too old, `50016` for fewer than 2 or
    /// more than 100 messages.
    pub fn bulk_delete_messages<'a>(
        &'a self,
        channel_id: ChannelId,
        body: &'a BulkDeleteMessages,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::BulkDeleteMessages { channel_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Get Channel Pins](https://docs.discord.com/developers/resources/message#get-channel-pins):
    /// returns a page of the pinned messages, newest first.
    ///
    /// Needs `VIEW_CHANNEL`; without `READ_MESSAGE_HISTORY` the list is empty.
    ///
    /// discord.js counterpart: `MessageManager#fetchPins`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_channel_pins<'a>(
        &'a self,
        channel_id: ChannelId,
        query: &'a GetChannelPins,
    ) -> impl Future<Output = Result<ChannelPins, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::GetChannelPins { channel_id }).query(query))
                .await
        })
    }

    /// [Pin Message](https://docs.discord.com/developers/resources/message#pin-message):
    /// pins a message. Needs `PIN_MESSAGES`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Message#pin`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30003` when the channel has the maximum number of pins.
    pub fn pin_message<'a>(
        &'a self,
        channel_id: ChannelId,
        message_id: MessageId,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::PinMessage {
                    channel_id,
                    message_id,
                })
                .reason(reason),
            )
            .await
        })
    }

    /// [Unpin Message](https://docs.discord.com/developers/resources/message#unpin-message):
    /// unpins a message. Needs `PIN_MESSAGES`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Message#unpin`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn unpin_message<'a>(
        &'a self,
        channel_id: ChannelId,
        message_id: MessageId,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::UnpinMessage {
                    channel_id,
                    message_id,
                })
                .reason(reason),
            )
            .await
        })
    }
}
