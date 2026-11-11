//! [Application commands](https://docs.discord.com/developers/interactions/application-commands):
//! slash commands, user and message context menu commands, and Activity entry points.
//!
//! These are the commands as Discord returns them (`GET .../commands`). Their options are an
//! enum keyed by option type (D-069), with typed choices.

use std::collections::BTreeMap;

use compact_str::CompactString;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as DeError, Unexpected, Visitor},
};

use crate::{
    application::IntegrationType,
    channel::ChannelType,
    id::{ApplicationId, CommandId, CommandVersionId, GenericId, GuildId},
    interaction::InteractionContextType,
    macros::int_enum,
    permissions::Permissions,
    util::Locale,
};

/// An [application command](https://docs.discord.com/developers/interactions/application-commands#application-command-object).
///
/// Contains floats (number option bounds and choices), so it is `PartialEq` but not `Eq`
/// (D-064).
///
/// discord.js counterpart: `ApplicationCommand`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ApplicationCommand {
    /// The command's ID.
    pub id: CommandId,
    /// The kind of command (Discord's default: [`ChatInput`](CommandType::ChatInput)).
    #[serde(rename = "type", default)]
    pub kind: CommandType,
    /// The application that owns the command.
    pub application_id: ApplicationId,
    /// The guild of a guild command (`None` for global commands).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The command's name (1-32 characters; lowercase for slash commands).
    pub name: CompactString,
    /// Translations of the name, keyed by locale (D-058).
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "crate::serde_util::null_as_default"
    )]
    pub name_localizations: BTreeMap<Locale, CompactString>,
    /// The description (1-100 characters; empty for user and message commands).
    #[serde(default)]
    pub description: CompactString,
    /// Translations of the description, keyed by locale (D-058).
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "crate::serde_util::null_as_default"
    )]
    pub description_localizations: BTreeMap<Locale, CompactString>,
    /// The name in the requester's locale, returned instead of
    /// [`name_localizations`](Self::name_localizations) by the endpoints that list commands
    /// (unless asked for the full translations).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_localized: Option<CompactString>,
    /// The description in the requester's locale (see [`name_localized`](Self::name_localized)).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_localized: Option<CompactString>,
    /// The parameters of a slash command (max 25).
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "crate::serde_util::null_as_default"
    )]
    pub options: Vec<CommandOption>,
    /// The permissions a member needs by default (`None`: everyone; empty: administrators
    /// only). Guild admins can override them.
    #[serde(default)]
    pub default_member_permissions: Option<Permissions>,
    /// Whether the command is available in DMs. **Deprecated** by Discord in favor of
    /// [`contexts`](Self::contexts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dm_permission: Option<bool>,
    /// Whether the command is enabled by default. **Deprecated** by Discord in favor of
    /// [`default_member_permissions`](Self::default_member_permissions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_permission: Option<bool>,
    /// Whether the command is age-restricted.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub nsfw: bool,
    /// The installation contexts the command is available in.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "crate::serde_util::null_as_default"
    )]
    pub integration_types: Vec<IntegrationType>,
    /// The interaction contexts the command can be used in.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "crate::serde_util::null_as_default"
    )]
    pub contexts: Vec<InteractionContextType>,
    /// An auto-incrementing version, updated on substantial changes.
    pub version: CommandVersionId,
    /// Who handles a primary entry point command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handler: Option<EntryPointHandlerType>,
}

int_enum! {
    /// The [type of an application command](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-types).
    ///
    /// discord.js counterpart: `ApplicationCommandType`.
    #[doc(alias = "ApplicationCommandType")]
    #[derive(Default)]
    pub enum CommandType: u8 {
        /// A slash command, shown when typing `/`.
        #[default]
        ChatInput = 1,
        /// A context menu command on users.
        User = 2,
        /// A context menu command on messages.
        Message = 3,
        /// The primary way to launch the app's Activity.
        PrimaryEntryPoint = 4,
    }
}

int_enum! {
    /// [Who handles a primary entry point command](https://docs.discord.com/developers/interactions/application-commands#application-command-object-entry-point-command-handler-types).
    ///
    /// discord.js counterpart: `EntryPointCommandHandlerType`.
    #[doc(alias = "EntryPointCommandHandlerType")]
    pub enum EntryPointHandlerType: u8 {
        /// The app handles the interaction with its interaction token.
        AppHandler = 1,
        /// Discord launches the Activity and sends a follow-up message itself.
        DiscordLaunchActivity = 2,
    }
}

