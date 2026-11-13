//! [Application command endpoints](https://docs.discord.com/developers/interactions/application-commands#endpoints):
//! registering slash, user and message commands, globally or in one guild, and their
//! per-guild permissions.
//!
//! Global commands are available in every guild and DM the application is installed in;
//! guild commands only in their guild. An application can have 100 global slash commands, 5
//! user and 5 message commands (and as many again per guild). Creating more than 200
//! commands a day in one guild is rate limited, so registering on every start is best done
//! with a single bulk overwrite.

use djs_model::{
    application_command::{ApplicationCommand, GuildCommandPermissions},
    id::{ApplicationId, CommandId, GuildId},
    rest::{
        CreateApplicationCommand, EditApplicationCommand, EditApplicationCommandPermissions,
        GetApplicationCommands,
    },
};

use crate::client::boxed;
use crate::{Error, Request, Rest, Route};

impl Rest {
    /// [Get Global Application Commands](https://docs.discord.com/developers/interactions/application-commands#get-global-application-commands):
    /// returns the application's global commands.
    ///
    /// discord.js counterpart: `ApplicationCommandManager#fetch`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_global_application_commands<'a>(
        &'a self,
        application_id: ApplicationId,
        query: &'a GetApplicationCommands,
    ) -> impl Future<Output = Result<Vec<ApplicationCommand>, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::GetGlobalApplicationCommands { application_id }).query(query),
            )
            .await
        })
    }

    /// [Create Global Application Command](https://docs.discord.com/developers/interactions/application-commands#create-global-application-command):
    /// registers a global command, or updates the one with the same name and type. Discord
    /// answers `201 Created` for a new command and `200 OK` for an update.
    ///
    /// discord.js counterpart: `ApplicationCommandManager#create`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, application_id: djs_model::id::ApplicationId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::CreateApplicationCommand;
    ///
    /// let ping = CreateApplicationCommand::chat_input("ping", "Checks that the bot is alive");
    /// let command = rest
    ///     .create_global_application_command(application_id, &ping)
    ///     .await?;
    /// println!("registered /ping as {}", command.id);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30032` at the maximum number of commands, `50035` for an
    /// invalid command (see [`ApiError::field_errors`](crate::ApiError::field_errors)).
    pub fn create_global_application_command<'a>(
        &'a self,
        application_id: ApplicationId,
        body: &'a CreateApplicationCommand,
    ) -> impl Future<Output = Result<ApplicationCommand, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::CreateGlobalApplicationCommand { application_id })
                    .json(body)?,
            )
            .await
        })
    }

    /// [Get Global Application Command](https://docs.discord.com/developers/interactions/application-commands#get-global-application-command):
    /// returns one global command.
    ///
    /// discord.js counterpart: `ApplicationCommandManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10063` Unknown Application Command.
    pub fn get_global_application_command(
        &self,
        application_id: ApplicationId,
        command_id: CommandId,
    ) -> impl Future<Output = Result<ApplicationCommand, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGlobalApplicationCommand {
                application_id,
                command_id,
            }))
            .await
        })
    }

    /// [Edit Global Application Command](https://docs.discord.com/developers/interactions/application-commands#edit-global-application-command):
    /// changes the fields of a global command that are set in `body`.
    ///
    /// discord.js counterpart: `ApplicationCommandManager#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn edit_global_application_command<'a>(
        &'a self,
        application_id: ApplicationId,
        command_id: CommandId,
        body: &'a EditApplicationCommand,
    ) -> impl Future<Output = Result<ApplicationCommand, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::EditGlobalApplicationCommand {
                    application_id,
                    command_id,
                })
                .json(body)?,
            )
            .await
        })
    }

    /// [Delete Global Application Command](https://docs.discord.com/developers/interactions/application-commands#delete-global-application-command):
    /// unregisters a global command.
    ///
    /// discord.js counterpart: `ApplicationCommandManager#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn delete_global_application_command(
        &self,
        application_id: ApplicationId,
        command_id: CommandId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::DeleteGlobalApplicationCommand {
                application_id,
                command_id,
            }))
            .await
        })
    }

    /// [Bulk Overwrite Global Application Commands](https://docs.discord.com/developers/interactions/application-commands#bulk-overwrite-global-application-commands):
    /// replaces all global commands with `commands`, and returns them.
    ///
    /// Commands not in the list are deleted (an empty list deletes all); listed commands keep
    /// their IDs when their name and type match an existing command. This is the usual way
    /// to register commands when a bot starts.
    ///
    /// discord.js counterpart: `ApplicationCommandManager#set`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, application_id: djs_model::id::ApplicationId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::CreateApplicationCommand;
    ///
    /// let commands = [
    ///     CreateApplicationCommand::chat_input("ping", "Checks that the bot is alive"),
    ///     CreateApplicationCommand::user("Show avatar"),
    /// ];
    /// rest.bulk_overwrite_global_application_commands(application_id, &commands)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn bulk_overwrite_global_application_commands<'a>(
        &'a self,
        application_id: ApplicationId,
        commands: &'a [CreateApplicationCommand],
    ) -> impl Future<Output = Result<Vec<ApplicationCommand>, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::BulkOverwriteGlobalApplicationCommands { application_id })
                    .json(commands)?,
            )
            .await
        })
    }

    /// [Get Guild Application Commands](https://docs.discord.com/developers/interactions/application-commands#get-guild-application-commands):
    /// returns the application's commands in a guild (not its global commands).
    ///
    /// discord.js counterpart: `GuildApplicationCommandManager#fetch`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_application_commands<'a>(
        &'a self,
        application_id: ApplicationId,
        guild_id: GuildId,
        query: &'a GetApplicationCommands,
    ) -> impl Future<Output = Result<Vec<ApplicationCommand>, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::GetGuildApplicationCommands {
                    application_id,
                    guild_id,
                })
                .query(query),
            )
            .await
        })
    }

    /// [Create Guild Application Command](https://docs.discord.com/developers/interactions/application-commands#create-guild-application-command):
    /// registers a command in one guild, or updates the one with the same name and type.
    /// Guild commands are available at once, which makes them handy while developing.
    ///
    /// discord.js counterpart: `GuildApplicationCommandManager#create`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50001` Missing Access when the application is not in the guild
    /// with the `applications.commands` scope.
    pub fn create_guild_application_command<'a>(
        &'a self,
        application_id: ApplicationId,
        guild_id: GuildId,
        body: &'a CreateApplicationCommand,
    ) -> impl Future<Output = Result<ApplicationCommand, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::CreateGuildApplicationCommand {
                    application_id,
                    guild_id,
                })
                .json(body)?,
            )
            .await
        })
    }

    /// [Get Guild Application Command](https://docs.discord.com/developers/interactions/application-commands#get-guild-application-command):
    /// returns one guild command.
    ///
    /// discord.js counterpart: `GuildApplicationCommandManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10063` Unknown Application Command.
    pub fn get_guild_application_command(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
        command_id: CommandId,
    ) -> impl Future<Output = Result<ApplicationCommand, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildApplicationCommand {
                application_id,
                guild_id,
                command_id,
            }))
            .await
        })
    }

    /// [Edit Guild Application Command](https://docs.discord.com/developers/interactions/application-commands#edit-guild-application-command):
    /// changes the fields of a guild command that are set in `body`.
    ///
    /// discord.js counterpart: `GuildApplicationCommandManager#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn edit_guild_application_command<'a>(
        &'a self,
        application_id: ApplicationId,
        guild_id: GuildId,
        command_id: CommandId,
        body: &'a EditApplicationCommand,
    ) -> impl Future<Output = Result<ApplicationCommand, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::EditGuildApplicationCommand {
                    application_id,
                    guild_id,
                    command_id,
                })
                .json(body)?,
            )
            .await
        })
    }

    /// [Delete Guild Application Command](https://docs.discord.com/developers/interactions/application-commands#delete-guild-application-command):
    /// unregisters a guild command.
    ///
    /// discord.js counterpart: `GuildApplicationCommandManager#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn delete_guild_application_command(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
        command_id: CommandId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::DeleteGuildApplicationCommand {
                application_id,
                guild_id,
                command_id,
            }))
            .await
        })
    }

    /// [Bulk Overwrite Guild Application Commands](https://docs.discord.com/developers/interactions/application-commands#bulk-overwrite-guild-application-commands):
    /// replaces all of the application's commands in a guild with `commands`, and returns
    /// them; see
    /// [`bulk_overwrite_global_application_commands`](Self::bulk_overwrite_global_application_commands).
    ///
    /// discord.js counterpart: `GuildApplicationCommandManager#set`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn bulk_overwrite_guild_application_commands<'a>(
        &'a self,
        application_id: ApplicationId,
        guild_id: GuildId,
        commands: &'a [CreateApplicationCommand],
    ) -> impl Future<Output = Result<Vec<ApplicationCommand>, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::BulkOverwriteGuildApplicationCommands {
                    application_id,
                    guild_id,
                })
                .json(commands)?,
            )
            .await
        })
    }

    /// [Get Guild Application Command Permissions](https://docs.discord.com/developers/interactions/application-commands#get-guild-application-command-permissions):
    /// returns the permission overwrites of the application's commands in a guild (only
    /// commands that have overwrites).
    ///
    /// discord.js counterpart: `ApplicationCommandPermissionsManager#fetch`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_application_command_permissions(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<GuildCommandPermissions>, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildApplicationCommandPermissions {
                application_id,
                guild_id,
            }))
            .await
        })
    }

    /// [Get Application Command Permissions](https://docs.discord.com/developers/interactions/application-commands#get-application-command-permissions):
    /// returns the permission overwrites of one command in a guild.
    ///
    /// `command_id` may also be the application ID, for the overwrites that apply to all of
    /// its commands without their own.
    ///
    /// discord.js counterpart: `ApplicationCommandPermissionsManager#fetch({ command })`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10066` Unknown Application Command Permissions when the
    /// command has none.
    pub fn get_application_command_permissions(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
        command_id: CommandId,
    ) -> impl Future<Output = Result<GuildCommandPermissions, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetApplicationCommandPermissions {
                application_id,
                guild_id,
                command_id,
            }))
            .await
        })
    }

    /// [Edit Application Command Permissions](https://docs.discord.com/developers/interactions/application-commands#edit-application-command-permissions):
    /// replaces the permission overwrites of one command in a guild.
    ///
    /// Discord rejects bot tokens here: the client must be built with
    /// [`RestBuilder::bearer_token`](crate::RestBuilder::bearer_token), holding the `OAuth2`
    /// access token (scope `applications.commands.permissions.update`) of a user who can
    /// manage the guild and the command (D-108).
    ///
    /// discord.js counterpart: `ApplicationCommandPermissionsManager#set`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(access_token: String, application_id: djs_model::id::ApplicationId, guild_id: djs_model::id::GuildId, command_id: djs_model::id::CommandId, role: djs_model::id::RoleId) -> Result<(), djs_rest::Error> {
    /// use djs_model::{
    ///     application_command::{CommandPermission, CommandPermissionType},
    ///     rest::EditApplicationCommandPermissions,
    /// };
    /// use djs_rest::Rest;
    ///
    /// let user = Rest::builder().bearer_token(access_token).build();
    /// let allow_role = CommandPermission::new(role.cast(), CommandPermissionType::Role, true);
    /// let body = EditApplicationCommandPermissions::new(vec![allow_role]);
    /// user.edit_application_command_permissions(application_id, guild_id, command_id, &body)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `401 Unauthorized` with a bot token, `50001` Missing Access
    /// when the user cannot manage the guild.
    pub fn edit_application_command_permissions<'a>(
        &'a self,
        application_id: ApplicationId,
        guild_id: GuildId,
        command_id: CommandId,
        body: &'a EditApplicationCommandPermissions,
    ) -> impl Future<Output = Result<GuildCommandPermissions, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::EditApplicationCommandPermissions {
                    application_id,
                    guild_id,
                    command_id,
                })
                .json(body)?,
            )
            .await
        })
    }
}
