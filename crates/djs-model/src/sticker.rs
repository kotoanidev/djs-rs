//! [Stickers](https://docs.discord.com/developers/resources/sticker): standard sticker packs
//! and stickers uploaded to guilds.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    id::{GenericId, GuildId, SkuId, StickerId, StickerPackId},
    macros::int_enum,
    user::User,
};

/// A [sticker](https://docs.discord.com/developers/resources/sticker#sticker-object) that can be
/// sent in messages.
///
/// discord.js counterpart: `Sticker`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Sticker {
    /// The sticker's ID.
    pub id: StickerId,
    /// For standard stickers, the pack the sticker belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_id: Option<StickerPackId>,
    /// The sticker's name.
    pub name: CompactString,
    /// The sticker's description.
    pub description: Option<CompactString>,
    /// Autocomplete/suggestion tags (up to 200 characters). Standard stickers use a
    /// comma-separated list of keywords, but that is only a convention; for guild stickers this
    /// is the name of a Unicode emoji.
    pub tags: CompactString,
    /// Whether this is a standard or a guild sticker.
    #[serde(rename = "type")]
    pub kind: StickerType,
    /// The sticker's file format.
    pub format_type: StickerFormatType,
    /// Whether the guild sticker can be used; `false` when the guild lost the Server Boosts it
    /// needs. `true` when Discord omits the field, as it does for standard stickers (D-055).
    #[serde(default = "crate::serde_util::default_true")]
    pub available: bool,
    /// The guild that owns the sticker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// Who uploaded the guild sticker (requires the `MANAGE_GUILD_EXPRESSIONS` or
    /// `CREATE_GUILD_EXPRESSIONS` permission).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<Box<User>>,
    /// The standard sticker's position within its pack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_value: Option<u32>,
}

int_enum! {
    /// The [type of a sticker](https://docs.discord.com/developers/resources/sticker#sticker-object-sticker-types).
    ///
    /// discord.js counterpart: `StickerType`.
    pub enum StickerType: u8 {
        /// An official sticker from a pack.
        Standard = 1,
        /// A sticker uploaded to a guild for its members.
        Guild = 2,
    }
}

int_enum! {
    /// The [file format of a sticker](https://docs.discord.com/developers/resources/sticker#sticker-object-sticker-format-types).
    ///
    /// discord.js counterpart: `StickerFormatType`.
    pub enum StickerFormatType: u8 {
        /// PNG image.
        Png = 1,
        /// Animated PNG image.
        Apng = 2,
        /// [Lottie](https://airbnb.io/lottie/) JSON animation.
        Lottie = 3,
        /// GIF image.
        Gif = 4,
    }
}

/// The minimal [sticker item](https://docs.discord.com/developers/resources/sticker#sticker-item-object)
/// attached to messages.
///
/// discord.js counterpart: the stickers of `Message#stickers`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StickerItem {
    /// The sticker's ID.
    pub id: StickerId,
    /// The sticker's name.
    pub name: CompactString,
    /// The sticker's file format.
    pub format_type: StickerFormatType,
}

/// A [pack of standard stickers](https://docs.discord.com/developers/resources/sticker#sticker-pack-object).
///
/// discord.js counterpart: `StickerPack`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StickerPack {
    /// The pack's ID.
    pub id: StickerPackId,
    /// The stickers in the pack.
    pub stickers: Vec<Sticker>,
    /// The pack's name.
    pub name: CompactString,
    /// The pack's SKU.
    pub sku_id: SkuId,
    /// The sticker shown as the pack's icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_sticker_id: Option<StickerId>,
    /// The pack's description.
    pub description: CompactString,
    /// The ID of the pack's [banner image](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_asset_id: Option<GenericId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_sticker_is_available() {
        let json = r#"{"id":"749054660769218631","name":"Wave","tags":"wumpus, hello, sup, hi, oi, heyo, heya, yo, wave",
            "type":1,"format_type":3,"description":"Wumpus waves hello","asset":"",
            "pack_id":"847199849233514549","sort_value":12}"#;
        let sticker: Sticker = serde_json::from_str(json).unwrap();
        assert_eq!(sticker.kind, StickerType::Standard);
        assert_eq!(sticker.format_type, StickerFormatType::Lottie);
        assert!(sticker.available);
        assert_eq!(sticker.sort_value, Some(12));
    }

    #[test]
    fn sticker_item() {
        let item: StickerItem =
            serde_json::from_str(r#"{"id":"1","name":"x","format_type":4}"#).unwrap();
        assert_eq!(item.format_type, StickerFormatType::Gif);
    }
}
