//! [Polls](https://docs.discord.com/developers/resources/poll) attached to messages.
//!
//! A poll lives on its [message](crate::message::Message): it has a question, up to 10
//! answers and, once people vote, [results](Results). The request body that creates a poll
//! belongs to the REST layer.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{emoji::PartialEmoji, macros::int_enum, util::Timestamp};

/// A [poll](https://docs.discord.com/developers/resources/poll#poll-object).
///
/// discord.js counterpart: `Poll`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Poll {
    /// The question (only [`text`](Media::text) is supported).
    pub question: Media,
    /// The answers to choose from.
    pub answers: Vec<Answer>,
    /// When the poll ends. Discord documents it as nullable for polls that may never expire,
    /// although currently every poll has an expiry.
    pub expiry: Option<Timestamp>,
    /// Whether a user can choose several answers.
    pub allow_multiselect: bool,
    /// How the poll is displayed.
    pub layout_type: LayoutType,
    /// The vote counts. `None` means *unknown* results, not *no* votes: some responses omit
    /// them because Discord did not fetch them (results received earlier stay valid). Ended
    /// polls always have results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Results>,
}

int_enum! {
    /// The [layout of a poll](https://docs.discord.com/developers/resources/poll#layout-type).
    ///
    /// discord.js counterpart: `PollLayoutType`.
    #[doc(alias = "PollLayoutType")]
    #[derive(Default)]
    pub enum LayoutType: u8 {
        /// The default layout.
        #[default]
        Default = 1,
    }
}

/// The [text and/or emoji](https://docs.discord.com/developers/resources/poll#poll-media-object)
/// of a poll question or answer.
///
/// discord.js counterpart: `PollQuestionMedia` / `PollAnswer` (text and emoji).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "PollMedia")]
pub struct Media {
    /// The text (questions: max 300 characters, answers: max 55 characters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<CompactString>,
    /// The emoji (answers only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<PartialEmoji>,
}

/// An [answer of a poll](https://docs.discord.com/developers/resources/poll#poll-answer-object).
///
/// discord.js counterpart: `PollAnswer`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "PollAnswer")]
pub struct Answer {
    /// The answer's ID: currently `1`, `2`, … in order, but treat it as opaque.
    pub answer_id: u32,
    /// The answer's text and/or emoji.
    pub poll_media: Media,
}

/// The [results of a poll](https://docs.discord.com/developers/resources/poll#poll-results-object).
///
/// While a poll is running, the counts are usually but not always accurate; after it ends, a
/// final tally runs, and once [`is_finalized`](Self::is_finalized) is `true` they are exact. An
/// answer without an entry in [`answer_counts`](Self::answer_counts) has no votes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "PollResults")]
pub struct Results {
    /// Whether the votes have been precisely counted.
    pub is_finalized: bool,
    /// The counts per answer.
    pub answer_counts: Vec<AnswerCount>,
}

/// The [vote count of one answer](https://docs.discord.com/developers/resources/poll#poll-results-object-poll-answer-count-object-structure).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "PollAnswerCount")]
pub struct AnswerCount {
    /// The [`Answer::answer_id`].
    pub id: u32,
    /// The number of votes.
    pub count: u32,
    /// Whether the current user voted for this answer.
    pub me_voted: bool,
}

/// A [poll to create](https://docs.discord.com/developers/resources/poll#poll-create-request-object),
/// sent with a new message or an interaction response.
///
/// discord.js counterpart: `PollData` (the `poll` option of message create options).
///
/// # Examples
///
/// ```
/// use djs_model::poll::{CreatePoll, Media};
///
/// let mut poll = CreatePoll::new("Tabs or spaces?", [Media::new("Tabs"), Media::new("Spaces")]);
/// poll.duration = Some(24);
/// assert_eq!(
///     serde_json::to_string(&poll).unwrap(),
///     concat!(
///         r#"{"question":{"text":"Tabs or spaces?"},"answers":[{"poll_media":{"text":"Tabs"}},"#,
///         r#"{"poll_media":{"text":"Spaces"}}],"duration":24}"#,
///     )
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "PollCreateRequest")]
pub struct CreatePoll {
    /// The question (text only, max 300 characters).
    pub question: Media,
    /// The answers (up to 10; text max 55 characters, optionally an emoji), written as answer
    /// objects without IDs.
    #[serde(with = "create_answers")]
    pub answers: Vec<Media>,
    /// How long the poll is open, in hours (up to 32 days; Discord's default is 24 hours).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    /// Whether users can select several answers (Discord's default: `false`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_multiselect: Option<bool>,
    /// The layout (Discord's default: [`LayoutType::Default`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_type: Option<LayoutType>,
}

impl CreatePoll {
    /// Creates a poll with Discord's defaults for everything else.
    #[must_use]
    pub fn new(
        question: impl Into<CompactString>,
        answers: impl IntoIterator<Item = Media>,
    ) -> Self {
        Self {
            question: Media::new(question),
            answers: answers.into_iter().collect(),
            duration: None,
            allow_multiselect: None,
            layout_type: None,
        }
    }
}

impl Media {
    /// Creates media with text and no emoji (set [`emoji`](Self::emoji) for an answer emoji).
    #[must_use]
    pub fn new(text: impl Into<CompactString>) -> Self {
        Self {
            text: Some(text.into()),
            emoji: None,
        }
    }
}

/// (De)serializes the answers of a [`CreatePoll`] as `[{"poll_media": …}]`.
mod create_answers {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeSeq};

    use super::Media;

    #[derive(Serialize)]
    struct AnswerRef<'a> {
        poll_media: &'a Media,
    }

    #[derive(Deserialize)]
    struct Answer {
        poll_media: Media,
    }

    pub(super) fn serialize<S: Serializer>(
        answers: &[Media],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(answers.len()))?;
        for poll_media in answers {
            seq.serialize_element(&AnswerRef { poll_media })?;
        }
        seq.end()
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<Media>, D::Error> {
        let answers = Vec::<Answer>::deserialize(deserializer)?;
        Ok(answers
            .into_iter()
            .map(|answer| answer.poll_media)
            .collect())
    }
}
