//! [`CommandManager`] and [`GuildCommandManager`] (D-244).

use std::future::Future;

use djs_cache::Backend;
use djs_collection::Collection;
use djs_model::{
    application_command::{ApplicationCommand, GuildCommandPermissions},
    id::{ApplicationId, CommandId, GenericId, GuildId},
    rest::{CreateApplicationCommand, EditApplicationCommand, GetApplicationCommands},
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

fn by_id(commands: Vec<ApplicationCommand>) -> Collection<CommandId, ApplicationCommand> {
    commands
        .into_iter()
        .map(|command| (command.id, command))
        .collect()
}

/// The bot's global application commands. Returned by [`Context::commands`].
///
/// The cache does not keep commands, so the [`djs-model`](djs_model) [`ApplicationCommand`]
/// is returned. Build the bodies with `djs_rs::builders` (`SlashCommandBuilder`, …).
///
/// discord.js counterpart: `ApplicationCommandManager` (`client.application.commands`).
#[derive(Debug)]
pub struct CommandManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    application_id: ApplicationId,
}

impl<B: Backend> Clone for CommandManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for CommandManager<'_, B> {}

impl<'a, B: Backend> CommandManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, application_id: ApplicationId) -> Self {
        Self {
            ctx,
            application_id,
        }
    }

    /// The application these commands belong to.
    #[must_use]
    pub const fn application_id(self) -> ApplicationId {
        self.application_id
    }

    /// A global command ([Get Global Application
    /// Command](djs_rest::Rest::get_global_application_command)).
    ///
    /// discord.js counterpart: `ApplicationCommandManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub fn fetch(
        self,
        command_id: CommandId,
    ) -> Fetch<
        ApplicationCommand,
        impl Future<Output = Result<ApplicationCommand, Error>> + Send + 'a,
    > {
        let Self {
            ctx,
            application_id,
        } = self;
        Fetch::new(None, async move {
            ctx.rest()
                .get_global_application_command(application_id, command_id)
                .await
        })
    }

    /// Every global command ([Get Global Application
    /// Commands](djs_rest::Rest::get_global_application_commands)).
    ///
    /// discord.js counterpart: `ApplicationCommandManager#fetch()`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list(
        self,
        query: &GetApplicationCommands,
    ) -> Result<Collection<CommandId, ApplicationCommand>, Error> {
        let commands = self
            .ctx
            .rest()
            .get_global_application_commands(self.application_id, query)
            .await?;
        Ok(by_id(commands))
    }

    /// Creates a global command, or replaces the one with the same name and type ([Create
    /// Global Application Command](djs_rest::Rest::create_global_application_command)).
    ///
    /// discord.js counterpart: `ApplicationCommandManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (at most 200 creations a day).
    pub async fn create(
        self,
        body: &CreateApplicationCommand,
    ) -> Result<ApplicationCommand, Error> {
        self.ctx
            .rest()
            .create_global_application_command(self.application_id, body)
            .await
    }

    /// Edits a global command ([Edit Global Application
    /// Command](djs_rest::Rest::edit_global_application_command)).
    ///
    /// discord.js counterpart: `ApplicationCommandManager#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn edit(
        self,
        command_id: CommandId,
        body: &EditApplicationCommand,
    ) -> Result<ApplicationCommand, Error> {
        self.ctx
            .rest()
            .edit_global_application_command(self.application_id, command_id, body)
            .await
    }

    /// Deletes a global command ([Delete Global Application
    /// Command](djs_rest::Rest::delete_global_application_command)).
    ///
    /// discord.js counterpart: `ApplicationCommandManager#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn delete(self, command_id: CommandId) -> Result<(), Error> {
        self.ctx
            .rest()
            .delete_global_application_command(self.application_id, command_id)
            .await
    }

    /// Replaces every global command with `commands` ([Bulk Overwrite Global Application
    /// Commands](djs_rest::Rest::bulk_overwrite_global_application_commands)); an empty slice
    /// deletes them all. Commands left unchanged keep their ID.
    ///
    /// discord.js counterpart: `ApplicationCommandManager#set`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn set(
        self,
        commands: &[CreateApplicationCommand],
    ) -> Result<Collection<CommandId, ApplicationCommand>, Error> {
        let commands = self
            .ctx
            .rest()
            .bulk_overwrite_global_application_commands(self.application_id, commands)
            .await?;
        Ok(by_id(commands))
    }
}

/// The bot's application commands in one guild, and the command permissions there.
/// Returned by [`Context::guild_commands`].
///
/// Guild commands update at once (global ones may take a while), which makes them the
/// usual choice while developing.
///
/// discord.js counterpart: `GuildApplicationCommandManager` (`guild.commands`).
#[derive(Debug)]
pub struct GuildCommandManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    application_id: ApplicationId,
    guild_id: GuildId,
}

impl<B: Backend> Clone for GuildCommandManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for GuildCommandManager<'_, B> {}

