//! The select menu builders: [`StringSelectMenuBuilder`] with
//! [`StringSelectMenuOptionBuilder`], and the menus Discord fills itself (users, roles,
//! mentionables, channels).

use compact_str::CompactString;
use djs_model::{
    channel::ChannelType,
    component::{
        AutoPopulatedSelect, ChannelSelect, Component, SelectDefaultValue, SelectDefaultValueType,
        SelectOption, StringSelect,
    },
    emoji::PartialEmoji,
    id::{ChannelId, RoleId, UserId},
};

use super::validate;
use crate::{ValidationError, check::Path};

/// Implements the setters every select menu has on a builder whose model value is
/// `self.select`, and `build` / `build_unchecked` / the conversion into [`Component`].
macro_rules! select_menu {
    ($builder:ident => $variant:ident) => {
        impl $builder {
            /// Sets the text shown while nothing is selected (at most 150 characters).
            ///
            /// discord.js counterpart: `setPlaceholder`.
            pub fn placeholder(mut self, placeholder: impl Into<CompactString>) -> Self {
                self.select.placeholder = Some(placeholder.into());
                self
            }

            /// Sets how many items must be chosen at least (0-25; Discord's default: 1).
            ///
            /// discord.js counterpart: `setMinValues`.
            pub fn min_values(mut self, min_values: u8) -> Self {
                self.select.min_values = Some(min_values);
                self
            }

            /// Sets how many items can be chosen at most (1-25, at least
            /// [`min_values`](Self::min_values); Discord's default: 1).
            ///
            /// discord.js counterpart: `setMaxValues`.
            pub fn max_values(mut self, max_values: u8) -> Self {
                self.select.max_values = Some(max_values);
                self
            }

            /// Sets whether the menu is greyed out (messages only; modals reject it).
            ///
            /// discord.js counterpart: `setDisabled`.
            pub fn disabled(mut self, disabled: bool) -> Self {
                self.select.disabled = disabled;
                self
            }

            /// Sets whether an answer is required when the menu is in a modal (default: yes;
            /// ignored in messages).
            ///
            /// discord.js counterpart: `setRequired`.
            pub fn required(mut self, required: bool) -> Self {
                self.select.required = required;
                self
            }

            /// Sets the component's numeric identifier (Discord generates one otherwise); it
            /// must be unique in the message or modal.
            ///
            /// discord.js counterpart: `setId`.
            pub fn id(mut self, id: i32) -> Self {
                self.select.id = Some(id);
                self
            }

            /// Checks the menu and returns it as a [`Component`].
            ///
            /// # Errors
            ///
            /// The first [`ValidationError`] found: the `custom_id` (1-100 characters), the
            /// placeholder (at most 150), `min_values` (0-25) and `max_values` (1-25,
            /// at least `min_values`), then what the menu offers (see the type docs).
            pub fn build(self) -> Result<Component, ValidationError> {
                let component = Component::from(self);
                validate::component(&Path::Root, &component)?;
                Ok(component)
            }

            /// Returns the menu without any check (D-180).
            #[must_use]
            pub fn build_unchecked(self) -> Component {
                self.into()
            }
        }

        impl From<$builder> for Component {
            fn from(menu: $builder) -> Self {
                Self::$variant(menu.select)
            }
        }
    };
}

/// Builds a [string select menu](https://docs.discord.com/developers/components/reference#string-select):
/// a drop-down of up to 25 options you define, sending the chosen values to your app.
///
/// Add plain options with [`option`](Self::option) and detailed ones with
/// [`option_with`](Self::option_with) (D-203). [`build`](Self::build) checks what Discord
/// checks (D-200): 1-25 options and at least [`max_values`](Self::max_values) of them, labels
/// of 1-100 characters, unique values of 1-100 characters, descriptions of at most 100.
///
/// discord.js counterpart: `StringSelectMenuBuilder`.
///
/// # Examples
///
/// ```
/// use djs_builders::{ActionRowBuilder, StringSelectMenuBuilder, StringSelectMenuOptionBuilder};
///
/// let menu = StringSelectMenuBuilder::new("color")
///     .placeholder("Pick colors")
///     .option("Red", "red")
///     .option("Green", "green")
///     .option_with(
///         StringSelectMenuOptionBuilder::new("Blue", "blue")
///             .description("Like the sky")
///             .default(true),
///     )
///     .max_values(2);
/// let row = ActionRowBuilder::new().component(menu).build().unwrap();
/// assert_eq!(row.children().len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a row or built"]
pub struct StringSelectMenuBuilder {
    select: StringSelect,
}