/// An [option (parameter) of a slash command](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-option-structure),
/// one variant per option type (D-069).
///
/// discord.js counterpart: `ApplicationCommandOption` (the `ApplicationCommand*Option` types).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
#[doc(alias = "ApplicationCommandOption")]
pub enum CommandOption {
    /// A subcommand, with its own options.
    SubCommand(SubCommandOption),
    /// A group of subcommands (its options are subcommands).
    SubCommandGroup(SubCommandOption),
    /// A string.
    String(StringOption),
    /// An integer between -2^53+1 and 2^53-1.
    Integer(IntegerOption),
    /// A boolean.
    Boolean(BasicOption),
    /// A user.
    User(BasicOption),
    /// A channel (including categories).
    Channel(ChannelOption),
    /// A role.
    Role(BasicOption),
    /// A user or a role.
    Mentionable(BasicOption),
    /// A double between -2^53 and 2^53.
    Number(NumberOption),
    /// An uploaded file.
    Attachment(AttachmentOption),
    /// An option type this version of the library does not know about.
    Unknown(UnknownOption),
}

/// The fields every [`CommandOption`] has, as returned by [`CommandOption::common`].
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct OptionInfo<'a> {
    /// The option's name.
    pub name: &'a str,
    /// Translations of the name.
    pub name_localizations: &'a BTreeMap<Locale, CompactString>,
    /// The option's description.
    pub description: &'a str,
    /// Translations of the description.
    pub description_localizations: &'a BTreeMap<Locale, CompactString>,
    /// The name in the requester's locale, if Discord sent it.
    pub name_localized: Option<&'a str>,
    /// The description in the requester's locale, if Discord sent it.
    pub description_localized: Option<&'a str>,
}

macro_rules! common_fields {
    ($self:ident) => {
        match $self {
            Self::SubCommand(option) | Self::SubCommandGroup(option) => OptionInfo {
                name: &option.name,
                name_localizations: &option.name_localizations,
                description: &option.description,
                description_localizations: &option.description_localizations,
                name_localized: option.name_localized.as_deref(),
                description_localized: option.description_localized.as_deref(),
            },
            Self::String(option) => OptionInfo {
                name: &option.name,
                name_localizations: &option.name_localizations,
                description: &option.description,
                description_localizations: &option.description_localizations,
                name_localized: option.name_localized.as_deref(),
                description_localized: option.description_localized.as_deref(),
            },
            Self::Integer(option) => OptionInfo {
                name: &option.name,
                name_localizations: &option.name_localizations,
                description: &option.description,
                description_localizations: &option.description_localizations,
                name_localized: option.name_localized.as_deref(),
                description_localized: option.description_localized.as_deref(),
            },
            Self::Boolean(option)
            | Self::User(option)
            | Self::Role(option)
            | Self::Mentionable(option) => OptionInfo {
                name: &option.name,
                name_localizations: &option.name_localizations,
                description: &option.description,
                description_localizations: &option.description_localizations,
                name_localized: option.name_localized.as_deref(),
                description_localized: option.description_localized.as_deref(),
            },
            Self::Channel(option) => OptionInfo {
                name: &option.name,
                name_localizations: &option.name_localizations,
                description: &option.description,
                description_localizations: &option.description_localizations,
                name_localized: option.name_localized.as_deref(),
                description_localized: option.description_localized.as_deref(),
            },
            Self::Number(option) => OptionInfo {
                name: &option.name,
                name_localizations: &option.name_localizations,
                description: &option.description,
                description_localizations: &option.description_localizations,
                name_localized: option.name_localized.as_deref(),
                description_localized: option.description_localized.as_deref(),
            },
            Self::Attachment(option) => OptionInfo {
                name: &option.name,
                name_localizations: &option.name_localizations,
                description: &option.description,
                description_localizations: &option.description_localizations,
                name_localized: option.name_localized.as_deref(),
                description_localized: option.description_localized.as_deref(),
            },
            Self::Unknown(option) => OptionInfo {
                name: &option.name,
                name_localizations: &option.name_localizations,
                description: &option.description,
                description_localizations: &option.description_localizations,
                name_localized: option.name_localized.as_deref(),
                description_localized: option.description_localized.as_deref(),
            },
        }
    };
}

