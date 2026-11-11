//! Bodies of the [interaction endpoints](https://docs.discord.com/developers/interactions/receiving-and-responding#endpoints).
//!
//! The initial response is an [`InteractionResponse`](crate::interaction::InteractionResponse);
//! edits of the original response and of follow-ups use
//! [`EditWebhookMessage`](super::EditWebhookMessage).

use compact_str::CompactString;
use serde::{Serialize, Serializer};

use super::{Attachments, FileUpload};
use crate::{
    component::Component,
    message::{AllowedMentions, AttachmentRequest, Embed, MessageFlags},
    poll::CreatePoll,
};

/// The body of [Create Followup Message](https://docs.discord.com/developers/interactions/receiving-and-responding#create-followup-message):
/// the Execute Webhook body without the fields follow-ups do not support (D-107).
///
/// Right after a deferred response (`DEFERRED_CHANNEL_MESSAGE_WITH_SOURCE`), the first
/// follow-up edits the "thinking…" message instead of creating a new one.
///
/// discord.js counterpart: `InteractionReplyOptions` of `followUp` /
/// `RESTPostAPIInteractionFollowupJSONBody`.
///
/// # Examples
///
/// ```
/// use djs_model::{message::MessageFlags, rest::CreateFollowupMessage};
///
/// let mut body = CreateFollowupMessage::from("Only you can see this.");
/// body.flags = Some(MessageFlags::EPHEMERAL);
/// assert_eq!(
///     serde_json::to_string(&body).unwrap(),
///     r#"{"content":"Only you can see this.","flags":64}"#
/// );
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct CreateFollowupMessage {
    /// The text (up to 2000 characters).
    pub content: Option<CompactString>,
    /// Whether the message is read aloud.
    pub tts: bool,
    /// Up to 10 rich embeds.
    pub embeds: Vec<Embed>,
    /// Which mentions in the content notify.
    pub allowed_mentions: Option<AllowedMentions>,
    /// The components.
    pub components: Vec<Component>,
    /// Explicit attachment entries; files get theirs automatically (D-094).
    pub attachments: Option<Vec<AttachmentRequest>>,
    /// Only [`EPHEMERAL`](MessageFlags::EPHEMERAL),
    /// [`SUPPRESS_EMBEDS`](MessageFlags::SUPPRESS_EMBEDS),
    /// [`SUPPRESS_NOTIFICATIONS`](MessageFlags::SUPPRESS_NOTIFICATIONS) and
    /// [`IS_COMPONENTS_V2`](MessageFlags::IS_COMPONENTS_V2) can be set.
    pub flags: Option<MessageFlags>,
    /// A poll to create with the message.
    pub poll: Option<CreatePoll>,
    /// Files to upload with the message (D-087).
    pub files: Vec<FileUpload>,
}

impl Serialize for CreateFollowupMessage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            content: Option<&'a CompactString>,
            #[serde(skip_serializing_if = "core::ops::Not::not")]
            tts: bool,
            #[serde(skip_serializing_if = "<[_]>::is_empty")]
            embeds: &'a [Embed],
            #[serde(skip_serializing_if = "Option::is_none")]
            allowed_mentions: Option<&'a AllowedMentions>,
            #[serde(skip_serializing_if = "<[_]>::is_empty")]
            components: &'a [Component],
            #[serde(skip_serializing_if = "Attachments::is_absent")]
            attachments: Attachments<'a>,
            #[serde(skip_serializing_if = "Option::is_none")]
            flags: Option<MessageFlags>,
            #[serde(skip_serializing_if = "Option::is_none")]
            poll: Option<&'a CreatePoll>,
        }

        // Destructured without `..`, so a new field cannot be forgotten here.
        let Self {
            content,
            tts,
            embeds,
            allowed_mentions,
            components,
            attachments,
            flags,
            poll,
            files,
        } = self;
        Wire {
            content: content.as_ref(),
            tts: *tts,
            embeds,
            allowed_mentions: allowed_mentions.as_ref(),
            components,
            attachments: Attachments::new(attachments.as_deref(), files),
            flags: *flags,
            poll: poll.as_ref(),
        }
        .serialize(serializer)
    }
}

impl From<&str> for CreateFollowupMessage {
    fn from(content: &str) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::default()
        }
    }
}

impl From<String> for CreateFollowupMessage {
    fn from(content: String) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::default()
        }
    }
}
