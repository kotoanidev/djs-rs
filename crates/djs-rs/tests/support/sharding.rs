//! Mock REST API and gateway shared by the multi-process sharding tests and benchmarks.

#![allow(dead_code, reason = "each test crate uses some of the helpers")]

use std::{
    convert::Infallible,
    time::{Duration, Instant},
};

use bytes::Bytes;
use djs_rs::rest::Rest;
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::{Request, Response, body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
    time::timeout,
};
use tokio_websockets::{Message, ServerBuilder};

/// How long a mock session waits for the client's IDENTIFY.
const STEP: Duration = Duration::from_secs(60);

/// A REST client for a mock API that answers every request with a Get Gateway Bot response.
pub(crate) async fn mock_rest(
    gateway_url: &str,
    shards: u32,
    remaining: u32,
    max_concurrency: u32,
) -> Rest {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/api", listener.local_addr().unwrap());
    let body = json!({"url": gateway_url, "shards": shards, "session_start_limit": {
        "total": 1000, "remaining": remaining, "reset_after": 3_600_000,
        "max_concurrency": max_concurrency
    }})
    .to_string();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let body = body.clone();
            tokio::spawn(async move {
                let service = service_fn(move |_: Request<Incoming>| {
                    let body = body.clone();
                    async move { Ok::<_, Infallible>(Response::new(Full::new(Bytes::from(body)))) }
                });
                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await;
            });
        }
    });
    Rest::builder().token("test-token").api(base).build()
}

/// A mock gateway: every connection gets a session (Hello, READY after IDENTIFY, then
/// `dispatches`); the `[shard, total]` of each IDENTIFY, and when it came, are sent to the
/// returned channel.
pub(crate) async fn mock_gateway(
    dispatches: Vec<Value>,
) -> (String, mpsc::UnboundedReceiver<(Value, Instant)>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let (identified_tx, identified_rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            tokio::spawn(mock_session(
                stream,
                dispatches.clone(),
                identified_tx.clone(),
            ));
        }
    });
    (url, identified_rx)
}

async fn mock_session(
    stream: TcpStream,
    dispatches: Vec<Value>,
    identified: mpsc::UnboundedSender<(Value, Instant)>,
) {
    let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();
    let send = |payload: Value| Message::text(payload.to_string());
    // Heartbeats every second, each acknowledged: the shards report a latency.
    ws.send(send(json!({"op": 10, "d": {"heartbeat_interval": 1_000}})))
        .await
        .unwrap();
    let identify = loop {
        let Some(Ok(message)) = timeout(STEP, ws.next()).await.unwrap() else {
            return;
        };
        if let Some(text) = message.as_text() {
            let payload: Value = serde_json::from_str(text).unwrap();
            match payload["op"].as_u64() {
                Some(1) => ws.send(send(json!({"op": 11}))).await.unwrap(),
                Some(2) => break payload,
                _ => {}
            }
        }
    };
    let _ = identified.send((identify["d"]["shard"].clone(), Instant::now()));
    ws.send(send(json!({"t": "READY", "s": 1, "op": 0, "d": {
        "v": 10,
        "user": {"id": "1", "username": "test-bot", "discriminator": "0", "global_name": null,
                 "avatar": null, "bot": true},
        "guilds": [], "session_id": "s", "resume_gateway_url": "ws://127.0.0.1:9",
        "application": {"id": "1"}, "shard": identify["d"]["shard"]
    }})))
    .await
    .unwrap();
    for dispatch in dispatches {
        ws.send(send(dispatch)).await.unwrap();
    }
    // Until the client closes the connection.
    while let Some(Ok(message)) = ws.next().await {
        if message.is_close() {
            break;
        }
        let heartbeat = message
            .as_text()
            .and_then(|text| serde_json::from_str::<Value>(text).ok())
            .is_some_and(|payload| payload["op"] == 1);
        if heartbeat && ws.send(send(json!({"op": 11}))).await.is_err() {
            break;
        }
    }
}
