//! Bodies, query parameters and responses of the
//! [message endpoints](https://docs.discord.com/developers/resources/message#get-channel-messages).

use compact_str::CompactString;
use serde::{Deserialize, Serialize, Serializer};

use super::{Attachments, FileUpload};
use crate::{
    channel::{Channel, ThreadMember},
    component::Component,
    id::{ChannelId, MessageId, RoleId, StickerId, UserId},
    macros::str_enum,
    message::{
        AllowedMentions, AttachmentRequest, Embed, Message, MessageFlags, MessagePin,
        MessageReference, Nonce, ReactionType, SharedClientTheme,
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

/// The query of [Search Guild Messages](https://docs.discord.com/developers/resources/message#search-guild-messages)
/// (`GET /guilds/{guild.id}/messages/search`). Every filter is optional; list filters match
/// any of their values.
///
/// The endpoint needs `READ_MESSAGE_HISTORY` and depends on the `MESSAGE_CONTENT` privileged
/// intent. While the guild is not indexed yet, the request fails with
/// `ErrorKind::NotYetIndexed` of `djs-rest` (D-104).
///
/// # Examples
///
/// ```
/// use djs_model::rest::{SearchFilter, SearchGuildMessages, SearchHasType};
///
/// let mut query = SearchGuildMessages::default();
/// query.content = Some("release".into());
/// query.has = vec![SearchFilter::Include(SearchHasType::Link)];
/// query.author_type = vec![SearchFilter::Exclude(djs_model::rest::SearchAuthorType::Bot)];
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SearchGuildMessages {
    /// How many messages (1-25; Discord's default is 25).
    pub limit: Option<u8>,
    /// How many results to skip (max 9975).
    pub offset: Option<u16>,
    /// Messages before this message.
    pub max_id: Option<MessageId>,
    /// Messages after this message.
    pub min_id: Option<MessageId>,
    /// How many words may lie between the matching words of `content` (max 100; default 2).
    pub slop: Option<u8>,
    /// Text the messages contain (max 1024 characters).
    pub content: Option<CompactString>,
    /// Messages in these channels (max 500).
    pub channel_id: Vec<ChannelId>,
    /// Messages by these kinds of authors (or not, with [`SearchFilter::Exclude`]).
    pub author_type: Vec<SearchFilter<SearchAuthorType>>,
    /// Messages by these users (max 100).
    pub author_id: Vec<UserId>,
    /// Messages mentioning these users (max 100).
    pub mentions: Vec<UserId>,
    /// Messages mentioning these roles (max 100).
    pub mentions_role_id: Vec<RoleId>,
    /// Messages that do (or do not) mention `@everyone`.
    pub mention_everyone: Option<bool>,
    /// Messages replying to these users (max 100).
    pub replied_to_user_id: Vec<UserId>,
    /// Messages replying to these messages (max 100).
    pub replied_to_message_id: Vec<MessageId>,
    /// Messages that are (or are not) pinned.
    pub pinned: Option<bool>,
    /// Messages that have (or do not have) these things.
    pub has: Vec<SearchFilter<SearchHasType>>,
    /// Messages with these kinds of embeds.
    pub embed_type: Vec<SearchEmbedType>,
    /// Messages with embeds from these providers, case-sensitive (e.g. `Tenor`; max 100).
    pub embed_provider: Vec<CompactString>,
    /// Messages with links to these host names (e.g. `discordapp.com`; max 100).
    pub link_hostname: Vec<CompactString>,
    /// Messages with attachments of these file names (max 100).
    pub attachment_filename: Vec<CompactString>,
    /// Messages with attachments of these extensions (e.g. `txt`; max 100).
    pub attachment_extension: Vec<CompactString>,
    /// How to sort (Discord's default: by time).
    pub sort_by: Option<SearchSortMode>,
    /// The direction (Discord's default: descending); ignored when sorting by relevance.
    pub sort_order: Option<SearchSortOrder>,
    /// Whether to include age-restricted channels (Discord's default: no).
    pub include_nsfw: Option<bool>,
}

/// A search filter value that includes or excludes matching messages (Discord writes an
/// exclusion with a leading `-`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SearchFilter<T> {
    /// Only messages that match.
    Include(T),
    /// Only messages that do not match.
    Exclude(T),
}

impl<T> From<T> for SearchFilter<T> {
    /// Includes the value.
    fn from(value: T) -> Self {
        Self::Include(value)
    }
}

str_enum! {
    /// An [author type](https://docs.discord.com/developers/resources/message#search-guild-messages-author-types)
    /// of a message search.
    pub enum SearchAuthorType {
        /// Messages of user accounts.
        User = "user",
        /// Messages of bot accounts.
        Bot = "bot",
        /// Messages of webhooks.
        Webhook = "webhook",
    }
}

str_enum! {
    /// Something a message [has](https://docs.discord.com/developers/resources/message#search-guild-messages-search-has-types),
    /// for a message search.
    pub enum SearchHasType {
        /// An image.
        Image = "image",
        /// A sound attachment.
        Sound = "sound",
        /// A video.
        Video = "video",
        /// An attachment.
        File = "file",
        /// A sticker.
        Sticker = "sticker",
        /// An embed.
        Embed = "embed",
        /// A link.
        Link = "link",
        /// A poll.
        Poll = "poll",
        /// A forwarded message.
        Snapshot = "snapshot",
    }
}

str_enum! {
    /// An [embed type](https://docs.discord.com/developers/resources/message#search-guild-messages-search-embed-types)
    /// of a message search (broader than the embed types of messages).
    pub enum SearchEmbedType {
        /// Image embeds.
        Image = "image",
        /// Video embeds.
        Video = "video",
        /// GIF (`gifv`) embeds.
        Gif = "gif",
        /// Sound embeds.
        Sound = "sound",
        /// Article embeds.
        Article = "article",
    }
}

str_enum! {
    /// A [sort mode](https://docs.discord.com/developers/resources/message#search-guild-messages-search-sort-modes)
    /// of a message search.
    pub enum SearchSortMode {
        /// By creation time (the default).
        Timestamp = "timestamp",
        /// By relevance to the search.
        Relevance = "relevance",
    }
}

str_enum! {
    /// The direction of a message search's sorting.
    pub enum SearchSortOrder {
        /// Oldest first.
        Ascending = "asc",
        /// Newest first (the default).
        Descending = "desc",
    }
}

/// The response of [Search Guild Messages](https://docs.discord.com/developers/resources/message#search-guild-messages-response-body).
///
/// Messages come without their `reactions`. Do not paginate by the number of messages
/// returned: the search may return fewer than the limit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MessageSearchResults {
    /// Whether the guild is going through a deep historical indexing.
    pub doing_deep_historical_index: bool,
    /// How many documents the current indexing has processed, if one is running.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documents_indexed: Option<u64>,
    /// How many messages match in total (may be inexact while messages are being created
    /// or deleted).
    pub total_results: u64,
    /// The matching messages; each inner list holds one message (Discord used to add
    /// surrounding messages for context).
    pub messages: Vec<Vec<Message>>,
    /// The threads the messages are in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub threads: Vec<Channel>,
    /// The current user's thread member for each returned thread it has joined.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<ThreadMember>,
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