impl CommandOption {
    /// Returns the option's type.
    #[must_use]
    pub const fn kind(&self) -> CommandOptionType {
        match self {
            Self::SubCommand(_) => CommandOptionType::SubCommand,
            Self::SubCommandGroup(_) => CommandOptionType::SubCommandGroup,
            Self::String(_) => CommandOptionType::String,
            Self::Integer(_) => CommandOptionType::Integer,
            Self::Boolean(_) => CommandOptionType::Boolean,
            Self::User(_) => CommandOptionType::User,
            Self::Channel(_) => CommandOptionType::Channel,
            Self::Role(_) => CommandOptionType::Role,
            Self::Mentionable(_) => CommandOptionType::Mentionable,
            Self::Number(_) => CommandOptionType::Number,
            Self::Attachment(_) => CommandOptionType::Attachment,
            Self::Unknown(option) => option.kind,
        }
    }

    /// Returns the fields every option has (name, description and their translations).
    #[must_use]
    pub fn common(&self) -> OptionInfo<'_> {
        common_fields!(self)
    }

    /// Returns the option's name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.common().name
    }

    /// Returns the nested options of a subcommand or subcommand group (empty otherwise).
    #[must_use]
    pub fn options(&self) -> &[Self] {
        match self {
            Self::SubCommand(option) | Self::SubCommandGroup(option) => &option.options,
            _ => &[],
        }
    }
}

int_enum! {
    /// The [type of a command option](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-option-type).
    ///
    /// discord.js counterpart: `ApplicationCommandOptionType`.
    #[doc(alias = "ApplicationCommandOptionType")]
    pub enum CommandOptionType: u8 {
        /// [`CommandOption::SubCommand`].
        SubCommand = 1,
        /// [`CommandOption::SubCommandGroup`].
        SubCommandGroup = 2,
        /// [`CommandOption::String`].
        String = 3,
        /// [`CommandOption::Integer`].
        Integer = 4,
        /// [`CommandOption::Boolean`].
        Boolean = 5,
        /// [`CommandOption::User`].
        User = 6,
        /// [`CommandOption::Channel`].
        Channel = 7,
        /// [`CommandOption::Role`].
        Role = 8,
        /// [`CommandOption::Mentionable`].
        Mentionable = 9,
        /// [`CommandOption::Number`].
        Number = 10,
        /// [`CommandOption::Attachment`].
        Attachment = 11,
    }
}

/// A subcommand or a subcommand group.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SubCommandOption {
    /// The name (1-32 characters).
    pub name: CompactString,
    /// Translations of the name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub name_localizations: BTreeMap<Locale, CompactString>,
    /// The description (1-100 characters).
    pub description: CompactString,
    /// Translations of the description.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub description_localizations: BTreeMap<Locale, CompactString>,
    /// The name in the requester's locale, from endpoints that do not return the full
    /// translations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localized: Option<CompactString>,
    /// The description in the requester's locale (see [`name_localized`](Self::name_localized)).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localized: Option<CompactString>,
    /// The options of a subcommand, or the subcommands of a group.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<CommandOption>,
}

/// A string option.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct StringOption {
    /// The name (1-32 characters).
    pub name: CompactString,
    /// Translations of the name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub name_localizations: BTreeMap<Locale, CompactString>,
    /// The description (1-100 characters).
    pub description: CompactString,
    /// Translations of the description.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub description_localizations: BTreeMap<Locale, CompactString>,
    /// The name in the requester's locale, from endpoints that do not return the full
    /// translations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localized: Option<CompactString>,
    /// The description in the requester's locale (see [`name_localized`](Self::name_localized)).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localized: Option<CompactString>,
    /// Whether the option must be filled.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub required: bool,
    /// The values to choose from (max 25; values up to 100 characters).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<Choice<CompactString>>,
    /// The minimum length (0-6000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u16>,
    /// The maximum length (1-6000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u16>,
    /// Whether the app suggests values through autocomplete interactions (not with
    /// [`choices`](Self::choices)).
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub autocomplete: bool,
}

