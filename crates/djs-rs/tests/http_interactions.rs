//! HTTP interactions end to end (D-398–D-403): signed requests through
//! `InteractionsEndpoint::handle`, the routes and handlers of a `Client` that never logs in,
//! and a mock REST API for what goes through REST.

use std::{
    convert::Infallible,
    fmt::Write as _,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use bytes::Bytes;
use djs_rs::{
    Client, Events,
    ext::InteractionExt,
    http_interactions::InteractionsEndpoint,
    model::{
        id::ApplicationId, interaction::MessageCallbackData, message::MessageFlags,
        rest::FileUpload,
    },
    rest::Rest,
};
use ed25519_dalek::{Signer, SigningKey};
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use tokio::{net::TcpListener, sync::Notify};

/// The requests the mock API received: method and path.
type Log = Arc<Mutex<Vec<(String, String)>>>;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

fn key() -> SigningKey {
    SigningKey::from_bytes(&[42; 32])
}

/// The signature and timestamp headers Discord would send with `body`.
fn sign(body: &[u8]) -> (String, String) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    let mut message = timestamp.as_bytes().to_vec();
    message.extend_from_slice(body);
    (hex(&key().sign(&message).to_bytes()), timestamp)
}

fn command(name: &str) -> Vec<u8> {
    json!({
        "id": "500", "application_id": "1", "type": 2, "token": "the-token", "version": 1,
        "guild_id": "100", "channel_id": "200", "app_permissions": "0",
        "member": {"user": {"id": "7", "username": "user", "discriminator": "0",
                            "global_name": null, "avatar": null},
                   "roles": [], "joined_at": "2026-01-01T00:00:00.000000+00:00",
                   "deaf": false, "mute": false, "flags": 0, "pending": false},
        "data": {"id": "91", "name": name, "type": 1}
    })
    .to_string()
    .into_bytes()
}

/// A mock API answering every request with an empty message, logging it.
async fn mock_rest() -> (Rest, Log) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api = format!("http://{}/api", listener.local_addr().unwrap());
    let log: Log = Arc::default();
    let state = Arc::clone(&log);
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let state = Arc::clone(&state);
            tokio::spawn(async move {
                let service = service_fn(move |request: Request<Incoming>| {
                    let state = Arc::clone(&state);
                    async move {
                        let path = request
                            .uri()
                            .path()
                            .trim_start_matches("/api/v10")
                            .to_owned();
                        state
                            .lock()
                            .unwrap()
                            .push((request.method().to_string(), path.clone()));
                        let _ = request.into_body().collect().await;
                        let message = json!({
                            "id": "900", "channel_id": "200", "content": "",
                            "author": {"id": "1", "username": "bot", "discriminator": "0",
                                       "global_name": null, "avatar": null},
                            "timestamp": "2026-11-17T00:00:00.000000+00:00",
                            "edited_timestamp": null, "tts": false, "mention_everyone": false,
                            "pinned": false, "type": 0, "mentions": [], "mention_roles": [],
                            "attachments": [], "embeds": []
                        });
                        // Create Interaction Response answers 204 without `with_response`.
                        let (status, body) = if path.ends_with("/callback") {
                            (204, Bytes::new())
                        } else {
                            (200, Bytes::from(message.to_string()))
                        };
                        let response = Response::builder()
                            .status(status)
                            .header("content-type", "application/json")
                            .body(Full::new(body))
                            .unwrap();
                        Ok::<_, Infallible>(response)
                    }
                });
                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await;
            });
        }
    });
    (Rest::builder().token("test-token").api(api).build(), log)
}

async fn setup() -> (Client, InteractionsEndpoint, Log) {
    let (rest, log) = mock_rest().await;
    let client = Client::builder("test-token").rest(rest).build();
    let endpoint = client
        .interactions_endpoint(&hex(&key().verifying_key().to_bytes()))
        .unwrap();
    (client, endpoint, log)
}

async fn post(endpoint: &InteractionsEndpoint, body: &[u8]) -> (u16, Value) {
    let (signature, timestamp) = sign(body);
    let (response, background) = endpoint.handle(&signature, &timestamp, body).await;
    background.await;
    let body = if response.body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&response.body).unwrap()
    };
    (response.status, body)
}

