//! Multi-process sharding (D-284, D-296–D-300) across real processes: the manager starts this
//! test binary again, running only [`cluster_process`], against a mock REST API and a mock
//! gateway on `127.0.0.1`.

#![cfg(feature = "sharding")]

use std::{convert::Infallible, ffi::OsString, process::ExitStatus, time::Duration};

use bytes::Bytes;
use djs_rs::{
    Shards,
    gateway::Compression,
    prelude::*,
    rest::Rest,
    sharding::{self, ClusterError, ClusterEvent, ClusterEvents, ClusterManager},
};
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

/// How long one step may take; processes start in milliseconds, but CI machines are slow.
const STEP: Duration = Duration::from_secs(20);

/// The environment variable that gives [`cluster_process`] the mock REST API.
const API_ENV: &str = "DJS_RS_TEST_API";

/// The process side, started by the manager tests (a plain `cargo test` skips it; run
/// without a manager, it does nothing).
#[tokio::test]
#[ignore = "run by the manager tests, in the processes they start"]
async fn cluster_process() {
    let Some(assignment) = sharding::process().unwrap() else {
        return;
    };
    let api = std::env::var(API_ENV).unwrap();
    let client = Client::builder("test-token")
        .rest(Rest::builder().token("test-token").api(api).build())
        .intents(Intents::GUILD_MESSAGES)
        .compression(Compression::None)
        .cluster(assignment)
        .build();
    client.on(Events::MessageCreate, |ctx, msg| async move {
        if msg.content == "!stop" {
            ctx.shutdown();
        }
        Ok(())
    });
    client.login().await.unwrap();
}

/// The arguments that make this test binary run only [`cluster_process`].
fn child_args() -> [&'static str; 4] {
    ["cluster_process", "--exact", "--ignored", "--quiet"]
}

