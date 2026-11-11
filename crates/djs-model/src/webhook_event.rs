//! [Webhook events](https://docs.discord.com/developers/events/webhook-events): events Discord
//! sends over HTTP to an app's webhook events URL (D-076).
//!
//! Each request body is a [`WebhookEventPayload`]: either a `PING` (sent when the URL is
//! configured) or an [event](WebhookEvent) whose [data](WebhookEventData) depends on its type.
//! Unlike [gateway events](crate::gateway), webhook events are not realtime and may arrive out
//! of order. Every request must be verified with its `X-Signature-Ed25519` and
//! `X-Signature-Timestamp` headers (answer `401` when the signature is invalid) and acknowledged
//! with `204 No Content` within 3 seconds, otherwise Discord retries it.
//!
//! The events are chosen in the app's settings or with the application's
//! [`event_webhooks_types`](crate::application::Application::event_webhooks_types).
//!
//! # Examples
//!
//! ```
//! use djs_model::webhook_event::{PayloadType, WebhookEventData, WebhookEventPayload};
//!
//! let body = r#"{
//!     "version": 1, "application_id": "1234560123453231555", "type": 1,
//!     "event": {
//!         "type": "APPLICATION_DEAUTHORIZED", "timestamp": "2024-10-18T14:42:53.064834",
//!         "data": { "user": { "id": "111178765189277770", "username": "nelly",
//!                             "discriminator": "0", "global_name": null, "avatar": null } }
//!     }
//! }"#;
//! let payload: WebhookEventPayload = serde_json::from_str(body)?;
//! assert_eq!(payload.kind, PayloadType::Event);
//! let event = payload.event.expect("events carry a body");
//! let WebhookEventData::ApplicationDeauthorized(data) = &event.data else {
//!     panic!("expected APPLICATION_DEAUTHORIZED");
//! };
//! assert_eq!(data.user.username, "nelly");
//! # Ok::<(), serde_json::Error>(())
//! ```

use core::fmt;

use compact_str::CompactString;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as DeError, Visitor},
};

use crate::{
    application::{IntegrationType, PartialApplication},
    channel::Channel,
    component::Component,
    entitlement::Entitlement,
    guild::Guild,
    id::{ApplicationId, ChannelId, GenericId, LobbyId, MessageId},
    lobby::LobbyMessage,
    macros::{int_enum, str_enum},
    message::{Attachment, Embed, MessageActivity, MessageFlags, MessageType},
    oauth2::Scope,
    user::User,
    util::{JsonValue, JsonValueError, Timestamp},
};

/// The [outer payload](https://docs.discord.com/developers/events/webhook-events#payload-structure)
/// of a webhook event request.
///
/// Answer a [`PayloadType::Ping`] with `204 No Content` (and an empty body) to validate the URL.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WebhookEventPayload {
    /// The payload's version scheme (currently always `1`).
    pub version: u8,
    /// Your app's ID.
    pub application_id: ApplicationId,
    /// Whether this is a `PING` or an event.
    #[serde(rename = "type")]
    pub kind: PayloadType,
    /// The event (for [`PayloadType::Event`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<WebhookEvent>,
}

int_enum! {
    /// The [type of a webhook event payload](https://docs.discord.com/developers/events/webhook-events#webhook-types).
    pub enum PayloadType: u8 {
        /// Sent to check that the URL is active; answer with `204 No Content`.
        Ping = 0,
        /// An event, in [`WebhookEventPayload::event`].
        Event = 1,
    }
}

