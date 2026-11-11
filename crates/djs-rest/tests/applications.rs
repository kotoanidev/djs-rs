//! The application endpoints against the mock API: method, path and body of every request.

use djs_model::{
    application::{EventWebhookStatus, IntegrationTypeConfig, IntegrationTypesConfig},
    id::{ApplicationId, UserId},
    rest::EditCurrentApplication,
    util::Patch,
    webhook_event::WebhookEventType,
};

mod common;

use common::{Canned, mock, seen};

const APPLICATION: &str =
    include_str!("../../djs-model/tests/fixtures/docs/application_application.json");
const INSTANCE: &str =
    include_str!("../../djs-model/tests/fixtures/docs/application_activity_instance.json");
const INSTANCE_ID: &str = "i-1276580072400224306-gc-912952092627435520-912954213460484116";

#[tokio::test]
async fn application_endpoints() {
    let (rest, state) = mock(vec![
        Canned::json(200, APPLICATION),
        Canned::json(200, APPLICATION),
        Canned::json(200, INSTANCE),
    ])
    .await;

    let application = rest.get_current_application().await.unwrap();
    assert_eq!(application.id, ApplicationId::new(172_150_183_260_323_840));

    let mut body = EditCurrentApplication::default();
    body.tags = Some(vec!["rust".into()]);
    body.icon = Patch::Clear;
    let mut contexts = IntegrationTypesConfig::default();
    contexts.user_install = Some(IntegrationTypeConfig::default());
    body.integration_types_config = Some(contexts);
    body.event_webhooks_status = Some(EventWebhookStatus::Enabled);
    body.event_webhooks_types = Some(vec![WebhookEventType::ApplicationAuthorized]);
    rest.edit_current_application(&body).await.unwrap();

    let app = ApplicationId::new(1_215_413_995_645_968_394);
    let instance = rest
        .get_application_activity_instance(app, INSTANCE_ID)
        .await
        .unwrap();
    assert_eq!(instance.users, [UserId::new(205_519_959_982_473_217)]);

    let seen = seen(&state);
    assert_eq!(seen.len(), 3);
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path, "/api/v10/applications/@me");
    assert_eq!(seen[1].method, "PATCH");
    assert_eq!(seen[1].path, "/api/v10/applications/@me");
    assert_eq!(
        seen[1].body,
        r#"{"integration_types_config":{"1":{}},"icon":null,"tags":["rust"],"event_webhooks_status":2,"event_webhooks_types":["APPLICATION_AUTHORIZED"]}"#
    );
    assert_eq!(seen[2].method, "GET");
    assert_eq!(
        seen[2].path,
        format!("/api/v10/applications/{app}/activity-instances/{INSTANCE_ID}")
    );
}
