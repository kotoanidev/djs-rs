//! Files attached to messages.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use super::AttachmentFlags;
use crate::{application::PartialApplication, id::AttachmentId, user::User, util::Timestamp};

/// A [file attached to a message](https://docs.discord.com/developers/resources/message#attachment-object).
///
/// Contains a float ([`duration_secs`](Self::duration_secs)), so it is `PartialEq` but not
/// `Eq` (D-064).
///
/// discord.js counterpart: `Attachment`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Attachment {
    /// The attachment's ID.
    pub id: AttachmentId,
    /// The file name.
    pub filename: CompactString,
    /// The title of the file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<CompactString>,
    /// The description (alt text, max 1024 characters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<CompactString>,
    /// The [media type](https://en.wikipedia.org/wiki/Media_type).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<CompactString>,
    /// The size in bytes.
    pub size: u64,
    /// The source URL.
    pub url: CompactString,
    /// The URL through Discord's media proxy. It serves images, videos and audio; other files
    /// get `415 Unsupported Media Type`.
    pub proxy_url: CompactString,
    /// The height of an image or video, in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// The width of an image or video, in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// A [thumbhash](https://evanw.github.io/thumbhash/) placeholder of an image or video.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<CompactString>,
    /// The version of the placeholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder_version: Option<u32>,
    /// Whether the attachment is ephemeral: it is removed after a while (but stays available as
    /// long as its message exists).
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub ephemeral: bool,
    /// The duration of an audio or video file, in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,
    /// A base64-encoded sampled waveform (voice messages).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waveform: Option<CompactString>,
    /// The attachment's flags.
    #[serde(default)]
    pub flags: AttachmentFlags,
    /// For clips: the users who were in the stream.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clip_participants: Vec<User>,
    /// For clips: when the clip was created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip_created_at: Option<Timestamp>,
    /// For clips: the application in the stream, if recognized.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application: Option<Box<PartialApplication>>,
}

/// An [attachment in a create or edit request](https://docs.discord.com/developers/resources/message#attachment-object-attachment-request-structure):
/// a new file uploaded with the request (`files[n]`), or an existing attachment to keep.
///
/// Sent in message create/edit requests, webhook messages and interaction responses.
///
/// discord.js counterpart: the `attachments` option of `MessageCreateOptions` / `AttachmentBuilder`.
///
/// # Examples
///
/// ```
/// use djs_model::message::AttachmentRequest;
///
/// let mut file = AttachmentRequest::upload(0, "ferris.png");
/// file.description = Some("Ferris the crab".into());
/// assert_eq!(
///     serde_json::to_string(&file).unwrap(),
///     r#"{"id":0,"filename":"ferris.png","description":"Ferris the crab"}"#
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AttachmentRequest {
    /// The upload index or the existing attachment.
    pub id: AttachmentRequestId,
    /// The file name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<CompactString>,
    /// The file's title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<CompactString>,
    /// The alt text (max 1024 characters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<CompactString>,
    /// The duration of an audio or video file in seconds (required for voice messages).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,
    /// A base64-encoded sampled waveform (required for voice messages).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waveform: Option<CompactString>,
    /// Whether the file is blurred until clicked (sets [`AttachmentFlags::IS_SPOILER`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_spoiler: Option<bool>,
}

impl AttachmentRequest {
    /// Describes the file uploaded as `files[index]` in the same request.
    #[must_use]
    pub fn upload(index: u32, filename: impl Into<CompactString>) -> Self {
        Self::with_id(AttachmentRequestId::Upload(index), Some(filename.into()))
    }

    /// Keeps an existing attachment (when editing a message).
    #[must_use]
    pub fn existing(id: AttachmentId) -> Self {
        Self::with_id(AttachmentRequestId::Existing(id), None)
    }

    const fn with_id(id: AttachmentRequestId, filename: Option<CompactString>) -> Self {
        Self {
            id,
            filename,
            title: None,
            description: None,
            duration_secs: None,
            waveform: None,
            is_spoiler: None,
        }
    }
}

/// The `id` of an [`AttachmentRequest`]: the index `n` of a file uploaded as `files[n]`, or
/// the ID of an existing attachment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttachmentRequestId {
    /// A new file, uploaded as `files[n]` in the same multipart request (written as a number).
    Upload(u32),
    /// An existing attachment (written as a snowflake string).
    Existing(AttachmentId),
}

impl Serialize for AttachmentRequestId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Upload(index) => serializer.serialize_u32(*index),
            Self::Existing(id) => id.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for AttachmentRequestId {
    /// Numbers that fit in a `u32` are upload indices; strings and larger numbers are IDs.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct AttachmentRequestIdVisitor;

        impl serde::de::Visitor<'_> for AttachmentRequestIdVisitor {
            type Value = AttachmentRequestId;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("an upload index or a snowflake ID")
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                if let Ok(index) = u32::try_from(value) {
                    return Ok(AttachmentRequestId::Upload(index));
                }
                AttachmentId::new_checked(value)
                    .map(AttachmentRequestId::Existing)
                    .ok_or_else(|| E::invalid_value(serde::de::Unexpected::Unsigned(value), &self))
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                u64::try_from(value)
                    .map_err(|_| E::invalid_value(serde::de::Unexpected::Signed(value), &self))
                    .and_then(|value| self.visit_u64(value))
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                value
                    .parse()
                    .map(AttachmentRequestId::Existing)
                    .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_any(AttachmentRequestIdVisitor)
    }
}
