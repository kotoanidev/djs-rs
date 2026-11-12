//! [`ContextMenuCommandBuilder`].

use compact_str::CompactString;
use djs_model::rest::CreateApplicationCommand;

use super::validate;
use crate::ValidationError;

/// Builds a [context-menu command](https://docs.discord.com/developers/interactions/application-commands#user-commands):
/// an entry in the "Apps" menu of a user or of a message, with a name and no options.
///
/// The name can have 1-32 characters of any kind — upper-case letters, spaces and emoji
/// are fine (D-195). Register the result like a slash command.
///
/// discord.js counterpart: `ContextMenuCommandBuilder` (the type is chosen by the constructor
/// instead of `setType`).
///
/// # Examples
///
/// ```
/// use djs_builders::ContextMenuCommandBuilder;
/// use djs_model::{application_command::CommandType, util::Locale};
///
/// let report = ContextMenuCommandBuilder::message("Report message")
///     .name_localization(Locale::Polish, "Zgłoś wiadomość")
///     .build()
///     .unwrap();
/// assert_eq!(report.kind, Some(CommandType::Message));
/// assert_eq!(report.description, None);
///
/// let profile = ContextMenuCommandBuilder::user("Show profile").build().unwrap();
/// assert_eq!(profile.kind, Some(CommandType::User));
/// ```
#[derive(Clone, Debug, PartialEq)]
#[must_use = "a builder does nothing until `build` is called"]
pub struct ContextMenuCommandBuilder {
    command: CreateApplicationCommand,
}

impl ContextMenuCommandBuilder {
    /// Creates a command in the context menu of users, named `name` (1-32 characters).
    ///
    /// discord.js counterpart: `setType(ApplicationCommandType.User)`.
    pub fn user(name: impl Into<CompactString>) -> Self {
        Self {
            command: CreateApplicationCommand::user(name),
        }
    }

    /// Creates a command in the context menu of messages, named `name` (1-32 characters).
    ///
    /// discord.js counterpart: `setType(ApplicationCommandType.Message)`.
    pub fn message(name: impl Into<CompactString>) -> Self {
        Self {
            command: CreateApplicationCommand::message(name),
        }
    }

    name_localizations!(command);
    command_settings!();

    /// Checks the name and its translations (1-32 characters, not blank) and returns the
    /// command.
    ///
    /// discord.js counterpart: `ContextMenuCommandBuilder#toJSON`.
    ///
    /// # Errors
    ///
    /// [`ValidationError::Missing`] for a blank name, [`ValidationError::TooLong`] for one
    /// above 32 characters.
    pub fn build(self) -> Result<CreateApplicationCommand, ValidationError> {
        validate::context_menu_command(&self.command)?;
        Ok(self.command)
    }

    /// Returns the command without any check (D-180).
    #[must_use]
    pub fn build_unchecked(self) -> CreateApplicationCommand {
        self.command
    }
}
