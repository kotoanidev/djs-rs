//! The type-specific [`data`](super::Interaction::data) of interactions (D-067).

use std::collections::BTreeMap;

use compact_str::{CompactString, ToCompactString};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as DeError, Unexpected, Visitor},
};

use super::{InteractionChannel, InteractionMember, InteractionType};
use crate::{
    application_command::{CommandOptionType, CommandType},
    component::ComponentType,
    guild::Role,
    id::{
        AttachmentId, ChannelId, CommandId, GenericId, GuildId, Id, MessageId, RoleId, UserId,
        marker::{ChannelMarker, GenericMarker, RoleMarker, UserMarker},
    },
    message::{Attachment, Message},
    user::User,
};

/// The [data](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-object-interaction-data)
/// of an [`Interaction`](super::Interaction), by interaction type (D-067).
///
/// The variant decides the interaction's [`type`](super::Interaction::kind): it is decoded
/// from `type` and `data` together and written back as both.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum InteractionData {
    /// Discord checks the app's HTTP interactions endpoint; answer with
    /// [`InteractionResponse::Pong`](super::InteractionResponse::Pong).
    Ping,
    /// A user ran one of the app's commands.
    ///
    /// discord.js counterpart: `ChatInputCommandInteraction`, `UserContextMenuCommandInteraction`,
    /// `MessageContextMenuCommandInteraction`, `PrimaryEntryPointCommandInteraction`.
    ApplicationCommand(Box<CommandData>),
    /// A user clicked a button or chose in a select menu on one of the app's messages.
    ///
    /// discord.js counterpart: `ButtonInteraction`, `StringSelectMenuInteraction` and the other
    /// select menu interactions.
    MessageComponent(Box<ComponentData>),
    /// A user is typing an option with autocomplete; one option is
    /// [`Focused`](CommandOptionValue::Focused).
    ///
    /// discord.js counterpart: `AutocompleteInteraction`.
    Autocomplete(Box<CommandData>),
    /// A user submitted one of the app's modals.
    ///
    /// discord.js counterpart: `ModalSubmitInteraction`.
    ModalSubmit(Box<ModalSubmitData>),
    /// An interaction type this version does not know; its data is not kept.
    Unknown(InteractionType),
}

impl InteractionData {
    /// Returns the interaction type this data belongs to.
    #[must_use]
    pub const fn kind(&self) -> InteractionType {
        match self {
            Self::Ping => InteractionType::Ping,
            Self::ApplicationCommand(_) => InteractionType::ApplicationCommand,
            Self::MessageComponent(_) => InteractionType::MessageComponent,
            Self::Autocomplete(_) => InteractionType::ApplicationCommandAutocomplete,
            Self::ModalSubmit(_) => InteractionType::ModalSubmit,
            Self::Unknown(kind) => *kind,
        }
    }

    /// Returns the command data of command and autocomplete interactions (see
    /// [`Interaction::command`](super::Interaction::command) and
    /// [`Interaction::autocomplete`](super::Interaction::autocomplete) for one kind).
    #[must_use]
    pub fn command(&self) -> Option<&CommandData> {
        match self {
            Self::ApplicationCommand(data) | Self::Autocomplete(data) => Some(data),
            _ => None,
        }
    }

    /// Returns the custom ID of the component or modal (component and modal-submit
    /// interactions).
    #[must_use]
    pub fn custom_id(&self) -> Option<&str> {
        match self {
            Self::MessageComponent(data) => Some(&data.custom_id),
            Self::ModalSubmit(data) => Some(&data.custom_id),
            _ => None,
        }
    }

    /// Returns the resolved users, members, roles, channels, messages and attachments, if any.
    #[must_use]
    pub fn resolved(&self) -> Option<&Resolved> {
        match self {
            Self::ApplicationCommand(data) | Self::Autocomplete(data) => data.resolved.as_deref(),
            Self::MessageComponent(data) => data.resolved.as_deref(),
            Self::ModalSubmit(data) => data.resolved.as_deref(),
            Self::Ping | Self::Unknown(_) => None,
        }
    }

    /// Borrows the data for serialization (`None` for pings and unknown types).
    pub(super) fn as_ref(&self) -> Option<DataRef<'_>> {
        match self {
            Self::ApplicationCommand(data) | Self::Autocomplete(data) => {
                Some(DataRef::Command(data))
            }
            Self::MessageComponent(data) => Some(DataRef::Component(data)),
            Self::ModalSubmit(data) => Some(DataRef::ModalSubmit(data)),
            Self::Ping | Self::Unknown(_) => None,
        }
    }
}

