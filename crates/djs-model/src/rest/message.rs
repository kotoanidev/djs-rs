//! Bodies, query parameters and responses of the
//! [message endpoints](https://docs.discord.com/developers/resources/message#get-channel-messages).

use compact_str::CompactString;
use serde::{Deserialize, Serialize, Serializer};

use super::{Attachments, FileUpload};
use crate::{
    component::Component,
    id::{MessageId, StickerId, UserId},
    message::{
        AllowedMentions, AttachmentRequest, Embed, MessageFlags, MessagePin, MessageReference,
        Nonce, ReactionType, SharedClientTheme,
    },
    poll::CreatePoll,
    util::{Patch, Timestamp},
};

/// The body of [Create Message](https://docs.discord.com/developers/resources/message#create-message)
/// (`POST /channels/{channel.id}/messages`).
///
/// At least one of `content`, `embeds`, `sticker_ids`, `components`, `files`, `poll` or
/// `shared_client_theme` must be set; a forward needs only `message_reference`. With
/// [`MessageFlags::IS_COMPONENTS_V2`], only `components` (and files) may be set.
///
/// Every file in [`files`](Self::files) gets its `attachments` entry automatically (D-094),
/// and the request is then sent as `multipart/form-data`.
///
/// discord.js counterpart: `MessageCreateOptions` / `RESTPostAPIChannelMessageJSONBody`.
///
/// # Examples
///
/// ```
/// use djs_model::rest::{CreateMessage, FileUpload};
///
/// let body = CreateMessage::from("Pong!");
/// assert_eq!(serde_json::to_string(&body).unwrap(), r#"{"content":"Pong!"}"#);
///
/// let mut body = CreateMessage::from("A log");
/// body.files.push(FileUpload::new("log.txt", "line 1\nline 2\n"));
/// assert_eq!(
///     serde_json::to_string(&body).unwrap(),
///     r#"{"content":"A log","attachments":[{"id":0,"filename":"log.txt"}]}"#
/// );
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct CreateMessage {
    /// The text (up to 2000 characters).
    pub content: Option<CompactString>,
    /// A value to recognize the message in its `MESSAGE_CREATE` event (up to 25 characters).
    pub nonce: Option<Nonce>,
    /// Whether the message is read aloud (needs `SEND_TTS_MESSAGES`).
    pub tts: bool,
    /// Up to 10 rich embeds (up to 6000 characters in total).
    pub embeds: Vec<Embed>,
    /// Which mentions in the content notify.
    pub allowed_mentions: Option<AllowedMentions>,
    /// The message to reply to or forward.
    pub message_reference: Option<MessageReference>,
    /// The components.
    pub components: Vec<Component>,
    /// Up to 3 stickers of the guild to send.
    pub sticker_ids: Vec<StickerId>,
    /// Explicit attachment entries. Usually left out: every file gets one automatically,
    /// unless an entry with its `files[n]` index is listed here.
    pub attachments: Option<Vec<AttachmentRequest>>,
    /// Only [`SUPPRESS_EMBEDS`](MessageFlags::SUPPRESS_EMBEDS),
    /// [`SUPPRESS_NOTIFICATIONS`](MessageFlags::SUPPRESS_NOTIFICATIONS),
    /// [`IS_VOICE_MESSAGE`](MessageFlags::IS_VOICE_MESSAGE) and
    /// [`IS_COMPONENTS_V2`](MessageFlags::IS_COMPONENTS_V2) can be set.
    pub flags: Option<MessageFlags>,
    /// With a [`nonce`](Self::nonce): if the same author sent a message with the same nonce in
    /// the last few minutes, that message is returned instead of creating a new one.
    pub enforce_nonce: bool,
    /// A poll to create with the message.
    pub poll: Option<CreatePoll>,
    /// A client theme to share through the message.
    pub shared_client_theme: Option<SharedClientTheme>,
    /// Files to upload with the message (D-087).
    pub files: Vec<FileUpload>,
}

