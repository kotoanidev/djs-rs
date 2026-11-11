//! Request bodies and response types of the [REST API](https://docs.discord.com/developers/reference#http-api).
//!
//! These are plain data types; sending requests is the job of the `djs-rest` crate.
//!
//! discord.js counterpart: the `RESTPost…JSONBody` / `RESTGet…Result` types of
//! discord-api-types.

use core::fmt;

use bytes::Bytes;
use compact_str::CompactString;
use serde::{Deserialize, Serialize, Serializer, ser::SerializeSeq};

use crate::message::{AttachmentRequest, AttachmentRequestId};

mod application_command;
mod channel;
mod emoji;
mod error_code;
mod guild;
mod interaction;
mod message;
mod user;
mod webhook;

pub use self::{
    application_command::{
        CreateApplicationCommand, EditApplicationCommand, EditApplicationCommandPermissions,
        GetApplicationCommands,
    },
    channel::{
        ArchivedThreads, CreateChannelInvite, EditChannelPermissions, FollowAnnouncementChannel,
        ForumTagData, ForumThread, ForumThreadMessage, GetChannelPermissions, GetThreadMember,
        GroupDmAddRecipient, ListArchivedThreads, ListJoinedPrivateArchivedThreads,
        ListThreadMembers, ModifyGroupDm, ModifyGuildChannel, ModifyThread, SetVoiceChannelStatus,
        StartThreadFromMessage, StartThreadInForumOrMediaChannel, StartThreadWithoutMessage,
    },
    emoji::{CreateApplicationEmoji, CreateGuildEmoji, ModifyApplicationEmoji, ModifyGuildEmoji},
    error_code::ErrorCode,
    guild::{
        ActiveThreads, AddGuildMember, BeginGuildPrune, BulkBanResponse, BulkGuildBan,
        CreateGuildBan, CreateGuildChannel, CreateGuildRole, GetGuild, GetGuildBans,
        GetGuildPruneCount, ListGuildMembers, ModifyCurrentMember, ModifyGuild,
        ModifyGuildChannelPosition, ModifyGuildIncidentActions, ModifyGuildMember,
        ModifyGuildOnboarding, ModifyGuildRole, ModifyGuildRolePosition, ModifyGuildWelcomeScreen,
        ModifyGuildWidget, OnboardingPromptData, PromptOptionData, SearchGuildMembers, VanityUrl,
    },
    interaction::CreateFollowupMessage,
    message::{
        BulkDeleteMessages, ChannelPins, CreateMessage, EditMessage, GetChannelMessages,
        GetChannelPins, GetReactions, MessageSearchResults, SearchAuthorType, SearchEmbedType,
        SearchFilter, SearchGuildMessages, SearchHasType, SearchSortMode, SearchSortOrder,
    },
    user::{CreateDm, CreateGroupDm, CurrentUserGuild, GetCurrentUserGuilds, ModifyCurrentUser},
    webhook::{CreateWebhook, EditWebhookMessage, ExecuteWebhook, ModifyWebhook},
};

/// The response of [Get Gateway Bot](https://docs.discord.com/developers/events/gateway#get-gateway-bot)
/// (`GET /gateway/bot`): where and how to connect to the gateway.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GatewayBot {
    /// The WSS URL to connect to, e.g. `wss://gateway.discord.gg`.
    pub url: CompactString,
    /// The recommended number of shards.
    pub shards: u32,
    /// Information on the current session start limit.
    pub session_start_limit: SessionStartLimit,
}

/// How many gateway sessions the bot may still start
/// ([session start limit object](https://docs.discord.com/developers/events/gateway#session-start-limit-object)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SessionStartLimit {
    /// The total number of session starts allowed per reset period (usually 1000 per day).
    pub total: u32,
    /// The remaining number of session starts.
    pub remaining: u32,
    /// Milliseconds until the limit resets.
    pub reset_after: u64,
    /// How many identify requests are allowed per 5 seconds (per rate-limit key).
    pub max_concurrency: u32,
}