/// Borrowed data, written as the interaction's `data` object.
#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum DataRef<'a> {
    Command(&'a CommandData),
    Component(&'a ComponentData),
    ModalSubmit(&'a ModalSubmitData),
}

/// The [data of a command interaction](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-object-application-command-data-structure)
/// (and of an autocomplete interaction).
///
/// discord.js counterpart: the command fields of `CommandInteraction` (`commandId`,
/// `commandName`, `commandType`, `options`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct CommandData {
    /// The command's ID.
    pub id: CommandId,
    /// The command's name.
    pub name: CompactString,
    /// The command's type.
    #[serde(rename = "type")]
    pub kind: CommandType,
    /// The users, members, roles, channels, messages and attachments the options refer to
    /// (and the target of context menu commands).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved: Option<Box<Resolved>>,
    /// The options the user filled in (in autocomplete interactions: possibly partial).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<CommandDataOption>,
    /// The guild the command is registered in (guild commands).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The user or message a context menu command was used on (in [`resolved`](Self::resolved)).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<GenericId>,
}

impl CommandData {
    /// Returns the option that is being typed in an autocomplete interaction, searching
    /// subcommands and groups.
    #[must_use]
    pub fn focused(&self) -> Option<&CommandDataOption> {
        fn find(options: &[CommandDataOption]) -> Option<&CommandDataOption> {
            options.iter().find_map(|option| match &option.value {
                CommandOptionValue::Focused { .. } => Some(option),
                CommandOptionValue::SubCommand(options)
                | CommandOptionValue::SubCommandGroup(options) => find(options),
                _ => None,
            })
        }

        find(&self.options)
    }
}

/// An [option filled in by the user](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-object-application-command-interaction-data-option-structure):
/// its name and its value (by option type).
///
/// discord.js counterpart: `CommandInteractionOption`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CommandDataOption {
    /// The option's name.
    pub name: CompactString,
    /// The value, or the options of a subcommand or group.
    pub value: CommandOptionValue,
}

impl CommandDataOption {
    /// Creates an option (for tests and mock servers).
    #[must_use]
    pub fn new(name: impl Into<CompactString>, value: CommandOptionValue) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

/// The value of a [`CommandDataOption`], by [option type](CommandOptionType).
///
/// Snowflake values (users, channels, roles, mentionables, attachments) are IDs; the objects
/// they refer to are in the command's [`Resolved`] data.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum CommandOptionValue {
    /// A subcommand and its options.
    SubCommand(Vec<CommandDataOption>),
    /// A subcommand group and its (single) subcommand.
    SubCommandGroup(Vec<CommandDataOption>),
    /// A string.
    String(CompactString),
    /// An integer.
    Integer(i64),
    /// A boolean.
    Boolean(bool),
    /// A user (also in [`Resolved::users`] and, in guilds, [`Resolved::members`]).
    User(UserId),
    /// A channel (in [`Resolved::channels`]).
    Channel(ChannelId),
    /// A role (in [`Resolved::roles`]).
    Role(RoleId),
    /// A user or a role (in the matching [`Resolved`] map).
    Mentionable(GenericId),
    /// A floating-point number.
    Number(f64),
    /// An attachment (in [`Resolved::attachments`]).
    Attachment(AttachmentId),
    /// The option being typed in an autocomplete interaction: what the user has typed so far,
    /// which need not be valid for the option's type yet.
    Focused {
        /// The option's type.
        kind: CommandOptionType,
        /// The text typed so far.
        value: CompactString,
    },
    /// An option type this version does not know; its value is not kept.
    Unknown(CommandOptionType),
}

impl CommandOptionValue {
    /// Returns the option type.
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
            Self::Focused { kind, .. } | Self::Unknown(kind) => *kind,
        }
    }
}

/// A JSON value of an option, before the option type says what it must be.
enum RawOptionValue {
    String(CompactString),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl<'de> Deserialize<'de> for RawOptionValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct RawOptionValueVisitor;

