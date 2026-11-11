//! Bodies of the [webhook endpoints](https://docs.discord.com/developers/resources/webhook).
//!
//! Execute Slack-Compatible and GitHub-Compatible Webhook are not modelled (D-120).

use compact_str::CompactString;
use serde::{Serialize, Serializer};

use super::{Attachments, FileUpload};
use crate::{
    component::Component,
    id::{ChannelId, TagId},
    message::{AllowedMentions, AttachmentRequest, Embed, MessageFlags},
    poll::CreatePoll,
    util::{ImageData, Patch},
};

/// The body of [Create Webhook](https://docs.discord.com/developers/resources/webhook#create-webhook).
///
/// The name (1-80 characters) cannot contain `clyde` or `discord`.
///
/// discord.js counterpart: `ChannelWebhookCreateOptions`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct CreateWebhook {
    /// The default name of the webhook's messages.
    pub name: CompactString,
    /// The default avatar of the webhook's messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<ImageData>,
}

impl CreateWebhook {
    /// A webhook named `name`, with the default avatar.
    #[must_use]
    pub fn new(name: impl Into<CompactString>) -> Self {
        Self {
            name: name.into(),
            avatar: None,
        }
    }
}

/// The body of [Modify Webhook](https://docs.discord.com/developers/resources/webhook#modify-webhook)
/// and Modify Webhook with Token (which does not accept [`channel_id`](Self::channel_id)).
///
/// discord.js counterpart: `WebhookEditOptions`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyWebhook {
    /// The default name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The default avatar.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub avatar: Patch<ImageData>,
    /// The channel to move the webhook to (not with a token).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,
}

/// The body of [Execute Webhook](https://docs.discord.com/developers/resources/webhook#execute-webhook):
/// a message sent through a webhook.
///
/// At least one of `content`, `embeds`, `components`, `files` or `poll` is required. In a
/// forum or media channel, set [`thread_id`](Self::thread_id) or
/// [`thread_name`](Self::thread_name). Files get their `attachments` entries automatically
/// (D-094); [`thread_id`](Self::thread_id) and [`with_components`](Self::with_components)
/// are sent in the query string (D-119).
///
/// discord.js counterpart: `WebhookMessageCreateOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::rest::ExecuteWebhook;
///
/// let mut body = ExecuteWebhook::from("Build #42 passed");
/// body.username = Some("CI".into());
/// assert_eq!(
///     serde_json::to_string(&body).unwrap(),
///     r#"{"content":"Build #42 passed","username":"CI"}"#
/// );
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct ExecuteWebhook {
    /// The text (up to 2000 characters).
    pub content: Option<CompactString>,
    /// Overrides the webhook's name for this message.
    pub username: Option<CompactString>,
    /// Overrides the webhook's avatar for this message (an image URL).
    pub avatar_url: Option<CompactString>,
    /// Whether the message is read aloud.
    pub tts: bool,
    /// Up to 10 rich embeds.
    pub embeds: Vec<Embed>,
    /// Which mentions in the content notify.
    pub allowed_mentions: Option<AllowedMentions>,
    /// The components; webhooks not owned by an application need
    /// [`with_components`](Self::with_components) and can only send non-interactive ones.
    pub components: Vec<Component>,
    /// Explicit attachment entries; files get theirs automatically (D-094).
    pub attachments: Option<Vec<AttachmentRequest>>,
    /// Only [`SUPPRESS_EMBEDS`](MessageFlags::SUPPRESS_EMBEDS),
    /// [`SUPPRESS_NOTIFICATIONS`](MessageFlags::SUPPRESS_NOTIFICATIONS) and
    /// [`IS_COMPONENTS_V2`](MessageFlags::IS_COMPONENTS_V2) can be set.
    pub flags: Option<MessageFlags>,
    /// In a forum or media channel: creates a post with this name.
    pub thread_name: Option<CompactString>,
    /// The tags of the post created with [`thread_name`](Self::thread_name).
    pub applied_tags: Vec<TagId>,
    /// A poll to create with the message.
    pub poll: Option<CreatePoll>,
    /// Files to upload with the message (D-087).
    pub files: Vec<FileUpload>,
    /// Sends the message in this thread of the webhook's channel (unarchiving it); sent in
    /// the query string.
    pub thread_id: Option<ChannelId>,
    /// Whether Discord uses [`components`](Self::components) (Discord's default: only for
    /// application-owned webhooks); sent in the query string.
    pub with_components: Option<bool>,
}

impl Serialize for ExecuteWebhook {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            content: Option<&'a CompactString>,
            #[serde(skip_serializing_if = "Option::is_none")]
            username: Option<&'a CompactString>,
            #[serde(skip_serializing_if = "Option::is_none")]
            avatar_url: Option<&'a CompactString>,
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
            thread_name: Option<&'a CompactString>,
            #[serde(skip_serializing_if = "<[_]>::is_empty")]
            applied_tags: &'a [TagId],
            #[serde(skip_serializing_if = "Option::is_none")]
            poll: Option<&'a CreatePoll>,
        }

        // Destructured without `..`, so a new field cannot be forgotten here. The query
        // fields are written by the REST client.
        let Self {
            content,
            username,
            avatar_url,
            tts,
            embeds,
            allowed_mentions,
            components,
            attachments,
            flags,
            thread_name,
            applied_tags,
            poll,
            files,
            thread_id: _,
            with_components: _,
        } = self;
        Wire {
            content: content.as_ref(),
            username: username.as_ref(),
            avatar_url: avatar_url.as_ref(),
            tts: *tts,
            embeds,
            allowed_mentions: allowed_mentions.as_ref(),
            components,
            attachments: Attachments::new(attachments.as_deref(), files),
            flags: *flags,
            thread_name: thread_name.as_ref(),
            applied_tags,
            poll: poll.as_ref(),
        }
        .serialize(serializer)
    }
}

impl From<&str> for ExecuteWebhook {
    fn from(content: &str) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::default()
        }
    }
}

impl From<String> for ExecuteWebhook {
    fn from(content: String) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::default()
        }
    }
}

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
    /// The thread the message is in; sent in the query string (D-119).
    pub thread_id: Option<ChannelId>,
    /// Whether Discord uses [`components`](Self::components) (Discord's default: only for
    /// application-owned webhooks); sent in the query string.
    pub with_components: Option<bool>,
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

        // Destructured without `..`, so a new field cannot be forgotten here. The query
        // fields are written by the REST client.
        let Self {
            content,
            embeds,
            flags,
            allowed_mentions,
            components,
            attachments,
            poll,
            files,
            thread_id: _,
            with_components: _,
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
    fn execute_webhook_keeps_query_fields_out_of_the_json() {
        let mut body = ExecuteWebhook::from("hi");
        body.thread_id = Some(ChannelId::new(5));
        body.with_components = Some(true);
        body.thread_name = Some("Release".into());
        body.applied_tags.push(TagId::new(3));
        body.files.push(FileUpload::new("notes.md", "# 1.0"));
        assert_eq!(
            serde_json::to_string(&body).unwrap(),
            concat!(
                r#"{"content":"hi","attachments":[{"id":0,"filename":"notes.md"}],"#,
                r#""thread_name":"Release","applied_tags":["3"]}"#
            )
        );

        let modify = ModifyWebhook {
            avatar: Patch::Clear,
            ..ModifyWebhook::default()
        };
        assert_eq!(
            serde_json::to_string(&modify).unwrap(),
            r#"{"avatar":null}"#
        );
    }

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
