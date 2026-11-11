//! [Interaction responses](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-response-object)
//! and the [callback response](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-callback)
//! Discord returns for them (D-068).

use compact_str::CompactString;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};

use super::InteractionType;
use crate::{
    application_command::{Choice, RawValue, convert_choices},
    component::Component,
    id::{InteractionId, MessageId},
    macros::int_enum,
    message::{AllowedMentions, AttachmentRequest, Embed, Message, MessageFlags},
    poll::CreatePoll,
    rest::{Attachments, FileUpload},
};

int_enum! {
    /// The [type of an interaction response](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-response-object-interaction-callback-type).
    ///
    /// discord.js counterpart: `InteractionResponseType`.
    #[doc(alias = "InteractionResponseType")]
    pub enum InteractionCallbackType: u8 {
        /// [`InteractionResponse::Pong`].
        Pong = 1,
        /// [`InteractionResponse::ChannelMessageWithSource`].
        ChannelMessageWithSource = 4,
        /// [`InteractionResponse::DeferredChannelMessageWithSource`].
        DeferredChannelMessageWithSource = 5,
        /// [`InteractionResponse::DeferredUpdateMessage`].
        DeferredUpdateMessage = 6,
        /// [`InteractionResponse::UpdateMessage`].
        UpdateMessage = 7,
        /// [`InteractionResponse::AutocompleteResult`].
        ApplicationCommandAutocompleteResult = 8,
        /// [`InteractionResponse::Modal`].
        Modal = 9,
        /// [`InteractionResponse::PremiumRequired`] (deprecated).
        PremiumRequired = 10,
        /// [`InteractionResponse::LaunchActivity`].
        LaunchActivity = 12,
    }
}

/// An [interaction response](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-response-object):
/// how the app answers an [`Interaction`](super::Interaction), within 3 seconds, by
/// `POST /interactions/{id}/{token}/callback` or as the body of the HTTP endpoint's reply.
///
/// Written as `{"type": …, "data": …}`; the variant decides the type.
///
/// discord.js counterpart: what `reply`, `deferReply`, `update`, `deferUpdate`, `respond`,
/// `showModal` and `launchActivity` send.
///
/// # Examples
///
/// ```
/// use djs_model::{
///     interaction::{InteractionResponse, MessageCallbackData},
///     message::MessageFlags,
/// };
///
/// let mut reply = MessageCallbackData::from("Only you can see this.");
/// reply.flags = Some(MessageFlags::EPHEMERAL);
/// let response = InteractionResponse::ChannelMessageWithSource(Box::new(reply));
/// assert_eq!(
///     serde_json::to_string(&response).unwrap(),
///     r#"{"type":4,"data":{"content":"Only you can see this.","flags":64}}"#
/// );
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum InteractionResponse {
    /// Acknowledges a [`Ping`](super::InteractionData::Ping).
    Pong,
    /// Responds with a message.
    ///
    /// discord.js counterpart: `reply`.
    ChannelMessageWithSource(Box<MessageCallbackData>),
    /// Acknowledges the interaction and shows "thinking…"; the response message is sent later
    /// by editing the original response.
    ///
    /// discord.js counterpart: `deferReply`.
    DeferredChannelMessageWithSource(DeferredCallbackData),
    /// Acknowledges a component or modal-submit interaction without a loading state; the
    /// message is edited later.
    ///
    /// discord.js counterpart: `deferUpdate`.
    DeferredUpdateMessage,
    /// Edits the message the component is on (component and modal-submit interactions).
    ///
    /// discord.js counterpart: `update`.
    UpdateMessage(Box<MessageCallbackData>),
    /// Suggests choices to an [`Autocomplete`](super::InteractionData::Autocomplete)
    /// interaction.
    ///
    /// discord.js counterpart: `AutocompleteInteraction#respond`.
    AutocompleteResult(AutocompleteCallbackData),
    /// Opens a modal (not available for modal-submit and ping interactions).
    ///
    /// discord.js counterpart: `showModal`.
    Modal(Box<ModalCallbackData>),
    /// **Deprecated**: responds with an upgrade button (monetized apps); use a premium
    /// [`Button`](crate::component::Button::Premium) instead.
    PremiumRequired,
    /// Launches the app's Activity (apps with Activities).
    ///
    /// discord.js counterpart: `launchActivity`.
    LaunchActivity,
    /// A response type this version does not know (only produced when decoding); its data is
    /// not kept.
    Unknown(InteractionCallbackType),
}