#[tokio::test]
async fn a_ping_gets_a_pong_and_bad_requests_are_refused() {
    let (_client, endpoint, log) = setup().await;
    let ping = br#"{"id":"1","application_id":"1","type":1,"token":"t","version":1}"#;
    assert_eq!(post(&endpoint, ping).await, (200, json!({"type": 1})));

    // A wrong signature, a missing one, a stale timestamp.
    let (signature, timestamp) = sign(ping);
    let (response, _) = endpoint.handle(&signature, &timestamp, b"{}").await;
    assert_eq!(response.status, 401);
    let (response, _) = endpoint.handle("", "", ping).await;
    assert_eq!(response.status, 401);
    let mut message = b"1000000000".to_vec();
    message.extend_from_slice(ping);
    let old = hex(&key().sign(&message).to_bytes());
    let (response, _) = endpoint.handle(&old, "1000000000", ping).await;
    assert_eq!(response.status, 401);

    // Signed, but not an interaction.
    assert_eq!(
        post(&endpoint, br#"{"type":"nope"}"#).await,
        (400, Value::Null)
    );
    assert!(log.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_reply_is_the_http_body_and_the_handlers_all_run() {
    let (client, endpoint, log) = setup().await;
    let created = Arc::new(AtomicBool::new(false));
    let seen = Arc::clone(&created);
    client.on(Events::InteractionCreate, move |ctx, interaction| {
        let seen = Arc::clone(&seen);
        async move {
            assert_eq!(interaction.id.get(), 500);
            assert_eq!(ctx.application_id(), Some(ApplicationId::new(1)));
            seen.store(true, Ordering::Relaxed);
            Ok(())
        }
    });
    client.command("ping", |ctx, cmd| async move {
        cmd.reply(&ctx, "Pong!").await?;
        Ok(())
    });
    let (status, body) = post(&endpoint, &command("ping")).await;
    assert_eq!(status, 200);
    assert_eq!(body["type"], 4);
    assert_eq!(body["data"]["content"], "Pong!");
    assert!(created.load(Ordering::Relaxed));
    assert!(log.lock().unwrap().is_empty(), "no REST request");
}

#[tokio::test]
async fn a_deferral_returns_before_the_follow_up() {
    let (client, endpoint, log) = setup().await;
    let go = Arc::new(Notify::new());
    let wait = Arc::clone(&go);
    client.command("slow", move |ctx, cmd| {
        let wait = Arc::clone(&wait);
        async move {
            cmd.defer_reply(&ctx, MessageFlags::EPHEMERAL).await?;
            wait.notified().await;
            cmd.edit_reply(&ctx, "Done").await?;
            Ok(())
        }
    });
    let body = command("slow");
    let (signature, timestamp) = sign(&body);
    // The handler is still waiting when the response is known.
    let (response, background) = endpoint.handle(&signature, &timestamp, &body).await;
    assert_eq!(response.status, 200);
    let json: Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(json, json!({"type": 5, "data": {"flags": 64}}));
    assert!(log.lock().unwrap().is_empty());
    go.notify_one();
    background.await;
    assert_eq!(
        *log.lock().unwrap(),
        [(
            "PATCH".to_owned(),
            "/webhooks/1/the-token/messages/@original".to_owned()
        )]
    );
}

#[tokio::test]
async fn files_go_through_rest_and_no_response_is_an_error() {
    let (client, endpoint, log) = setup().await;
    client.command("file", |ctx, cmd| async move {
        let mut reply = MessageCallbackData::from("Here");
        reply.files.push(FileUpload::new("a.txt", "hello"));
        cmd.reply(&ctx, reply).await?;
        Ok(())
    });
    client.command("silent", |_, _| async { Ok(()) });
    assert_eq!(post(&endpoint, &command("file")).await, (202, Value::Null));
    assert_eq!(
        *log.lock().unwrap(),
        [(
            "POST".to_owned(),
            "/interactions/500/the-token/callback".to_owned()
        )]
    );
    assert_eq!(
        post(&endpoint, &command("silent")).await,
        (500, Value::Null)
    );
    assert_eq!(
        post(&endpoint, &command("unrouted")).await,
        (500, Value::Null)
    );
}