        impl Visitor<'_> for RawOptionValueVisitor {
            type Value = RawOptionValue;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("a string, a number or a boolean")
            }

            fn visit_bool<E: DeError>(self, value: bool) -> Result<RawOptionValue, E> {
                Ok(RawOptionValue::Boolean(value))
            }

            fn visit_i64<E: DeError>(self, value: i64) -> Result<RawOptionValue, E> {
                Ok(RawOptionValue::Integer(value))
            }

            fn visit_u64<E: DeError>(self, value: u64) -> Result<RawOptionValue, E> {
                i64::try_from(value)
                    .map(RawOptionValue::Integer)
                    .map_err(|_| E::invalid_value(Unexpected::Unsigned(value), &self))
            }

            fn visit_f64<E: DeError>(self, value: f64) -> Result<RawOptionValue, E> {
                Ok(RawOptionValue::Float(value))
            }

            fn visit_str<E: DeError>(self, value: &str) -> Result<RawOptionValue, E> {
                Ok(RawOptionValue::String(value.into()))
            }
        }

        deserializer.deserialize_any(RawOptionValueVisitor)
    }
}

impl RawOptionValue {
    fn unexpected(&self) -> Unexpected<'_> {
        match self {
            Self::String(value) => Unexpected::Str(value),
            Self::Integer(value) => Unexpected::Signed(*value),
            Self::Float(value) => Unexpected::Float(*value),
            Self::Boolean(value) => Unexpected::Bool(*value),
        }
    }

    /// The text of a focused option (Discord sends it as a string; numbers are accepted too).
    fn into_text(self) -> CompactString {
        match self {
            Self::String(value) => value,
            Self::Integer(value) => value.to_compact_string(),
            Self::Float(value) => value.to_compact_string(),
            Self::Boolean(value) => value.to_compact_string(),
        }
    }

    fn into_string<E: DeError>(self) -> Result<CompactString, E> {
        match self {
            Self::String(value) => Ok(value),
            other => Err(E::invalid_type(other.unexpected(), &"a string")),
        }
    }

    fn into_integer<E: DeError>(self) -> Result<i64, E> {
        match self {
            Self::Integer(value) => Ok(value),
            Self::String(ref text) => text
                .parse()
                .map_err(|_| E::invalid_type(self.unexpected(), &"an integer")),
            other => Err(E::invalid_type(other.unexpected(), &"an integer")),
        }
    }

    fn into_number<E: DeError>(self) -> Result<f64, E> {
        match self {
            Self::Float(value) => Ok(value),
            #[expect(
                clippy::cast_precision_loss,
                reason = "number options are doubles; Discord limits them to ±2^53"
            )]
            Self::Integer(value) => Ok(value as f64),
            Self::String(ref text) => text
                .parse()
                .map_err(|_| E::invalid_type(self.unexpected(), &"a number")),
            Self::Boolean(_) => Err(E::invalid_type(self.unexpected(), &"a number")),
        }
    }

    fn into_boolean<E: DeError>(self) -> Result<bool, E> {
        match self {
            Self::Boolean(value) => Ok(value),
            other => Err(E::invalid_type(other.unexpected(), &"a boolean")),
        }
    }

    fn into_id<T, E: DeError>(self) -> Result<Id<T>, E> {
        match self {
            Self::String(ref text) => text
                .parse()
                .map_err(|_| E::invalid_value(self.unexpected(), &"a snowflake ID")),
            Self::Integer(value) => u64::try_from(value)
                .ok()
                .and_then(Id::new_checked)
                .ok_or_else(|| E::invalid_value(self.unexpected(), &"a snowflake ID")),
            other => Err(E::invalid_type(other.unexpected(), &"a snowflake ID")),
        }
    }
}

/// Every field of an option, decoded in one pass.
#[derive(Deserialize)]
struct RawDataOption {
    name: CompactString,
    #[serde(rename = "type")]
    kind: CommandOptionType,
    #[serde(default)]
    value: Option<RawOptionValue>,
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    options: Vec<CommandDataOption>,
    #[serde(default)]
    focused: bool,
}

impl<'de> Deserialize<'de> for CommandDataOption {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawDataOption::deserialize(deserializer)?;
        let kind = raw.kind;
        let value = match kind {
            CommandOptionType::SubCommand => CommandOptionValue::SubCommand(raw.options),
            CommandOptionType::SubCommandGroup => CommandOptionValue::SubCommandGroup(raw.options),
            CommandOptionType::Unknown(_) => CommandOptionValue::Unknown(kind),
            _ => {
                let value = raw.value.ok_or_else(|| D::Error::missing_field("value"))?;
                if raw.focused {
                    CommandOptionValue::Focused {
                        kind,
                        value: value.into_text(),
                    }
                } else {
                    match kind {
                        CommandOptionType::String => {
                            CommandOptionValue::String(value.into_string()?)
                        }
                        CommandOptionType::Integer => {
                            CommandOptionValue::Integer(value.into_integer()?)
                        }
                        CommandOptionType::Boolean => {
                            CommandOptionValue::Boolean(value.into_boolean()?)
                        }
                        CommandOptionType::User => CommandOptionValue::User(value.into_id()?),
                        CommandOptionType::Channel => CommandOptionValue::Channel(value.into_id()?),
                        CommandOptionType::Role => CommandOptionValue::Role(value.into_id()?),
                        CommandOptionType::Mentionable => {
                            CommandOptionValue::Mentionable(value.into_id()?)
                        }
                        CommandOptionType::Number => {
                            CommandOptionValue::Number(value.into_number()?)
                        }
                        _ => CommandOptionValue::Attachment(value.into_id()?),
                    }
                }
            }
        };
        Ok(Self {
            name: raw.name,
            value,
        })
    }
}