impl InteractionResponse {
    /// Returns the files to upload with the response (those of its message data); empty for
    /// responses without a message.
    #[must_use]
    pub fn files(&self) -> &[FileUpload] {
        match self {
            Self::ChannelMessageWithSource(data) | Self::UpdateMessage(data) => &data.files,
            _ => &[],
        }
    }

    /// Returns the response type.
    #[must_use]
    pub const fn kind(&self) -> InteractionCallbackType {
        match self {
            Self::Pong => InteractionCallbackType::Pong,
            Self::ChannelMessageWithSource(_) => InteractionCallbackType::ChannelMessageWithSource,
            Self::DeferredChannelMessageWithSource(_) => {
                InteractionCallbackType::DeferredChannelMessageWithSource
            }
            Self::DeferredUpdateMessage => InteractionCallbackType::DeferredUpdateMessage,
            Self::UpdateMessage(_) => InteractionCallbackType::UpdateMessage,
            Self::AutocompleteResult(_) => {
                InteractionCallbackType::ApplicationCommandAutocompleteResult
            }
            Self::Modal(_) => InteractionCallbackType::Modal,
            Self::PremiumRequired => InteractionCallbackType::PremiumRequired,
            Self::LaunchActivity => InteractionCallbackType::LaunchActivity,
            Self::Unknown(kind) => *kind,
        }
    }
}

impl Serialize for InteractionResponse {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(untagged)]
        enum Data<'a> {
            Message(&'a MessageCallbackData),
            Deferred(&'a DeferredCallbackData),
            Autocomplete(&'a AutocompleteCallbackData),
            Modal(&'a ModalCallbackData),
        }

        #[derive(Serialize)]
        struct Flat<'a> {
            #[serde(rename = "type")]
            kind: InteractionCallbackType,
            #[serde(skip_serializing_if = "Option::is_none")]
            data: Option<Data<'a>>,
        }

        let data = match self {
            Self::ChannelMessageWithSource(data) | Self::UpdateMessage(data) => {
                Some(Data::Message(data))
            }
            Self::DeferredChannelMessageWithSource(data) if !data.flags.is_empty() => {
                Some(Data::Deferred(data))
            }
            Self::AutocompleteResult(data) => Some(Data::Autocomplete(data)),
            Self::Modal(data) => Some(Data::Modal(data)),
            _ => None,
        };
        Flat {
            kind: self.kind(),
            data,
        }
        .serialize(serializer)
    }
}

/// Every field of every kind of callback data, decoded in one pass.
#[derive(Deserialize)]
struct RawCallbackData {
    #[serde(default)]
    tts: bool,
    #[serde(default)]
    content: Option<CompactString>,
    #[serde(default)]
    embeds: Option<Vec<Embed>>,
    #[serde(default)]
    allowed_mentions: Option<AllowedMentions>,
    #[serde(default)]
    flags: Option<MessageFlags>,
    #[serde(default)]
    components: Option<Vec<Component>>,
    #[serde(default)]
    attachments: Option<Vec<AttachmentRequest>>,
    #[serde(default)]
    poll: Option<CreatePoll>,
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    choices: Vec<Choice<RawValue>>,
    #[serde(default)]
    custom_id: Option<CompactString>,
    #[serde(default)]
    title: Option<CompactString>,
}

impl RawCallbackData {
    fn into_message(self) -> MessageCallbackData {
        MessageCallbackData {
            tts: self.tts,
            content: self.content,
            embeds: self.embeds,
            allowed_mentions: self.allowed_mentions,
            flags: self.flags,
            components: self.components,
            attachments: self.attachments,
            poll: self.poll,
            files: Vec::new(),
        }
    }
}

