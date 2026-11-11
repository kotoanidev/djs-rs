//! [Application identity profiles](https://docs.discord.com/developers/resources/application-identity-profile):
//! game stats an app stores on a user's identity in its game, shown in Game Stats Widgets on
//! Discord profiles (D-072).
//!
//! An identity is keyed by the provider type, an optional provider ID and the user's ID in the
//! external system ([`ApplicationIdentity`]); its [`ApplicationIdentityProfile`] holds the
//! stats.

use std::collections::BTreeMap;

use compact_str::CompactString;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};

use crate::{
    id::UserId,
    macros::int_enum,
    util::{JsonValue, JsonValueError},
};

/// An [application identity](https://docs.discord.com/developers/resources/application-identity-profile#application-identity-object):
/// a user's account in the app's external system.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ApplicationIdentity {
    /// The Discord user (included in the identity lists the API returns).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<UserId>,
    /// The external account provider type (`NONE` for identities created by a profile write).
    pub provider_type: CompactString,
    /// A provider-specific ID that disambiguates identities (absent when empty).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<CompactString>,
    /// The user's ID in the external system.
    pub provider_issued_user_id: CompactString,
}

/// The [profile of an application identity](https://docs.discord.com/developers/resources/application-identity-profile#application-identity-profile-object):
/// the user's game stats.
///
/// Contains floats, so it is `PartialEq` but not `Eq` (D-064).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ApplicationIdentityProfile {
    /// The user's username in the external system.
    pub username: Option<CompactString>,
    /// Arbitrary game-defined data, stored for the app's own use (D-072).
    pub metadata: Option<BTreeMap<CompactString, JsonValue>>,
    /// The game stats.
    pub data: Option<ProfileData>,
}

/// The [game stats](https://docs.discord.com/developers/resources/application-identity-profile#profile-data-object)
/// of a profile: preconfigured fields and custom ones.
///
/// Also the `data` sent to update a profile (max 10 KB serialized, up to 30 dynamic fields).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProfileData {
    /// The preconfigured stat fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<Box<PrimaryProfileData>>,
    /// Custom stat fields.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dynamic: Vec<DynamicField>,
}

/// The [preconfigured stat fields](https://docs.discord.com/developers/resources/application-identity-profile#primary-profile-data-object)
/// of a profile (text values: max 100 characters).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PrimaryProfileData {
    /// The current season's name (e.g. "Season 3").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season: Option<CompactString>,
    /// The current rank's name (e.g. "Silver").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank_name: Option<CompactString>,
    /// An image of the current rank.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank_image: Option<ProfileMedia>,
    /// The highest rank achieved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highest_rank: Option<CompactString>,
    /// An image of the highest rank achieved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highest_rank_image: Option<ProfileMedia>,
    /// The featured played character's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub featured_played_character: Option<CompactString>,
    /// An image of the featured played character.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub featured_played_character_image: Option<ProfileMedia>,
    /// The total playtime in hours (fractions allowed, e.g. `69.41`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playtime_hours: Option<f64>,
    /// The total number of wins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_wins: Option<u64>,
    /// The wins in the current period (e.g. season).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_period_wins: Option<u64>,
    /// The total number of games played.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_games: Option<u64>,
    /// The games played in the current period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_period_games: Option<u64>,
    /// The total number of kills.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_kills: Option<u64>,
    /// The kills in the current period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_period_kills: Option<u64>,
    /// The total number of assists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_assists: Option<u64>,
    /// The assists in the current period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_period_assists: Option<u64>,
    /// The total number of deaths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_deaths: Option<u64>,
    /// The deaths in the current period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_period_deaths: Option<u64>,
}

/// An image in profile data ([media object](https://docs.discord.com/developers/resources/application-identity-profile#media-object)).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProfileMedia {
    /// The image's URL.
    pub url: CompactString,
}

impl ProfileMedia {
    /// Creates a media object.
    #[must_use]
    pub fn new(url: impl Into<CompactString>) -> Self {
        Self { url: url.into() }
    }
}

/// A [custom stat field](https://docs.discord.com/developers/resources/application-identity-profile#dynamic-field-object):
/// a name and a typed value.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DynamicField {
    /// The field's name (max 100 characters).
    pub name: CompactString,
    /// The field's value.
    pub value: DynamicFieldValue,
}

impl DynamicField {
    /// Creates a field.
    #[must_use]
    pub fn new(name: impl Into<CompactString>, value: DynamicFieldValue) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

/// The value of a [`DynamicField`], by [field type](DynamicFieldType).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DynamicFieldValue {
    /// Text.
    String(CompactString),
    /// A number.
    Number(f64),
    /// An image.
    Media(ProfileMedia),
    /// A field type this version does not know; its value is not kept.
    Unknown(DynamicFieldType),
}

impl DynamicFieldValue {
    /// Returns the field type.
    #[must_use]
    pub const fn kind(&self) -> DynamicFieldType {
        match self {
            Self::String(_) => DynamicFieldType::String,
            Self::Number(_) => DynamicFieldType::Number,
            Self::Media(_) => DynamicFieldType::Media,
            Self::Unknown(kind) => *kind,
        }
    }
}

int_enum! {
    /// The [type of a dynamic field](https://docs.discord.com/developers/resources/application-identity-profile#dynamic-field-object-dynamic-field-types).
    pub enum DynamicFieldType: u8 {
        /// [`DynamicFieldValue::String`].
        String = 1,
        /// [`DynamicFieldValue::Number`].
        Number = 2,
        /// [`DynamicFieldValue::Media`].
        Media = 3,
    }
}

#[derive(Deserialize)]
struct RawDynamicField {
    #[serde(rename = "type")]
    kind: DynamicFieldType,
    name: CompactString,
    #[serde(default)]
    value: Option<JsonValue>,
}

impl<'de> Deserialize<'de> for DynamicField {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawDynamicField::deserialize(deserializer)?;
        let convert = |error: JsonValueError| D::Error::custom(format_args!("value: {error}"));
        let value = match raw.kind {
            DynamicFieldType::Unknown(_) => DynamicFieldValue::Unknown(raw.kind),
            kind => {
                let value = raw.value.ok_or_else(|| D::Error::missing_field("value"))?;
                match kind {
                    DynamicFieldType::String => {
                        DynamicFieldValue::String(value.to_type().map_err(convert)?)
                    }
                    DynamicFieldType::Number => {
                        DynamicFieldValue::Number(value.to_type().map_err(convert)?)
                    }
                    _ => DynamicFieldValue::Media(value.to_type().map_err(convert)?),
                }
            }
        };
        Ok(Self {
            name: raw.name,
            value,
        })
    }
}

impl Serialize for DynamicField {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(untagged)]
        enum Value<'a> {
            String(&'a str),
            Number(f64),
            Media(&'a ProfileMedia),
        }

        #[derive(Serialize)]
        struct Flat<'a> {
            #[serde(rename = "type")]
            kind: DynamicFieldType,
            name: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            value: Option<Value<'a>>,
        }

        Flat {
            kind: self.value.kind(),
            name: &self.name,
            value: match &self.value {
                DynamicFieldValue::String(value) => Some(Value::String(value)),
                DynamicFieldValue::Number(value) => Some(Value::Number(*value)),
                DynamicFieldValue::Media(value) => Some(Value::Media(value)),
                DynamicFieldValue::Unknown(_) => None,
            },
        }
        .serialize(serializer)
    }
}
