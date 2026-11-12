//! The option types of slash commands: `SlashCommandStringOption`, `SlashCommandIntegerOption`,
//! and so on, one per [option type](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-option-type).

use compact_str::CompactString;
use djs_model::{
    application_command::{
        AttachmentOption, BasicOption, ChannelOption, Choice, CommandOption, IntegerOption,
        NumberOption, StringOption,
    },
    channel::ChannelType,
};

use super::validate;
use crate::ValidationError;

/// Defines an option type: the struct around its model value, `new`, `required`, the
/// translations, `build` / `build_unchecked` and the conversion into [`CommandOption`].
macro_rules! option_type {
    (
        $(#[$meta:meta])*
        $name:ident($model:ident) => $variant:ident
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq)]
        #[must_use = "an option does nothing until it is added to a command or built"]
        pub struct $name {
            option: $model,
        }

        impl $name {
            /// Creates an optional option named `name` (1-32 lower-case letters, numbers, `-`,
            /// `_` or `ʼ`) and described by `description` (1-100 characters).
            pub fn new(
                name: impl Into<CompactString>,
                description: impl Into<CompactString>,
            ) -> Self {
                Self {
                    option: $model::new(name, description),
                }
            }

            /// Sets whether users must fill the option in (default: no). Required options
            /// must come before optional ones.
            ///
            /// discord.js counterpart: `setRequired`.
            pub fn required(mut self, required: bool) -> Self {
                self.option.required = required;
                self
            }

            name_localizations!(option);
            description_localizations!(option);

            /// Checks the option on its own and returns it as a [`CommandOption`]; the paths
            /// in a [`ValidationError`] start at its fields (`name`, `choices[0].name`).
            ///
            /// Adding the builder to a command checks it as part of the command instead.
            ///
            /// # Errors
            ///
            /// The first [`ValidationError`] found, as in
            /// [`SlashCommandBuilder::build`](crate::SlashCommandBuilder::build).
            pub fn build(self) -> Result<CommandOption, ValidationError> {
                let option = CommandOption::from(self);
                validate::option_alone(&option)?;
                Ok(option)
            }

            /// Returns the option without any check (D-180).
            #[must_use]
            pub fn build_unchecked(self) -> CommandOption {
                self.into()
            }
        }

        impl From<$name> for CommandOption {
            fn from(option: $name) -> Self {
                Self::$variant(option.option)
            }
        }
    };
}

option_type! {
    /// Builds a [string option](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-option-type)
    /// of a slash command: free text, optionally limited in length, picked from up to 25
    /// [choices](Self::choice), or suggested by the app through
    /// [autocomplete](Self::autocomplete) (not together with choices, D-196).
    ///
    /// discord.js counterpart: `SlashCommandStringOption`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_builders::{SlashCommandBuilder, SlashCommandStringOption};
    ///
    /// let command = SlashCommandBuilder::new("language", "Sets your language")
    ///     .option(
    ///         SlashCommandStringOption::new("language", "The language")
    ///             .required(true)
    ///             .choice("English", "en")
    ///             .choice("Polski", "pl"),
    ///     )
    ///     .option(SlashCommandStringOption::new("note", "Anything else").max_length(200))
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(command.options.len(), 2);
    /// ```
    SlashCommandStringOption(StringOption) => String
}

impl SlashCommandStringOption {
    /// Adds a choice: `name` is shown to users (1-100 characters), `value` is what the app
    /// receives (at most 100 characters). An option has at most 25 choices.
    ///
    /// discord.js counterpart: `addChoices`.
    pub fn choice(
        mut self,
        name: impl Into<CompactString>,
        value: impl Into<CompactString>,
    ) -> Self {
        self.option.choices.push(Choice::new(name, value.into()));
        self
    }