#[derive(Deserialize)]
struct RawResponse {
    #[serde(rename = "type")]
    kind: InteractionCallbackType,
    #[serde(default)]
    data: Option<RawCallbackData>,
}

impl<'de> Deserialize<'de> for InteractionResponse {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawResponse::deserialize(deserializer)?;
        let data = raw.data;
        let missing = || D::Error::missing_field("data");
        Ok(match raw.kind {
            InteractionCallbackType::Pong => Self::Pong,
            InteractionCallbackType::ChannelMessageWithSource => {
                Self::ChannelMessageWithSource(Box::new(data.ok_or_else(missing)?.into_message()))
            }
            InteractionCallbackType::DeferredChannelMessageWithSource => {
                Self::DeferredChannelMessageWithSource(DeferredCallbackData {
                    flags: data.and_then(|data| data.flags).unwrap_or_default(),
                })
            }
            InteractionCallbackType::DeferredUpdateMessage => Self::DeferredUpdateMessage,
            InteractionCallbackType::UpdateMessage => Self::UpdateMessage(Box::new(
                data.map(RawCallbackData::into_message).unwrap_or_default(),
            )),
            InteractionCallbackType::ApplicationCommandAutocompleteResult => {
                let data = data.ok_or_else(missing)?;
                Self::AutocompleteResult(AutocompleteCallbackData {
                    choices: AutocompleteChoices::from_raw(data.choices)?,
                })
            }
            InteractionCallbackType::Modal => {
                let data = data.ok_or_else(missing)?;
                Self::Modal(Box::new(ModalCallbackData {
                    custom_id: data
                        .custom_id
                        .ok_or_else(|| D::Error::missing_field("custom_id"))?,
                    title: data.title.ok_or_else(|| D::Error::missing_field("title"))?,
                    components: data.components.unwrap_or_default(),
                }))
            }
            InteractionCallbackType::PremiumRequired => Self::PremiumRequired,
            InteractionCallbackType::LaunchActivity => Self::LaunchActivity,
            kind @ InteractionCallbackType::Unknown(_) => Self::Unknown(kind),
        })
    }
}

/// The [message data of a response](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-response-object-messages):
/// a new message ([`ChannelMessageWithSource`](InteractionResponse::ChannelMessageWithSource))
/// or the edit of the component's message ([`UpdateMessage`](InteractionResponse::UpdateMessage)).
///
/// Every field is optional. For updates, an absent field keeps the message's current value,
/// while `Some(vec![])` removes all embeds, components or attachments.
///
/// discord.js counterpart: `InteractionReplyOptions` / `InteractionUpdateOptions`.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct MessageCallbackData {
    /// Whether the message is read aloud.
    pub tts: bool,
    /// The text (up to 2000 characters).
    pub content: Option<CompactString>,
    /// The embeds (up to 10).
    pub embeds: Option<Vec<Embed>>,
    /// Which mentions in the content notify.
    pub allowed_mentions: Option<AllowedMentions>,
    /// Only [`EPHEMERAL`](MessageFlags::EPHEMERAL), [`SUPPRESS_EMBEDS`](MessageFlags::SUPPRESS_EMBEDS),
    /// [`SUPPRESS_NOTIFICATIONS`](MessageFlags::SUPPRESS_NOTIFICATIONS),
    /// [`IS_COMPONENTS_V2`](MessageFlags::IS_COMPONENTS_V2) and
    /// [`IS_VOICE_MESSAGE`](MessageFlags::IS_VOICE_MESSAGE) can be set.
    pub flags: Option<MessageFlags>,
    /// The components.
    pub components: Option<Vec<Component>>,
    /// Explicit attachment entries: for updates, the existing attachments to keep. Every file
    /// in [`files`](Self::files) gets an entry automatically, unless one with its `files[n]`
    /// index is listed here (D-094).
    pub attachments: Option<Vec<AttachmentRequest>>,
    /// A poll to create with the message.
    pub poll: Option<CreatePoll>,
    /// Files to upload with the response (sent as `multipart/form-data`, D-087). Not part of
    /// the JSON; empty when decoded.
    pub files: Vec<FileUpload>,
}