impl StringSelectMenuBuilder {
    /// Creates a menu without options, sending `custom_id` (1-100 characters) to your app.
    ///
    /// discord.js counterpart: `new StringSelectMenuBuilder().setCustomId(…)`.
    pub fn new(custom_id: impl Into<CompactString>) -> Self {
        Self {
            select: StringSelect::new(custom_id, Vec::new()),
        }
    }

    /// Adds an option showing `label` (1-100 characters) and sending `value` (1-100
    /// characters, unique in the menu).
    ///
    /// discord.js counterpart: `addOptions({ label, value })`.
    pub fn option(
        mut self,
        label: impl Into<CompactString>,
        value: impl Into<CompactString>,
    ) -> Self {
        self.select.options.push(SelectOption::new(label, value));
        self
    }

    /// Adds an option with a description, an emoji or selected by default.
    ///
    /// discord.js counterpart: `addOptions(new StringSelectMenuOptionBuilder()…)`.
    pub fn option_with(mut self, option: StringSelectMenuOptionBuilder) -> Self {
        self.select.options.push(option.option);
        self
    }

    /// Adds several options: [`StringSelectMenuOptionBuilder`]s or `djs-model`
    /// [`SelectOption`]s.
    ///
    /// discord.js counterpart: `addOptions`.
    pub fn options<O: Into<SelectOption>>(mut self, options: impl IntoIterator<Item = O>) -> Self {
        self.select
            .options
            .extend(options.into_iter().map(Into::into));
        self
    }
}

select_menu!(StringSelectMenuBuilder => StringSelect);

/// Builds an [option of a string select menu](https://docs.discord.com/developers/components/reference#string-select-select-option-structure)
/// with more than a label and a value, for [`StringSelectMenuBuilder::option_with`].
///
/// discord.js counterpart: `StringSelectMenuOptionBuilder`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "an option does nothing until it is added to a menu"]
pub struct StringSelectMenuOptionBuilder {
    option: SelectOption,
}

impl StringSelectMenuOptionBuilder {
    /// Creates an option showing `label` (1-100 characters) and sending `value` (1-100
    /// characters).
    ///
    /// discord.js counterpart: `new StringSelectMenuOptionBuilder().setLabel(…).setValue(…)`.
    pub fn new(label: impl Into<CompactString>, value: impl Into<CompactString>) -> Self {
        Self {
            option: SelectOption::new(label, value),
        }
    }

    /// Sets the smaller text under the label (at most 100 characters).
    ///
    /// discord.js counterpart: `setDescription`.
    pub fn description(mut self, description: impl Into<CompactString>) -> Self {
        self.option.description = Some(description.into());
        self
    }

    /// Sets the emoji shown before the label.
    ///
    /// discord.js counterpart: `setEmoji`.
    pub fn emoji(mut self, emoji: impl Into<PartialEmoji>) -> Self {
        self.option.emoji = Some(emoji.into());
        self
    }

    /// Sets whether the option is selected when the menu appears.
    ///
    /// discord.js counterpart: `setDefault`.
    pub fn default(mut self, default: bool) -> Self {
        self.option.default = default;
        self
    }
}

impl From<StringSelectMenuOptionBuilder> for SelectOption {
    fn from(option: StringSelectMenuOptionBuilder) -> Self {
        option.option
    }
}

/// Implements the default-value setter `$method` adding IDs of type `$id` as `$kind`.
macro_rules! default_values {
    ($method:ident, $id:ty, $kind:ident, $what:literal, $djs:literal) => {
        #[doc = concat!("Adds ", $what, " selected when the menu appears; their number must be within")]
        /// [`min_values`](Self::min_values) and [`max_values`](Self::max_values).
        ///
        #[doc = concat!("discord.js counterpart: `", $djs, "`.")]
        pub fn $method(mut self, ids: impl IntoIterator<Item = $id>) -> Self {
            self.select.default_values.extend(
                ids.into_iter()
                    .map(|id| SelectDefaultValue::new(id.cast(), SelectDefaultValueType::$kind)),
            );
            self
        }
    };
}

