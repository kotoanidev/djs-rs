//! [`SlashCommandBuilder`] and its subcommands.

use compact_str::CompactString;
use djs_model::{
    application_command::{CommandOption, SubCommandOption},
    rest::CreateApplicationCommand,
};

use super::validate;
use crate::ValidationError;

/// Builds a [slash command](https://docs.discord.com/developers/interactions/application-commands#slash-commands)
/// (`CHAT_INPUT`): a name users type after `/`, a description, and options, subcommands or
/// subcommand groups.
///
/// Register the result with `Rest::create_global_application_command` (or the guild or bulk
/// variants). As for every builder, setters only store and [`build`](Self::build) checks
/// everything (D-179); [`build_unchecked`](Self::build_unchecked) skips the checks (D-180).
///
/// `build` checks what Discord checks, as measured on the live API (D-194–D-196):
///
/// | Part | Rule |
/// |------|------|
/// | names (command, subcommands, options, translations) | 1-32 lower-case letters, numbers, `-`, `_`, `ʼ` |
/// | descriptions (and translations) | 1-100 characters, not blank |
/// | options per level | at most 25, unique names, required ones first |
/// | the command's own options | all subcommands / groups, or none |
/// | a group's options | subcommands only |
/// | a subcommand's options | no subcommands or groups |
/// | choices | at most 25; names 1-100 characters; string values at most 100 |
/// | `min_length` / `max_length` | 0-6000 / 1-6000, `max_length` ≥ `min_length` |
/// | integers / numbers | within ±(2^53 − 1) / ±2^53; `max_value` ≥ `min_value` |
/// | choices and autocomplete | not both |
/// | the whole command | at most 8000 characters |
///
/// Characters are Unicode code points; descriptions and choice texts are measured after
/// trimming. The 8000 total counts every name, description and string choice value, taking
/// the longest translation of each (D-195).
///
/// discord.js counterpart: `SlashCommandBuilder`. Unlike discord.js, options are added with one
/// [`option`](Self::option) method and mixing subcommands with options is reported by `build`
/// instead of the type system (D-192, D-193).
///
/// # Examples
///
/// ```
/// use djs_builders::{
///     SlashCommandBuilder, SlashCommandIntegerOption, SlashCommandStringOption,
///     SlashCommandUserOption,
/// };
/// use djs_model::{interaction::InteractionContextType, permissions::Permissions, util::Locale};
///
/// # fn main() -> Result<(), djs_builders::ValidationError> {
/// let ban = SlashCommandBuilder::new("ban", "Bans a member")
///     .name_localization(Locale::Polish, "zbanuj")
///     .description_localization(Locale::Polish, "Banuje członka")
///     .default_member_permissions(Permissions::BAN_MEMBERS)
///     .contexts([InteractionContextType::Guild])
///     .option(SlashCommandUserOption::new("user", "Who to ban").required(true))
///     .option(SlashCommandStringOption::new("reason", "Why").max_length(512))
///     .option(
///         SlashCommandIntegerOption::new("days", "Days of messages to delete")
///             .choice("None", 0)
///             .choice("A week", 7),
///     )
///     .build()?;
/// assert_eq!(ban.options.len(), 3);
/// # Ok(())
/// # }
/// ```
///
/// A problem is reported by `build`:
///
/// ```
/// use djs_builders::{SlashCommandBuilder, SlashCommandStringOption, ValidationError};
///
/// let error = SlashCommandBuilder::new("Ping", "Checks the bot").build().unwrap_err();
/// assert_eq!(error.to_string(), "name must be lower case: use 'p' instead of 'P'");
///
/// let error = SlashCommandBuilder::new("echo", "Repeats a text")
///     .option(SlashCommandStringOption::new("prefix", "Put before"))
///     .option(SlashCommandStringOption::new("text", "What to repeat").required(true))
///     .build()
///     .unwrap_err();
/// assert_eq!(error.field(), "options[1].required");
/// ```
#[derive(Clone, Debug, PartialEq)]
#[must_use = "a builder does nothing until `build` is called"]
pub struct SlashCommandBuilder {
    command: CreateApplicationCommand,
}