impl Serialize for MessageCallbackData {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            #[serde(skip_serializing_if = "core::ops::Not::not")]
            tts: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            content: Option<&'a CompactString>,
            #[serde(skip_serializing_if = "Option::is_none")]
            embeds: Option<&'a [Embed]>,
            #[serde(skip_serializing_if = "Option::is_none")]
            allowed_mentions: Option<&'a AllowedMentions>,
            #[serde(skip_serializing_if = "Option::is_none")]
            flags: Option<MessageFlags>,
            #[serde(skip_serializing_if = "Option::is_none")]
            components: Option<&'a [Component]>,
            #[serde(skip_serializing_if = "Attachments::is_absent")]
            attachments: Attachments<'a>,
            #[serde(skip_serializing_if = "Option::is_none")]
            poll: Option<&'a CreatePoll>,
        }

        // Destructured without `..`, so a new field cannot be forgotten here.
        let Self {
            tts,
            content,
            embeds,
            allowed_mentions,
            flags,
            components,
            attachments,
            poll,
            files,
        } = self;
        Wire {
            tts: *tts,
            content: content.as_ref(),
            embeds: embeds.as_deref(),
            allowed_mentions: allowed_mentions.as_ref(),
            flags: *flags,
            components: components.as_deref(),
            attachments: Attachments::new(attachments.as_deref(), files),
            poll: poll.as_ref(),
        }
        .serialize(serializer)
    }
}

impl From<&str> for MessageCallbackData {
    fn from(content: &str) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::default()
        }
    }
}

impl From<String> for MessageCallbackData {
    fn from(content: String) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::default()
        }
    }
}

impl From<CompactString> for MessageCallbackData {
    fn from(content: CompactString) -> Self {
        Self {
            content: Some(content),
            ..Self::default()
        }
    }
}

/// The data of a [deferred message response](InteractionResponse::DeferredChannelMessageWithSource):
/// only the flags of the message that follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct DeferredCallbackData {
    /// Only [`EPHEMERAL`](MessageFlags::EPHEMERAL) is valid here (the later message is then
    /// ephemeral).
    pub flags: MessageFlags,
}

impl DeferredCallbackData {
    /// Creates the data with the given flags.
    #[must_use]
    pub const fn new(flags: MessageFlags) -> Self {
        Self { flags }
    }
}

/// The [data of an autocomplete response](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-response-object-autocomplete):
/// the suggested choices.
///
/// # Examples
///
/// ```
/// use djs_model::{
///     application_command::Choice,
///     interaction::{AutocompleteCallbackData, AutocompleteChoices, InteractionResponse},
/// };
///
/// let response = InteractionResponse::AutocompleteResult(AutocompleteCallbackData::new(
///     AutocompleteChoices::Integer(vec![Choice::new("Ten", 10), Choice::new("Twenty", 20)]),
/// ));
/// assert_eq!(
///     serde_json::to_string(&response).unwrap(),
///     r#"{"type":8,"data":{"choices":[{"name":"Ten","value":10},{"name":"Twenty","value":20}]}}"#
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct AutocompleteCallbackData {
    /// The choices (up to 25; empty for "no suggestions").
    pub choices: AutocompleteChoices,
}

impl AutocompleteCallbackData {
    /// Creates the data.
    #[must_use]
    pub const fn new(choices: AutocompleteChoices) -> Self {
        Self { choices }
    }
}

/// Autocomplete suggestions, typed like the option being completed.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum AutocompleteChoices {
    /// Choices of a string option.
    String(Vec<Choice<CompactString>>),
    /// Choices of an integer option.
    Integer(Vec<Choice<i64>>),
    /// Choices of a number option.
    Number(Vec<Choice<f64>>),
}

