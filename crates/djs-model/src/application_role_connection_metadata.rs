//! [Application role connection metadata](https://docs.discord.com/developers/resources/application-role-connection-metadata):
//! the fields an app defines for linked roles.
//!
//! An app with a `role_connections_verification_url` declares up to 5 metadata records. Guilds
//! then require values for them on a role (e.g. "account age ≥ 30 days"), and the app stores
//! each user's values as an [`ApplicationRoleConnection`](crate::user::ApplicationRoleConnection).

use std::collections::BTreeMap;

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{macros::int_enum, util::Locale};

/// An [application role connection metadata record](https://docs.discord.com/developers/resources/application-role-connection-metadata#application-role-connection-metadata-object):
/// one field that guilds can compare against when granting a linked role.
///
/// discord.js counterpart: `ApplicationRoleConnectionMetadata`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ApplicationRoleConnectionMetadata {
    /// How the user's value is compared with the guild's configured value.
    #[serde(rename = "type")]
    pub kind: MetadataType,
    /// The dictionary key of the field (`a-z`, `0-9` and `_`; 1-50 characters).
    pub key: CompactString,
    /// The field's name (1-100 characters).
    pub name: CompactString,
    /// Translations of the name, keyed by locale (D-058).
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "crate::serde_util::null_as_default"
    )]
    pub name_localizations: BTreeMap<Locale, CompactString>,
    /// The field's description (1-200 characters).
    pub description: CompactString,
    /// Translations of the description, keyed by locale (D-058).
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "crate::serde_util::null_as_default"
    )]
    pub description_localizations: BTreeMap<Locale, CompactString>,
}

int_enum! {
    /// The [comparison](https://docs.discord.com/developers/resources/application-role-connection-metadata#application-role-connection-metadata-object-application-role-connection-metadata-type)
    /// of an [`ApplicationRoleConnectionMetadata`] record.
    ///
    /// The user's value is stored as a string in
    /// [`ApplicationRoleConnection::metadata`](crate::user::ApplicationRoleConnection::metadata):
    /// an integer for the integer and boolean (`1`/`0`) comparisons, an ISO 8601 timestamp for
    /// the date-time ones (the guild configures a number of days before today).
    ///
    /// discord.js counterpart: `ApplicationRoleConnectionMetadataType`.
    #[doc(alias = "ApplicationRoleConnectionMetadataType")]
    pub enum MetadataType: u8 {
        /// The user's integer is less than or equal to the guild's.
        IntegerLessThanOrEqual = 1,
        /// The user's integer is greater than or equal to the guild's.
        IntegerGreaterThanOrEqual = 2,
        /// The user's integer is equal to the guild's.
        IntegerEqual = 3,
        /// The user's integer is not equal to the guild's.
        IntegerNotEqual = 4,
        /// The user's date-time is at most the guild's number of days before today.
        DatetimeLessThanOrEqual = 5,
        /// The user's date-time is at least the guild's number of days before today.
        DatetimeGreaterThanOrEqual = 6,
        /// The user's boolean is `1` (true).
        BooleanEqual = 7,
        /// The user's boolean is not `1` (true).
        BooleanNotEqual = 8,
    }
}