    /// Adds several choices, after those already added — for example with translated names
    /// ([`Choice::name_localizations`]).
    ///
    /// discord.js counterpart: `addChoices`.
    pub fn choices(mut self, choices: impl IntoIterator<Item = Choice<CompactString>>) -> Self {
        self.option.choices.extend(choices);
        self
    }

    /// Sets the shortest text users may enter (0-6000).
    ///
    /// discord.js counterpart: `setMinLength`.
    pub fn min_length(mut self, min_length: u16) -> Self {
        self.option.min_length = Some(min_length);
        self
    }

    /// Sets the longest text users may enter (1-6000, at least the
    /// [`min_length`](Self::min_length)).
    ///
    /// discord.js counterpart: `setMaxLength`.
    pub fn max_length(mut self, max_length: u16) -> Self {
        self.option.max_length = Some(max_length);
        self
    }

    /// Sets whether the app suggests values as users type, through autocomplete interactions
    /// (not together with choices).
    ///
    /// discord.js counterpart: `setAutocomplete`.
    pub fn autocomplete(mut self, autocomplete: bool) -> Self {
        self.option.autocomplete = autocomplete;
        self
    }
}

option_type! {
    /// Builds an [integer option](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-option-type)
    /// of a slash command: a whole number between -(2^53 − 1) and 2^53 − 1, optionally bounded,
    /// picked from up to 25 [choices](Self::choice), or suggested through
    /// [autocomplete](Self::autocomplete).
    ///
    /// discord.js counterpart: `SlashCommandIntegerOption`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_builders::{SlashCommandBuilder, SlashCommandIntegerOption};
    ///
    /// let command = SlashCommandBuilder::new("roll", "Rolls a die")
    ///     .option(SlashCommandIntegerOption::new("sides", "How many sides").min_value(2).max_value(100))
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(command.options[0].name(), "sides");
    /// ```
    SlashCommandIntegerOption(IntegerOption) => Integer
}

impl SlashCommandIntegerOption {
    /// Adds a choice: `name` is shown to users (1-100 characters), `value` is what the app
    /// receives. An option has at most 25 choices.
    ///
    /// discord.js counterpart: `addChoices`.
    pub fn choice(mut self, name: impl Into<CompactString>, value: i64) -> Self {
        self.option.choices.push(Choice::new(name, value));
        self
    }

    /// Adds several choices, after those already added.
    ///
    /// discord.js counterpart: `addChoices`.
    pub fn choices(mut self, choices: impl IntoIterator<Item = Choice<i64>>) -> Self {
        self.option.choices.extend(choices);
        self
    }

    /// Sets the smallest value users may enter.
    ///
    /// discord.js counterpart: `setMinValue`.
    pub fn min_value(mut self, min_value: i64) -> Self {
        self.option.min_value = Some(min_value);
        self
    }

    /// Sets the largest value users may enter (at least the [`min_value`](Self::min_value)).
    ///
    /// discord.js counterpart: `setMaxValue`.
    pub fn max_value(mut self, max_value: i64) -> Self {
        self.option.max_value = Some(max_value);
        self
    }

    /// Sets whether the app suggests values as users type (not together with choices).
    ///
    /// discord.js counterpart: `setAutocomplete`.
    pub fn autocomplete(mut self, autocomplete: bool) -> Self {
        self.option.autocomplete = autocomplete;
        self
    }
}

option_type! {
    /// Builds a [number option](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-option-type)
    /// of a slash command: a finite double between -2^53 and 2^53, optionally bounded, picked
    /// from up to 25 [choices](Self::choice), or suggested through
    /// [autocomplete](Self::autocomplete).
    ///
    /// discord.js counterpart: `SlashCommandNumberOption`.
    SlashCommandNumberOption(NumberOption) => Number
}

impl SlashCommandNumberOption {
    /// Adds a choice: `name` is shown to users (1-100 characters), `value` is what the app
    /// receives. An option has at most 25 choices.
    ///
    /// discord.js counterpart: `addChoices`.
    pub fn choice(mut self, name: impl Into<CompactString>, value: f64) -> Self {
        self.option.choices.push(Choice::new(name, value));
        self
    }