impl Serialize for CommandDataOption {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(untagged)]
        enum Value<'a> {
            String(&'a str),
            Integer(i64),
            Number(f64),
            Boolean(bool),
            Id(GenericId),
        }

        #[derive(Serialize)]
        struct Flat<'a> {
            name: &'a str,
            #[serde(rename = "type")]
            kind: CommandOptionType,
            #[serde(skip_serializing_if = "Option::is_none")]
            value: Option<Value<'a>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            options: Option<&'a [CommandDataOption]>,
            #[serde(skip_serializing_if = "core::ops::Not::not")]
            focused: bool,
        }

        let mut flat = Flat {
            name: &self.name,
            kind: self.value.kind(),
            value: None,
            options: None,
            focused: false,
        };
        match &self.value {
            CommandOptionValue::SubCommand(options)
            | CommandOptionValue::SubCommandGroup(options) => flat.options = Some(options),
            CommandOptionValue::String(value) => flat.value = Some(Value::String(value)),
            CommandOptionValue::Integer(value) => flat.value = Some(Value::Integer(*value)),
            CommandOptionValue::Boolean(value) => flat.value = Some(Value::Boolean(*value)),
            CommandOptionValue::User(id) => flat.value = Some(Value::Id(id.cast())),
            CommandOptionValue::Channel(id) => flat.value = Some(Value::Id(id.cast())),
            CommandOptionValue::Role(id) => flat.value = Some(Value::Id(id.cast())),
            CommandOptionValue::Mentionable(id) => flat.value = Some(Value::Id(*id)),
            CommandOptionValue::Number(value) => flat.value = Some(Value::Number(*value)),
            CommandOptionValue::Attachment(id) => flat.value = Some(Value::Id(id.cast())),
            CommandOptionValue::Focused { value, .. } => {
                flat.value = Some(Value::String(value));
                flat.focused = true;
            }
            CommandOptionValue::Unknown(_) => {}
        }
        flat.serialize(serializer)
    }
}

/// The [data of a component interaction](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-object-message-component-data-structure):
/// which button or select menu was used, and what was selected.
///
/// discord.js counterpart: `MessageComponentInteraction` (`customId`, `componentType`) and the
/// select menu interactions (`values`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ComponentData {
    /// The component's custom ID.
    pub custom_id: CompactString,
    /// The component's type.
    pub component_type: ComponentType,
    /// The selected values (select menus): option values of string selects, IDs (as strings)
    /// of the other selects (see [`ids`](Self::ids)).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<CompactString>,
    /// The selected users, members, roles and channels (entity select menus).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved: Option<Box<Resolved>>,
}

impl ComponentData {
    /// Parses the selected [`values`](Self::values) of a user, role, mentionable or channel
    /// select menu as IDs, skipping values that are not snowflakes.
    pub fn ids<T>(&self) -> impl Iterator<Item = Id<T>> + '_ {
        self.values.iter().filter_map(|value| value.parse().ok())
    }
}

/// The [data of a modal-submit interaction](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-object-modal-submit-data-structure):
/// the values the user entered in the modal's components.
///
/// discord.js counterpart: `ModalSubmitInteraction` (`customId`, `fields`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ModalSubmitData {
    /// The modal's custom ID.
    pub custom_id: CompactString,
    /// The submitted components, in the modal's layout.
    pub components: Vec<SubmittedComponent>,
    /// The selected users, members, roles, channels and uploaded attachments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<Box<Resolved>>,
}

