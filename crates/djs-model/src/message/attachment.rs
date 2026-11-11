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