str_enum! {
    /// A [webhook event type](https://docs.discord.com/developers/events/webhook-events#event-types)
    /// an app can subscribe to.
    ///
    /// discord.js counterpart: `ApplicationWebhookEventType`.
    pub enum WebhookEventType {
        /// The app was authorized by a user or in a guild.
        ApplicationAuthorized = "APPLICATION_AUTHORIZED",
        /// The app was deauthorized by a user.
        ApplicationDeauthorized = "APPLICATION_DEAUTHORIZED",
        /// An entitlement was created.
        EntitlementCreate = "ENTITLEMENT_CREATE",
        /// An entitlement was updated.
        EntitlementUpdate = "ENTITLEMENT_UPDATE",
        /// An entitlement was deleted.
        EntitlementDelete = "ENTITLEMENT_DELETE",
        /// A user was added to a Quest.
        QuestUserEnrollment = "QUEST_USER_ENROLLMENT",
        /// A message was created in a lobby.
        LobbyMessageCreate = "LOBBY_MESSAGE_CREATE",
        /// A message was updated in a lobby.
        LobbyMessageUpdate = "LOBBY_MESSAGE_UPDATE",
        /// A message was deleted from a lobby.
        LobbyMessageDelete = "LOBBY_MESSAGE_DELETE",
        /// A direct message was created during a game session.
        GameDirectMessageCreate = "GAME_DIRECT_MESSAGE_CREATE",
        /// A direct message was updated during a game session.
        GameDirectMessageUpdate = "GAME_DIRECT_MESSAGE_UPDATE",
        /// A direct message was deleted during a game session.
        GameDirectMessageDelete = "GAME_DIRECT_MESSAGE_DELETE",
    }
}

/// The [event body](https://docs.discord.com/developers/events/webhook-events#event-body-object)
/// of a webhook event: when it happened and its data.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct WebhookEvent {
    /// When the event occurred. Discord sends it without an offset; it is read as UTC.
    pub timestamp: Timestamp,
    /// The event's data, by [type](WebhookEventData::kind).
    pub data: WebhookEventData,
}

impl WebhookEvent {
    /// Returns the event's type.
    #[must_use]
    pub fn kind(&self) -> WebhookEventType {
        self.data.kind()
    }
}

/// The data of a [`WebhookEvent`], by [event type](https://docs.discord.com/developers/events/webhook-events#event-types).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WebhookEventData {
    /// [`APPLICATION_AUTHORIZED`](https://docs.discord.com/developers/events/webhook-events#application-authorized):
    /// the app was added to a guild or a user account.
    ApplicationAuthorized(Box<ApplicationAuthorized>),
    /// [`APPLICATION_DEAUTHORIZED`](https://docs.discord.com/developers/events/webhook-events#application-deauthorized):
    /// a user deauthorized the app. For Social SDK apps this is the only signal outside the game
    /// that the user's account link was removed.
    ApplicationDeauthorized(Box<ApplicationDeauthorized>),
    /// [`ENTITLEMENT_CREATE`](https://docs.discord.com/developers/events/webhook-events#entitlement-create):
    /// a user purchased or was granted one of the app's SKUs.
    EntitlementCreate(Box<Entitlement>),
    /// [`ENTITLEMENT_UPDATE`](https://docs.discord.com/developers/events/webhook-events#entitlement-update).
    EntitlementUpdate(Box<Entitlement>),
    /// [`ENTITLEMENT_DELETE`](https://docs.discord.com/developers/events/webhook-events#entitlement-delete).
    EntitlementDelete(Box<Entitlement>),
    /// [`QUEST_USER_ENROLLMENT`](https://docs.discord.com/developers/events/webhook-events#quest-user-enrollment):
    /// a user was added to a Quest. Apps cannot receive it yet; it has no documented data.
    QuestUserEnrollment,
    /// [`LOBBY_MESSAGE_CREATE`](https://docs.discord.com/developers/events/webhook-events#lobby-message-create).
    LobbyMessageCreate(Box<LobbyMessage>),
    /// [`LOBBY_MESSAGE_UPDATE`](https://docs.discord.com/developers/events/webhook-events#lobby-message-update).
    LobbyMessageUpdate(Box<LobbyMessage>),
    /// [`LOBBY_MESSAGE_DELETE`](https://docs.discord.com/developers/events/webhook-events#lobby-message-delete).
    LobbyMessageDelete(LobbyMessageDelete),
    /// [`GAME_DIRECT_MESSAGE_CREATE`](https://docs.discord.com/developers/events/webhook-events#game-direct-message-create):
    /// a direct message was sent while a participant had an active Social SDK session.
    GameDirectMessageCreate(Box<GameDirectMessage>),
    /// [`GAME_DIRECT_MESSAGE_UPDATE`](https://docs.discord.com/developers/events/webhook-events#game-direct-message-update).
    GameDirectMessageUpdate(Box<GameDirectMessage>),
    /// [`GAME_DIRECT_MESSAGE_DELETE`](https://docs.discord.com/developers/events/webhook-events#game-direct-message-delete).
    GameDirectMessageDelete(Box<GameDirectMessage>),
    /// An event type this version does not know; its data is not kept.
    Unknown(WebhookEventType),
}

