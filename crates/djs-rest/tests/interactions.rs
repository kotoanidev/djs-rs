//! The interaction endpoints against the mock API.

use djs_model::{
    id::{ApplicationId, InteractionId, MessageId},
    interaction::{InteractionResponse, MessageCallbackData},
    message::MessageFlags,
    rest::{CreateFollowupMessage, EditWebhookMessage, FileUpload},
    util::Patch,
};

mod common;

use common::{Canned, mock, seen};

const MESSAGE: &str = r#"{
    "id": "1300000000000000001", "channel_id": "1200000000000000000",
    "author": {"id": "1", "username": "bot", "discriminator": "0", "global_name": null,
               "avatar": null, "bot": true},
    "content": "Pong!", "timestamp": "2026-11-11T12:00:00.000000+00:00",
    "edited_timestamp": null, "tts": false, "mention_everyone": false, "pinned": false,
    "type": 20
}"#;

const APP: ApplicationId = ApplicationId::new(1_100_000_000_000_000_001);
const INTERACTION: InteractionId = InteractionId::new(1_100_000_000_000_000_002);
const TOKEN: &str = "interaction-token-for-tests";
const FOLLOWUP: MessageId = MessageId::new(1_300_000_000_000_000_001);

#[tokio::test]
async fn interaction_endpoints() {
    let no_content = || Canned::json(204, "");
    let message = || Canned::json(200, MESSAGE);
    let (rest, state) = mock(vec![
        no_content(),
        Canned::json(
            200,
            r#"{"interaction":{"id":"1100000000000000002","type":2,"response_message_id":"1300000000000000001"}}"#,
        ),
        message(),
        message(),
        no_content(),
        message(),
        message(),
        message(),
        no_content(),
    ])
    .await;

    let reply =
        InteractionResponse::ChannelMessageWithSource(Box::new(MessageCallbackData::from("Pong!")));
    rest.create_interaction_response(INTERACTION, TOKEN, &reply)
        .await
        .unwrap();
    let callback = rest
        .create_interaction_response_with_response(
            INTERACTION,
            TOKEN,
            &InteractionResponse::DeferredUpdateMessage,
        )
        .await
        .unwrap();
    assert_eq!(callback.interaction.response_message_id, Some(FOLLOWUP));

    rest.get_original_interaction_response(APP, TOKEN)
        .await
        .unwrap();
    let edit = EditWebhookMessage::from("edited");
    rest.edit_original_interaction_response(APP, TOKEN, &edit)
        .await
        .unwrap();
    rest.delete_original_interaction_response(APP, TOKEN)
        .await
        .unwrap();

    let mut followup = CreateFollowupMessage::from("more");
    followup.flags = Some(MessageFlags::EPHEMERAL);
    rest.create_followup_message(APP, TOKEN, &followup)
        .await
        .unwrap();
    rest.get_followup_message(APP, TOKEN, FOLLOWUP)
        .await
        .unwrap();
    let mut edit = EditWebhookMessage::default();
    edit.components = Patch::Clear;
    rest.edit_followup_message(APP, TOKEN, FOLLOWUP, &edit)
        .await
        .unwrap();
    rest.delete_followup_message(APP, TOKEN, FOLLOWUP)
        .await
        .unwrap();

    let expected = expectations();
    let seen = seen(&state);
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, query, body)) in seen.iter().zip(&expected) {
        assert_eq!(
            (
                &*request.method,
                &request.path,
                &*request.query,
                &*request.body
            ),
            (*method, path, *query, *body)
        );
    }
}

/// The requests of `interaction_endpoints`, in order: method, path, query, body.
fn expectations() -> [(&'static str, String, &'static str, &'static str); 9] {
    let callback_path = format!("/api/v10/interactions/{INTERACTION}/{TOKEN}/callback");
    let webhook = format!("/api/v10/webhooks/{APP}/{TOKEN}");
    [
        (
            "POST",
            callback_path.clone(),
            "",
            r#"{"type":4,"data":{"content":"Pong!"}}"#,
        ),
        ("POST", callback_path, "with_response=true", r#"{"type":6}"#),
        ("GET", format!("{webhook}/messages/@original"), "", ""),
        (
            "PATCH",
            format!("{webhook}/messages/@original"),
            "",
            r#"{"content":"edited"}"#,
        ),
        ("DELETE", format!("{webhook}/messages/@original"), "", ""),
        (
            "POST",
            webhook.clone(),
            "",
            r#"{"content":"more","flags":64}"#,
        ),
        ("GET", format!("{webhook}/messages/{FOLLOWUP}"), "", ""),
        (
            "PATCH",
            format!("{webhook}/messages/{FOLLOWUP}"),
            "",
            r#"{"components":null}"#,
        ),
        ("DELETE", format!("{webhook}/messages/{FOLLOWUP}"), "", ""),
    ]
}

#[tokio::test]
async fn interaction_response_files_are_uploaded() {
    let (rest, state) = mock(vec![Canned::json(204, "")]).await;
    let mut data = MessageCallbackData::from("chart");
    data.files.push(FileUpload::new("chart.png", &b"png"[..]));
    let response = InteractionResponse::ChannelMessageWithSource(Box::new(data));
    rest.create_interaction_response(INTERACTION, TOKEN, &response)
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
    assert!(request.body.contains("filename=\"chart.png\""));
    assert!(request.body.contains(
        r#"{"type":4,"data":{"content":"chart","attachments":[{"id":0,"filename":"chart.png"}]}}"#
    ));
}