/// A file uploaded with a request, e.g. a message attachment.
///
/// Request bodies that accept files have a `files` field (D-087). `djs-rest` then sends the
/// request as `multipart/form-data`: the file at index `n` becomes the part `files[n]`, the
/// JSON body the part `payload_json`, and the body's `attachments` gets an entry for each file
/// with its name and metadata (D-094). Embeds can show an uploaded image with the URL
/// `attachment://{name}`.
///
/// The data is [`Bytes`] (D-093): cloning a `FileUpload` (to send the same file to many
/// channels, or for a retry) does not copy the file, and neither do `Bytes::from_static`
/// (e.g. with `include_bytes!`) and `Bytes::from(Vec<u8>)`.
///
/// When [`content_type`](Self::content_type) is not set, it is guessed from the extension of
/// [`name`](Self::name) (D-095).
///
/// See [Uploading Files](https://docs.discord.com/developers/reference#uploading-files).
///
/// discord.js counterpart: `AttachmentBuilder` / `RawFile` of `@discordjs/rest`.
///
/// # Examples
///
/// ```
/// use djs_model::rest::FileUpload;
///
/// let mut image = FileUpload::new("ferris.png", &b"\x89PNG\r\n\x1a\n..."[..]);
/// image.description = Some("Ferris the crab".into());
/// image.is_spoiler = true;
///
/// // Cloning shares the data.
/// let copy = image.clone();
/// assert_eq!(copy.data.as_ptr(), image.data.as_ptr());
/// ```
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub struct FileUpload {
    /// The file name, including its extension (e.g. `ferris.png`).
    pub name: CompactString,
    /// The file's contents.
    pub data: Bytes,
    /// The [media type](https://en.wikipedia.org/wiki/Media_type) (e.g. `image/png`); guessed
    /// from the file name when `None`.
    pub content_type: Option<CompactString>,
    /// The alt text (max 1024 characters).
    pub description: Option<CompactString>,
    /// The file's title.
    pub title: Option<CompactString>,
    /// The duration of an audio or video file in seconds (required for voice messages).
    pub duration_secs: Option<f64>,
    /// A base64-encoded sampled waveform (required for voice messages).
    pub waveform: Option<CompactString>,
    /// Whether the file is blurred until clicked (sets
    /// [`AttachmentFlags::IS_SPOILER`](crate::message::AttachmentFlags::IS_SPOILER)).
    pub is_spoiler: bool,
}

impl FileUpload {
    /// Creates an upload of `data` named `name`, without metadata.
    ///
    /// `data` can be anything that converts into [`Bytes`] without copying: a `Vec<u8>`, a
    /// `String`, a `&'static [u8]` or a `&'static str`.
    #[must_use]
    pub fn new(name: impl Into<CompactString>, data: impl Into<Bytes>) -> Self {
        Self {
            name: name.into(),
            data: data.into(),
            content_type: None,
            description: None,
            title: None,
            duration_secs: None,
            waveform: None,
            is_spoiler: false,
        }
    }
}

impl fmt::Debug for FileUpload {
    /// Shows the size of the data instead of the data.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileUpload")
            .field("name", &self.name)
            .field("data", &format_args!("<{} bytes>", self.data.len()))
            .field("content_type", &self.content_type)
            .field("description", &self.description)
            .field("title", &self.title)
            .field("duration_secs", &self.duration_secs)
            .field("waveform", &self.waveform)
            .field("is_spoiler", &self.is_spoiler)
            .finish()
    }
}

/// The serialized `attachments` of a body with files (D-094): the explicit entries, then an
/// entry for every file whose `files[n]` index has no explicit entry.
#[derive(Clone, Copy)]
pub(crate) struct Attachments<'a> {
    explicit: Option<&'a [AttachmentRequest]>,
    files: &'a [FileUpload],
}

impl<'a> Attachments<'a> {
    pub(crate) const fn new(
        explicit: Option<&'a [AttachmentRequest]>,
        files: &'a [FileUpload],
    ) -> Self {
        Self { explicit, files }
    }

    /// Whether the field is left out: nothing explicit and no files. `Some(vec![])` is kept,
    /// because when editing it removes every attachment.
    pub(crate) const fn is_absent(&self) -> bool {
        self.explicit.is_none() && self.files.is_empty()
    }

    /// The attachments of a modify body: [`Patch::Keep`](crate::util::Patch::Keep) sends
    /// nothing (unless there are files), [`Clear`](crate::util::Patch::Clear) an empty list
    /// (plus the files), [`Set`](crate::util::Patch::Set) the given ones (plus the files).
    pub(crate) fn from_patch(
        attachments: &'a crate::util::Patch<Vec<AttachmentRequest>>,
        files: &'a [FileUpload],
    ) -> Self {
        use crate::util::Patch;

        let explicit = match attachments {
            Patch::Keep => None,
            Patch::Clear => Some(&[][..]),
            Patch::Set(attachments) => Some(attachments.as_slice()),
        };
        Self::new(explicit, files)
    }