impl WebhookEventData {
    /// Returns the event's type.
    #[must_use]
    pub fn kind(&self) -> WebhookEventType {
        match self {
            Self::ApplicationAuthorized(_) => WebhookEventType::ApplicationAuthorized,
            Self::ApplicationDeauthorized(_) => WebhookEventType::ApplicationDeauthorized,
            Self::EntitlementCreate(_) => WebhookEventType::EntitlementCreate,
            Self::EntitlementUpdate(_) => WebhookEventType::EntitlementUpdate,
            Self::EntitlementDelete(_) => WebhookEventType::EntitlementDelete,
            Self::QuestUserEnrollment => WebhookEventType::QuestUserEnrollment,
            Self::LobbyMessageCreate(_) => WebhookEventType::LobbyMessageCreate,
            Self::LobbyMessageUpdate(_) => WebhookEventType::LobbyMessageUpdate,
            Self::LobbyMessageDelete(_) => WebhookEventType::LobbyMessageDelete,
            Self::GameDirectMessageCreate(_) => WebhookEventType::GameDirectMessageCreate,
            Self::GameDirectMessageUpdate(_) => WebhookEventType::GameDirectMessageUpdate,
            Self::GameDirectMessageDelete(_) => WebhookEventType::GameDirectMessageDelete,
            Self::Unknown(kind) => kind.clone(),
        }
    }
}

/// The data of an [Application Authorized](https://docs.discord.com/developers/events/webhook-events#application-authorized)
/// webhook event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ApplicationAuthorized {
    /// Where the app was installed: [`IntegrationType::GuildInstall`] (a guild) or
    /// [`IntegrationType::UserInstall`] (the user's account).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration_type: Option<IntegrationType>,
    /// The user who authorized the app.
    pub user: User,
    /// The `OAuth2` [scopes](Scope) the user authorized.
    #[serde(default)]
    pub scopes: Vec<Scope>,
    /// The guild the app was authorized in (guild installs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild: Option<Box<Guild>>,
}

/// The data of an [Application Deauthorized](https://docs.discord.com/developers/events/webhook-events#application-deauthorized)
/// webhook event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ApplicationDeauthorized {
    /// The user who deauthorized the app.
    pub user: User,
}

/// The data of a [Lobby Message Delete](https://docs.discord.com/developers/events/webhook-events#lobby-message-delete)
/// webhook event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LobbyMessageDelete {
    /// The deleted message.
    pub id: MessageId,
    /// The lobby it was deleted from.
    pub lobby_id: LobbyId,
}

