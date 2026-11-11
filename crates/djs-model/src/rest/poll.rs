//! Queries of the [poll endpoints](https://docs.discord.com/developers/resources/poll).
//!
//! Polls are created by sending a message with a
//! [`CreatePoll`](crate::poll::CreatePoll) in its `poll` field.

use crate::id::UserId;

/// The query of [Get Answer Voters](https://docs.discord.com/developers/resources/poll#get-answer-voters).
///
/// discord.js counterpart: the options of `PollAnswerVoterManager#fetch`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetAnswerVoters {
    /// Voters after this user.
    pub after: Option<UserId>,
    /// How many voters (1-100; Discord's default is 25).
    pub limit: Option<u8>,
}