impl ModalSubmitData {
    /// Finds the submitted input with the given custom ID, inside labels and action rows.
    #[must_use]
    pub fn get(&self, custom_id: &str) -> Option<&SubmittedComponent> {
        fn find<'a>(
            components: &'a [SubmittedComponent],
            custom_id: &str,
        ) -> Option<&'a SubmittedComponent> {
            components.iter().find_map(|component| match component {
                SubmittedComponent::ActionRow(row) => find(&row.components, custom_id),
                SubmittedComponent::Label(label) => {
                    find(core::slice::from_ref(&*label.component), custom_id)
                }
                other => (other.custom_id() == Some(custom_id)).then_some(other),
            })
        }

        find(&self.components, custom_id)
    }

    /// Returns the text entered in the text input with the given custom ID.
    #[must_use]
    pub fn text_value(&self, custom_id: &str) -> Option<&str> {
        match self.get(custom_id)? {
            SubmittedComponent::TextInput(input) => Some(&input.value),
            _ => None,
        }
    }
}

/// A [submitted modal component](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-object-component-interaction-response-structures):
/// what the user entered, by component type.
///
/// discord.js counterpart: the entries of `ModalSubmitFields`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
#[doc(alias = "ModalSubmitComponent")]
pub enum SubmittedComponent {
    /// An action row (legacy modals with text inputs in rows).
    ActionRow(SubmittedActionRow),
    /// A text input.
    TextInput(SubmittedTextInput),
    /// A string select menu.
    StringSelect(SubmittedSelect),
    /// A user select menu.
    UserSelect(SubmittedEntitySelect<UserMarker>),
    /// A role select menu.
    RoleSelect(SubmittedEntitySelect<RoleMarker>),
    /// A mentionable (user or role) select menu.
    MentionableSelect(SubmittedEntitySelect<GenericMarker>),
    /// A channel select menu.
    ChannelSelect(SubmittedEntitySelect<ChannelMarker>),
    /// A text display (no value).
    TextDisplay(SubmittedTextDisplay),
    /// A label and its component.
    Label(SubmittedLabel),
    /// A file upload.
    FileUpload(SubmittedFileUpload),
    /// A radio group.
    RadioGroup(SubmittedRadioGroup),
    /// A checkbox group.
    CheckboxGroup(SubmittedSelect),
    /// A checkbox.
    Checkbox(SubmittedCheckbox),
    /// A component type this version does not know; only its type and ID are kept.
    Unknown(UnknownSubmittedComponent),
}

impl SubmittedComponent {
    /// Returns the component type.
    #[must_use]
    pub const fn kind(&self) -> ComponentType {
        match self {
            Self::ActionRow(_) => ComponentType::ActionRow,
            Self::TextInput(_) => ComponentType::TextInput,
            Self::StringSelect(_) => ComponentType::StringSelect,
            Self::UserSelect(_) => ComponentType::UserSelect,
            Self::RoleSelect(_) => ComponentType::RoleSelect,
            Self::MentionableSelect(_) => ComponentType::MentionableSelect,
            Self::ChannelSelect(_) => ComponentType::ChannelSelect,
            Self::TextDisplay(_) => ComponentType::TextDisplay,
            Self::Label(_) => ComponentType::Label,
            Self::FileUpload(_) => ComponentType::FileUpload,
            Self::RadioGroup(_) => ComponentType::RadioGroup,
            Self::CheckboxGroup(_) => ComponentType::CheckboxGroup,
            Self::Checkbox(_) => ComponentType::Checkbox,
            Self::Unknown(component) => component.kind,
        }
    }

    /// Returns the component's numeric ID.
    #[must_use]
    pub const fn id(&self) -> Option<u32> {
        match self {
            Self::ActionRow(component) => component.id,
            Self::TextInput(component) => component.id,
            Self::StringSelect(component) | Self::CheckboxGroup(component) => component.id,
            Self::UserSelect(component) => component.id,
            Self::RoleSelect(component) => component.id,
            Self::MentionableSelect(component) => component.id,
            Self::ChannelSelect(component) => component.id,
            Self::TextDisplay(component) => component.id,
            Self::Label(component) => component.id,
            Self::FileUpload(component) => component.id,
            Self::RadioGroup(component) => component.id,
            Self::Checkbox(component) => component.id,
            Self::Unknown(component) => component.id,
        }
    }

    /// Returns the custom ID of an input (not of action rows, labels and text displays).
    #[must_use]
    pub fn custom_id(&self) -> Option<&str> {
        match self {
            Self::TextInput(component) => Some(&component.custom_id),
            Self::StringSelect(component) | Self::CheckboxGroup(component) => {
                Some(&component.custom_id)
            }
            Self::UserSelect(component) => Some(&component.custom_id),
            Self::RoleSelect(component) => Some(&component.custom_id),
            Self::MentionableSelect(component) => Some(&component.custom_id),
            Self::ChannelSelect(component) => Some(&component.custom_id),
            Self::FileUpload(component) => Some(&component.custom_id),
            Self::RadioGroup(component) => Some(&component.custom_id),
            Self::Checkbox(component) => Some(&component.custom_id),
            Self::ActionRow(_) | Self::TextDisplay(_) | Self::Label(_) | Self::Unknown(_) => None,
        }
    }
}

