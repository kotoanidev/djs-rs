//! Builders for [application commands](https://docs.discord.com/developers/interactions/application-commands):
//! slash commands with their options and subcommands, and context-menu commands.

/// Implements the name translation setters on a builder whose model value is `self.$field`.
macro_rules! name_localizations {
    ($field:ident) => {
        /// Sets the name shown to users whose Discord language is `locale`. It follows the same
        /// rules as the name itself.
        ///
        /// discord.js counterpart: `setNameLocalization`.
        pub fn name_localization(
            mut self,
            locale: ::djs_model::util::Locale,
            name: impl Into<::compact_str::CompactString>,
        ) -> Self {
            self.$field.name_localizations.insert(locale, name.into());
            self
        }

        /// Sets several name translations at once, keeping those set before (a locale given
        /// again gets the new name).
        ///
        /// discord.js counterpart: `setNameLocalizations` (which replaces them all).
        pub fn name_localizations<N: Into<::compact_str::CompactString>>(
            mut self,
            localizations: impl IntoIterator<Item = (::djs_model::util::Locale, N)>,
        ) -> Self {
            self.$field.name_localizations.extend(
                localizations
                    .into_iter()
                    .map(|(locale, name)| (locale, name.into())),
            );
            self
        }
    };
}

/// Implements the description translation setters on a builder whose model value is
/// `self.$field`.
macro_rules! description_localizations {
    ($field:ident) => {
        /// Sets the description shown to users whose Discord language is `locale` (1-100
        /// characters, like the description).
        ///
        /// discord.js counterpart: `setDescriptionLocalization`.
        pub fn description_localization(
            mut self,
            locale: ::djs_model::util::Locale,
            description: impl Into<::compact_str::CompactString>,
        ) -> Self {
            self.$field
                .description_localizations
                .insert(locale, description.into());
            self
        }

        /// Sets several description translations at once, keeping those set before.
        ///
        /// discord.js counterpart: `setDescriptionLocalizations` (which replaces them all).
        pub fn description_localizations<D: Into<::compact_str::CompactString>>(
            mut self,
            localizations: impl IntoIterator<Item = (::djs_model::util::Locale, D)>,
        ) -> Self {
            self.$field.description_localizations.extend(
                localizations
                    .into_iter()
                    .map(|(locale, description)| (locale, description.into())),
            );
            self
        }
    };
}

/// Implements the settings every command has on a builder whose model value is
/// `self.command`.
macro_rules! command_settings {
    () => {
        /// Sets the permissions a member needs to use the command by default; server
        /// administrators can change who may use it.
        /// [`Permissions::empty()`](::djs_model::permissions::Permissions::empty) limits it to
        /// administrators. Without this, everyone can use it.
        ///
        /// discord.js counterpart: `setDefaultMemberPermissions`.
        pub fn default_member_permissions(
            mut self,
            permissions: ::djs_model::permissions::Permissions,
        ) -> Self {
            self.command.default_member_permissions = Some(permissions);
            self
        }

        /// Sets where the command can be used: in servers, in DMs with the bot, and in other
        /// DMs and group DMs. Without this, Discord allows all of them.
        ///
        /// discord.js counterpart: `setContexts`.
        pub fn contexts(
            mut self,
            contexts: impl IntoIterator<Item = ::djs_model::interaction::InteractionContextType>,
        ) -> Self {
            self.command.contexts = contexts.into_iter().collect();
            self
        }

        /// Sets which installations offer the command: the app installed to a server, to a
        /// user, or both. Without this, Discord uses the installation types the app supports.
        ///
        /// discord.js counterpart: `setIntegrationTypes`.
        pub fn integration_types(
            mut self,
            integration_types: impl IntoIterator<Item = ::djs_model::application::IntegrationType>,
        ) -> Self {
            self.command.integration_types = integration_types.into_iter().collect();
            self
        }

        /// Marks the command as age-restricted, so it can only be used in age-restricted
        /// channels.
        ///
        /// discord.js counterpart: `setNSFW`.
        pub fn nsfw(mut self, nsfw: bool) -> Self {
            self.command.nsfw = nsfw;
            self
        }
    };
}

mod context_menu;
mod option;
mod slash;
mod validate;

pub use self::{
    context_menu::ContextMenuCommandBuilder,
    option::{
        SlashCommandAttachmentOption, SlashCommandBooleanOption, SlashCommandChannelOption,
        SlashCommandIntegerOption, SlashCommandMentionableOption, SlashCommandNumberOption,
        SlashCommandRoleOption, SlashCommandStringOption, SlashCommandUserOption,
    },
    slash::{
        SlashCommandBuilder, SlashCommandSubcommandBuilder, SlashCommandSubcommandGroupBuilder,
    },
};
