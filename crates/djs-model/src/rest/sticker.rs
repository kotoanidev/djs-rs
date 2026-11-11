//! Bodies of the [sticker endpoints](https://docs.discord.com/developers/resources/sticker).

use compact_str::CompactString;
use serde::Serialize;

use super::FileUpload;
use crate::util::Patch;

/// The body of [Create Guild Sticker](https://docs.discord.com/developers/resources/sticker#create-guild-sticker).
///
/// Unlike most bodies it is sent as `multipart/form-data` fields, not JSON, so it does not
/// implement `Serialize`. The file must be a PNG, APNG, GIF or Lottie JSON file of at most
/// 512 KiB and 320×320 pixels (animated: at most 5 seconds); Lottie stickers need a verified or
/// partnered guild.
///
/// discord.js counterpart: `GuildStickerCreateOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::rest::{CreateGuildSticker, FileUpload};
///
/// let file = FileUpload::new("wave.png", b"\x89PNG\r\n\x1a\n...".to_vec());
/// let mut body = CreateGuildSticker::new("Wave", "wave", file);
/// body.description = "A friendly wave".into();
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CreateGuildSticker {
    /// The name (2–30 characters).
    pub name: CompactString,
    /// The description: empty (the default) or 2–100 characters.
    pub description: CompactString,
    /// The autocomplete and suggestion tags (max 200 characters); Discord's client uses the
    /// name of a related Unicode emoji, such as `wave`.
    pub tags: CompactString,
    /// The sticker image.
    pub file: FileUpload,
}

impl CreateGuildSticker {
    /// A sticker named `name`, suggested for `tags`, showing `file`, without a description.
    #[must_use]
    pub fn new(
        name: impl Into<CompactString>,
        tags: impl Into<CompactString>,
        file: FileUpload,
    ) -> Self {
        Self {
            name: name.into(),
            description: CompactString::const_new(""),
            tags: tags.into(),
            file,
        }
    }
}

/// The body of [Modify Guild Sticker](https://docs.discord.com/developers/resources/sticker#modify-guild-sticker).
///
/// Every field is optional; the image cannot be changed.
///
/// discord.js counterpart: `GuildStickerEditOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::{rest::ModifyGuildSticker, util::Patch};
///
/// let mut body = ModifyGuildSticker::default();
/// body.tags = Some("smile".into());
/// body.description = Patch::Clear;
/// assert_eq!(serde_json::to_string(&body)?, r#"{"description":null,"tags":"smile"}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyGuildSticker {
    /// The new name (2–30 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The new description (2–100 characters); [`Patch::Clear`] removes it.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub description: Patch<CompactString>,
    /// The new autocomplete and suggestion tags (max 200 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<CompactString>,
}
