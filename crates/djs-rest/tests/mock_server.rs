//! Tests `Rest` against a local mock of the Discord API.

use std::time::{Duration, Instant};

use djs_model::{
    id::{ApplicationId, ChannelId, InteractionId, MessageId},
    interaction::InteractionResponse,
    message::MessageReference,
    rest::{CreateMessage, FileUpload},
};
use djs_rest::{ErrorKind, Rest, Route};
use http::StatusCode;

mod common;

use common::{Canned, mock, mock_with};

const MESSAGE: &str = r#"{
    "id": "1300000000000000001", "channel_id": "1200000000000000000",
    "author": {"id": "1", "username": "bot", "discriminator": "0", "global_name": null,
               "avatar": null, "bot": true},
    "content": "Pong!", "timestamp": "2026-11-10T19:42:07.123000+00:00",
    "edited_timestamp": null, "tts": false, "mention_everyone": false, "pinned": false,
    "type": 19,
    "message_reference": {"type": 0, "message_id": "1300000000000000000",
                          "channel_id": "1200000000000000000"}
}"#;

const CHANNEL: ChannelId = ChannelId::new(1_200_000_000_000_000_000);

#[tokio::test]
async fn create_message_sends_expected_request() {
    let (rest, state) = mock(vec![Canned::json(200, MESSAGE)]).await;

    let mut body = CreateMessage::from("Pong!");
    body.message_reference = Some(MessageReference::reply(MessageId::new(
        1_300_000_000_000_000_000,
    )));
    let message = rest.create_message(CHANNEL, &body).await.unwrap();
    assert_eq!(message.content, "Pong!");
    assert_eq!(
        message.message_reference.unwrap().message_id,
        Some(MessageId::new(1_300_000_000_000_000_000))
    );

    let seen = state.lock().unwrap().seen.clone();
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.path,
        "/api/v10/channels/1200000000000000000/messages"
    );
    assert_eq!(request.authorization.as_deref(), Some("Bot test-token"));
    assert_eq!(request.content_type.as_deref(), Some("application/json"));
    assert!(
        request
            .user_agent
            .starts_with("DiscordBot (https://github.com/kotoanidev/djs-rs, ")
    );
    assert_eq!(
        request.body,
        r#"{"content":"Pong!","message_reference":{"type":0,"message_id":"1300000000000000000"}}"#
    );
}

#[tokio::test]
async fn get_gateway_bot_has_no_body() {
    let (rest, state) = mock(vec![Canned::json(
        200,
        r#"{"url":"wss://gateway.discord.gg","shards":1,
            "session_start_limit":{"total":1000,"remaining":999,"reset_after":0,"max_concurrency":1}}"#,
    )])
    .await;
    let gateway = rest.get_gateway_bot().await.unwrap();
    assert_eq!(gateway.url, "wss://gateway.discord.gg");
    let seen = state.lock().unwrap().seen.clone();
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path, "/api/v10/gateway/bot");
    assert_eq!(seen[0].content_type, None);
    assert_eq!(seen[0].body, "");
}

#[tokio::test]
async fn api_errors_are_parsed() {
    let (rest, _) = mock(vec![Canned::json(
        403,
        r#"{"message":"Missing Permissions","code":50013}"#,
    )])
    .await;
    let error = rest
        .create_message(CHANNEL, &"hi".into())
        .await
        .unwrap_err();
    assert_eq!(error.status(), Some(StatusCode::FORBIDDEN));
    let api = error.api_error().unwrap();
    assert_eq!(api.code, djs_model::rest::ErrorCode::MissingPermissions);
    assert_eq!(api.message, "Missing Permissions");
}

#[tokio::test]
async fn rate_limited_request_is_retried_after_retry_after() {
    let (rest, state) = mock(vec![
        Canned::json(
            429,
            r#"{"message":"You are being rate limited.","retry_after":0.2,"global":false}"#,
        )
        .header("x-ratelimit-scope", "user")
        .header("x-ratelimit-remaining", "0")
        .header("x-ratelimit-reset-after", "0.2")
        .header("x-ratelimit-bucket", "abcd1234"),
        Canned::json(200, MESSAGE),
    ])
    .await;
    rest.create_message(CHANNEL, &"hi".into()).await.unwrap();

    let seen = state.lock().unwrap().seen.clone();
    assert_eq!(seen.len(), 2);
    let waited = seen[1].at - seen[0].at;
    assert!(
        waited >= Duration::from_millis(200),
        "waited only {waited:?}"
    );
}

