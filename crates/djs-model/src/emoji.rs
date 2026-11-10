//! [Emojis](https://docs.discord.com/developers/resources/emoji): custom guild and application
//! emojis, and the partial emojis used in reactions, components and polls.

use core::fmt;

use compact_str::CompactString;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as DeError, IgnoredAny, MapAccess, Visitor},
    ser::SerializeStruct,
};

use crate::{
    id::{EmojiId, RoleId},
    user::User,
};

/// A custom emoji uploaded to a guild or owned by an application.
///
/// See the [emoji object](https://docs.discord.com/developers/resources/emoji#emoji-object).
/// Emojis inside reactions, buttons and polls are [`PartialEmoji`]s (D-056).
///
/// discord.js counterpart: `GuildEmoji` / `ApplicationEmoji`.
///
/// # Examples
///
/// ```
/// use djs_model::emoji::Emoji;
///
/// let emoji: Emoji = serde_json::from_str(
///     r#"{"id":"41771983429993937","name":"LUL","roles":[],"require_colons":true,
///         "managed":false,"animated":false}"#,
/// )?;
/// assert_eq!(emoji.name, "LUL");
/// assert!(emoji.available, "absent `available` means usable (D-055)");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Emoji {
    /// The emoji's ID.
    pub id: EmojiId,
    /// The emoji's name.
    pub name: CompactString,
    /// The roles allowed to use the emoji; empty means everyone. Emojis restricted to
    /// subscription roles are premium emojis.
    #[serde(default)]
    pub roles: Vec<RoleId>,
    /// Who uploaded the emoji: for guild emojis only visible with the
    /// [`MANAGE_GUILD_EXPRESSIONS`](crate::permissions::Permissions::MANAGE_GUILD_EXPRESSIONS)
    /// permission; for application emojis the team member or the bot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<Box<User>>,
    /// Whether the emoji must be wrapped in colons to be used.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub require_colons: bool,
    /// Whether the emoji is managed by an integration (e.g. Twitch).
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub managed: bool,
    /// Whether the emoji is animated.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub animated: bool,
    /// Whether the emoji can be used; `false` when the guild lost the Server Boosts it needs.
    /// `true` when Discord omits the field (D-055).
    #[serde(default = "crate::serde_util::default_true")]
    pub available: bool,
}

/// An emoji as used in reactions, message components and polls: either a custom emoji or a
/// Unicode emoji (D-056).
///
/// See the [emoji object](https://docs.discord.com/developers/resources/emoji#emoji-object)
/// examples for standard and custom emojis.
///
/// discord.js counterpart: `PartialEmoji` / `ReactionEmoji`.
///
/// # Examples
///
/// ```
/// use djs_model::emoji::PartialEmoji;
///
/// let fire: PartialEmoji = serde_json::from_str(r#"{"id":null,"name":"🔥"}"#)?;
/// assert_eq!(fire, PartialEmoji::Unicode { name: "🔥".into() });
///
/// let custom: PartialEmoji = serde_json::from_str(r#"{"id":"41771983429993937","name":"LUL","animated":true}"#)?;
/// assert!(matches!(custom, PartialEmoji::Custom { animated: true, .. }));
/// assert_eq!(custom.name(), Some("LUL"));
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PartialEmoji {
    /// A custom (guild or application) emoji.
    Custom {
        /// The emoji's ID.
        id: EmojiId,
        /// The emoji's name; `None` when Discord no longer has the emoji's data (for example a
        /// deleted emoji in reaction events).
        name: Option<CompactString>,
        /// Whether the emoji is animated.
        animated: bool,
    },
    /// A standard Unicode emoji such as `🔥`.
    Unicode {
        /// The emoji itself.
        name: CompactString,
    },
}

impl PartialEmoji {
    /// Returns the custom emoji's ID, or `None` for a Unicode emoji.
    #[must_use]
    pub const fn id(&self) -> Option<EmojiId> {
        match *self {
            Self::Custom { id, .. } => Some(id),
            Self::Unicode { .. } => None,
        }
    }

