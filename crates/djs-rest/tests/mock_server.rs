//! Tests `Rest` against a local mock of the Discord API.

use std::{
    collections::VecDeque,
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use bytes::Bytes;
use djs_model::{
    id::{ApplicationId, ChannelId, InteractionId, MessageId},
    interaction::InteractionResponse,
    message::MessageReference,
    rest::CreateMessage,
};
use djs_rest::{ErrorKind, Rest, Route};
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

/// A request as seen by the mock server.
#[derive(Debug, Clone)]
struct Seen {
    method: String,
    path: String,
    authorization: Option<String>,
    content_type: Option<String>,
    user_agent: String,
    body: String,
    at: Instant,
}

/// A canned response.
struct Canned {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: String,
}

impl Canned {
    fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.to_owned(),
        }
    }

    fn header(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.headers.push((name, value.into()));
        self
    }
}

#[derive(Default)]
struct State {
    responses: VecDeque<Canned>,
    seen: Vec<Seen>,
}

/// Starts a mock server that answers with `responses` in order.
async fn mock(responses: Vec<Canned>) -> (Rest, Arc<Mutex<State>>) {
    mock_with(responses, |builder| builder).await
}

/// Like [`mock`], with extra client options.
async fn mock_with(
    responses: Vec<Canned>,
    configure: impl FnOnce(djs_rest::RestBuilder) -> djs_rest::RestBuilder,
) -> (Rest, Arc<Mutex<State>>) {
    let state = Arc::new(Mutex::new(State {
        responses: responses.into(),
        seen: Vec::new(),
    }));
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();

    let server_state = Arc::clone(&state);
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let state = Arc::clone(&server_state);
            tokio::spawn(async move {
                let service = service_fn(move |request: Request<Incoming>| {
                    let state = Arc::clone(&state);
                    async move { Ok::<_, Infallible>(handle(&state, request).await) }
                });
                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await;
            });
        }
    });

    let rest = configure(
        Rest::builder()
            .token("test-token")
            .api(format!("http://{address}/api"))
            .offset(Duration::ZERO),
    )
    .build();
    (rest, state)
}

async fn handle(state: &Mutex<State>, request: Request<Incoming>) -> Response<Full<Bytes>> {
    let header = |name: &str| {
        request
            .headers()
            .get(name)
            .map(|value| value.to_str().unwrap().to_owned())
    };
    let mut seen = Seen {
        method: request.method().to_string(),
        path: request.uri().path().to_owned(),
        authorization: header("authorization"),
        content_type: header("content-type"),
        user_agent: header("user-agent").unwrap_or_default(),
        body: String::new(),
        at: Instant::now(),
    };
    let body = request.into_body().collect().await.unwrap().to_bytes();
    seen.body = String::from_utf8(body.to_vec()).unwrap();

    let mut state = state.lock().unwrap();
    state.seen.push(seen);
    let canned = state
        .responses
        .pop_front()
        .expect("mock received more requests than expected");
    let mut response = Response::builder().status(canned.status);
    for (name, value) in canned.headers {
        response = response.header(name, value);
    }
    response.body(Full::new(Bytes::from(canned.body))).unwrap()
}

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

    let body = CreateMessage {
        content: Some("Pong!".into()),
        message_reference: Some(MessageReference::reply(MessageId::new(
            1_300_000_000_000_000_000,
        ))),
        ..CreateMessage::default()
    };
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
    assert_eq!(api.code, 50013);
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