/// A mock REST API that answers every request with a Get Gateway Bot response.
async fn mock_rest(gateway_url: &str, shards: u32) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/api", listener.local_addr().unwrap());
    let body = json!({"url": gateway_url, "shards": shards, "session_start_limit": {
        "total": 1000, "remaining": 1000, "reset_after": 3_600_000, "max_concurrency": 16
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
    base
}

/// A mock gateway: every connection gets a session (Hello, READY after IDENTIFY, then
/// `dispatches`); the `[shard, total]` of each IDENTIFY is sent to the returned channel.
async fn mock_gateway(dispatches: Vec<Value>) -> (String, mpsc::UnboundedReceiver<Value>) {
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
    identified: mpsc::UnboundedSender<Value>,
) {
    let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();
    let send = |payload: Value| Message::text(payload.to_string());
    ws.send(send(json!({"op": 10, "d": {"heartbeat_interval": 45_000}})))
        .await
        .unwrap();
    let identify = loop {
        let Some(Ok(message)) = timeout(STEP, ws.next()).await.unwrap() else {
            return;
        };
        if let Some(text) = message.as_text() {
            let payload: Value = serde_json::from_str(text).unwrap();
            if payload["op"] == 2 {
                break payload;
            }
        }
    };
    let _ = identified.send(identify["d"]["shard"].clone());
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
    }
}

fn stop_message() -> Value {
    json!({"t": "MESSAGE_CREATE", "s": 2, "op": 0, "d": {
        "id": "10", "channel_id": "20", "guild_id": "2",
        "author": {"id": "3", "username": "nelly", "discriminator": "0", "global_name": null,
                   "avatar": null},
        "content": "!stop", "timestamp": "2026-11-14T12:00:00.000000+00:00",
        "edited_timestamp": null, "tts": false, "mention_everyone": false, "pinned": false,
        "type": 0
    }})
}

async fn next_event(events: &mut ClusterEvents) -> ClusterEvent {
    timeout(STEP, events.next())
        .await
        .expect("no cluster event in time")
        .expect("the manager is gone")
}

/// Collects the exit statuses of `count` clusters, by cluster ID.
async fn exits(events: &mut ClusterEvents, count: usize) -> Vec<(u32, ExitStatus)> {
    let mut exits = Vec::new();
    while exits.len() < count {
        if let ClusterEvent::Exited { cluster, status } = next_event(events).await {
            exits.push((cluster, status));
        }
    }
    exits.sort_by_key(|(cluster, _)| *cluster);
    exits
}

#[tokio::test]
async fn the_manager_runs_its_shards_in_processes() {
    let (gateway_url, mut identified) = mock_gateway(Vec::new()).await;
    let api = mock_rest(&gateway_url, 3).await;
    // Discord recommends 3 shards: 2 processes, of 2 and 1.
    let manager = ClusterManager::builder("test-token")
        .rest(Rest::builder().token("test-token").api(api.clone()).build())
        .shards_per_process(2)
        .executable(std::env::current_exe().unwrap())
        .args(child_args())
        .env(API_ENV, api)
        .build();
    let mut events = manager.events();
    let running = tokio::spawn({
        let manager = manager.clone();
        async move { manager.spawn().await }
    });

    let mut connected = Vec::new();
    while connected.len() < 2 {
        match next_event(&mut events).await {
            ClusterEvent::Connected { cluster } => connected.push(cluster),
            ClusterEvent::Spawned { pid, .. } => assert!(pid.is_some()),
            ClusterEvent::Exited { cluster, status } => panic!("cluster {cluster}: {status}"),
            other => panic!("unexpected {other:?}"),
        }
    }
    connected.sort_unstable();
    assert_eq!(connected, [0, 1]);
    // Every shard identifies once, from its process.
    let mut shards = Vec::new();
    for _ in 0..3 {
        shards.push(timeout(STEP, identified.recv()).await.unwrap().unwrap());
    }
    shards.sort_by_key(|shard| shard[0].as_u64());
    assert_eq!(shards, [json!([0, 3]), json!([1, 3]), json!([2, 3])]);

    let clusters = manager.clusters();
    assert_eq!(clusters.len(), 2);
    assert_eq!((clusters[0].id, clusters[0].shards.clone()), (0, 0..2));
    assert_eq!((clusters[1].id, clusters[1].shards.clone()), (1, 2..3));
    assert!(
        clusters
            .iter()
            .all(|cluster| cluster.connected && cluster.pid.is_some())
    );
    // One manager runs once at a time.
    assert!(matches!(
        manager.spawn().await,
        Err(ClusterError::AlreadyRunning)
    ));

    // A shutdown stops every process cleanly, and `spawn` returns.
    manager.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    let exits = exits(&mut events, 2).await;
    assert_eq!(
        exits
            .iter()
            .map(|(cluster, _)| *cluster)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert!(
        exits.iter().all(|(_, status)| status.success()),
        "{exits:?}"
    );
    assert!(
        manager
            .clusters()
            .iter()
            .all(|cluster| !cluster.connected && cluster.pid.is_none())
    );
}

#[tokio::test]
async fn spawn_returns_once_the_processes_stop_on_their_own() {
    let (gateway_url, _identified) = mock_gateway(vec![stop_message()]).await;
    let api = mock_rest(&gateway_url, 1).await;
    let manager = ClusterManager::builder("test-token")
        .shards(Shards::Count(1))
        .executable(std::env::current_exe().unwrap())
        .args(child_args())
        .env(API_ENV, api)
        .build();
    let mut events = manager.events();
    // The process's handler stops its client on `!stop`: nothing calls `shutdown` here.
    timeout(STEP, manager.spawn()).await.unwrap().unwrap();
    assert!(matches!(
        next_event(&mut events).await,
        ClusterEvent::Spawned { cluster: 0, .. }
    ));
    assert_eq!(
        next_event(&mut events).await,
        ClusterEvent::Connected { cluster: 0 }
    );
    let [(0, status)] = exits(&mut events, 1).await[..] else {
        panic!("one exit expected");
    };
    assert!(status.success(), "{status}");
}

#[cfg(unix)]
#[tokio::test]
async fn processes_that_do_not_stop_are_killed() {
    // `sleep` never connects, so it cannot be asked to stop.
    let manager = ClusterManager::builder("test-token")
        .shards(Shards::Count(2))
        .executable("sleep")
        .args([OsString::from("60")])
        .shutdown_timeout(Duration::from_millis(200))
        .build();
    let mut events = manager.events();
    let running = tokio::spawn({
        let manager = manager.clone();
        async move { manager.spawn().await }
    });
    for _ in 0..2 {
        assert!(matches!(
            next_event(&mut events).await,
            ClusterEvent::Spawned { .. }
        ));
    }
    manager.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    let exits = exits(&mut events, 2).await;
    assert!(
        exits.iter().all(|(_, status)| !status.success()),
        "{exits:?}"
    );
}

#[tokio::test]
async fn a_missing_executable_is_an_error() {
    let manager = ClusterManager::builder("test-token")
        .shards(Shards::Count(1))
        .executable("/nonexistent/djs-rs-cluster")
        .build();
    let result = timeout(STEP, manager.spawn()).await.unwrap();
    assert!(matches!(result, Err(ClusterError::Io(_))), "{result:?}");
    // The manager can be spawned again after an error.
    let result = timeout(STEP, manager.spawn()).await.unwrap();
    assert!(matches!(result, Err(ClusterError::Io(_))), "{result:?}");
}
