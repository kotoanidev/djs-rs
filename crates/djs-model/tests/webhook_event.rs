//! Webhook events (D-076): every example of the docs' webhook events page, a `PING`, and the
//! fields specific to lobby and game direct messages (D-078, D-079).

use djs_model::{
    application::IntegrationType,
    oauth2::Scope,
    webhook_event::{PayloadType, WebhookEventData, WebhookEventPayload, WebhookEventType},
};
use serde_json::{Value, json};

mod common;

use common::assert_subset;

/// The docs' examples (with a synthetic user in place of `// user data`, C-026).
fn examples() -> Vec<(String, Value)> {
    let examples: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/docs/webhook_events_examples.json")).unwrap();
    examples
        .into_iter()
        .map(|mut example| {
            let title = example["title"].as_str().unwrap().to_owned();
            (title, example["json"].take())
        })
        .collect()
}

fn example(title: &str) -> Value {
    examples()
        .into_iter()
        .find(|(name, _)| name == title)
        .unwrap_or_else(|| panic!("no example {title}"))
        .1
}

/// Parses `json`, checks that serializing it again keeps every field, and returns it.
fn round_trip(title: &str, json: &Value) -> WebhookEventPayload {
    let payload: WebhookEventPayload =
        serde_json::from_value(json.clone()).unwrap_or_else(|error| panic!("{title}: {error}"));
    let mut expected = json.clone();
    if let Some(event) = expected.get_mut("event") {
        // Event times are sent without an offset and written back in UTC.
        let timestamp = event["timestamp"].as_str().unwrap();
        event["timestamp"] = Value::from(format!("{timestamp}+00:00"));
        if let Some(data) = event.get_mut("data").and_then(Value::as_object_mut) {
            // Shown in the entitlement examples but documented nowhere (see tests/application.rs).
            data.remove("gift_code_flags");
        }
    }
    let actual = serde_json::to_value(&payload).unwrap();
    assert_subset(&expected, &actual, title);
    payload
}

#[test]
fn every_docs_example_decodes_and_round_trips() {
    let examples = examples();
    assert_eq!(examples.len(), 11);
    for (title, json) in &examples {
        let payload = round_trip(title, json);
        assert_eq!(payload.version, 1);
        assert_eq!(payload.kind, PayloadType::Event);
        let event = payload.event.as_ref().unwrap();
        let kind = event.kind();
        assert!(!kind.is_unknown(), "{title}");
        assert_eq!(kind.as_str(), json["event"]["type"], "{title}");
        assert!(
            !matches!(event.data, WebhookEventData::Unknown(_)),
            "{title}"
        );
    }
}

#[test]
fn ping() {
    let json = json!({ "version": 1, "application_id": "1234560123453231555", "type": 0 });
    let payload: WebhookEventPayload = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(payload.kind, PayloadType::Ping);
    assert!(payload.event.is_none());
    assert_eq!(serde_json::to_value(&payload).unwrap(), json);
}

#[test]
fn application_authorized() {
    let payload = round_trip("user install", &example("Application Authorized Example"));
    let WebhookEventData::ApplicationAuthorized(data) = payload.event.unwrap().data else {
        panic!("expected APPLICATION_AUTHORIZED");
    };
    assert_eq!(data.integration_type, Some(IntegrationType::UserInstall));
    assert_eq!(data.scopes, [Scope::ApplicationsCommands]);
    assert!(data.guild.is_none());

    // A guild install carries the guild.
    let mut json = example("Application Authorized Example");
    let data = &mut json["event"]["data"];
    data["integration_type"] = json!(0);
    data["scopes"] = json!(["bot", "applications.commands"]);
    data["guild"] = serde_json::from_str(include_str!("fixtures/docs/guild.json")).unwrap();
    let payload = round_trip("guild install", &json);
    let WebhookEventData::ApplicationAuthorized(data) = payload.event.unwrap().data else {
        panic!("expected APPLICATION_AUTHORIZED");
    };
    assert_eq!(data.integration_type, Some(IntegrationType::GuildInstall));
    assert_eq!(data.scopes, [Scope::Bot, Scope::ApplicationsCommands]);
    assert_eq!(data.guild.unwrap().name, "Discord Testers");
}