/// A submitted [action row](SubmittedComponent::ActionRow).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SubmittedActionRow {
    /// The component's numeric ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    /// The submitted components in the row.
    pub components: Vec<SubmittedComponent>,
}

/// A submitted [text input](SubmittedComponent::TextInput).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SubmittedTextInput {
    /// The component's numeric ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    /// The input's custom ID.
    pub custom_id: CompactString,
    /// The text the user entered.
    pub value: CompactString,
}

/// A submitted [string select](SubmittedComponent::StringSelect) or
/// [checkbox group](SubmittedComponent::CheckboxGroup).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SubmittedSelect {
    /// The component's numeric ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    /// The component's custom ID.
    pub custom_id: CompactString,
    /// The values of the selected options (empty if none).
    pub values: Vec<CompactString>,
}

/// A submitted user, role, mentionable or channel select menu: the IDs of the selected
/// entities, whose objects are in [`ModalSubmitData::resolved`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(bound = "")]
#[non_exhaustive]
pub struct SubmittedEntitySelect<T> {
    /// The component's numeric ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    /// The select menu's custom ID.
    pub custom_id: CompactString,
    /// The IDs of the selected entities.
    pub values: Vec<Id<T>>,
}

/// A submitted [text display](SubmittedComponent::TextDisplay).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SubmittedTextDisplay {
    /// The component's numeric ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
}

/// A submitted [label](SubmittedComponent::Label) and the input inside it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SubmittedLabel {
    /// The component's numeric ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    /// The submitted input inside the label.
    pub component: Box<SubmittedComponent>,
}

/// A submitted [file upload](SubmittedComponent::FileUpload): the IDs of the uploaded files,
/// whose objects are in [`Resolved::attachments`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SubmittedFileUpload {
    /// The component's numeric ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    /// The component's custom ID.
    pub custom_id: CompactString,
    /// The IDs of the uploaded files.
    pub values: Vec<AttachmentId>,
}

/// A submitted [radio group](SubmittedComponent::RadioGroup).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SubmittedRadioGroup {
    /// The component's numeric ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    /// The component's custom ID.
    pub custom_id: CompactString,
    /// The value of the selected option (`None` if nothing is selected).
    pub value: Option<CompactString>,
}

/// A submitted [checkbox](SubmittedComponent::Checkbox).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SubmittedCheckbox {
    /// The component's numeric ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    /// The component's custom ID.
    pub custom_id: CompactString,
    /// Whether the checkbox is checked.
    pub value: bool,
}

/// A submitted component of a type this version does not know.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct UnknownSubmittedComponent {
    /// The component's type.
    #[serde(skip)]
    pub kind: ComponentType,
    /// The component's numeric ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
}

/// The `value` of a submitted component: text (text inputs, radio groups) or a boolean
/// (checkboxes).
enum RawSubmittedValue {
    Text(CompactString),
    Boolean(bool),
}

impl<'de> Deserialize<'de> for RawSubmittedValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct RawSubmittedValueVisitor;

        impl Visitor<'_> for RawSubmittedValueVisitor {
            type Value = RawSubmittedValue;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("a string or a boolean")
            }

            fn visit_bool<E: DeError>(self, value: bool) -> Result<RawSubmittedValue, E> {
                Ok(RawSubmittedValue::Boolean(value))
            }

            fn visit_str<E: DeError>(self, value: &str) -> Result<RawSubmittedValue, E> {
                Ok(RawSubmittedValue::Text(value.into()))
            }
        }

        deserializer.deserialize_any(RawSubmittedValueVisitor)
    }
}

/// Every field of a submitted component, decoded in one pass.
#[derive(Deserialize)]
struct RawSubmittedComponent {
    #[serde(rename = "type")]
    kind: ComponentType,
    #[serde(default)]
    id: Option<u32>,
    #[serde(default)]
    custom_id: Option<CompactString>,
    #[serde(default)]
    value: Option<RawSubmittedValue>,
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    values: Vec<CompactString>,
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    components: Vec<SubmittedComponent>,
    #[serde(default)]
    component: Option<Box<SubmittedComponent>>,
}