impl SlashCommandBuilder {
    /// Creates a slash command named `name` (1-32 lower-case letters, numbers, `-`, `_` or
    /// `ʼ`) and described by `description` (1-100 characters).
    ///
    /// discord.js counterpart: `new SlashCommandBuilder().setName(…).setDescription(…)`.
    pub fn new(name: impl Into<CompactString>, description: impl Into<CompactString>) -> Self {
        Self {
            command: CreateApplicationCommand::chat_input(name, description),
        }
    }

    name_localizations!(command);
    description_localizations!(command);
    command_settings!();

    /// Adds an option (a parameter users fill in), after those already added: any
    /// `SlashCommand…Option` of this crate, or a [`CommandOption`] of `djs-model`.
    ///
    /// A command has at most 25 options, with unique names and the required ones first. It
    /// either has options or subcommands, not both.
    ///
    /// discord.js counterpart: `addStringOption`, `addIntegerOption`, … (one per type).
    pub fn option(mut self, option: impl Into<CommandOption>) -> Self {
        self.command.options.push(option.into());
        self
    }

    /// Adds a subcommand (`/command subcommand`).
    ///
    /// discord.js counterpart: `addSubcommand`.
    pub fn subcommand(self, subcommand: SlashCommandSubcommandBuilder) -> Self {
        self.option(subcommand)
    }

    /// Adds a group of subcommands (`/command group subcommand`).
    ///
    /// discord.js counterpart: `addSubcommandGroup`.
    pub fn subcommand_group(self, group: SlashCommandSubcommandGroupBuilder) -> Self {
        self.option(group)
    }

    /// Checks the command against Discord's rules (see the [type docs](Self)) and returns it.
    ///
    /// discord.js counterpart: `SlashCommandBuilder#toJSON`.
    ///
    /// # Errors
    ///
    /// The first [`ValidationError`] found, checking the name, its translations, the
    /// description, its translations, then each option in order (with what it contains), and
    /// finally the 8000-character total ([`TooLong`](ValidationError::TooLong) with the field
    /// `command`).
    pub fn build(self) -> Result<CreateApplicationCommand, ValidationError> {
        validate::slash_command(&self.command)?;
        Ok(self.command)
    }

    /// Returns the command without any check (D-180).
    #[must_use]
    pub fn build_unchecked(self) -> CreateApplicationCommand {
        self.command
    }
}

/// Builds a [subcommand](https://docs.discord.com/developers/interactions/application-commands#subcommands-and-subcommand-groups)
/// of a slash command (`/command subcommand`), with its own options.
///
/// Add it with [`SlashCommandBuilder::subcommand`] or
/// [`SlashCommandSubcommandGroupBuilder::subcommand`]. [`build`](Self::build) checks it on its
/// own, with paths starting at its fields.
///
/// discord.js counterpart: `SlashCommandSubcommandBuilder`.
///
/// # Examples
///
/// ```
/// use djs_builders::{SlashCommandBuilder, SlashCommandStringOption, SlashCommandSubcommandBuilder};
///
/// let tag = SlashCommandBuilder::new("tag", "Manages tags")
///     .subcommand(
///         SlashCommandSubcommandBuilder::new("show", "Shows a tag")
///             .option(SlashCommandStringOption::new("name", "The tag").required(true)),
///     )
///     .subcommand(SlashCommandSubcommandBuilder::new("list", "Lists every tag"))
///     .build()
///     .unwrap();
/// assert_eq!(tag.options[1].name(), "list");
/// ```
#[derive(Clone, Debug, PartialEq)]
#[must_use = "a builder does nothing until it is added to a command or built"]
pub struct SlashCommandSubcommandBuilder {
    option: SubCommandOption,
}

impl SlashCommandSubcommandBuilder {
    /// Creates a subcommand named `name` (1-32 lower-case letters, numbers, `-`, `_` or `ʼ`)
    /// and described by `description` (1-100 characters).
    pub fn new(name: impl Into<CompactString>, description: impl Into<CompactString>) -> Self {
        Self {
            option: SubCommandOption::new(name, description),
        }
    }