impl<'a, B: Backend> GuildCommandManager<'a, B> {
    pub(crate) const fn new(
        ctx: &'a Context<B>,
        application_id: ApplicationId,
        guild_id: GuildId,
    ) -> Self {
        Self {
            ctx,
            application_id,
            guild_id,
        }
    }

    /// The guild these commands belong to.
    #[must_use]
    pub const fn guild_id(self) -> GuildId {
        self.guild_id
    }

    /// A guild command ([Get Guild Application
    /// Command](djs_rest::Rest::get_guild_application_command)).
    ///
    /// discord.js counterpart: `GuildApplicationCommandManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub fn fetch(
        self,
        command_id: CommandId,
    ) -> Fetch<
        ApplicationCommand,
        impl Future<Output = Result<ApplicationCommand, Error>> + Send + 'a,
    > {
        let Self {
            ctx,
            application_id,
            guild_id,
        } = self;
        Fetch::new(None, async move {
            ctx.rest()
                .get_guild_application_command(application_id, guild_id, command_id)
                .await
        })
    }

    /// Every command of the guild ([Get Guild Application
    /// Commands](djs_rest::Rest::get_guild_application_commands)).
    ///
    /// discord.js counterpart: `GuildApplicationCommandManager#fetch()`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list(
        self,
        query: &GetApplicationCommands,
    ) -> Result<Collection<CommandId, ApplicationCommand>, Error> {
        let commands = self
            .ctx
            .rest()
            .get_guild_application_commands(self.application_id, self.guild_id, query)
            .await?;
        Ok(by_id(commands))
    }

    /// Creates a guild command, or replaces the one with the same name and type ([Create
    /// Guild Application Command](djs_rest::Rest::create_guild_application_command)).
    ///
    /// discord.js counterpart: `GuildApplicationCommandManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn create(
        self,
        body: &CreateApplicationCommand,
    ) -> Result<ApplicationCommand, Error> {
        self.ctx
            .rest()
            .create_guild_application_command(self.application_id, self.guild_id, body)
            .await
    }

    /// Edits a guild command ([Edit Guild Application
    /// Command](djs_rest::Rest::edit_guild_application_command)).
    ///
    /// discord.js counterpart: `GuildApplicationCommandManager#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn edit(
        self,
        command_id: CommandId,
        body: &EditApplicationCommand,
    ) -> Result<ApplicationCommand, Error> {
        self.ctx
            .rest()
            .edit_guild_application_command(self.application_id, self.guild_id, command_id, body)
            .await
    }

    /// Deletes a guild command ([Delete Guild Application
    /// Command](djs_rest::Rest::delete_guild_application_command)).
    ///
    /// discord.js counterpart: `GuildApplicationCommandManager#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn delete(self, command_id: CommandId) -> Result<(), Error> {
        self.ctx
            .rest()
            .delete_guild_application_command(self.application_id, self.guild_id, command_id)
            .await
    }

    /// Replaces every command of the guild with `commands` ([Bulk Overwrite Guild
    /// Application Commands](djs_rest::Rest::bulk_overwrite_guild_application_commands)).
    ///
    /// discord.js counterpart: `GuildApplicationCommandManager#set`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn set(
        self,
        commands: &[CreateApplicationCommand],
    ) -> Result<Collection<CommandId, ApplicationCommand>, Error> {
        let commands = self
            .ctx
            .rest()
            .bulk_overwrite_guild_application_commands(self.application_id, self.guild_id, commands)
            .await?;
        Ok(by_id(commands))
    }

    /// The command permissions set in the guild, by command (the application's ID for the
    /// permissions of every command) ([Get Guild Application Command
    /// Permissions](djs_rest::Rest::get_guild_application_command_permissions)). Editing them
    /// needs a user's `OAuth2` token, so it is left to
    /// [`Rest::edit_application_command_permissions`](djs_rest::Rest::edit_application_command_permissions).
    ///
    /// discord.js counterpart: `ApplicationCommandPermissionsManager#fetch({ guild })`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn permissions(
        self,
    ) -> Result<Collection<GenericId, GuildCommandPermissions>, Error> {
        let permissions = self
            .ctx
            .rest()
            .get_guild_application_command_permissions(self.application_id, self.guild_id)
            .await?;
        Ok(permissions
            .into_iter()
            .map(|permissions| (permissions.id, permissions))
            .collect())
    }

    /// The permissions of one command in the guild ([Get Application Command
    /// Permissions](djs_rest::Rest::get_application_command_permissions)), global or guild.
    ///
    /// discord.js counterpart: `ApplicationCommandPermissionsManager#fetch({ guild, command })`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (404 when none are set).
    pub async fn command_permissions(
        self,
        command_id: CommandId,
    ) -> Result<GuildCommandPermissions, Error> {
        self.ctx
            .rest()
            .get_application_command_permissions(self.application_id, self.guild_id, command_id)
            .await
    }
}