impl RawSubmittedComponent {
    fn custom_id<E: DeError>(&mut self) -> Result<CompactString, E> {
        self.custom_id
            .take()
            .ok_or_else(|| E::missing_field("custom_id"))
    }

    fn entity_select<T, E: DeError>(mut self) -> Result<SubmittedEntitySelect<T>, E> {
        Ok(SubmittedEntitySelect {
            id: self.id,
            custom_id: self.custom_id()?,
            values: parse_ids(&self.values)?,
        })
    }
}

fn parse_ids<T, E: DeError>(values: &[CompactString]) -> Result<Vec<Id<T>>, E> {
    values
        .iter()
        .map(|value| {
            value
                .parse()
                .map_err(|_| E::invalid_value(Unexpected::Str(value), &"a snowflake ID"))
        })
        .collect()
}

impl<'de> Deserialize<'de> for SubmittedComponent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut raw = RawSubmittedComponent::deserialize(deserializer)?;
        let id = raw.id;
        Ok(match raw.kind {
            ComponentType::ActionRow => Self::ActionRow(SubmittedActionRow {
                id,
                components: raw.components,
            }),
            ComponentType::TextInput => Self::TextInput(SubmittedTextInput {
                id,
                custom_id: raw.custom_id()?,
                value: match raw.value {
                    Some(RawSubmittedValue::Text(value)) => value,
                    Some(RawSubmittedValue::Boolean(value)) => {
                        return Err(D::Error::invalid_type(Unexpected::Bool(value), &"a string"));
                    }
                    None => return Err(D::Error::missing_field("value")),
                },
            }),
            kind @ (ComponentType::StringSelect | ComponentType::CheckboxGroup) => {
                let select = SubmittedSelect {
                    id,
                    custom_id: raw.custom_id()?,
                    values: raw.values,
                };
                if kind == ComponentType::StringSelect {
                    Self::StringSelect(select)
                } else {
                    Self::CheckboxGroup(select)
                }
            }
            ComponentType::UserSelect => Self::UserSelect(raw.entity_select()?),
            ComponentType::RoleSelect => Self::RoleSelect(raw.entity_select()?),
            ComponentType::MentionableSelect => Self::MentionableSelect(raw.entity_select()?),
            ComponentType::ChannelSelect => Self::ChannelSelect(raw.entity_select()?),
            ComponentType::TextDisplay => Self::TextDisplay(SubmittedTextDisplay { id }),
            ComponentType::Label => Self::Label(SubmittedLabel {
                id,
                component: raw
                    .component
                    .ok_or_else(|| D::Error::missing_field("component"))?,
            }),
            ComponentType::FileUpload => Self::FileUpload(SubmittedFileUpload {
                id,
                custom_id: raw.custom_id()?,
                values: parse_ids(&raw.values)?,
            }),
            ComponentType::RadioGroup => Self::RadioGroup(SubmittedRadioGroup {
                id,
                custom_id: raw.custom_id()?,
                value: match raw.value {
                    Some(RawSubmittedValue::Text(value)) => Some(value),
                    Some(RawSubmittedValue::Boolean(value)) => {
                        return Err(D::Error::invalid_type(Unexpected::Bool(value), &"a string"));
                    }
                    None => None,
                },
            }),
            ComponentType::Checkbox => Self::Checkbox(SubmittedCheckbox {
                id,
                custom_id: raw.custom_id()?,
                value: match raw.value {
                    Some(RawSubmittedValue::Boolean(value)) => value,
                    Some(RawSubmittedValue::Text(value)) => {
                        return Err(D::Error::invalid_type(
                            Unexpected::Str(&value),
                            &"a boolean",
                        ));
                    }
                    None => false,
                },
            }),
            kind => Self::Unknown(UnknownSubmittedComponent { kind, id }),
        })
    }
}

/// Serializes a variant's struct together with its `type`.
#[derive(Serialize)]
struct Tagged<'a, T> {
    #[serde(rename = "type")]
    kind: ComponentType,
    #[serde(flatten)]
    component: &'a T,
}

