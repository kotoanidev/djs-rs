//! Bodies of the [emoji endpoints](https://docs.discord.com/developers/resources/emoji): guild
//! emojis and application emojis.

use compact_str::CompactString;
use serde::Serialize;

use crate::{
    id::RoleId,
    util::{ImageData, Patch},
};

/// The body of [Create Guild Emoji](https://docs.discord.com/developers/resources/emoji#create-guild-emoji).
///
/// The image (PNG, JPEG, GIF for animated emojis, or WebP) may be at most 256 KiB; Discord
/// shows it at 128×128.
///
/// discord.js counterpart: `GuildEmojiCreateOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::{id::RoleId, rest::CreateGuildEmoji, util::ImageData};
///
/// let png = b"\x89PNG\r\n\x1a\n...".to_vec();
/// let mut body = CreateGuildEmoji::new("ferris", ImageData::new(png)?);
/// body.roles.push(RoleId::new(1_100_000_000_000_000_001));
/// let json = serde_json::to_string(&body)?;
/// assert!(json.starts_with(r#"{"name":"ferris","image":"data:image/png;base64,"#));
/// assert!(json.ends_with(r#","roles":["1100000000000000001"]}"#));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct CreateGuildEmoji {
    /// The name (2–32 characters: letters, digits and underscores).
    pub name: CompactString,
    /// The image.
    pub image: ImageData,
    /// The roles allowed to use the emoji; empty (the default) means everyone.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<RoleId>,
}

impl CreateGuildEmoji {
    /// An emoji named `name` showing `image`, usable by everyone.
    #[must_use]
    pub fn new(name: impl Into<CompactString>, image: ImageData) -> Self {
        Self {
            name: name.into(),
            image,
            roles: Vec::new(),
        }
    }
}

/// The body of [Modify Guild Emoji](https://docs.discord.com/developers/resources/emoji#modify-guild-emoji).
///
/// Every field is optional; the image cannot be changed.
///
/// discord.js counterpart: `GuildEmojiEditOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::{rest::ModifyGuildEmoji, util::Patch};
///
/// let mut body = ModifyGuildEmoji::default();
/// body.name = Some("crab".into());
/// body.roles = Patch::Clear; // usable by everyone again
/// assert_eq!(serde_json::to_string(&body)?, r#"{"name":"crab","roles":null}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyGuildEmoji {
    /// The new name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The roles allowed to use the emoji; [`Patch::Clear`] allows everyone.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub roles: Patch<Vec<RoleId>>,
}

/// The body of [Create Application Emoji](https://docs.discord.com/developers/resources/emoji#create-application-emoji).
///
/// Application emojis belong to the bot itself and can be used by it in every guild and DM.
/// The image may be at most 256 KiB.
///
/// discord.js counterpart: `ApplicationEmojiCreateOptions`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct CreateApplicationEmoji {
    /// The name (2–32 characters: letters, digits and underscores).
    pub name: CompactString,
    /// The image.
    pub image: ImageData,
}

impl CreateApplicationEmoji {
    /// An emoji named `name` showing `image`.
    #[must_use]
    pub fn new(name: impl Into<CompactString>, image: ImageData) -> Self {
        Self {
            name: name.into(),
            image,
        }
    }
}

/// The body of [Modify Application Emoji](https://docs.discord.com/developers/resources/emoji#modify-application-emoji).
///
/// Only the name can be changed.
///
/// discord.js counterpart: `ApplicationEmojiEditOptions`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyApplicationEmoji {
    /// The new name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
}
