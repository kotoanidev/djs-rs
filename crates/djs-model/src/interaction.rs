//! [Interactions](https://docs.discord.com/developers/interactions/receiving-and-responding):
//! what a user does with an app (commands, components, modals).
//!
//! **Coverage:** the types messages refer to. The interaction object itself, its data and the
//! responses come with the interactions step of Phase 2.

use core::fmt;

use compact_str::CompactString;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as DeError, Unexpected, Visitor},
};

use crate::{
    guild::PartialMember,
    id::{GuildId, InteractionId, UserId},
    macros::int_enum,
    user::User,
};

int_enum! {
    /// The [type of an interaction](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-object-interaction-type).
    ///
    /// discord.js counterpart: `InteractionType`.
    pub enum InteractionType: u8 {
        /// Discord checks that an HTTP interactions endpoint works.
        Ping = 1,
        /// A slash command or a context menu command.
        ApplicationCommand = 2,
        /// A button, select menu or other message component.
        MessageComponent = 3,
        /// Autocomplete suggestions for a command option.
        ApplicationCommandAutocomplete = 4,
        /// A submitted modal.
        ModalSubmit = 5,
    }
}

/// The [interaction a message responds to](https://docs.discord.com/developers/interactions/receiving-and-responding#message-interaction-object)
/// (`Message::interaction`).
///
/// **Deprecated** by Discord in favor of
/// [`Message::interaction_metadata`](crate::message::Message::interaction_metadata).
///
/// discord.js counterpart: `MessageInteraction` (`Message#interaction`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MessageInteraction {
    /// The interaction's ID.
    pub id: InteractionId,
    /// The kind of interaction.
    #[serde(rename = "type")]
    pub kind: InteractionType,
    /// The name of the command, including subcommands and subcommand groups.
    pub name: CompactString,
    /// The user who invoked the interaction.
    pub user: User,
    /// The member who invoked the interaction, in guilds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Box<PartialMember>>,
}

/// The [installations that authorized an interaction](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-object-authorizing-integration-owners-object),
/// keyed by [`IntegrationType`](crate::application::IntegrationType).
///
/// A key is only present when the app is installed in that context, the interaction's context
/// supports it and, for commands, the command supports that installation context.
///
/// discord.js counterpart: `authorizingIntegrationOwners`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AuthorizingIntegrationOwners {
    /// The guild install (key `"0"`).
    #[serde(rename = "0", default, skip_serializing_if = "Option::is_none")]
    pub guild_install: Option<GuildInstallOwner>,
    /// The ID of the user who installed the app (key `"1"`).
    #[serde(rename = "1", default, skip_serializing_if = "Option::is_none")]
    pub user_install: Option<UserId>,
}

/// The owner of a guild install in [`AuthorizingIntegrationOwners`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GuildInstallOwner {
    /// The interaction happened in this guild.
    Guild(GuildId),
    /// The interaction happened in a DM with the app's bot user (Discord sends `"0"`).
    BotDm,
}

impl GuildInstallOwner {
    /// Returns the guild ID, unless the interaction happened in a DM with the bot.
    #[must_use]
    pub const fn guild_id(self) -> Option<GuildId> {
        match self {
            Self::Guild(id) => Some(id),
            Self::BotDm => None,
        }
    }
}

impl Serialize for GuildInstallOwner {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Guild(id) => id.serialize(serializer),
            Self::BotDm => serializer.serialize_str("0"),
        }
    }
}

impl<'de> Deserialize<'de> for GuildInstallOwner {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct OwnerVisitor;

        impl Visitor<'_> for OwnerVisitor {
            type Value = GuildInstallOwner;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a guild ID or \"0\"")
            }

            fn visit_str<E: DeError>(self, value: &str) -> Result<GuildInstallOwner, E> {
                if value == "0" {
                    return Ok(GuildInstallOwner::BotDm);
                }
                value
                    .parse()
                    .map(GuildInstallOwner::Guild)
                    .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_str(OwnerVisitor)
    }
}