impl Serialize for CreateMessage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            content: Option<&'a CompactString>,
            #[serde(skip_serializing_if = "Option::is_none")]
            nonce: Option<&'a Nonce>,
            #[serde(skip_serializing_if = "core::ops::Not::not")]
            tts: bool,
            #[serde(skip_serializing_if = "<[_]>::is_empty")]
            embeds: &'a [Embed],
            #[serde(skip_serializing_if = "Option::is_none")]
            allowed_mentions: Option<&'a AllowedMentions>,
            #[serde(skip_serializing_if = "Option::is_none")]
            message_reference: Option<&'a MessageReference>,
            #[serde(skip_serializing_if = "<[_]>::is_empty")]
            components: &'a [Component],
            #[serde(skip_serializing_if = "<[_]>::is_empty")]
            sticker_ids: &'a [StickerId],
            #[serde(skip_serializing_if = "Attachments::is_absent")]
            attachments: Attachments<'a>,
            #[serde(skip_serializing_if = "Option::is_none")]
            flags: Option<MessageFlags>,
            #[serde(skip_serializing_if = "core::ops::Not::not")]
            enforce_nonce: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            poll: Option<&'a CreatePoll>,
            #[serde(skip_serializing_if = "Option::is_none")]
            shared_client_theme: Option<&'a SharedClientTheme>,
        }

        // Destructured without `..`, so a new field cannot be forgotten here.
        let Self {
            content,
            nonce,
            tts,
            embeds,
            allowed_mentions,
            message_reference,
            components,
            sticker_ids,
            attachments,
            flags,
            enforce_nonce,
            poll,
            shared_client_theme,
            files,
        } = self;
        Wire {
            content: content.as_ref(),
            nonce: nonce.as_ref(),
            tts: *tts,
            embeds,
            allowed_mentions: allowed_mentions.as_ref(),
            message_reference: message_reference.as_ref(),
            components,
            sticker_ids,
            attachments: Attachments::new(attachments.as_deref(), files),
            flags: *flags,
            enforce_nonce: *enforce_nonce,
            poll: poll.as_ref(),
            shared_client_theme: shared_client_theme.as_ref(),
        }
        .serialize(serializer)
    }
}

impl From<&str> for CreateMessage {
    fn from(content: &str) -> Self {
        Self::from(CompactString::from(content))
    }
}

impl From<String> for CreateMessage {
    fn from(content: String) -> Self {
        Self::from(CompactString::from(content))
    }
}

impl From<CompactString> for CreateMessage {
    fn from(content: CompactString) -> Self {
        Self {
            content: Some(content),
            ..Self::default()
        }
    }
}

/// The body of [Edit Message](https://docs.discord.com/developers/resources/message#edit-message)
/// (`PATCH /channels/{channel.id}/messages/{message.id}`).
///
/// Every field is a [`Patch`]: fields left at [`Patch::Keep`] are not changed. The author can
/// edit everything; other users with `MANAGE_MESSAGES` only the flags.
///
/// **Attachments:** new [`files`](Self::files) are added to the attachments listed in
/// [`attachments`](Self::attachments), and every attachment that is in neither is removed
/// (API v10). To keep the existing ones, list them with [`AttachmentRequest::existing`];
/// [`Patch::Clear`] removes all of them (sent as an empty list).
///
/// discord.js counterpart: `MessageEditOptions` / `RESTPatchAPIChannelMessageJSONBody`.
///
/// # Examples
///
/// ```
/// use djs_model::{rest::EditMessage, util::Patch};
///
/// let mut body = EditMessage::default();
/// body.content = Patch::Set("Edited".into());
/// body.embeds = Patch::Clear;
/// assert_eq!(
///     serde_json::to_string(&body).unwrap(),
///     r#"{"content":"Edited","embeds":null}"#
/// );
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct EditMessage {
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
    /// Files to add (D-087).
    pub files: Vec<FileUpload>,
}

impl Serialize for EditMessage {
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
        }

        // Destructured without `..`, so a new field cannot be forgotten here.
        let Self {
            content,
            embeds,
            flags,
            allowed_mentions,
            components,
            attachments,
            files,
        } = self;
        let explicit = match attachments {
            Patch::Keep => None,
            Patch::Clear => Some(&[][..]),
            Patch::Set(attachments) => Some(attachments.as_slice()),
        };
        Wire {
            content: content.as_ref(),
            embeds: embeds.as_ref(),
            flags: *flags,
            allowed_mentions: allowed_mentions.as_ref(),
            components: components.as_ref(),
            attachments: Attachments::new(explicit, files),
        }
        .serialize(serializer)
    }
}

/// The query of [Get Channel Messages](https://docs.discord.com/developers/resources/message#get-channel-messages)
/// (`GET /channels/{channel.id}/messages`). Messages are returned newest first.
///
/// `around`, `before` and `after` are mutually exclusive: set at most one.
///
/// discord.js counterpart: `FetchMessagesOptions`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetChannelMessages {
    /// Messages around this message.
    pub around: Option<MessageId>,
    /// Messages before this message.
    pub before: Option<MessageId>,
    /// Messages after this message.
    pub after: Option<MessageId>,
    /// How many messages (1-100; Discord's default is 50).
    pub limit: Option<u8>,
}

