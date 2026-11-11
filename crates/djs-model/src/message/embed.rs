//! Embeds: rich content blocks attached to messages.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use super::{EmbedFlags, EmbedMediaFlags};
use crate::{macros::str_enum, util::Timestamp};

/// An [embed](https://docs.discord.com/developers/resources/message#embed-object): a rich
/// content block (link previews, images, videos, or a bot's custom "rich" embed).
///
/// Every part is optional. Sub-objects are boxed so an embed without them stays small (D-054).
/// Discord's [limits](https://docs.discord.com/developers/resources/message#embed-object-embed-limits)
/// apply when sending: title 256 characters, description 4096, up to 25 fields (name 256,
/// value 1024), footer text 2048, author name 256, and 6000 characters across all embeds of a
/// message. Building and validating embeds is the job of `djs-builders`.
///
/// discord.js counterpart: `Embed` (received) / `EmbedBuilder` (sent).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Embed {
    /// The title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<CompactString>,
    /// The kind of embed (always [`EmbedType::Rich`] for embeds sent by bots and webhooks).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<EmbedType>,
    /// The description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<CompactString>,
    /// The URL the title links to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<CompactString>,
    /// The timestamp shown in the footer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<Timestamp>,
    /// The color of the left border, as an RGB integer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    /// The footer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer: Option<Box<EmbedFooter>>,
    /// The large image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<Box<EmbedImage>>,
    /// The small image in the top-right corner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<Box<EmbedImage>>,
    /// The video (only set by Discord, never sent by bots).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<Box<EmbedVideo>>,
    /// The provider (only set by Discord, e.g. "YouTube").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Box<EmbedProvider>>,
    /// The author shown above the title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<Box<EmbedAuthor>>,
    /// The fields (max 25).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<EmbedField>,
    /// The embed's flags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<EmbedFlags>,
}

str_enum! {
    /// The [type of an embed](https://docs.discord.com/developers/resources/message#embed-object-embed-types).
    ///
    /// discord.js counterpart: `EmbedType`.
    pub enum EmbedType {
        /// A generic embed rendered from its fields (the only type bots can send).
        Rich = "rich",
        /// An image.
        Image = "image",
        /// A video.
        Video = "video",
        /// An animated GIF rendered as a video.
        Gifv = "gifv",
        /// An article.
        Article = "article",
        /// A link.
        Link = "link",
        /// An auto moderation alert in the alert channel.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may change
        /// without notice.
        AutoModerationMessage = "auto_moderation_message",
        /// The result of a [poll](crate::poll), in the message announcing it; the data is in
        /// the [fields](Embed::fields) (`poll_question_text`, `victor_answer_votes`,
        /// `total_votes`, `victor_answer_id`, `victor_answer_text`, `victor_answer_emoji_id`,
        /// `victor_answer_emoji_name`, `victor_answer_emoji_animated`).
        PollResult = "poll_result",
        /// A [link preview built from components](https://docs.discord.com/developers/link-previews/component-embeds).
        Components = "components",
    }
}

/// The [footer of an embed](https://docs.discord.com/developers/resources/message#embed-object-embed-footer-structure).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbedFooter {
    /// The footer text (max 2048 characters).
    pub text: CompactString,
    /// The URL of the footer icon (`http(s)` or `attachment://`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<CompactString>,
    /// The URL of the icon through Discord's media proxy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_icon_url: Option<CompactString>,
}

/// The [image or thumbnail of an embed](https://docs.discord.com/developers/resources/message#embed-object-embed-image-structure).
///
/// discord.js counterpart: `EmbedAssetData`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "EmbedThumbnail")]
pub struct EmbedImage {
    /// The source URL (`http(s)` or `attachment://`).
    pub url: CompactString,
    /// The URL through Discord's media proxy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<CompactString>,
    /// The height in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// The width in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// The [media type](https://en.wikipedia.org/wiki/Media_type).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<CompactString>,
    /// A [thumbhash](https://evanw.github.io/thumbhash/) placeholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<CompactString>,
    /// The version of the placeholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder_version: Option<u32>,
    /// The description (alt text).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<CompactString>,
    /// The media flags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<EmbedMediaFlags>,
}

/// The [video of an embed](https://docs.discord.com/developers/resources/message#embed-object-embed-video-structure).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbedVideo {
    /// The source URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<CompactString>,
    /// The URL through Discord's media proxy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<CompactString>,
    /// The height in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// The width in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// The [media type](https://en.wikipedia.org/wiki/Media_type).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<CompactString>,
    /// A [thumbhash](https://evanw.github.io/thumbhash/) placeholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<CompactString>,
    /// The version of the placeholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder_version: Option<u32>,
    /// The description (alt text).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<CompactString>,
    /// The media flags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<EmbedMediaFlags>,
}

/// The [provider of an embed](https://docs.discord.com/developers/resources/message#embed-object-embed-provider-structure),
/// e.g. the website of a link preview.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbedProvider {
    /// The provider's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The provider's URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<CompactString>,
}

/// The [author of an embed](https://docs.discord.com/developers/resources/message#embed-object-embed-author-structure).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbedAuthor {
    /// The author's name (max 256 characters).
    pub name: CompactString,
    /// The URL the name links to (`http(s)` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<CompactString>,
    /// The URL of the author icon (`http(s)` or `attachment://`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<CompactString>,
    /// The URL of the icon through Discord's media proxy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_icon_url: Option<CompactString>,
}

/// A [field of an embed](https://docs.discord.com/developers/resources/message#embed-object-embed-field-structure).
///
/// discord.js counterpart: `APIEmbedField`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbedField {
    /// The field's name (max 256 characters).
    pub name: CompactString,
    /// The field's value (max 1024 characters).
    pub value: CompactString,
    /// Whether the field is shown next to other inline fields.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub inline: bool,
}