    name_localizations!(option);
    description_localizations!(option);

    /// Adds an option of the subcommand (at most 25; not a subcommand or group).
    ///
    /// discord.js counterpart: `addStringOption`, `addIntegerOption`, … (one per type).
    pub fn option(mut self, option: impl Into<CommandOption>) -> Self {
        self.option.options.push(option.into());
        self
    }

    /// Checks the subcommand on its own and returns it as a [`CommandOption`].
    ///
    /// # Errors
    ///
    /// The first [`ValidationError`] found, as in [`SlashCommandBuilder::build`] (without the
    /// 8000-character total, which applies to the whole command).
    pub fn build(self) -> Result<CommandOption, ValidationError> {
        let option = CommandOption::from(self);
        validate::option_alone(&option)?;
        Ok(option)
    }

    /// Returns the subcommand without any check (D-180).
    #[must_use]
    pub fn build_unchecked(self) -> CommandOption {
        self.into()
    }
}

impl From<SlashCommandSubcommandBuilder> for CommandOption {
    fn from(subcommand: SlashCommandSubcommandBuilder) -> Self {
        Self::SubCommand(subcommand.option)
    }
}

/// Builds a [group of subcommands](https://docs.discord.com/developers/interactions/application-commands#subcommands-and-subcommand-groups)
/// of a slash command (`/command group subcommand`).
///
/// Add it with [`SlashCommandBuilder::subcommand_group`]. A group contains only subcommands.
///
/// discord.js counterpart: `SlashCommandSubcommandGroupBuilder`.
///
/// # Examples
///
/// ```
/// use djs_builders::{
///     SlashCommandBuilder, SlashCommandStringOption, SlashCommandSubcommandBuilder,
///     SlashCommandSubcommandGroupBuilder,
/// };
///
/// let config = SlashCommandBuilder::new("config", "Bot settings")
///     .subcommand_group(
///         SlashCommandSubcommandGroupBuilder::new("set", "Changes a setting").subcommand(
///             SlashCommandSubcommandBuilder::new("prefix", "Sets the prefix")
///                 .option(SlashCommandStringOption::new("value", "The new prefix").required(true)),
///         ),
///     )
///     .build()
///     .unwrap();
/// assert_eq!(config.options[0].options()[0].name(), "prefix");
/// ```
#[derive(Clone, Debug, PartialEq)]
#[must_use = "a builder does nothing until it is added to a command or built"]
pub struct SlashCommandSubcommandGroupBuilder {
    option: SubCommandOption,
}

impl SlashCommandSubcommandGroupBuilder {
    /// Creates a group named `name` (1-32 lower-case letters, numbers, `-`, `_` or `ʼ`) and
    /// described by `description` (1-100 characters).
    pub fn new(name: impl Into<CompactString>, description: impl Into<CompactString>) -> Self {
        Self {
            option: SubCommandOption::new(name, description),
        }
    }

    name_localizations!(option);
    description_localizations!(option);

    /// Adds a subcommand to the group (at most 25).
    ///
    /// discord.js counterpart: `addSubcommand`.
    pub fn subcommand(mut self, subcommand: SlashCommandSubcommandBuilder) -> Self {
        self.option.options.push(subcommand.into());
        self
    }

    /// Checks the group on its own and returns it as a [`CommandOption`].
    ///
    /// # Errors
    ///
    /// The first [`ValidationError`] found, as in [`SlashCommandBuilder::build`] (without the
    /// 8000-character total).
    pub fn build(self) -> Result<CommandOption, ValidationError> {
        let option = CommandOption::from(self);
        validate::option_alone(&option)?;
        Ok(option)
    }

    /// Returns the group without any check (D-180).
    #[must_use]
    pub fn build_unchecked(self) -> CommandOption {
        self.into()
    }
}

impl From<SlashCommandSubcommandGroupBuilder> for CommandOption {
    fn from(group: SlashCommandSubcommandGroupBuilder) -> Self {
        Self::SubCommandGroup(group.option)
    }
}
