//! Metadata about the interaction a message was sent for (D-063).

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use crate::{
    id::{InteractionId, MessageId},
    interaction::{AuthorizingIntegrationOwners, InteractionType},
    user::User,
};

/// [Metadata about the interaction](https://docs.discord.com/developers/resources/message#message-interaction-metadata-object)
/// a message was sent for: its ID, who triggered it and the installations that authorized it.
///
/// One variant per interaction type (D-063). The fields every variant has are available through
/// methods ([`id`](Self::id), [`user`](Self::user), …).
///
/// discord.js counterpart: `MessageInteractionMetadata` (`Message#interactionMetadata`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
#[doc(alias = "MessageInteractionMetadata")]
pub enum InteractionMetadata {
    /// A slash command or context menu command.
    ApplicationCommand(CommandMetadata),
    /// A message component (button, select menu, …).
    MessageComponent(ComponentMetadata),
    /// A submitted modal.
    ModalSubmit(ModalSubmitMetadata),
    /// An interaction type this version of the library does not know about.
    Unknown(UnknownMetadata),
}

/// The [metadata of an application command interaction](https://docs.discord.com/developers/resources/message#message-interaction-metadata-object-application-command-interaction-metadata-structure).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[doc(alias = "ApplicationCommandInteractionMetadata")]
pub struct CommandMetadata {
    /// The interaction's ID.
    pub id: InteractionId,
    /// The user who triggered the interaction.
    pub user: User,
    /// The installations that authorized the interaction.
    pub authorizing_integration_owners: AuthorizingIntegrationOwners,
    /// The original response message, on follow-up messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_response_message_id: Option<MessageId>,
    /// The user a user command was run on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_user: Option<Box<User>>,
    /// The message a message command was run on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_message_id: Option<MessageId>,
}

/// The [metadata of a message component interaction](https://docs.discord.com/developers/resources/message#message-interaction-metadata-object-message-component-interaction-metadata-structure).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[doc(alias = "MessageComponentInteractionMetadata")]
pub struct ComponentMetadata {
    /// The interaction's ID.
    pub id: InteractionId,
    /// The user who triggered the interaction.
    pub user: User,
    /// The installations that authorized the interaction.
    pub authorizing_integration_owners: AuthorizingIntegrationOwners,
    /// The original response message, on follow-up messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_response_message_id: Option<MessageId>,
    /// The message that contained the component.
    pub interacted_message_id: MessageId,
}

/// The [metadata of a modal submit interaction](https://docs.discord.com/developers/resources/message#message-interaction-metadata-object-modal-submit-interaction-metadata-structure).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[doc(alias = "ModalSubmitInteractionMetadata")]
pub struct ModalSubmitMetadata {
    /// The interaction's ID.
    pub id: InteractionId,
    /// The user who triggered the interaction.
    pub user: User,
    /// The installations that authorized the interaction.
    pub authorizing_integration_owners: AuthorizingIntegrationOwners,
    /// The original response message, on follow-up messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_response_message_id: Option<MessageId>,
    /// The interaction that opened the modal (an application command or a message component).
    pub triggering_interaction_metadata: Box<InteractionMetadata>,
}

/// The metadata of an interaction type this version of the library does not know about: the
/// fields every interaction metadata has.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct UnknownMetadata {
    /// The interaction type.
    #[serde(skip)]
    pub kind: InteractionType,
    /// The interaction's ID.
    pub id: InteractionId,
    /// The user who triggered the interaction.
    pub user: User,
    /// The installations that authorized the interaction.
    pub authorizing_integration_owners: AuthorizingIntegrationOwners,
    /// The original response message, on follow-up messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_response_message_id: Option<MessageId>,
}

impl InteractionMetadata {
    /// Returns the interaction's ID.
    #[must_use]
    pub const fn id(&self) -> InteractionId {
        match self {
            Self::ApplicationCommand(metadata) => metadata.id,
            Self::MessageComponent(metadata) => metadata.id,
            Self::ModalSubmit(metadata) => metadata.id,
            Self::Unknown(metadata) => metadata.id,
        }
    }

    /// Returns the interaction type.
    #[must_use]
    pub const fn kind(&self) -> InteractionType {
        match self {
            Self::ApplicationCommand(_) => InteractionType::ApplicationCommand,
            Self::MessageComponent(_) => InteractionType::MessageComponent,
            Self::ModalSubmit(_) => InteractionType::ModalSubmit,
            Self::Unknown(metadata) => metadata.kind,
        }
    }

