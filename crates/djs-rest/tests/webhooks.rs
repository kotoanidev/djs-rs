//! The webhook endpoints against the mock API: method, path, query, body, audit log reason
//! and whether the bot token was sent (D-121).

use djs_model::{
    id::{ChannelId, GuildId, MessageId, WebhookId},
    rest::{CreateWebhook, EditWebhookMessage, ExecuteWebhook, FileUpload, ModifyWebhook},
    util::Patch,
};

mod common;

use common::{Canned, mock, seen};

const WEBHOOK_JSON: &str =
    include_str!("../../djs-model/tests/fixtures/docs/webhook_incoming_webhook.json");

const MESSAGE: &str = r#"{
    "id": "1300000000000000001", "channel_id": "1200000000000000000",
    "author": {"id": "1", "username": "CI", "discriminator": "0000", "global_name": null,
               "avatar": null, "bot": true},
    "content": "Deploy finished", "timestamp": "2026-11-11T12:00:00.000000+00:00",
    "edited_timestamp": null, "tts": false, "mention_everyone": false, "pinned": false,
    "type": 0, "webhook_id": "1400000000000000000"
}"#;

const CHANNEL: ChannelId = ChannelId::new(1_200_000_000_000_000_000);
const WEBHOOK: WebhookId = WebhookId::new(1_400_000_000_000_000_000);
const TOKEN: &str = "webhook-token-for-tests";
const MSG: MessageId = MessageId::new(1_300_000_000_000_000_001);
const THREAD: ChannelId = ChannelId::new(1_300_000_000_000_000_009);

/// `(method, path after /api/v10, query, body, audit log reason, bot token sent)`.
type Expected = (
    &'static str,
    String,
    &'static str,
    &'static str,
    Option<&'static str>,
    bool,
);

#[tokio::test]
async fn webhook_management_endpoints() {
    let webhook = || Canned::json(200, WEBHOOK_JSON);
    let (rest, state) = mock(vec![
        webhook(),
        Canned::json(200, &format!("[{WEBHOOK_JSON}]")),
        Canned::json(200, &format!("[{WEBHOOK_JSON}]")),
        webhook(),
        webhook(),
        webhook(),
        webhook(),
        Canned::json(204, ""),
        Canned::json(204, ""),
    ])
    .await;

    rest.create_webhook(CHANNEL, &CreateWebhook::new("CI"), Some("deploys"))
        .await
        .unwrap();
    rest.get_channel_webhooks(CHANNEL).await.unwrap();
    rest.get_guild_webhooks(GuildId::new(1_100_000_000_000_000_000))
        .await
        .unwrap();
    let fetched = rest.get_webhook(WEBHOOK).await.unwrap();
    assert!(fetched.token.is_some());
    rest.get_webhook_with_token(WEBHOOK, TOKEN).await.unwrap();
    let mut modify = ModifyWebhook::default();
    modify.channel_id = Some(CHANNEL);
    rest.modify_webhook(WEBHOOK, &modify, Some("move"))
        .await
        .unwrap();
    let mut rename = ModifyWebhook::default();
    rename.name = Some("Deploys".into());
    rename.avatar = Patch::Clear;
    rest.modify_webhook_with_token(WEBHOOK, TOKEN, &rename)
        .await
        .unwrap();
    rest.delete_webhook(WEBHOOK, None).await.unwrap();
    rest.delete_webhook_with_token(WEBHOOK, TOKEN)
        .await
        .unwrap();

    let webhook_path = format!("/webhooks/{WEBHOOK}");
    let token_path = format!("/webhooks/{WEBHOOK}/{TOKEN}");
    let expected: Vec<Expected> = vec![
        (
            "POST",
            format!("/channels/{CHANNEL}/webhooks"),
            "",
            r#"{"name":"CI"}"#,
            Some("deploys"),
            true,
        ),
        (
            "GET",
            format!("/channels/{CHANNEL}/webhooks"),
            "",
            "",
            None,
            true,
        ),
        (
            "GET",
            "/guilds/1100000000000000000/webhooks".into(),
            "",
            "",
            None,
            true,
        ),
        ("GET", webhook_path.clone(), "", "", None, true),
        ("GET", token_path.clone(), "", "", None, false),
        (
            "PATCH",
            webhook_path.clone(),
            "",
            r#"{"channel_id":"1200000000000000000"}"#,
            Some("move"),
            true,
        ),
        (
            "PATCH",
            token_path.clone(),
            "",
            r#"{"name":"Deploys","avatar":null}"#,
            None,
            false,
        ),
        ("DELETE", webhook_path, "", "", None, true),
        ("DELETE", token_path, "", "", None, false),
    ];
    check(&seen(&state), &expected);
}

