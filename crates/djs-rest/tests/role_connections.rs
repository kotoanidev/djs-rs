//! The application role connection metadata endpoints against the mock API.

use djs_model::{
    application_role_connection_metadata::{ApplicationRoleConnectionMetadata, MetadataType},
    id::ApplicationId,
    util::Locale,
};

mod common;

use common::{Canned, mock, seen};

const RECORDS: &str = r#"[{"type": 7, "key": "verified", "name": "Verified",
    "name_localizations": {"pl": "Zweryfikowany"}, "description": "Has a verified account",
    "description_localizations": null}]"#;

#[tokio::test]
async fn role_connection_metadata_endpoints() {
    let (rest, state) = mock(vec![Canned::json(200, RECORDS), Canned::json(200, RECORDS)]).await;
    let app = ApplicationId::new(5);

    let records = rest
        .get_application_role_connection_metadata_records(app)
        .await
        .unwrap();
    assert_eq!(records[0].kind, MetadataType::BooleanEqual);
    assert_eq!(
        records[0].name_localizations[&Locale::Polish],
        "Zweryfikowany"
    );

    let mut record = ApplicationRoleConnectionMetadata::new(
        MetadataType::BooleanEqual,
        "verified",
        "Verified",
        "Has a verified account",
    );
    record
        .name_localizations
        .insert(Locale::Polish, "Zweryfikowany".into());
    let updated = rest
        .update_application_role_connection_metadata_records(app, &[record])
        .await
        .unwrap();
    assert_eq!(updated, records);

    let seen = seen(&state);
    assert_eq!(seen.len(), 2);
    let path = format!("/api/v10/applications/{app}/role-connections/metadata");
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path, path);
    assert_eq!(seen[1].method, "PUT");
    assert_eq!(seen[1].path, path);
    assert_eq!(
        seen[1].body,
        r#"[{"type":7,"key":"verified","name":"Verified","name_localizations":{"pl":"Zweryfikowany"},"description":"Has a verified account"}]"#
    );
}
