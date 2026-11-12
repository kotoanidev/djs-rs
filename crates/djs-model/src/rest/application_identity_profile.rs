//! Bodies and queries of the [application identity profile endpoints](https://docs.discord.com/developers/resources/application-identity-profile).

use compact_str::CompactString;
use serde::Serialize;

use crate::application_identity_profile::ProfileData;

/// The body of [Update Application Identity Profile](https://docs.discord.com/developers/resources/application-identity-profile#update-application-identity-profile).
///
/// Every field is optional. [`data`](Self::data) replaces the stored stats as a whole, so send
/// all of them every time.
///
/// Contains floats, so it is `PartialEq` but not `Eq` (D-064).
///
/// # Examples
///
/// ```
/// use djs_model::rest::UpdateApplicationIdentityProfile;
///
/// let mut body = UpdateApplicationIdentityProfile::default();
/// body.username = Some("ferris#1".into());
/// assert_eq!(serde_json::to_string(&body)?, r#"{"username":"ferris#1"}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[non_exhaustive]
pub struct UpdateApplicationIdentityProfile {
    /// The user's username in the app's system (up to 1024 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<CompactString>,
    /// The complete game stats (at most 10 KB serialized, up to 30 dynamic fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ProfileData>,
}

/// The query of [Get Application Identities by External ID](https://docs.discord.com/developers/resources/application-identity-profile#get-application-identities-by-external-id).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetApplicationIdentitiesByExternalId {
    /// Disambiguates identities with the same provider type and external user ID.
    pub provider_id: Option<CompactString>,
}

/// The optional body of [Delete Application Identity](https://docs.discord.com/developers/resources/application-identity-profile#delete-application-identity).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct DeleteApplicationIdentity {
    /// Disambiguates identities with the same provider type and external user ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<CompactString>,
}