    /// Adds several choices, after those already added.
    ///
    /// discord.js counterpart: `addChoices`.
    pub fn choices(mut self, choices: impl IntoIterator<Item = Choice<f64>>) -> Self {
        self.option.choices.extend(choices);
        self
    }

    /// Sets the smallest value users may enter.
    ///
    /// discord.js counterpart: `setMinValue`.
    pub fn min_value(mut self, min_value: f64) -> Self {
        self.option.min_value = Some(min_value);
        self
    }

    /// Sets the largest value users may enter (at least the [`min_value`](Self::min_value)).
    ///
    /// discord.js counterpart: `setMaxValue`.
    pub fn max_value(mut self, max_value: f64) -> Self {
        self.option.max_value = Some(max_value);
        self
    }

    /// Sets whether the app suggests values as users type (not together with choices).
    ///
    /// discord.js counterpart: `setAutocomplete`.
    pub fn autocomplete(mut self, autocomplete: bool) -> Self {
        self.option.autocomplete = autocomplete;
        self
    }
}

option_type! {
    /// Builds a [boolean option](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-option-type)
    /// of a slash command: true or false.
    ///
    /// discord.js counterpart: `SlashCommandBooleanOption`.
    SlashCommandBooleanOption(BasicOption) => Boolean
}

option_type! {
    /// Builds a [user option](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-option-type)
    /// of a slash command: a user (and their member, in a server).
    ///
    /// discord.js counterpart: `SlashCommandUserOption`.
    SlashCommandUserOption(BasicOption) => User
}

option_type! {
    /// Builds a [role option](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-option-type)
    /// of a slash command.
    ///
    /// discord.js counterpart: `SlashCommandRoleOption`.
    SlashCommandRoleOption(BasicOption) => Role
}

option_type! {
    /// Builds a [mentionable option](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-option-type)
    /// of a slash command: a user or a role.
    ///
    /// discord.js counterpart: `SlashCommandMentionableOption`.
    SlashCommandMentionableOption(BasicOption) => Mentionable
}

option_type! {
    /// Builds a [channel option](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-option-type)
    /// of a slash command: a channel or category, optionally only of some
    /// [types](Self::channel_types).
    ///
    /// discord.js counterpart: `SlashCommandChannelOption`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_builders::SlashCommandChannelOption;
    /// use djs_model::channel::ChannelType;
    ///
    /// let option = SlashCommandChannelOption::new("channel", "Where to post")
    ///     .channel_types([ChannelType::GuildText, ChannelType::GuildAnnouncement])
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(option.name(), "channel");
    /// ```
    SlashCommandChannelOption(ChannelOption) => Channel
}

impl SlashCommandChannelOption {
    /// Sets the channel types offered (all when not set), replacing those set before.
    ///
    /// discord.js counterpart: `addChannelTypes`.
    pub fn channel_types(mut self, channel_types: impl IntoIterator<Item = ChannelType>) -> Self {
        self.option.channel_types = channel_types.into_iter().collect();
        self
    }
}

option_type! {
    /// Builds an [attachment option](https://docs.discord.com/developers/interactions/application-commands#application-command-object-application-command-option-type)
    /// of a slash command: a file users upload.
    ///
    /// discord.js counterpart: `SlashCommandAttachmentOption`.
    SlashCommandAttachmentOption(AttachmentOption) => Attachment
}

impl SlashCommandAttachmentOption {
    /// Sets the [file types](https://docs.discord.com/developers/reference#file-type-filtering)
    /// users may pick, as extensions such as `png` (all when not set), replacing those set
    /// before.
    pub fn file_types<T: Into<CompactString>>(
        mut self,
        file_types: impl IntoIterator<Item = T>,
    ) -> Self {
        self.option.file_types = file_types.into_iter().map(Into::into).collect();
        self
    }
}