impl AutocompleteChoices {
    /// Types decoded choices: strings stay strings, integers stay integers, and a list with
    /// any fractional number is a number list (an empty list is a string list).
    fn from_raw<E: DeError>(choices: Vec<Choice<RawValue>>) -> Result<Self, E> {
        Ok(match choices.first().map(|choice| &choice.value) {
            None | Some(RawValue::String(_)) => {
                Self::String(convert_choices(choices, RawValue::into_string)?)
            }
            Some(_)
                if choices
                    .iter()
                    .any(|choice| matches!(choice.value, RawValue::Float(_))) =>
            {
                Self::Number(convert_choices(choices, RawValue::into_number)?)
            }
            Some(_) => Self::Integer(convert_choices(choices, RawValue::into_integer)?),
        })
    }
}

/// The [data of a modal response](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-response-object-modal):
/// the popup's ID, title and components.
///
/// discord.js counterpart: `ModalBuilder` (what `showModal` sends).
///
/// # Examples
///
/// ```
/// use djs_model::{
///     component::{Label, TextInput, TextInputStyle},
///     interaction::{InteractionResponse, ModalCallbackData},
/// };
///
/// let input = TextInput::new("feedback", TextInputStyle::Paragraph);
/// let modal = ModalCallbackData::new(
///     "feedback_modal",
///     "Feedback",
///     vec![Label::new("Your feedback", input.into()).into()],
/// );
/// let json = serde_json::to_value(InteractionResponse::Modal(Box::new(modal))).unwrap();
/// assert_eq!(json["type"], 9);
/// assert_eq!(json["data"]["components"][0]["component"]["custom_id"], "feedback");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ModalCallbackData {
    /// The modal's custom ID (1-100 characters), sent back in
    /// [`ModalSubmitData::custom_id`](super::ModalSubmitData::custom_id).
    pub custom_id: CompactString,
    /// The title (max 45 characters).
    pub title: CompactString,
    /// The components (1-5): labels, text displays and (deprecated) action rows with text
    /// inputs.
    pub components: Vec<Component>,
}

impl ModalCallbackData {
    /// Creates the data.
    #[must_use]
    pub fn new(
        custom_id: impl Into<CompactString>,
        title: impl Into<CompactString>,
        components: Vec<Component>,
    ) -> Self {
        Self {
            custom_id: custom_id.into(),
            title: title.into(),
            components,
        }
    }
}

/// The [callback response](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-callback-interaction-callback-response-object)
/// Discord returns for a response sent with `with_response=true`.
///
/// discord.js counterpart: `InteractionCallbackResponse`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InteractionCallbackResponse {
    /// The interaction that was responded to.
    pub interaction: InteractionCallback,
    /// What the response created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<CallbackResource>,
}

/// The [interaction](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-callback-interaction-callback-object)
/// a callback response belongs to.
///
/// discord.js counterpart: `InteractionCallback`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InteractionCallback {
    /// The interaction's ID.
    pub id: InteractionId,
    /// The interaction's type.
    #[serde(rename = "type")]
    pub kind: InteractionType,
    /// The Activity instance that was launched or joined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity_instance_id: Option<CompactString>,
    /// The message the response created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_message_id: Option<MessageId>,
    /// Whether the message is in a loading state (deferred responses).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_message_loading: Option<bool>,
    /// Whether the message is ephemeral.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_message_ephemeral: Option<bool>,
}

/// The [resource](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-callback-interaction-callback-resource-object)
/// a response created.
///
/// discord.js counterpart: `InteractionCallbackResource`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CallbackResource {
    /// The response type.
    #[serde(rename = "type")]
    pub kind: InteractionCallbackType,
    /// The launched Activity ([`LaunchActivity`](InteractionCallbackType::LaunchActivity)).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity_instance: Option<ActivityInstanceResource>,
    /// The created or updated message
    /// ([`ChannelMessageWithSource`](InteractionCallbackType::ChannelMessageWithSource),
    /// [`UpdateMessage`](InteractionCallbackType::UpdateMessage)).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<Box<Message>>,
}

/// The [Activity instance](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-callback-interaction-callback-activity-instance-resource)
/// a [`LaunchActivity`](InteractionResponse::LaunchActivity) response launched.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ActivityInstanceResource {
    /// The instance ID.
    pub id: CompactString,
}