/// An integer option.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct IntegerOption {
    /// The name (1-32 characters).
    pub name: CompactString,
    /// Translations of the name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub name_localizations: BTreeMap<Locale, CompactString>,
    /// The description (1-100 characters).
    pub description: CompactString,
    /// Translations of the description.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub description_localizations: BTreeMap<Locale, CompactString>,
    /// The name in the requester's locale, from endpoints that do not return the full
    /// translations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localized: Option<CompactString>,
    /// The description in the requester's locale (see [`name_localized`](Self::name_localized)).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localized: Option<CompactString>,
    /// Whether the option must be filled.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub required: bool,
    /// The values to choose from (max 25).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<Choice<i64>>,
    /// The minimum value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<i64>,
    /// The maximum value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<i64>,
    /// Whether the app suggests values through autocomplete interactions.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub autocomplete: bool,
}

/// A number (double) option.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct NumberOption {
    /// The name (1-32 characters).
    pub name: CompactString,
    /// Translations of the name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub name_localizations: BTreeMap<Locale, CompactString>,
    /// The description (1-100 characters).
    pub description: CompactString,
    /// Translations of the description.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub description_localizations: BTreeMap<Locale, CompactString>,
    /// The name in the requester's locale, from endpoints that do not return the full
    /// translations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localized: Option<CompactString>,
    /// The description in the requester's locale (see [`name_localized`](Self::name_localized)).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localized: Option<CompactString>,
    /// Whether the option must be filled.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub required: bool,
    /// The values to choose from (max 25).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<Choice<f64>>,
    /// The minimum value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,
    /// The maximum value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,
    /// Whether the app suggests values through autocomplete interactions.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub autocomplete: bool,
}

/// A boolean, user, role or mentionable option (they share this structure).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct BasicOption {
    /// The name (1-32 characters).
    pub name: CompactString,
    /// Translations of the name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub name_localizations: BTreeMap<Locale, CompactString>,
    /// The description (1-100 characters).
    pub description: CompactString,
    /// Translations of the description.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub description_localizations: BTreeMap<Locale, CompactString>,
    /// The name in the requester's locale, from endpoints that do not return the full
    /// translations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localized: Option<CompactString>,
    /// The description in the requester's locale (see [`name_localized`](Self::name_localized)).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localized: Option<CompactString>,
    /// Whether the option must be filled.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub required: bool,
}

/// A channel option.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ChannelOption {
    /// The name (1-32 characters).
    pub name: CompactString,
    /// Translations of the name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub name_localizations: BTreeMap<Locale, CompactString>,
    /// The description (1-100 characters).
    pub description: CompactString,
    /// Translations of the description.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub description_localizations: BTreeMap<Locale, CompactString>,
    /// The name in the requester's locale, from endpoints that do not return the full
    /// translations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localized: Option<CompactString>,
    /// The description in the requester's locale (see [`name_localized`](Self::name_localized)).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localized: Option<CompactString>,
    /// Whether the option must be filled.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub required: bool,
    /// The channel types offered (all when empty).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub channel_types: Vec<ChannelType>,
}

/// An attachment option.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct AttachmentOption {
    /// The name (1-32 characters).
    pub name: CompactString,
    /// Translations of the name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub name_localizations: BTreeMap<Locale, CompactString>,
    /// The description (1-100 characters).
    pub description: CompactString,
    /// Translations of the description.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub description_localizations: BTreeMap<Locale, CompactString>,
    /// The name in the requester's locale, from endpoints that do not return the full
    /// translations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localized: Option<CompactString>,
    /// The description in the requester's locale (see [`name_localized`](Self::name_localized)).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localized: Option<CompactString>,
    /// Whether the option must be filled.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub required: bool,
    /// The [file types to accept](https://docs.discord.com/developers/reference#file-type-filtering)
    /// (extensions only; all when empty).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub file_types: Vec<CompactString>,
}

