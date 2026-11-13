//! [Application identity profile endpoints](https://docs.discord.com/developers/resources/application-identity-profile):
//! game stats shown in Game Stats Widgets on Discord profiles.
//!
//! Every endpoint takes the bot token of the application, and the user must have authorized
//! it with the `application_identities.write` scope.

use djs_model::{
    application_identity_profile::{ApplicationIdentity, ApplicationIdentityProfile},
    id::{ApplicationId, UserId},
    rest::{
        DeleteApplicationIdentity, GetApplicationIdentitiesByExternalId,
        UpdateApplicationIdentityProfile,
    },
};
use serde::Deserialize;

use crate::client::boxed;
use crate::{Error, Request, Rest, Route};

/// The response of the identity lists, unwrapped by the methods (C-065).
#[derive(Deserialize)]
struct Identities {
    identities: Vec<ApplicationIdentity>,
}

impl Rest {
    /// [Update Application Identity Profile](https://docs.discord.com/developers/resources/application-identity-profile#update-application-identity-profile):
    /// stores a user's game stats for their account `provider_issued_user_id` in the app's
    /// system. The first write creates the identity (provider type `NONE`) and returns its
    /// profile; later writes return `None` (D-135).
    ///
    /// discord.js counterpart: none.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, application_id: djs_model::id::ApplicationId, user_id: djs_model::id::UserId) -> Result<(), djs_rest::Error> {
    /// use djs_model::{application_identity_profile::ProfileData, rest::UpdateApplicationIdentityProfile};
    ///
    /// let mut body = UpdateApplicationIdentityProfile::default();
    /// body.username = Some("ferris".into());
    /// body.data = Some(ProfileData::default());
    /// rest.update_application_identity_profile(application_id, user_id, "player-42", &body)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `403` when the application may not store game stats; `400` for
    /// data over 10 KB or an external ID that belongs to another user or does not match the
    /// user's identity.
    pub fn update_application_identity_profile<'a>(
        &'a self,
        application_id: ApplicationId,
        user_id: UserId,
        provider_issued_user_id: &'a str,
        body: &'a UpdateApplicationIdentityProfile,
    ) -> impl Future<Output = Result<Option<ApplicationIdentityProfile>, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::UpdateApplicationIdentityProfile {
                    application_id,
                    user_id,
                    provider_issued_user_id,
                })
                .json(body)?,
            )
            .await
        })
    }

    /// [Get Application Identity Profile](https://docs.discord.com/developers/resources/application-identity-profile#get-application-identity-profile):
    /// returns the game stats stored for a user's account in the app's system.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_application_identity_profile<'a>(
        &'a self,
        application_id: ApplicationId,
        user_id: UserId,
        provider_issued_user_id: &'a str,
    ) -> impl Future<Output = Result<ApplicationIdentityProfile, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::GetApplicationIdentityProfile {
                application_id,
                user_id,
                provider_issued_user_id,
            }))
            .await
        })
    }

    /// [Get Application Identities by User ID](https://docs.discord.com/developers/resources/application-identity-profile#get-application-identities-by-user-id):
    /// returns a user's identities in the application (without profiles), e.g. to find the
    /// values [`delete_application_identity`](Self::delete_application_identity) needs.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_application_identities_by_user_id(
        &self,
        user_id: UserId,
        application_id: ApplicationId,
    ) -> impl Future<Output = Result<Vec<ApplicationIdentity>, Error>> + Send + '_ {
        boxed(async move {
            let identities: Identities = self
                .request(Request::new(Route::GetApplicationIdentitiesByUserId {
                    user_id,
                    application_id,
                }))
                .await?;
            Ok(identities.identities)
        })
    }

    /// [Get Application Identities by External ID](https://docs.discord.com/developers/resources/application-identity-profile#get-application-identities-by-external-id):
    /// returns the identities of the Discord user that an external account belongs to (the
    /// identities carry the [`user_id`](ApplicationIdentity::user_id)).
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_application_identities_by_external_id<'a>(
        &'a self,
        application_id: ApplicationId,
        provider_type: &'a str,
        provider_issued_user_id: &'a str,
        query: &'a GetApplicationIdentitiesByExternalId,
    ) -> impl Future<Output = Result<Vec<ApplicationIdentity>, Error>> + Send + 'a {
        boxed(async move {
            let identities: Identities = self
                .request(
                    Request::new(Route::GetApplicationIdentitiesByExternalId {
                        application_id,
                        provider_type,
                        provider_issued_user_id,
                    })
                    .query(query),
                )
                .await?;
            Ok(identities.identities)
        })
    }

    /// [Delete Application Identity](https://docs.discord.com/developers/resources/application-identity-profile#delete-application-identity):
    /// deletes one of a user's identities and its game stats — e.g. a stale external ID that
    /// blocks writing the current one. The user's last account-linking identity cannot be
    /// deleted.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn delete_application_identity<'a>(
        &'a self,
        user_id: UserId,
        application_id: ApplicationId,
        provider_type: &'a str,
        provider_issued_user_id: &'a str,
        body: &'a DeleteApplicationIdentity,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.request_ignoring_body(
                Request::new(Route::DeleteApplicationIdentity {
                    user_id,
                    application_id,
                    provider_type,
                    provider_issued_user_id,
                })
                .json(body)?,
            )
            .await
        })
    }
}