/// A direct message in a `GAME_DIRECT_MESSAGE_*` webhook event (D-079).
///
/// Covers both kinds of messages the docs describe: a regular
/// [message](https://docs.discord.com/developers/events/webhook-events#message-object) (with
/// `channel` and, in linked channels, `lobby_id`) and an
/// [SDK DM message](https://docs.discord.com/developers/events/webhook-events#sdk-dm-message-object)
/// between provisional accounts, which exists only in the game. Only `id`, `content` and
/// `author` are required; message fields not listed here are ignored.
///
/// Contains floats (attachment durations), so it is `PartialEq` but not `Eq` (D-064).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GameDirectMessage {
    /// The message's ID.
    pub id: MessageId,
    /// The message's type.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<MessageType>,
    /// The message's text.
    pub content: CompactString,
    /// The DM channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,
    /// The DM channel, with its recipients.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<Box<Channel>>,
    /// The lobby the message was sent in (linked channel messages).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lobby_id: Option<LobbyId>,
    /// The author.
    pub author: User,
    /// The recipient, as shown in the docs' update example (not documented further).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient_id: Option<GenericId>,
    /// When the message was sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<Timestamp>,
    /// When the message was last edited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_timestamp: Option<Timestamp>,
    /// The message's flags.
    #[serde(default)]
    pub flags: MessageFlags,
    /// The application that created the message (SDK DM messages).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_id: Option<ApplicationId>,
    /// The attached files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,
    /// The embeds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<Embed>,
    /// The message components.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    /// Sent with Rich Presence-related chat embeds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<MessageActivity>,
    /// Sent with Rich Presence-related chat embeds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application: Option<Box<PartialApplication>>,
}

/// Reads an event body timestamp, which has no offset (read as UTC).
fn naive_timestamp<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Timestamp, D::Error> {
    struct NaiveVisitor;

    impl Visitor<'_> for NaiveVisitor {
        type Value = Timestamp;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("an ISO 8601 timestamp")
        }

        fn visit_str<E: DeError>(self, value: &str) -> Result<Timestamp, E> {
            Timestamp::parse_assume_utc(value).map_err(E::custom)
        }
    }

    deserializer.deserialize_str(NaiveVisitor)
}

#[derive(Deserialize)]
struct RawEvent {
    #[serde(rename = "type")]
    kind: WebhookEventType,
    #[serde(deserialize_with = "naive_timestamp")]
    timestamp: Timestamp,
    // Buffered: `data` may come before `type`. Webhook events are rare, so the second pass is
    // cheap.
    #[serde(default)]
    data: Option<JsonValue>,
}

impl<'de> Deserialize<'de> for WebhookEvent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawEvent::deserialize(deserializer)?;
        let convert = |error: JsonValueError| D::Error::custom(format_args!("data: {error}"));
        let data = || {
            raw.data
                .as_ref()
                .ok_or_else(|| D::Error::missing_field("data"))
        };
        let data = match &raw.kind {
            WebhookEventType::QuestUserEnrollment => WebhookEventData::QuestUserEnrollment,
            WebhookEventType::Unknown(_) => WebhookEventData::Unknown(raw.kind.clone()),
            WebhookEventType::ApplicationAuthorized => {
                WebhookEventData::ApplicationAuthorized(data()?.to_type().map_err(convert)?)
            }
            WebhookEventType::ApplicationDeauthorized => {
                WebhookEventData::ApplicationDeauthorized(data()?.to_type().map_err(convert)?)
            }
            WebhookEventType::EntitlementCreate => {
                WebhookEventData::EntitlementCreate(data()?.to_type().map_err(convert)?)
            }
            WebhookEventType::EntitlementUpdate => {
                WebhookEventData::EntitlementUpdate(data()?.to_type().map_err(convert)?)
            }
            WebhookEventType::EntitlementDelete => {
                WebhookEventData::EntitlementDelete(data()?.to_type().map_err(convert)?)
            }
            WebhookEventType::LobbyMessageCreate => {
                WebhookEventData::LobbyMessageCreate(data()?.to_type().map_err(convert)?)
            }
            WebhookEventType::LobbyMessageUpdate => {
                WebhookEventData::LobbyMessageUpdate(data()?.to_type().map_err(convert)?)
            }
            WebhookEventType::LobbyMessageDelete => {
                WebhookEventData::LobbyMessageDelete(data()?.to_type().map_err(convert)?)
            }
            WebhookEventType::GameDirectMessageCreate => {
                WebhookEventData::GameDirectMessageCreate(data()?.to_type().map_err(convert)?)
            }
            WebhookEventType::GameDirectMessageUpdate => {
                WebhookEventData::GameDirectMessageUpdate(data()?.to_type().map_err(convert)?)
            }
            WebhookEventType::GameDirectMessageDelete => {
                WebhookEventData::GameDirectMessageDelete(data()?.to_type().map_err(convert)?)
            }
        };
        Ok(Self {
            timestamp: raw.timestamp,
            data,
        })
    }
}