/// Builds a [user select menu](https://docs.discord.com/developers/components/reference#user-select),
/// which Discord fills with the server's members.
///
/// discord.js counterpart: `UserSelectMenuBuilder`.
///
/// # Examples
///
/// ```
/// use djs_builders::UserSelectMenuBuilder;
/// use djs_model::id::UserId;
///
/// let menu = UserSelectMenuBuilder::new("invite")
///     .max_values(3)
///     .default_users([UserId::new(80_351_110_224_678_912)])
///     .build()
///     .unwrap();
/// assert_eq!(menu.custom_id(), Some("invite"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a row or built"]
pub struct UserSelectMenuBuilder {
    select: AutoPopulatedSelect,
}

impl UserSelectMenuBuilder {
    /// Creates a menu sending `custom_id` (1-100 characters) to your app.
    pub fn new(custom_id: impl Into<CompactString>) -> Self {
        Self {
            select: AutoPopulatedSelect::new(custom_id),
        }
    }

    default_values!(default_users, UserId, User, "users", "addDefaultUsers");
}

select_menu!(UserSelectMenuBuilder => UserSelect);

/// Builds a [role select menu](https://docs.discord.com/developers/components/reference#role-select),
/// which Discord fills with the server's roles.
///
/// discord.js counterpart: `RoleSelectMenuBuilder`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a row or built"]
pub struct RoleSelectMenuBuilder {
    select: AutoPopulatedSelect,
}

impl RoleSelectMenuBuilder {
    /// Creates a menu sending `custom_id` (1-100 characters) to your app.
    pub fn new(custom_id: impl Into<CompactString>) -> Self {
        Self {
            select: AutoPopulatedSelect::new(custom_id),
        }
    }

    default_values!(default_roles, RoleId, Role, "roles", "addDefaultRoles");
}

select_menu!(RoleSelectMenuBuilder => RoleSelect);

/// Builds a [mentionable select menu](https://docs.discord.com/developers/components/reference#mentionable-select),
/// which Discord fills with the server's members and roles.
///
/// discord.js counterpart: `MentionableSelectMenuBuilder`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a row or built"]
pub struct MentionableSelectMenuBuilder {
    select: AutoPopulatedSelect,
}

impl MentionableSelectMenuBuilder {
    /// Creates a menu sending `custom_id` (1-100 characters) to your app.
    pub fn new(custom_id: impl Into<CompactString>) -> Self {
        Self {
            select: AutoPopulatedSelect::new(custom_id),
        }
    }

    default_values!(default_users, UserId, User, "users", "addDefaultUsers");
    default_values!(default_roles, RoleId, Role, "roles", "addDefaultRoles");
}

select_menu!(MentionableSelectMenuBuilder => MentionableSelect);

/// Builds a [channel select menu](https://docs.discord.com/developers/components/reference#channel-select),
/// which Discord fills with the server's channels, optionally only of some
/// [types](Self::channel_types).
///
/// discord.js counterpart: `ChannelSelectMenuBuilder`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a row or built"]
pub struct ChannelSelectMenuBuilder {
    select: ChannelSelect,
}

impl ChannelSelectMenuBuilder {
    /// Creates a menu sending `custom_id` (1-100 characters) to your app.
    pub fn new(custom_id: impl Into<CompactString>) -> Self {
        Self {
            select: ChannelSelect::new(custom_id),
        }
    }

    /// Sets the channel types offered (all when not set), replacing those set before.
    ///
    /// discord.js counterpart: `setChannelTypes`.
    pub fn channel_types(mut self, channel_types: impl IntoIterator<Item = ChannelType>) -> Self {
        self.select.channel_types = channel_types.into_iter().collect();
        self
    }

    default_values!(
        default_channels,
        ChannelId,
        Channel,
        "channels",
        "addDefaultChannels"
    );
}

select_menu!(ChannelSelectMenuBuilder => ChannelSelect);