/// An option of a type this version of the library does not know about; the fields every
/// option has are kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct UnknownOption {
    /// The raw option type.
    #[serde(skip)]
    pub kind: CommandOptionType,
    /// The name.
    pub name: CompactString,
    /// Translations of the name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub name_localizations: BTreeMap<Locale, CompactString>,
    /// The description.
    pub description: CompactString,
    /// Translations of the description.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub description_localizations: BTreeMap<Locale, CompactString>,
    /// The name in the requester's locale, from endpoints that do not return the full
    /// translations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localized: Option<CompactString>,
    /// The description in the requester's locale (see [`name_localized`](Self::name_localized)).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localized: Option<CompactString>,
    /// Whether the option must be filled.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub required: bool,
}

/// A [choice of an option](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-option-choice-structure)
/// (also used for autocomplete suggestions). `T` is the option's value type.
///
/// discord.js counterpart: `APIApplicationCommandOptionChoice`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "ApplicationCommandOptionChoice")]
pub struct Choice<T> {
    /// The name shown to the user (1-100 characters).
    pub name: CompactString,
    /// Translations of the name, keyed by locale (D-058).
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "crate::serde_util::null_as_default"
    )]
    pub name_localizations: BTreeMap<Locale, CompactString>,
    /// The value sent to the app.
    pub value: T,
}

impl<T> Choice<T> {
    /// Creates a choice without translations.
    #[must_use]
    pub fn new(name: impl Into<CompactString>, value: T) -> Self {
        Self {
            name: name.into(),
            name_localizations: BTreeMap::new(),
            value,
        }
    }
}

/// A JSON number or string as found in choices and option bounds, before the option type tells
/// what it must be.
#[derive(Clone, Debug, PartialEq)]
enum RawValue {
    String(CompactString),
    Integer(i64),
    Float(f64),
}

impl<'de> Deserialize<'de> for RawValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct RawValueVisitor;

        impl Visitor<'_> for RawValueVisitor {
            type Value = RawValue;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("a string or a number")
            }

            fn visit_i64<E: DeError>(self, value: i64) -> Result<RawValue, E> {
                Ok(RawValue::Integer(value))
            }

            fn visit_u64<E: DeError>(self, value: u64) -> Result<RawValue, E> {
                i64::try_from(value)
                    .map(RawValue::Integer)
                    .map_err(|_| E::invalid_value(Unexpected::Unsigned(value), &self))
            }

            fn visit_f64<E: DeError>(self, value: f64) -> Result<RawValue, E> {
                Ok(RawValue::Float(value))
            }

            fn visit_str<E: DeError>(self, value: &str) -> Result<RawValue, E> {
                Ok(RawValue::String(value.into()))
            }
        }

        deserializer.deserialize_any(RawValueVisitor)
    }
}

impl RawValue {
    fn into_string<E: DeError>(self) -> Result<CompactString, E> {
        match self {
            Self::String(value) => Ok(value),
            Self::Integer(value) => Err(E::invalid_type(Unexpected::Signed(value), &"a string")),
            Self::Float(value) => Err(E::invalid_type(Unexpected::Float(value), &"a string")),
        }
    }

    fn into_integer<E: DeError>(self) -> Result<i64, E> {
        match self {
            Self::Integer(value) => Ok(value),
            Self::String(value) => Err(E::invalid_type(Unexpected::Str(&value), &"an integer")),
            Self::Float(value) => Err(E::invalid_type(Unexpected::Float(value), &"an integer")),
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "Discord limits command numbers to ±2^53, which f64 represents exactly"
    )]
    fn into_number<E: DeError>(self) -> Result<f64, E> {
        match self {
            Self::Integer(value) => Ok(value as f64),
            Self::Float(value) => Ok(value),
            Self::String(value) => Err(E::invalid_type(Unexpected::Str(&value), &"a number")),
        }
    }
}

fn convert_choices<T, E: DeError>(
    choices: Vec<Choice<RawValue>>,
    convert: impl Fn(RawValue) -> Result<T, E>,
) -> Result<Vec<Choice<T>>, E> {
    choices
        .into_iter()
        .map(|choice| {
            Ok(Choice {
                name: choice.name,
                name_localizations: choice.name_localizations,
                value: convert(choice.value)?,
            })
        })
        .collect()
}

