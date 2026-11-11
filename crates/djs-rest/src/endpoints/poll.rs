//! [Poll endpoints](https://docs.discord.com/developers/resources/poll): the voters of an
//! answer and ending a poll early.
//!
//! Polls are created with [`create_message`](Rest::create_message), setting
//! [`CreateMessage::poll`](djs_model::rest::CreateMessage::poll).

use djs_model::{
    id::{ChannelId, MessageId},
    message::Message,
    rest::GetAnswerVoters,
    user::User,
};
use serde::Deserialize;

use crate::{Error, Request, Rest, Route};

/// The response of Get Answer Voters, unwrapped by the method (C-060).
#[derive(Deserialize)]
struct Voters {
    users: Vec<User>,
}

impl Rest {
    /// [Get Answer Voters](https://docs.discord.com/developers/resources/poll#get-answer-voters):
    /// returns a page of the users who voted for an answer, sorted by user ID (25 by
    /// default).
    ///
    /// `answer_id` is the answer's [`answer_id`](djs_model::poll::Answer::answer_id).
    ///
    /// discord.js counterpart: `PollAnswerVoterManager#fetch` (`PollAnswer#voters`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, channel_id: djs_model::id::ChannelId, message_id: djs_model::id::MessageId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::GetAnswerVoters;
    ///
    /// let mut query = GetAnswerVoters::default();
    /// query.limit = Some(100);
    /// let voters = rest.get_answer_voters(channel_id, message_id, 1, &query).await?;
    /// println!("{} users voted for the first answer", voters.len());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10008` Unknown Message.
    pub async fn get_answer_voters(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        answer_id: u32,
        query: &GetAnswerVoters,
    ) -> Result<Vec<User>, Error> {
        let voters: Voters = self
            .request(
                Request::new(Route::GetAnswerVoters {
                    channel_id,
                    message_id,
                    answer_id,
                })
                .query(query),
            )
            .await?;
        Ok(voters.users)
    }

    /// [End Poll](https://docs.discord.com/developers/resources/poll#end-poll): ends a poll
    /// now and returns its message. Only polls the bot created can be ended.
    ///
    /// discord.js counterpart: `Poll#end`, `MessageManager#endPoll`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10008` Unknown Message; `520001` for a poll that has already
    /// ended.
    pub async fn end_poll(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<Message, Error> {
        self.request(Request::new(Route::EndPoll {
            channel_id,
            message_id,
        }))
        .await
    }
}
