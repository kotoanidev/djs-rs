//! [Application role connection metadata endpoints](https://docs.discord.com/developers/resources/application-role-connection-metadata):
//! the fields of an app's linked roles.
//!
//! Each user's values are set with
//! [`update_current_user_application_role_connection`](Rest::update_current_user_application_role_connection),
//! using their access token.

use djs_model::{
    application_role_connection_metadata::ApplicationRoleConnectionMetadata, id::ApplicationId,
};

use crate::client::boxed;
use crate::{Error, Request, Rest, Route};

impl Rest {
    /// [Get Application Role Connection Metadata Records](https://docs.discord.com/developers/resources/application-role-connection-metadata#get-application-role-connection-metadata-records):
    /// returns the fields guilds can require for the application's linked roles.
    ///
    /// discord.js counterpart: `ClientApplication#fetchRoleConnectionMetadataRecords`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_application_role_connection_metadata_records(
        &self,
        application_id: ApplicationId,
    ) -> impl Future<Output = Result<Vec<ApplicationRoleConnectionMetadata>, Error>> + Send + '_
    {
        boxed(async move {
            self.call(Request::new(
                Route::GetApplicationRoleConnectionMetadataRecords { application_id },
            ))
            .await
        })
    }

    /// [Update Application Role Connection Metadata Records](https://docs.discord.com/developers/resources/application-role-connection-metadata#update-application-role-connection-metadata-records):
    /// replaces the fields of the application's linked roles (up to 5) and returns them.
    ///
    /// discord.js counterpart: `ClientApplication#editRoleConnectionMetadataRecords`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, application_id: djs_model::id::ApplicationId) -> Result<(), djs_rest::Error> {
    /// use djs_model::application_role_connection_metadata::{
    ///     ApplicationRoleConnectionMetadata, MetadataType,
    /// };
    ///
    /// let records = [ApplicationRoleConnectionMetadata::new(
    ///     MetadataType::DatetimeGreaterThanOrEqual,
    ///     "joined",
    ///     "Member since",
    ///     "Days since the player joined",
    /// )];
    /// rest.update_application_role_connection_metadata_records(application_id, &records)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50035` for more than 5 records or an invalid key.
    pub fn update_application_role_connection_metadata_records<'a>(
        &'a self,
        application_id: ApplicationId,
        records: &'a [ApplicationRoleConnectionMetadata],
    ) -> impl Future<Output = Result<Vec<ApplicationRoleConnectionMetadata>, Error>> + Send + 'a
    {
        boxed(async move {
            self.call(
                Request::new(Route::UpdateApplicationRoleConnectionMetadataRecords {
                    application_id,
                })
                .json(records)?,
            )
            .await
        })
    }
}