/// Every field of every option type, decoded in one pass and converted by `type` (D-069).
#[derive(Deserialize)]
struct RawOption {
    #[serde(rename = "type")]
    kind: CommandOptionType,
    name: CompactString,
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    name_localizations: BTreeMap<Locale, CompactString>,
    #[serde(default)]
    description: CompactString,
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    description_localizations: BTreeMap<Locale, CompactString>,
    #[serde(default)]
    name_localized: Option<CompactString>,
    #[serde(default)]
    description_localized: Option<CompactString>,
    #[serde(default)]
    required: bool,
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    choices: Vec<Choice<RawValue>>,
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    options: Vec<CommandOption>,
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    channel_types: Vec<ChannelType>,
    #[serde(default)]
    min_value: Option<RawValue>,
    #[serde(default)]
    max_value: Option<RawValue>,
    #[serde(default)]
    min_length: Option<u16>,
    #[serde(default)]
    max_length: Option<u16>,
    #[serde(default)]
    autocomplete: bool,
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    file_types: Vec<CompactString>,
}

impl RawOption {
    fn into_option<E: DeError>(self) -> Result<CommandOption, E> {
        let basic = |raw: Self| BasicOption {
            name: raw.name,
            name_localizations: raw.name_localizations,
            description: raw.description,
            description_localizations: raw.description_localizations,
            name_localized: raw.name_localized,
            description_localized: raw.description_localized,
            required: raw.required,
        };
        Ok(match self.kind {
            kind @ (CommandOptionType::SubCommand | CommandOptionType::SubCommandGroup) => {
                let option = SubCommandOption {
                    name: self.name,
                    name_localizations: self.name_localizations,
                    description: self.description,
                    description_localizations: self.description_localizations,
                    name_localized: self.name_localized,
                    description_localized: self.description_localized,
                    options: self.options,
                };
                if kind == CommandOptionType::SubCommand {
                    CommandOption::SubCommand(option)
                } else {
                    CommandOption::SubCommandGroup(option)
                }
            }
            CommandOptionType::String => CommandOption::String(StringOption {
                choices: convert_choices(self.choices, RawValue::into_string)?,
                name: self.name,
                name_localizations: self.name_localizations,
                description: self.description,
                description_localizations: self.description_localizations,
                name_localized: self.name_localized,
                description_localized: self.description_localized,
                required: self.required,
                min_length: self.min_length,
                max_length: self.max_length,
                autocomplete: self.autocomplete,
            }),
            CommandOptionType::Integer => CommandOption::Integer(IntegerOption {
                choices: convert_choices(self.choices, RawValue::into_integer)?,
                min_value: self.min_value.map(RawValue::into_integer).transpose()?,
                max_value: self.max_value.map(RawValue::into_integer).transpose()?,
                name: self.name,
                name_localizations: self.name_localizations,
                description: self.description,
                description_localizations: self.description_localizations,
                name_localized: self.name_localized,
                description_localized: self.description_localized,
                required: self.required,
                autocomplete: self.autocomplete,
            }),
            CommandOptionType::Number => CommandOption::Number(NumberOption {
                choices: convert_choices(self.choices, RawValue::into_number)?,
                min_value: self.min_value.map(RawValue::into_number).transpose()?,
                max_value: self.max_value.map(RawValue::into_number).transpose()?,
                name: self.name,
                name_localizations: self.name_localizations,
                description: self.description,
                description_localizations: self.description_localizations,
                name_localized: self.name_localized,
                description_localized: self.description_localized,
                required: self.required,
                autocomplete: self.autocomplete,
            }),
            CommandOptionType::Boolean => CommandOption::Boolean(basic(self)),
            CommandOptionType::User => CommandOption::User(basic(self)),
            CommandOptionType::Role => CommandOption::Role(basic(self)),
            CommandOptionType::Mentionable => CommandOption::Mentionable(basic(self)),
            CommandOptionType::Channel => CommandOption::Channel(ChannelOption {
                name: self.name,
                name_localizations: self.name_localizations,
                description: self.description,
                description_localizations: self.description_localizations,
                name_localized: self.name_localized,
                description_localized: self.description_localized,
                required: self.required,
                channel_types: self.channel_types,
            }),
            CommandOptionType::Attachment => CommandOption::Attachment(AttachmentOption {
                name: self.name,
                name_localizations: self.name_localizations,
                description: self.description,
                description_localizations: self.description_localizations,
                name_localized: self.name_localized,
                description_localized: self.description_localized,
                required: self.required,
                file_types: self.file_types,
            }),
            kind @ CommandOptionType::Unknown(_) => CommandOption::Unknown(UnknownOption {
                kind,
                name: self.name,
                name_localizations: self.name_localizations,
                description: self.description,
                description_localizations: self.description_localizations,
                name_localized: self.name_localized,
                description_localized: self.description_localized,
                required: self.required,
            }),
        })
    }
}