    /// The files that get a generated entry, with their `files[n]` index.
    fn generated(self) -> impl Iterator<Item = (u32, &'a FileUpload)> {
        let explicit = self.explicit.unwrap_or_default();
        self.files
            .iter()
            .enumerate()
            .map(|(index, file)| (u32::try_from(index).unwrap_or(u32::MAX), file))
            .filter(move |(index, _)| {
                !explicit
                    .iter()
                    .any(|entry| entry.id == AttachmentRequestId::Upload(*index))
            })
    }
}

impl Serialize for Attachments<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        /// The entry of a file, borrowing its metadata.
        #[derive(Serialize)]
        struct Generated<'a> {
            id: u32,
            filename: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            title: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            duration_secs: Option<f64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            waveform: Option<&'a str>,
            #[serde(skip_serializing_if = "core::ops::Not::not")]
            is_spoiler: bool,
        }

        let explicit = self.explicit.unwrap_or_default();
        let mut seq = serializer.serialize_seq(Some(explicit.len() + self.generated().count()))?;
        for entry in explicit {
            seq.serialize_element(entry)?;
        }
        for (id, file) in self.generated() {
            seq.serialize_element(&Generated {
                id,
                filename: &file.name,
                title: file.title.as_deref(),
                description: file.description.as_deref(),
                duration_secs: file.duration_secs,
                waveform: file.waveform.as_deref(),
                is_spoiler: file.is_spoiler,
            })?;
        }
        seq.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_bot_documented_example() {
        let json = r#"{
            "url": "wss://gateway.discord.gg/",
            "shards": 9,
            "session_start_limit": {
                "total": 1000,
                "remaining": 999,
                "reset_after": 14400000,
                "max_concurrency": 1
            }
        }"#;
        let info: GatewayBot = serde_json::from_str(json).unwrap();
        assert_eq!(info.url, "wss://gateway.discord.gg/");
        assert_eq!(info.shards, 9);
        assert_eq!(info.session_start_limit.max_concurrency, 1);
    }

    fn json(body: &CreateMessage) -> String {
        serde_json::to_string(body).unwrap()
    }

    #[test]
    fn files_get_attachment_entries() {
        let mut body = CreateMessage::default();
        assert_eq!(json(&body), "{}");

        let mut voice = FileUpload::new("voice.ogg", Vec::from(*b"OggS"));
        voice.duration_secs = Some(2.5);
        voice.waveform = Some("AAAA".into());
        let mut image = FileUpload::new("ferris.png", &b"png"[..]);
        image.title = Some("Ferris".into());
        image.description = Some("Ferris the crab".into());
        image.is_spoiler = true;
        body.files = vec![voice, image];
        assert_eq!(
            json(&body),
            r#"{"attachments":[{"id":0,"filename":"voice.ogg","duration_secs":2.5,"waveform":"AAAA"},{"id":1,"filename":"ferris.png","title":"Ferris","description":"Ferris the crab","is_spoiler":true}]}"#
        );

        // An explicit entry for an index replaces the generated one; others are kept first.
        let mut renamed = AttachmentRequest::upload(1, "crab.png");
        renamed.description = Some("A crab".into());
        let keep =
            AttachmentRequest::existing(crate::id::AttachmentId::new(1_300_000_000_000_000_000));
        body.attachments = Some(vec![keep, renamed]);
        assert_eq!(
            json(&body),
            r#"{"attachments":[{"id":"1300000000000000000"},{"id":1,"filename":"crab.png","description":"A crab"},{"id":0,"filename":"voice.ogg","duration_secs":2.5,"waveform":"AAAA"}]}"#
        );

        // Without files, an explicit empty list is sent (it removes attachments on edits).
        body.files.clear();
        body.attachments = Some(Vec::new());
        assert_eq!(json(&body), r#"{"attachments":[]}"#);
    }

    #[test]
    fn file_upload_debug_hides_the_data() {
        let file = FileUpload::new("secret.txt", "not shown");
        let debug = format!("{file:?}");
        assert!(
            debug.contains("<9 bytes>") && !debug.contains("not shown"),
            "{debug}"
        );
    }
}