#[tokio::test]
async fn execute_webhook_and_its_messages() {
    let message = || Canned::json(200, MESSAGE);
    let (rest, state) = mock(vec![
        Canned::json(204, ""),
        message(),
        message(),
        message(),
        Canned::json(204, ""),
    ])
    .await;

    let mut body = ExecuteWebhook::from("Deploy finished");
    body.username = Some("CI".into());
    rest.execute_webhook(WEBHOOK, TOKEN, &body).await.unwrap();
    body.thread_id = Some(THREAD);
    body.with_components = Some(true);
    let sent = rest
        .execute_webhook_and_wait(WEBHOOK, TOKEN, &body)
        .await
        .unwrap();
    assert_eq!(sent.webhook_id, Some(WEBHOOK));

    rest.get_webhook_message(WEBHOOK, TOKEN, MSG, None)
        .await
        .unwrap();
    let mut edit = EditWebhookMessage::from("Deploy rolled back");
    edit.thread_id = Some(THREAD);
    rest.edit_webhook_message(WEBHOOK, TOKEN, MSG, &edit)
        .await
        .unwrap();
    rest.delete_webhook_message(WEBHOOK, TOKEN, MSG, Some(THREAD))
        .await
        .unwrap();

    let token_path = format!("/webhooks/{WEBHOOK}/{TOKEN}");
    let message_path = format!("{token_path}/messages/{MSG}");
    let execute_body = r#"{"content":"Deploy finished","username":"CI"}"#;
    let expected: Vec<Expected> = vec![
        ("POST", token_path.clone(), "", execute_body, None, false),
        (
            "POST",
            token_path,
            "wait=true&thread_id=1300000000000000009&with_components=true",
            execute_body,
            None,
            false,
        ),
        ("GET", message_path.clone(), "", "", None, false),
        (
            "PATCH",
            message_path.clone(),
            "thread_id=1300000000000000009",
            r#"{"content":"Deploy rolled back"}"#,
            None,
            false,
        ),
        (
            "DELETE",
            message_path,
            "thread_id=1300000000000000009",
            "",
            None,
            false,
        ),
    ];
    check(&seen(&state), &expected);
}

#[tokio::test]
async fn execute_webhook_uploads_files() {
    let (rest, state) = mock(vec![Canned::json(200, MESSAGE)]).await;
    let mut body = ExecuteWebhook::from("log");
    body.files.push(FileUpload::new("build.log", "ok\n"));
    rest.execute_webhook_and_wait(WEBHOOK, TOKEN, &body)
        .await
        .unwrap();
    let request = &seen(&state)[0];
    assert!(
        request
            .content_type
            .as_deref()
            .unwrap()
            .starts_with("multipart/form-data")
    );
    assert!(
        request
            .body
            .contains("name=\"files[0]\"; filename=\"build.log\"")
    );
    assert!(
        request
            .body
            .contains(r#"{"content":"log","attachments":[{"id":0,"filename":"build.log"}]}"#)
    );
}

fn check(seen: &[common::Seen], expected: &[Expected]) {
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, query, body, reason, authenticated)) in seen.iter().zip(expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(request.path, format!("/api/v10{path}"), "{what}");
        assert_eq!(request.query, *query, "{what}");
        assert_eq!(request.body, *body, "{what}");
        assert_eq!(request.audit_log_reason.as_deref(), *reason, "{what}");
        assert_eq!(request.authorization.is_some(), *authenticated, "{what}");
    }
}