impl Serialize for SubmittedComponent {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let kind = self.kind();
        match self {
            Self::ActionRow(component) => Tagged { kind, component }.serialize(serializer),
            Self::TextInput(component) => Tagged { kind, component }.serialize(serializer),
            Self::StringSelect(component) | Self::CheckboxGroup(component) => {
                Tagged { kind, component }.serialize(serializer)
            }
            Self::UserSelect(component) => Tagged { kind, component }.serialize(serializer),
            Self::RoleSelect(component) => Tagged { kind, component }.serialize(serializer),
            Self::MentionableSelect(component) => Tagged { kind, component }.serialize(serializer),
            Self::ChannelSelect(component) => Tagged { kind, component }.serialize(serializer),
            Self::TextDisplay(component) => Tagged { kind, component }.serialize(serializer),
            Self::Label(component) => Tagged { kind, component }.serialize(serializer),
            Self::FileUpload(component) => Tagged { kind, component }.serialize(serializer),
            Self::RadioGroup(component) => Tagged { kind, component }.serialize(serializer),
            Self::Checkbox(component) => Tagged { kind, component }.serialize(serializer),
            Self::Unknown(component) => Tagged { kind, component }.serialize(serializer),
        }
    }
}

/// The [resolved data](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-object-resolved-data-structure)
/// of an interaction: the objects behind the IDs in options, select menus and file uploads,
/// keyed by ID (D-058).
///
/// A member's user is in [`users`](Self::users) under the same ID.
///
/// discord.js counterpart: the resolved structures of `CommandInteractionOptionResolver` and
/// the select menu interactions (`users`, `members`, `roles`, `channels`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Resolved {
    /// Users.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub users: BTreeMap<UserId, User>,
    /// Members (without user; with their permissions in the channel).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub members: BTreeMap<UserId, InteractionMember>,
    /// Roles.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub roles: BTreeMap<RoleId, Role>,
    /// Partial channels (with the invoking member's permissions in them).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub channels: BTreeMap<ChannelId, InteractionChannel>,
    /// Messages (the target of message commands).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub messages: BTreeMap<MessageId, Message>,
    /// Attachments (attachment options, file uploads).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attachments: BTreeMap<AttachmentId, Attachment>,
}

/// Every field of every kind of interaction data, decoded in one pass; converted by the
/// interaction's `type` (D-067).
#[derive(Deserialize)]
pub(super) struct RawData {
    #[serde(default)]
    id: Option<CommandId>,
    #[serde(default)]
    name: Option<CompactString>,
    #[serde(default, rename = "type")]
    kind: Option<CommandType>,
    #[serde(default)]
    resolved: Option<Box<Resolved>>,
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    options: Vec<CommandDataOption>,
    #[serde(default)]
    guild_id: Option<GuildId>,
    #[serde(default)]
    target_id: Option<GenericId>,
    #[serde(default)]
    custom_id: Option<CompactString>,
    #[serde(default)]
    component_type: Option<ComponentType>,
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    values: Vec<CompactString>,
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    components: Vec<SubmittedComponent>,
}

impl RawData {
    /// Converts the data of an interaction of type `kind`.
    pub(super) fn into_data<E: DeError>(
        data: Option<Self>,
        kind: InteractionType,
    ) -> Result<InteractionData, E> {
        let missing = || E::missing_field("data");
        Ok(match kind {
            InteractionType::Ping => InteractionData::Ping,
            InteractionType::ApplicationCommand => {
                InteractionData::ApplicationCommand(Box::new(data.ok_or_else(missing)?.command()?))
            }
            InteractionType::ApplicationCommandAutocomplete => {
                InteractionData::Autocomplete(Box::new(data.ok_or_else(missing)?.command()?))
            }
            InteractionType::MessageComponent => {
                let data = data.ok_or_else(missing)?;
                InteractionData::MessageComponent(Box::new(ComponentData {
                    custom_id: data
                        .custom_id
                        .ok_or_else(|| E::missing_field("custom_id"))?,
                    component_type: data
                        .component_type
                        .ok_or_else(|| E::missing_field("component_type"))?,
                    values: data.values,
                    resolved: data.resolved,
                }))
            }
            InteractionType::ModalSubmit => {
                let data = data.ok_or_else(missing)?;
                InteractionData::ModalSubmit(Box::new(ModalSubmitData {
                    custom_id: data
                        .custom_id
                        .ok_or_else(|| E::missing_field("custom_id"))?,
                    components: data.components,
                    resolved: data.resolved,
                }))
            }
            InteractionType::Unknown(_) => InteractionData::Unknown(kind),
        })
    }

    fn command<E: DeError>(self) -> Result<CommandData, E> {
        Ok(CommandData {
            id: self.id.ok_or_else(|| E::missing_field("id"))?,
            name: self.name.ok_or_else(|| E::missing_field("name"))?,
            kind: self.kind.unwrap_or_default(),
            resolved: self.resolved,
            options: self.options,
            guild_id: self.guild_id,
            target_id: self.target_id,
        })
    }
}