#[tokio::test]
async fn global_rate_limit_pauses_and_retries() {
    let (rest, state) = mock(vec![
        Canned::json(
            429,
            r#"{"message":"global","retry_after":0.15,"global":true}"#,
        )
        .header("x-ratelimit-global", "true")
        .header("x-ratelimit-scope", "global"),
        Canned::json(200, MESSAGE),
    ])
    .await;
    rest.create_message(CHANNEL, &"hi".into()).await.unwrap();
    let seen = state.lock().unwrap().seen.clone();
    assert!(seen[1].at - seen[0].at >= Duration::from_millis(150));
}

#[tokio::test]
async fn exhausted_bucket_delays_next_request() {
    let (rest, state) = mock(vec![
        Canned::json(200, MESSAGE)
            .header("x-ratelimit-bucket", "abcd1234")
            .header("x-ratelimit-limit", "5")
            .header("x-ratelimit-remaining", "0")
            .header("x-ratelimit-reset-after", "0.25"),
        Canned::json(200, MESSAGE),
    ])
    .await;
    rest.create_message(CHANNEL, &"one".into()).await.unwrap();
    rest.create_message(CHANNEL, &"two".into()).await.unwrap();
    let seen = state.lock().unwrap().seen.clone();
    let waited = seen[1].at - seen[0].at;
    assert!(
        waited >= Duration::from_millis(250),
        "waited only {waited:?}"
    );
}

#[tokio::test]
async fn other_channels_are_not_delayed_by_an_exhausted_bucket() {
    let (rest, state) = mock(vec![
        Canned::json(200, MESSAGE)
            .header("x-ratelimit-bucket", "abcd1234")
            .header("x-ratelimit-remaining", "0")
            .header("x-ratelimit-reset-after", "5"),
        Canned::json(200, MESSAGE),
    ])
    .await;
    rest.create_message(CHANNEL, &"one".into()).await.unwrap();
    let other = ChannelId::new(1_200_000_000_000_000_001);
    rest.create_message(other, &"two".into()).await.unwrap();
    let seen = state.lock().unwrap().seen.clone();
    assert!(seen[1].at - seen[0].at < Duration::from_secs(1));
}

#[tokio::test]
async fn server_errors_are_retried() {
    let (rest, state) = mock(vec![
        Canned::json(502, "<html>Bad Gateway</html>"),
        Canned::json(500, "{}"),
        Canned::json(200, MESSAGE),
    ])
    .await;
    rest.create_message(CHANNEL, &"hi".into()).await.unwrap();
    assert_eq!(state.lock().unwrap().seen.len(), 3);
}

#[tokio::test]
async fn retries_are_bounded() {
    let (rest, state) = mock((0..4).map(|_| Canned::json(503, "unavailable")).collect()).await;
    let error = rest
        .create_message(CHANNEL, &"hi".into())
        .await
        .unwrap_err();
    assert_eq!(error.status(), Some(StatusCode::SERVICE_UNAVAILABLE));
    assert!(error.api_error().is_none());
    // One attempt plus three retries.
    assert_eq!(state.lock().unwrap().seen.len(), 4);
}

#[tokio::test]
async fn invalid_token_fails_without_sending() {
    let (_, state) = mock(Vec::new()).await;
    let rest = Rest::builder()
        .token("bad\ntoken")
        .api("http://127.0.0.1:9/api")
        .build();
    let error = rest.get_gateway_bot().await.unwrap_err();
    assert!(matches!(error.kind(), ErrorKind::BuildRequest));
    assert!(state.lock().unwrap().seen.is_empty());
    assert!(!format!("{rest:?}").contains("token\""));
}

#[tokio::test]
async fn debug_redacts_token() {
    let rest = Rest::new("super-secret");
    let debug = format!("{rest:?}");
    assert!(!debug.contains("super-secret"), "{debug}");
    assert!(debug.contains("<redacted>"));
    // The builder does not show it either.
    for builder in [
        Rest::builder().token("super-secret"),
        Rest::builder().bearer_token("super-secret"),
    ] {
        let debug = format!("{builder:?}");
        assert!(!debug.contains("super-secret"), "{debug}");
        assert!(debug.contains("<redacted>"), "{debug}");
    }
}