#[test]
fn entitlement_events() {
    for (title, kind) in [
        (
            "Entitlement Create Example",
            WebhookEventType::EntitlementCreate,
        ),
        (
            "Entitlement Update Example",
            WebhookEventType::EntitlementUpdate,
        ),
        (
            "Entitlement Delete Example",
            WebhookEventType::EntitlementDelete,
        ),
    ] {
        let event = round_trip(title, &example(title)).event.unwrap();
        assert_eq!(event.kind(), kind);
        let (WebhookEventData::EntitlementCreate(entitlement)
        | WebhookEventData::EntitlementUpdate(entitlement)
        | WebhookEventData::EntitlementDelete(entitlement)) = event.data
        else {
            panic!("{title}: expected an entitlement");
        };
        assert_eq!(entitlement.id.get(), 1_234_505_980_407_808_808);
    }
}

#[test]
fn lobby_messages() {
    let event = round_trip("create", &example("Lobby Message Create Example"))
        .event
        .unwrap();
    let WebhookEventData::LobbyMessageCreate(message) = event.data else {
        panic!("expected LOBBY_MESSAGE_CREATE");
    };
    assert_eq!(message.content, "welcome to the party!");
    assert_eq!(
        message.application_id.map(djs_model::id::Id::get),
        Some(1_234_567_765_431_056_709)
    );
    assert_eq!(message.flags.bits(), 65_536);
    assert!(message.timestamp.is_none());

    // Updates add the send and edit times and may omit `application_id` (D-078).
    let event = round_trip("update", &example("Lobby Message Update Example"))
        .event
        .unwrap();
    let WebhookEventData::LobbyMessageUpdate(message) = event.data else {
        panic!("expected LOBBY_MESSAGE_UPDATE");
    };
    assert!(message.application_id.is_none());
    assert_eq!(
        message.timestamp,
        Some("2025-08-05T20:38:43.660000+00:00".parse().unwrap())
    );
    assert!(message.edited_timestamp > message.timestamp);

    let event = round_trip("delete", &example("Lobby Message Delete Example"))
        .event
        .unwrap();
    assert_eq!(
        event.timestamp,
        "2025-08-05T21:44:09.412957Z".parse().unwrap()
    );
    let WebhookEventData::LobbyMessageDelete(deleted) = event.data else {
        panic!("expected LOBBY_MESSAGE_DELETE");
    };
    assert_eq!(deleted.lobby_id.get(), 1_402_399_883_394_285_659);
}

#[test]
fn game_direct_messages() {
    let event = round_trip("create", &example("Game Direct Message Create Example"))
        .event
        .unwrap();
    let WebhookEventData::GameDirectMessageCreate(message) = event.data else {
        panic!("expected GAME_DIRECT_MESSAGE_CREATE");
    };
    assert_eq!(message.content, "get in friend, we're going raiding");
    assert!(message.application_id.is_some() && message.timestamp.is_some());
    assert_eq!(message.author.username, "mixo");

    // The update example has neither `type` nor `timestamp`, but a `recipient_id` (D-079).
    let event = round_trip("update", &example("Game Direct Message Update Example"))
        .event
        .unwrap();
    let WebhookEventData::GameDirectMessageUpdate(message) = event.data else {
        panic!("expected GAME_DIRECT_MESSAGE_UPDATE");
    };
    assert!(message.kind.is_none() && message.timestamp.is_none());
    assert_eq!(
        message.recipient_id.map(djs_model::id::Id::get),
        Some(1_404_960_877_324_533_784)
    );

    let event = round_trip("delete", &example("Game Direct Message Delete Example"))
        .event
        .unwrap();
    let WebhookEventData::GameDirectMessageDelete(message) = event.data else {
        panic!("expected GAME_DIRECT_MESSAGE_DELETE");
    };
    assert!(message.components.is_empty() && message.attachments.is_empty());

    // An SDK DM message between provisional accounts carries its channel.
    let mut json = example("Game Direct Message Create Example");
    json["event"]["data"]["channel"] = json!({
        "id": "1405604229820715098", "type": 1, "last_message_id": "1405614357781545021",
        "recipients": [{ "id": "1300000000000000009", "username": "provisional",
                         "discriminator": "0", "global_name": null, "avatar": null }]
    });
    let event = round_trip("SDK DM", &json).event.unwrap();
    let WebhookEventData::GameDirectMessageCreate(message) = event.data else {
        panic!("expected GAME_DIRECT_MESSAGE_CREATE");
    };
    assert_eq!(
        message.channel.unwrap().id().get(),
        1_405_604_229_820_715_098
    );
}