impl Serialize for WebhookEvent {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(untagged)]
        enum DataRef<'a> {
            Authorized(&'a ApplicationAuthorized),
            Deauthorized(&'a ApplicationDeauthorized),
            Entitlement(&'a Entitlement),
            LobbyMessage(&'a LobbyMessage),
            LobbyMessageDelete(&'a LobbyMessageDelete),
            GameDirectMessage(&'a GameDirectMessage),
        }

        #[derive(Serialize)]
        struct Flat<'a> {
            #[serde(rename = "type")]
            kind: WebhookEventType,
            timestamp: Timestamp,
            #[serde(skip_serializing_if = "Option::is_none")]
            data: Option<DataRef<'a>>,
        }

        let data = match &self.data {
            WebhookEventData::ApplicationAuthorized(data) => Some(DataRef::Authorized(data)),
            WebhookEventData::ApplicationDeauthorized(data) => Some(DataRef::Deauthorized(data)),
            WebhookEventData::EntitlementCreate(data)
            | WebhookEventData::EntitlementUpdate(data)
            | WebhookEventData::EntitlementDelete(data) => Some(DataRef::Entitlement(data)),
            WebhookEventData::LobbyMessageCreate(data)
            | WebhookEventData::LobbyMessageUpdate(data) => Some(DataRef::LobbyMessage(data)),
            WebhookEventData::LobbyMessageDelete(data) => Some(DataRef::LobbyMessageDelete(data)),
            WebhookEventData::GameDirectMessageCreate(data)
            | WebhookEventData::GameDirectMessageUpdate(data)
            | WebhookEventData::GameDirectMessageDelete(data) => {
                Some(DataRef::GameDirectMessage(data))
            }
            WebhookEventData::QuestUserEnrollment | WebhookEventData::Unknown(_) => None,
        };
        Flat {
            kind: self.kind(),
            timestamp: self.timestamp,
            data,
        }
        .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: &str, data: &str) -> Result<WebhookEvent, serde_json::Error> {
        serde_json::from_str(&format!(
            r#"{{"data":{data},"type":"{kind}","timestamp":"2024-10-18T14:42:53.064834"}}"#
        ))
    }

    #[test]
    fn data_before_type_and_timestamp_as_utc() {
        let event = event("LOBBY_MESSAGE_DELETE", r#"{"id":"1","lobby_id":"2"}"#).unwrap();
        assert_eq!(event.kind(), WebhookEventType::LobbyMessageDelete);
        assert_eq!(
            event.timestamp,
            "2024-10-18T14:42:53.064834+00:00".parse().unwrap()
        );
        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"type":"LOBBY_MESSAGE_DELETE","timestamp":"2024-10-18T14:42:53.064834+00:00","data":{"id":"1","lobby_id":"2"}}"#
        );
    }

    #[test]
    fn unknown_and_dataless_events() {
        let unknown = event("FUTURE_EVENT", r#"{"anything":[1,2]}"#).unwrap();
        assert_eq!(
            unknown.data,
            WebhookEventData::Unknown(WebhookEventType::Unknown("FUTURE_EVENT".into()))
        );
        let quest = event("QUEST_USER_ENROLLMENT", "null").unwrap();
        assert_eq!(quest.data, WebhookEventData::QuestUserEnrollment);
        assert!(!serde_json::to_string(&quest).unwrap().contains("data"));
    }

    #[test]
    fn missing_or_invalid_data_is_an_error() {
        let missing = event("APPLICATION_DEAUTHORIZED", "null").unwrap_err();
        assert!(
            missing.to_string().contains("missing field `data`"),
            "{missing}"
        );
        let invalid = event("LOBBY_MESSAGE_DELETE", r#"{"id":"1"}"#).unwrap_err();
        assert!(invalid.to_string().starts_with("data: "), "{invalid}");
    }
}
