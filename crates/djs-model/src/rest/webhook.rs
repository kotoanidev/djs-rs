//! Bodies of the [webhook endpoints](https://docs.discord.com/developers/resources/webhook#create-webhook)
//! (more arrive with the webhook resource, D-102).

use compact_str::CompactString;
use serde::{Serialize, Serializer};

use super::{Attachments, FileUpload};
use crate::{
    component::Component,
    message::{AllowedMentions, AttachmentRequest, Embed, MessageFlags},
    poll::CreatePoll,
    util::Patch,
};

/// The body of [Edit Webhook Message](https://docs.discord.com/developers/resources/webhook#edit-webhook-message),
/// also used to edit the original response to an interaction and its follow-up messages.
///
/// Every field is a [`Patch`]: fields left at [`Patch::Keep`] are not changed. Attachments
/// work like in [`EditMessage`](super::EditMessage): new [`files`](Self::files) are added to
/// the attachments listed in [`attachments`](Self::attachments), every other attachment is
/// removed, and [`Patch::Clear`] removes all of them.
///
/// discord.js counterpart: `WebhookMessageEditOptions` / `InteractionEditReplyOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::{rest::EditWebhookMessage, util::Patch};
///
/// let mut body = EditWebhookMessage::default();
/// body.content = Patch::Set("Done!".into());
/// assert_eq!(serde_json::to_string(&body).unwrap(), r#"{"content":"Done!"}"#);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct EditWebhookMessage {
    /// The text (up to 2000 characters).
    pub content: Patch<CompactString>,
    /// Up to 10 rich embeds.
    pub embeds: Patch<Vec<Embed>>,
    /// Only [`SUPPRESS_EMBEDS`](MessageFlags::SUPPRESS_EMBEDS) can be set and unset;
    /// [`IS_COMPONENTS_V2`](MessageFlags::IS_COMPONENTS_V2) can only be set.
    pub flags: Patch<MessageFlags>,
    /// Which mentions in the content notify.
    pub allowed_mentions: Patch<AllowedMentions>,
    /// The components.
    pub components: Patch<Vec<Component>>,
    /// The attachments to keep (and the metadata of new files, which is otherwise generated).
    pub attachments: Patch<Vec<AttachmentRequest>>,
    /// A poll; it can only be added when editing a deferred interaction response.
    pub poll: Patch<CreatePoll>,
    /// Files to add (D-087).
    pub files: Vec<FileUpload>,
}

impl Serialize for EditWebhookMessage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            #[serde(skip_serializing_if = "Patch::is_keep")]
            content: Patch<&'a CompactString>,
            #[serde(skip_serializing_if = "Patch::is_keep")]
            embeds: Patch<&'a Vec<Embed>>,
            #[serde(skip_serializing_if = "Patch::is_keep")]
            flags: Patch<MessageFlags>,
            #[serde(skip_serializing_if = "Patch::is_keep")]
            allowed_mentions: Patch<&'a AllowedMentions>,
            #[serde(skip_serializing_if = "Patch::is_keep")]
            components: Patch<&'a Vec<Component>>,
            #[serde(skip_serializing_if = "Attachments::is_absent")]
            attachments: Attachments<'a>,
            #[serde(skip_serializing_if = "Patch::is_keep")]
            poll: Patch<&'a CreatePoll>,
        }

        // Destructured without `..`, so a new field cannot be forgotten here.
        let Self {
            content,
            embeds,
            flags,
            allowed_mentions,
            components,
            attachments,
            poll,
            files,
        } = self;
        Wire {
            content: content.as_ref(),
            embeds: embeds.as_ref(),
            flags: *flags,
            allowed_mentions: allowed_mentions.as_ref(),
            components: components.as_ref(),
            attachments: Attachments::from_patch(attachments, files),
            poll: poll.as_ref(),
        }
        .serialize(serializer)
    }
}

impl From<&str> for EditWebhookMessage {
    /// Replaces the content.
    fn from(content: &str) -> Self {
        Self {
            content: Patch::Set(content.into()),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_webhook_message() {
        let body = EditWebhookMessage {
            embeds: Patch::Clear,
            poll: Patch::Clear,
            files: vec![FileUpload::new("a.png", &b"a"[..])],
            ..EditWebhookMessage::from("x")
        };
        assert_eq!(
            serde_json::to_string(&body).unwrap(),
            r#"{"content":"x","embeds":null,"attachments":[{"id":0,"filename":"a.png"}],"poll":null}"#
        );
    }
}
