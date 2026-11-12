//! Bodies of the [voice endpoints](https://docs.discord.com/developers/resources/voice).
//!
//! Both bodies only work in stage channels the user has already joined.

use serde::Serialize;

use crate::{
    id::ChannelId,
    util::{Patch, Timestamp},
};

/// The body of [Modify Current User Voice State](https://docs.discord.com/developers/resources/voice#modify-current-user-voice-state).
///
/// Every field is optional. Unsuppressing needs `MUTE_MEMBERS` and requesting to speak needs
/// `REQUEST_TO_SPEAK`; suppressing oneself and withdrawing a request always work.
///
/// discord.js counterpart: the argument of `VoiceState#edit` for the client user
/// (`setSuppressed`, `setRequestToSpeak`).
///
/// # Examples
///
/// ```
/// use djs_model::{
///     id::ChannelId,
///     rest::ModifyCurrentUserVoiceState,
///     util::{Patch, Timestamp},
/// };
///
/// let mut body = ModifyCurrentUserVoiceState::default();
/// body.channel_id = Some(ChannelId::new(5));
/// body.request_to_speak_timestamp = Patch::Set(Timestamp::parse("2026-11-12T18:00:00Z")?);
/// assert_eq!(
///     serde_json::to_string(&body)?,
///     r#"{"channel_id":"5","request_to_speak_timestamp":"2026-11-12T18:00:00.000000+00:00"}"#
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyCurrentUserVoiceState {
    /// The stage channel the bot is in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,
    /// Whether the bot is suppressed (an audience member rather than a speaker).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress: Option<bool>,
    /// Requests to speak (any present or future time); [`Patch::Clear`] withdraws the request.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub request_to_speak_timestamp: Patch<Timestamp>,
}

/// The body of [Modify User Voice State](https://docs.discord.com/developers/resources/voice#modify-user-voice-state).
///
/// Every field is optional; needs `MUTE_MEMBERS`. Unsuppressing a user who is not a bot also
/// sets their request to speak to now; suppressing removes it.
///
/// discord.js counterpart: the argument of `VoiceState#edit` for other members
/// (`setSuppressed`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyUserVoiceState {
    /// The stage channel the user is in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,
    /// Whether the user is suppressed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress: Option<bool>,
}