#[tokio::test]
async fn bearer_tokens_use_the_bearer_scheme() {
    let (rest, state) = mock_with(vec![Canned::json(200, "[]")], |builder| {
        builder.bearer_token("user-access-token")
    })
    .await;
    rest.request::<Vec<serde_json::Value>>(djs_rest::Request::new(Route::GetChannelMessages {
        channel_id: CHANNEL,
    }))
    .await
    .unwrap();
    assert_eq!(
        state.lock().unwrap().seen[0].authorization.as_deref(),
        Some("Bearer user-access-token")
    );
}

#[tokio::test]
async fn invalid_requests_are_counted_and_can_stop_the_client() {
    let forbidden = || Canned::json(403, r#"{"message":"Missing Access","code":50001}"#);
    let shared_429 = Canned::json(
        429,
        r#"{"message":"You are being rate limited.","retry_after":0.01,"global":false}"#,
    )
    .header("x-ratelimit-scope", "shared");
    let (rest, state) = mock_with(
        vec![
            forbidden(),
            shared_429,
            Canned::json(200, MESSAGE),
            forbidden(),
        ],
        |builder| builder.invalid_request_limit(2),
    )
    .await;
    assert!(rest.create_message(CHANNEL, &"a".into()).await.is_err());
    assert_eq!(rest.invalid_requests(), 1);
    // A `shared` 429 is retried and not counted.
    rest.create_message(CHANNEL, &"b".into()).await.unwrap();
    assert_eq!(rest.invalid_requests(), 1);
    assert!(rest.create_message(CHANNEL, &"c".into()).await.is_err());
    assert_eq!(rest.invalid_requests(), 2);

    // The limit is reached: the next request fails without reaching the server.
    let error = rest.create_message(CHANNEL, &"d".into()).await.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::InvalidRequestLimit { count: 2, limit: 2 }
        ),
        "{error}"
    );
    assert_eq!(state.lock().unwrap().seen.len(), 4);
}

/// A response that exhausts its bucket for 5 seconds.
fn exhausting(status: u16, body: &str) -> Canned {
    Canned::json(status, body)
        .header("x-ratelimit-limit", "1")
        .header("x-ratelimit-remaining", "0")
        .header("x-ratelimit-reset-after", "5")
        .header("x-ratelimit-bucket", "interaction")
}

#[tokio::test]
async fn interaction_callbacks_are_neither_queued_nor_globally_limited() {
    let (rest, state) = mock_with(
        (0..6).map(|_| exhausting(204, "")).collect(),
        // A global limit of 1 request per second would serialize normal routes.
        |builder| builder.global_requests_per_second(1),
    )
    .await;
    let start = Instant::now();
    let mut callbacks = tokio::task::JoinSet::new();
    for i in 0..6_u64 {
        let rest = rest.clone();
        callbacks.spawn(async move {
            let token = format!("interaction-token-for-tests-{i}");
            let route = Route::CreateInteractionResponse {
                interaction_id: InteractionId::new(1_300_000_000_000_000_100 + i),
                interaction_token: &token,
            };
            let request = djs_rest::Request::new(route)
                .json(&InteractionResponse::DeferredUpdateMessage)
                .unwrap();
            rest.request::<()>(request).await
        });
    }
    for result in callbacks.join_all().await {
        result.unwrap();
    }
    // Neither the exhausted bucket nor the global limit delayed any callback.
    assert!(
        start.elapsed() < Duration::from_millis(500),
        "{:?}",
        start.elapsed()
    );
    let seen = state.lock().unwrap().seen.clone();
    assert_eq!(seen.len(), 6);
    assert!(seen.iter().all(|request| {
        request.method == "POST"
            && request
                .path
                .starts_with("/api/v10/interactions/1300000000000000")
            && request.path.ends_with("/callback")
            && request.body == r#"{"type":6}"#
    }));
}