    /// Returns the emoji's name (the emoji itself for Unicode emojis), if known.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Custom { name, .. } => name.as_deref(),
            Self::Unicode { name } => Some(name),
        }
    }
}

impl From<&Emoji> for PartialEmoji {
    fn from(emoji: &Emoji) -> Self {
        Self::Custom {
            id: emoji.id,
            name: Some(emoji.name.clone()),
            animated: emoji.animated,
        }
    }
}

impl Serialize for PartialEmoji {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Custom { id, name, animated } => {
                let mut state =
                    serializer.serialize_struct("PartialEmoji", 2 + usize::from(*animated))?;
                state.serialize_field("id", id)?;
                state.serialize_field("name", name)?;
                if *animated {
                    state.serialize_field("animated", &true)?;
                }
                state.end()
            }
            Self::Unicode { name } => {
                let mut state = serializer.serialize_struct("PartialEmoji", 2)?;
                state.serialize_field("id", &None::<EmojiId>)?;
                state.serialize_field("name", name)?;
                state.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for PartialEmoji {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Id,
            Name,
            Animated,
            #[serde(other)]
            Other,
        }

        struct EmojiVisitor;

        impl<'de> Visitor<'de> for EmojiVisitor {
            type Value = PartialEmoji;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a partial emoji object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<PartialEmoji, A::Error> {
                let (mut id, mut name, mut animated) = (None, None, false);
                while let Some(field) = map.next_key()? {
                    match field {
                        Field::Id => id = map.next_value::<Option<EmojiId>>()?,
                        Field::Name => name = map.next_value::<Option<CompactString>>()?,
                        Field::Animated => {
                            animated = map.next_value::<Option<bool>>()?.unwrap_or_default();
                        }
                        Field::Other => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }
                match (id, name) {
                    (Some(id), name) => Ok(PartialEmoji::Custom { id, name, animated }),
                    (None, Some(name)) => Ok(PartialEmoji::Unicode { name }),
                    (None, None) => Err(A::Error::custom("emoji has neither an id nor a name")),
                }
            }
        }

        deserializer.deserialize_map(EmojiVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_emoji_example() {
        let json = r#"{"id":"41771983429993937","name":"LUL",
            "roles":["41771983429993000","41771983429993111"],
            "user":{"username":"Luigi","discriminator":"0002","id":"96008815106887111",
                    "avatar":"5500909a3274e1812beb4e8de6631111","public_flags":131328},
            "require_colons":true,"managed":false,"animated":false}"#;
        let emoji: Emoji = serde_json::from_str(json).unwrap();
        assert_eq!(emoji.id.get(), 41_771_983_429_993_937);
        assert_eq!(emoji.roles.len(), 2);
        assert_eq!(emoji.user.as_ref().unwrap().discriminator, 2);
        assert!(emoji.require_colons);
        assert!(emoji.available);
        let unavailable: Emoji =
            serde_json::from_str(r#"{"id":"1","name":"x","available":false}"#).unwrap();
        assert!(!unavailable.available);
    }

    #[test]
    fn partial_emoji_shapes() {
        let deleted: PartialEmoji =
            serde_json::from_str(r#"{"id":"41771983429993937","name":null}"#).unwrap();
        assert_eq!(deleted.name(), None);
        assert_eq!(
            deleted.id().map(crate::id::Id::get),
            Some(41_771_983_429_993_937)
        );

        // Serialization mirrors the received shapes.
        for json in [
            r#"{"id":null,"name":"🔥"}"#,
            r#"{"id":"41771983429993937","name":"LUL","animated":true}"#,
            r#"{"id":"41771983429993937","name":null}"#,
        ] {
            let emoji: PartialEmoji = serde_json::from_str(json).unwrap();
            assert_eq!(serde_json::to_string(&emoji).unwrap(), json);
        }

        // Unknown fields are ignored; an emoji without id and name is rejected.
        assert!(serde_json::from_str::<PartialEmoji>(r#"{"name":"🔥","x":[1]}"#).is_ok());
        assert!(serde_json::from_str::<PartialEmoji>(r#"{"id":null,"name":null}"#).is_err());
    }
}
