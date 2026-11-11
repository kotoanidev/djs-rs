//! User connections, application role connections and their metadata records.
//!
//! The docs have no JSON examples for these objects; the payloads below follow the documented
//! structures field by field.

use djs_model::{
    application_role_connection_metadata::{ApplicationRoleConnectionMetadata, MetadataType},
    guild::IntegrationType,
    user::{ApplicationRoleConnection, Connection, ConnectionService, ConnectionVisibility},
    util::Locale,
};

mod common;

use common::round_trip;

const CONNECTION: &str = r#"{
    "id": "1053941",
    "name": "octocat",
    "type": "github",
    "revoked": false,
    "integrations": [
        {
            "id": "33590653072239123",
            "name": "octocat",
            "type": "twitch",
            "enabled": true,
            "account": { "id": "1053941", "name": "octocat" }
        }
    ],
    "verified": true,
    "friend_sync": false,
    "show_activity": true,
    "two_way_link": false,
    "visibility": 1
}"#;

#[test]
fn connection() {
    let connection: Connection = round_trip(CONNECTION, &[]);
    assert_eq!(connection.kind, ConnectionService::GitHub);
    assert_eq!(connection.visibility, ConnectionVisibility::Everyone);
    assert!(connection.verified && connection.show_activity);
    let integration = &connection.integrations[0];
    assert_eq!(integration.kind, Some(IntegrationType::Twitch));
    assert!(integration.enabled);
    assert_eq!(integration.account.as_ref().unwrap().name, "octocat");
}

#[test]
fn connection_services() {
    for (json, service) in [
        ("amazon-music", ConnectionService::AmazonMusic),
        ("bungie", ConnectionService::BungieNet),
        ("playstation", ConnectionService::PlayStationNetwork),
        ("twitter", ConnectionService::X),
        ("battlenet", ConnectionService::BattleNet),
    ] {
        assert_eq!(ConnectionService::from(json), service);
        assert_eq!(service.as_str(), json);
    }
    // Minimal connection (no `integrations`, no `revoked`) with a service unknown to us.
    let json = CONNECTION
        .replace(r#""type": "github""#, r#""type": "new-service""#)
        .replace(r#""revoked": false,"#, "");
    let connection: Connection = serde_json::from_str(&json).unwrap();
    assert!(connection.kind.is_unknown());
    assert_eq!(connection.kind.as_str(), "new-service");
    assert!(!connection.revoked);
}

#[test]
fn application_role_connection() {
    let connection: ApplicationRoleConnection = round_trip(
        r#"{
            "platform_name": "Example Game",
            "platform_username": "nelly",
            "metadata": { "level": "42", "verified": "1", "joined": "2024-05-01T12:00:00Z" }
        }"#,
        &[],
    );
    assert_eq!(connection.platform_name.as_deref(), Some("Example Game"));
    assert_eq!(connection.metadata.len(), 3);
    assert_eq!(connection.metadata["level"], "42");
    // Keys are written in sorted order (D-058).
    let json = serde_json::to_string(&connection.metadata).unwrap();
    assert_eq!(
        json,
        r#"{"joined":"2024-05-01T12:00:00Z","level":"42","verified":"1"}"#
    );

    let empty: ApplicationRoleConnection =
        serde_json::from_str(r#"{"platform_name":null,"platform_username":null,"metadata":{}}"#)
            .unwrap();
    assert_eq!(empty, ApplicationRoleConnection::default());
    assert_eq!(serde_json::to_string(&empty).unwrap(), "{}");
}

#[test]
fn application_role_connection_metadata() {
    let records: Vec<ApplicationRoleConnectionMetadata> = serde_json::from_str(
        r#"[
            {
                "type": 2,
                "key": "level",
                "name": "Level",
                "name_localizations": { "pl": "Poziom", "de": "Stufe", "xx-YY": "?" },
                "description": "Minimum level",
                "description_localizations": null
            },
            { "type": 7, "key": "verified", "name": "Verified", "description": "Is verified" }
        ]"#,
    )
    .unwrap();
    let level = &records[0];
    assert_eq!(level.kind, MetadataType::IntegerGreaterThanOrEqual);
    assert_eq!(level.name_localizations[&Locale::Polish], "Poziom");
    assert_eq!(level.name_localizations[&Locale::German], "Stufe");
    // Unknown locales are kept as keys too.
    assert_eq!(level.name_localizations[&Locale::from("xx-YY")], "?");
    assert!(level.description_localizations.is_empty());
    assert_eq!(records[1].kind, MetadataType::BooleanEqual);

    let json = serde_json::to_value(level).unwrap();
    assert_eq!(json["name_localizations"]["pl"], "Poziom");
    assert!(json.get("description_localizations").is_none());
    let again: ApplicationRoleConnectionMetadata = serde_json::from_value(json).unwrap();
    assert_eq!(&again, level);
}
