//! The stage instance endpoints against the mock API: method, path, body and audit log reason
//! of every request.

use djs_model::{
    id::ChannelId,
    rest::{CreateStageInstance, ModifyStageInstance},
    stage_instance::PrivacyLevel,
};

mod common;

use common::{Canned, mock, seen};

const STAGE_INSTANCE: &str = r#"{"id": "840647391636226060", "guild_id": "197038439483310086",
    "channel_id": "733488538393510049", "topic": "Testing Testing, 123", "privacy_level": 1,
    "discoverable_disabled": false, "guild_scheduled_event_id": "947656305244532806"}"#;

const STAGE: ChannelId = ChannelId::new(733_488_538_393_510_049);

#[tokio::test]
async fn stage_instance_endpoints() {
    let instance = || Canned::json(200, STAGE_INSTANCE);
    let (rest, state) = mock(vec![
        instance(),
        instance(),
        instance(),
        Canned::json(204, ""),
    ])
    .await;

    let mut create = CreateStageInstance::new(STAGE, "Q&A");
    create.send_start_notification = Some(true);
    rest.create_stage_instance(&create, Some("weekly"))
        .await
        .unwrap();
    let fetched = rest.get_stage_instance(STAGE).await.unwrap();
    assert_eq!(fetched.channel_id, STAGE);
    let mut modify = ModifyStageInstance::default();
    modify.topic = Some("Open mic".into());
    modify.privacy_level = Some(PrivacyLevel::GuildOnly);
    rest.modify_stage_instance(STAGE, &modify, None)
        .await
        .unwrap();
    rest.delete_stage_instance(STAGE, Some("done"))
        .await
        .unwrap();

    let path = format!("/stage-instances/{STAGE}");
    let expected = [
        (
            "POST",
            "/stage-instances".to_owned(),
            r#"{"channel_id":"733488538393510049","topic":"Q&A","send_start_notification":true}"#,
            Some("weekly"),
        ),
        ("GET", path.clone(), "", None),
        (
            "PATCH",
            path.clone(),
            r#"{"topic":"Open mic","privacy_level":2}"#,
            None,
        ),
        ("DELETE", path, "", Some("done")),
    ];
    let seen = seen(&state);
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, body, reason)) in seen.iter().zip(&expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(request.path, format!("/api/v10{path}"), "{what}");
        assert_eq!(request.body, *body, "{what}");
        assert_eq!(request.audit_log_reason.as_deref(), *reason, "{what}");
    }
}
