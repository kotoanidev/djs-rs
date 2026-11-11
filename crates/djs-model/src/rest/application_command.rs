//! Bodies and queries of the [application command endpoints](https://docs.discord.com/developers/interactions/application-commands#endpoints).

use std::collections::BTreeMap;

use compact_str::CompactString;
use serde::Serialize;

use crate::{
    application::IntegrationType,
    application_command::{CommandOption, CommandPermission, CommandType, EntryPointHandlerType},
    interaction::InteractionContextType,
    permissions::Permissions,
    util::{Locale, Patch},
};

/// The body of [Create Global Application Command](https://docs.discord.com/developers/interactions/application-commands#create-global-application-command)
/// and [Create Guild Application Command](https://docs.discord.com/developers/interactions/application-commands#create-guild-application-command),
/// and one entry of the bulk overwrite endpoints.
///
/// Creating a command with the name of an existing one (of the same type) updates it. The
/// deprecated `dm_permission` and `default_permission` fields are left out: use
/// [`contexts`](Self::contexts) and [`default_member_permissions`](Self::default_member_permissions).
///
/// discord.js counterpart: `ApplicationCommandData` / `RESTPostAPIApplicationCommandsJSONBody`.
///
/// # Examples
///
/// ```
/// use djs_model::rest::CreateApplicationCommand;
///
/// let ping = CreateApplicationCommand::chat_input("ping", "Checks that the bot is alive");
/// assert_eq!(
///     serde_json::to_string(&ping).unwrap(),
///     r#"{"name":"ping","description":"Checks that the bot is alive","type":1}"#
/// );
/// let report = CreateApplicationCommand::message("Report message");
/// assert_eq!(serde_json::to_string(&report).unwrap(), r#"{"name":"Report message","type":3}"#);
/// ```
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct CreateApplicationCommand {
    /// The [name](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-naming)
    /// (1-32 characters; lowercase without spaces for slash commands).
    pub name: CompactString,
    /// The name in other languages.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub name_localizations: BTreeMap<Locale, CompactString>,
    /// The description (1-100 characters) of slash and primary entry point commands; must be
    /// left out for user and message commands.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<CompactString>,
    /// The description in other languages.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub description_localizations: BTreeMap<Locale, CompactString>,
    /// The options of a slash command (max 25).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<CommandOption>,
    /// The permissions a member needs by default; [`Permissions::empty`] allows only
    /// administrators, `None` everyone. Guild administrators can change it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_member_permissions: Option<Permissions>,
    /// The installation contexts the command is available in (Discord's default: those the
    /// application supports).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub integration_types: Vec<IntegrationType>,
    /// The interaction contexts the command can be used in (Discord's default: all).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contexts: Vec<InteractionContextType>,
    /// The command's type (Discord's default: a slash command).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<CommandType>,
    /// Whether the command is age-restricted.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub nsfw: bool,
    /// Who handles a primary entry point command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler: Option<EntryPointHandlerType>,
}

impl CreateApplicationCommand {
    /// A command of type `kind` with only a name.
    #[must_use]
    pub fn new(name: impl Into<CompactString>, kind: CommandType) -> Self {
        Self {
            name: name.into(),
            name_localizations: BTreeMap::new(),
            description: None,
            description_localizations: BTreeMap::new(),
            options: Vec::new(),
            default_member_permissions: None,
            integration_types: Vec::new(),
            contexts: Vec::new(),
            kind: Some(kind),
            nsfw: false,
            handler: None,
        }
    }

    /// A slash command.
    #[must_use]
    pub fn chat_input(
        name: impl Into<CompactString>,
        description: impl Into<CompactString>,
    ) -> Self {
        Self {
            description: Some(description.into()),
            ..Self::new(name, CommandType::ChatInput)
        }
    }

    /// A user command (shown in the context menu of users).
    #[must_use]
    pub fn user(name: impl Into<CompactString>) -> Self {
        Self::new(name, CommandType::User)
    }

    /// A message command (shown in the context menu of messages).
    #[must_use]
    pub fn message(name: impl Into<CompactString>) -> Self {
        Self::new(name, CommandType::Message)
    }
}

/// The body of [Edit Global Application Command](https://docs.discord.com/developers/interactions/application-commands#edit-global-application-command)
/// and [Edit Guild Application Command](https://docs.discord.com/developers/interactions/application-commands#edit-guild-application-command).
///
/// Every field is optional; a field that is sent replaces the old value entirely (e.g. all
/// options). Nullable fields are [`Patch`]es.
///
/// discord.js counterpart: `ApplicationCommandManager#edit` data.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[non_exhaustive]
pub struct EditApplicationCommand {
    /// The name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The name in other languages.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub name_localizations: Patch<BTreeMap<Locale, CompactString>>,
    /// The description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<CompactString>,
    /// The description in other languages.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub description_localizations: Patch<BTreeMap<Locale, CompactString>>,
    /// The options (all of them).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<CommandOption>>,
    /// The permissions a member needs by default; [`Patch::Clear`] allows everyone.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub default_member_permissions: Patch<Permissions>,
    /// The installation contexts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_types: Option<Vec<IntegrationType>>,
    /// The interaction contexts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<InteractionContextType>>,
    /// Whether the command is age-restricted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,
    /// Who handles a primary entry point command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler: Option<EntryPointHandlerType>,
}

/// The query of [Get Global Application Commands](https://docs.discord.com/developers/interactions/application-commands#get-global-application-commands)
/// and [Get Guild Application Commands](https://docs.discord.com/developers/interactions/application-commands#get-guild-application-commands).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetApplicationCommands {
    /// Whether to return the full localization dictionaries instead of the `name_localized`
    /// and `description_localized` fields (Discord's default: no).
    pub with_localizations: Option<bool>,
}

/// The body of [Edit Application Command Permissions](https://docs.discord.com/developers/interactions/application-commands#edit-application-command-permissions).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct EditApplicationCommandPermissions {
    /// The permission overwrites of the command in the guild (max 100); they replace the
    /// existing ones.
    pub permissions: Vec<CommandPermission>,
}

impl EditApplicationCommandPermissions {
    /// Sets the permission overwrites to `permissions`.
    #[must_use]
    pub const fn new(permissions: Vec<CommandPermission>) -> Self {
        Self { permissions }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_edit() {
        let mut command = CreateApplicationCommand::chat_input("ban", "Bans a member");
        command.default_member_permissions = Some(Permissions::BAN_MEMBERS);
        command.contexts = vec![InteractionContextType::Guild];
        command
            .name_localizations
            .insert(Locale::Polish, "zbanuj".into());
        assert_eq!(
            serde_json::to_string(&command).unwrap(),
            concat!(
                r#"{"name":"ban","name_localizations":{"pl":"zbanuj"},"description":"Bans a member","#,
                r#""default_member_permissions":"4","contexts":[0],"type":1}"#
            )
        );

        let edit = EditApplicationCommand {
            description: Some("Bans someone".into()),
            default_member_permissions: Patch::Clear,
            name_localizations: Patch::Set(BTreeMap::new()),
            ..EditApplicationCommand::default()
        };
        assert_eq!(
            serde_json::to_string(&edit).unwrap(),
            r#"{"name_localizations":{},"description":"Bans someone","default_member_permissions":null}"#
        );
    }
}
