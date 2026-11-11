//! [Application endpoints](https://docs.discord.com/developers/resources/application): the
//! bot's own application and its running Activity instances.

use djs_model::{
    application::{ActivityInstance, Application},
    id::ApplicationId,
    rest::EditCurrentApplication,
};

use crate::{Error, Request, Rest, Route};

impl Rest {
    /// [Get Current Application](https://docs.discord.com/developers/resources/application#get-current-application):
    /// returns the application of the bot, with its owner or team and approximate install
    /// counts.
    ///
    /// discord.js counterpart: `ClientApplication#fetch`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest) -> Result<(), djs_rest::Error> {
    /// let application = rest.get_current_application().await?;
    /// println!(
    ///     "{} is in about {:?} guilds",
    ///     application.name, application.approximate_guild_count
    /// );
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn get_current_application(&self) -> Result<Application, Error> {
        self.request(Request::new(Route::GetCurrentApplication))
            .await
    }

    /// [Edit Current Application](https://docs.discord.com/developers/resources/application#edit-current-application):
    /// changes the settings of the bot's application and returns it.
    ///
    /// discord.js counterpart: `ClientApplication#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50035` for a field Discord does not accept (for example an
    /// interactions endpoint URL that fails validation).
    pub async fn edit_current_application(
        &self,
        body: &EditCurrentApplication,
    ) -> Result<Application, Error> {
        self.request(Request::new(Route::EditCurrentApplication).json(body)?)
            .await
    }

    /// [Get Application Activity Instance](https://docs.discord.com/developers/resources/application#get-application-activity-instance):
    /// returns a running instance of the application's Activity, with the users connected to
    /// it — for example to check that a user who claims to be in it really is.
    ///
    /// discord.js counterpart: `ClientApplication#fetchActivityInstance`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `404` when the instance does not exist (anymore).
    pub async fn get_application_activity_instance(
        &self,
        application_id: ApplicationId,
        instance_id: &str,
    ) -> Result<ActivityInstance, Error> {
        self.request(Request::new(Route::GetApplicationActivityInstance {
            application_id,
            instance_id,
        }))
        .await
    }
}