impl<'de> Deserialize<'de> for CommandOption {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        RawOption::deserialize(deserializer)?.into_option::<D::Error>()
    }
}

/// Serializes a variant's struct together with its `type`.
#[derive(Serialize)]
struct Tagged<'a, T> {
    #[serde(rename = "type")]
    kind: CommandOptionType,
    #[serde(flatten)]
    option: &'a T,
}

impl Serialize for CommandOption {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let kind = self.kind();
        match self {
            Self::SubCommand(option) | Self::SubCommandGroup(option) => {
                Tagged { kind, option }.serialize(serializer)
            }
            Self::String(option) => Tagged { kind, option }.serialize(serializer),
            Self::Integer(option) => Tagged { kind, option }.serialize(serializer),
            Self::Boolean(option)
            | Self::User(option)
            | Self::Role(option)
            | Self::Mentionable(option) => Tagged { kind, option }.serialize(serializer),
            Self::Channel(option) => Tagged { kind, option }.serialize(serializer),
            Self::Number(option) => Tagged { kind, option }.serialize(serializer),
            Self::Attachment(option) => Tagged { kind, option }.serialize(serializer),
            Self::Unknown(option) => Tagged { kind, option }.serialize(serializer),
        }
    }
}

/// The [permissions of an app's command(s) in a guild](https://docs.discord.com/developers/interactions/application-commands#application-command-permissions-object-guild-application-command-permissions-structure).
///
/// discord.js counterpart: `ApplicationCommandPermissions` (per guild).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "GuildApplicationCommandPermissions")]
pub struct GuildCommandPermissions {
    /// The command, or the application ID for permissions that apply to all its commands
    /// without their own.
    pub id: GenericId,
    /// The application the command belongs to.
    pub application_id: ApplicationId,
    /// The guild.
    pub guild_id: GuildId,
    /// The permissions (max 100).
    pub permissions: Vec<CommandPermission>,
}

/// One [command permission](https://docs.discord.com/developers/interactions/application-commands#application-command-permissions-object-application-command-permissions-structure):
/// allows or denies a command for a role, user or channel.
///
/// discord.js counterpart: `ApplicationCommandPermissions`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "ApplicationCommandPermissions")]
pub struct CommandPermission {
    /// The role, user or channel; see [`everyone`](Self::everyone) and
    /// [`all_channels`](Self::all_channels) for the special values.
    pub id: GenericId,
    /// What [`id`](Self::id) refers to.
    #[serde(rename = "type")]
    pub kind: CommandPermissionType,
    /// `true` to allow, `false` to deny.
    pub permission: bool,
}

impl CommandPermission {
    /// Creates a permission.
    #[must_use]
    pub const fn new(id: GenericId, kind: CommandPermissionType, permission: bool) -> Self {
        Self {
            id,
            kind,
            permission,
        }
    }

    /// The ID that stands for `@everyone` (all members) in `guild_id`: the guild ID itself.
    #[must_use]
    pub const fn everyone(guild_id: GuildId) -> GenericId {
        guild_id.cast()
    }

    /// The ID that stands for all channels in `guild_id`: the guild ID minus one, or `None`
    /// for the (impossible) guild ID `1`.
    #[must_use]
    pub const fn all_channels(guild_id: GuildId) -> Option<GenericId> {
        GenericId::new_checked(guild_id.get() - 1)
    }
}

int_enum! {
    /// [What a command permission applies to](https://docs.discord.com/developers/interactions/application-commands#application-command-permissions-object-application-command-permission-type).
    ///
    /// discord.js counterpart: `ApplicationCommandPermissionType`.
    #[doc(alias = "ApplicationCommandPermissionType")]
    pub enum CommandPermissionType: u8 {
        /// A role (or `@everyone`).
        Role = 1,
        /// A user.
        User = 2,
        /// A channel (or all channels).
        Channel = 3,
    }
}