/// The query of [Get Reactions](https://docs.discord.com/developers/resources/message#get-reactions)
/// (`GET /channels/{channel.id}/messages/{message.id}/reactions/{emoji}`).
///
/// discord.js counterpart: `ReactionManager#fetch` options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetReactions {
    /// Normal or super reactions (Discord's default: normal).
    pub kind: Option<ReactionType>,
    /// Users after this user.
    pub after: Option<UserId>,
    /// How many users (1-100; Discord's default is 25).
    pub limit: Option<u8>,
}

/// The body of [Bulk Delete Messages](https://docs.discord.com/developers/resources/message#bulk-delete-messages)
/// (`POST /channels/{channel.id}/messages/bulk-delete`).
///
/// discord.js counterpart: `TextChannel#bulkDelete`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct BulkDeleteMessages {
    /// The messages to delete: 2 to 100, none older than two weeks and no duplicates.
    pub messages: Vec<MessageId>,
}

impl BulkDeleteMessages {
    /// Deletes `messages`.
    #[must_use]
    pub const fn new(messages: Vec<MessageId>) -> Self {
        Self { messages }
    }
}

/// The query of [Get Channel Pins](https://docs.discord.com/developers/resources/message#get-channel-pins)
/// (`GET /channels/{channel.id}/messages/pins`). Pins are returned newest first; to get the
/// next page, pass the `pinned_at` of the last pin as `before`.
///
/// discord.js counterpart: `MessageManager#fetchPins` options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetChannelPins {
    /// Pins made before this time.
    pub before: Option<Timestamp>,
    /// How many pins (1-50; Discord's default is 50).
    pub limit: Option<u8>,
}

/// The response of [Get Channel Pins](https://docs.discord.com/developers/resources/message#get-channel-pins).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChannelPins {
    /// The pins, newest first.
    pub items: Vec<MessagePin>,
    /// Whether there are older pins.
    pub has_more: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{id::AttachmentId, message::Embed};

    fn json<T: Serialize>(body: &T) -> String {
        serde_json::to_string(body).unwrap()
    }

    #[test]
    fn create_message_fields() {
        let mut body = CreateMessage::from("Hello, World!");
        body.nonce = Some(Nonce::String("n-1".into()));
        body.enforce_nonce = true;
        body.tts = true;
        body.embeds.push(Embed {
            title: Some("Hello, Embed!".into()),
            ..Embed::default()
        });
        body.sticker_ids
            .push(StickerId::new(1_300_000_000_000_000_000));
        body.flags = Some(MessageFlags::SUPPRESS_NOTIFICATIONS);
        assert_eq!(
            json(&body),
            concat!(
                r#"{"content":"Hello, World!","nonce":"n-1","tts":true,"#,
                r#""embeds":[{"title":"Hello, Embed!"}],"sticker_ids":["1300000000000000000"],"#,
                r#""flags":4096,"enforce_nonce":true}"#
            )
        );
        assert_eq!(json(&CreateMessage::default()), "{}");
    }

    #[test]
    fn edit_message_patches() {
        assert_eq!(json(&EditMessage::default()), "{}");

        let body = EditMessage {
            flags: Patch::Set(MessageFlags::SUPPRESS_EMBEDS),
            components: Patch::Clear,
            allowed_mentions: Patch::Clear,
            ..EditMessage::default()
        };
        assert_eq!(
            json(&body),
            r#"{"flags":4,"allowed_mentions":null,"components":null}"#
        );

        // New files are added to the kept attachments.
        let mut body = EditMessage {
            attachments: Patch::Set(vec![AttachmentRequest::existing(AttachmentId::new(
                1_300_000_000_000_000_001,
            ))]),
            files: vec![FileUpload::new("new.png", &b"png"[..])],
            ..EditMessage::default()
        };
        assert_eq!(
            json(&body),
            r#"{"attachments":[{"id":"1300000000000000001"},{"id":0,"filename":"new.png"}]}"#
        );

        // Clearing removes every attachment: an empty list, or only the new files.
        body.attachments = Patch::Clear;
        assert_eq!(
            json(&body),
            r#"{"attachments":[{"id":0,"filename":"new.png"}]}"#
        );
        body.files.clear();
        assert_eq!(json(&body), r#"{"attachments":[]}"#);
    }

    #[test]
    fn bulk_delete_and_pins() {
        let body = BulkDeleteMessages::new(vec![MessageId::new(1), MessageId::new(2)]);
        assert_eq!(json(&body), r#"{"messages":["1","2"]}"#);

        let pins: ChannelPins = serde_json::from_str(r#"{"items":[],"has_more":false}"#).unwrap();
        assert!(pins.items.is_empty() && !pins.has_more);
    }
}