#[tokio::test]
async fn interaction_webhooks_skip_the_global_limit() {
    let (rest, state) = mock_with(
        (0..3).map(|_| Canned::json(200, MESSAGE)).collect(),
        |builder| builder.global_requests_per_second(1),
    )
    .await;
    let start = Instant::now();
    for i in 0..3 {
        let token = format!("interaction-token-for-tests-{i}");
        let route = Route::CreateFollowupMessage {
            application_id: ApplicationId::new(1_300_000_000_000_000_200),
            interaction_token: &token,
        };
        let request = djs_rest::Request::new(route)
            .json(&CreateMessage::from("followup"))
            .unwrap();
        rest.request::<serde_json::Value>(request).await.unwrap();
    }
    assert!(
        start.elapsed() < Duration::from_millis(500),
        "{:?}",
        start.elapsed()
    );
    assert_eq!(state.lock().unwrap().seen.len(), 3);
}

#[tokio::test]
async fn rate_limited_interaction_callback_waits_alone() {
    let (rest, state) = mock(vec![
        Canned::json(
            429,
            r#"{"message":"You are being rate limited.","retry_after":0.15,"global":false}"#,
        )
        .header("x-ratelimit-scope", "user"),
        Canned::json(204, ""),
    ])
    .await;
    let route = Route::CreateInteractionResponse {
        interaction_id: InteractionId::new(1_300_000_000_000_000_100),
        interaction_token: "interaction-token-for-tests",
    };
    let request = djs_rest::Request::new(route)
        .json(&InteractionResponse::Pong)
        .unwrap();
    rest.request::<()>(request).await.unwrap();
    let seen = state.lock().unwrap().seen.clone();
    assert_eq!(seen.len(), 2);
    assert!(seen[1].at - seen[0].at >= Duration::from_millis(150));
}

#[tokio::test]
async fn audit_log_reason_is_sent_percent_encoded() {
    let (rest, state) = mock(vec![Canned::json(204, ""), Canned::json(204, "")]).await;
    let route = Route::PinMessage {
        channel_id: CHANNEL,
        message_id: MessageId::new(1_300_000_000_000_000_000),
    };
    let request = djs_rest::Request::new(route).reason(Some("Spam: «bad» links\r\nX-Evil: 1"));
    rest.request::<()>(request).await.unwrap();
    rest.request::<()>(djs_rest::Request::new(route))
        .await
        .unwrap();

    let seen = state.lock().unwrap().seen.clone();
    assert_eq!(seen[0].method, "PUT");
    assert_eq!(
        seen[0].path,
        "/api/v10/channels/1200000000000000000/messages/pins/1300000000000000000"
    );
    // Non-ASCII text and line breaks cannot reach the header unencoded.
    assert_eq!(
        seen[0].audit_log_reason.as_deref(),
        Some("Spam%3A%20%C2%ABbad%C2%BB%20links%0D%0AX-Evil%3A%201")
    );
    assert_eq!(seen[1].audit_log_reason, None);
}

#[tokio::test]
async fn files_are_sent_as_multipart_and_resent_on_retry() {
    let (rest, state) = mock(vec![
        Canned::json(500, r#"{"message":"oops","code":0}"#),
        Canned::json(200, MESSAGE),
    ])
    .await;
    let mut body = CreateMessage::from("Logs");
    let mut log = FileUpload::new("today.log", "all good\n");
    log.description = Some("Today".into());
    body.files.push(log);
    body.files.push(FileUpload::new("data.json", r#"{"a":1}"#));
    rest.create_message(CHANNEL, &body).await.unwrap();

    let seen = state.lock().unwrap().seen.clone();
    assert_eq!(seen.len(), 2);
    let content_type = seen[0].content_type.as_deref().unwrap();
    let boundary = content_type
        .strip_prefix("multipart/form-data; boundary=")
        .unwrap();
    let expected = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"files[0]\"; filename=\"today.log\"\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\r\n\
         all good\n\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"files[1]\"; filename=\"data.json\"\r\n\
         Content-Type: application/json\r\n\r\n\
         {{\"a\":1}}\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"payload_json\"\r\n\
         Content-Type: application/json\r\n\r\n\
         {{\"content\":\"Logs\",\"attachments\":[\
         {{\"id\":0,\"filename\":\"today.log\",\"description\":\"Today\"}},\
         {{\"id\":1,\"filename\":\"data.json\"}}]}}\r\n\
         --{boundary}--\r\n"
    );
    assert_eq!(seen[0].body, expected);
    assert_eq!(seen[0].content_length, Some(expected.len().to_string()));
    // The retry after the server error sends the same body again.
    assert_eq!(seen[1].body, expected);
    assert_eq!(seen[1].content_type.as_deref(), Some(content_type));
}