    /// Returns the user who triggered the interaction.
    #[must_use]
    pub const fn user(&self) -> &User {
        match self {
            Self::ApplicationCommand(metadata) => &metadata.user,
            Self::MessageComponent(metadata) => &metadata.user,
            Self::ModalSubmit(metadata) => &metadata.user,
            Self::Unknown(metadata) => &metadata.user,
        }
    }

    /// Returns the installations that authorized the interaction.
    #[must_use]
    pub const fn authorizing_integration_owners(&self) -> &AuthorizingIntegrationOwners {
        match self {
            Self::ApplicationCommand(metadata) => &metadata.authorizing_integration_owners,
            Self::MessageComponent(metadata) => &metadata.authorizing_integration_owners,
            Self::ModalSubmit(metadata) => &metadata.authorizing_integration_owners,
            Self::Unknown(metadata) => &metadata.authorizing_integration_owners,
        }
    }

    /// Returns the original response message, on follow-up messages.
    #[must_use]
    pub const fn original_response_message_id(&self) -> Option<MessageId> {
        match self {
            Self::ApplicationCommand(metadata) => metadata.original_response_message_id,
            Self::MessageComponent(metadata) => metadata.original_response_message_id,
            Self::ModalSubmit(metadata) => metadata.original_response_message_id,
            Self::Unknown(metadata) => metadata.original_response_message_id,
        }
    }
}

/// Every field of every variant, decoded in one pass and then converted by `type` (D-063).
#[derive(Deserialize)]
struct RawMetadata {
    #[serde(rename = "type")]
    kind: InteractionType,
    id: InteractionId,
    user: User,
    authorizing_integration_owners: AuthorizingIntegrationOwners,
    #[serde(default)]
    original_response_message_id: Option<MessageId>,
    #[serde(default)]
    target_user: Option<Box<User>>,
    #[serde(default)]
    target_message_id: Option<MessageId>,
    #[serde(default)]
    interacted_message_id: Option<MessageId>,
    #[serde(default)]
    triggering_interaction_metadata: Option<Box<InteractionMetadata>>,
}

impl<'de> Deserialize<'de> for InteractionMetadata {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawMetadata::deserialize(deserializer)?;
        Ok(match raw.kind {
            InteractionType::ApplicationCommand => Self::ApplicationCommand(CommandMetadata {
                id: raw.id,
                user: raw.user,
                authorizing_integration_owners: raw.authorizing_integration_owners,
                original_response_message_id: raw.original_response_message_id,
                target_user: raw.target_user,
                target_message_id: raw.target_message_id,
            }),
            InteractionType::MessageComponent => Self::MessageComponent(ComponentMetadata {
                id: raw.id,
                user: raw.user,
                authorizing_integration_owners: raw.authorizing_integration_owners,
                original_response_message_id: raw.original_response_message_id,
                interacted_message_id: raw
                    .interacted_message_id
                    .ok_or_else(|| D::Error::missing_field("interacted_message_id"))?,
            }),
            InteractionType::ModalSubmit => Self::ModalSubmit(ModalSubmitMetadata {
                id: raw.id,
                user: raw.user,
                authorizing_integration_owners: raw.authorizing_integration_owners,
                original_response_message_id: raw.original_response_message_id,
                triggering_interaction_metadata: raw
                    .triggering_interaction_metadata
                    .ok_or_else(|| D::Error::missing_field("triggering_interaction_metadata"))?,
            }),
            kind => Self::Unknown(UnknownMetadata {
                kind,
                id: raw.id,
                user: raw.user,
                authorizing_integration_owners: raw.authorizing_integration_owners,
                original_response_message_id: raw.original_response_message_id,
            }),
        })
    }
}

/// Serializes a variant's struct together with its `type`.
#[derive(Serialize)]
struct Tagged<'a, T> {
    #[serde(rename = "type")]
    kind: InteractionType,
    #[serde(flatten)]
    metadata: &'a T,
}

impl Serialize for InteractionMetadata {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let kind = self.kind();
        match self {
            Self::ApplicationCommand(metadata) => Tagged { kind, metadata }.serialize(serializer),
            Self::MessageComponent(metadata) => Tagged { kind, metadata }.serialize(serializer),
            Self::ModalSubmit(metadata) => Tagged { kind, metadata }.serialize(serializer),
            Self::Unknown(metadata) => Tagged { kind, metadata }.serialize(serializer),
        }
    }
}
