//! Multi-process sharding (D-284, D-296–D-300, D-305) across real processes: the manager
//! starts this test binary again, running only [`cluster_process`], against a mock REST API
//! (asked by the manager only) and a mock gateway on `127.0.0.1`.

#![cfg(feature = "sharding")]

use std::{
    convert::Infallible,
    ffi::OsString,
    process::ExitStatus,
    time::{Duration, Instant},
};

use bytes::Bytes;
use djs_rs::{
    Shards,
    gateway::Compression,
    prelude::*,
    rest::Rest,
    sharding::{self, ClusterError, ClusterEvent, ClusterEvents, ClusterManager, Restart},
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

/// The environment variable naming a file: the first [`cluster_process`] that creates it
/// crashes once its shards are ready.
const CRASH_ONCE_ENV: &str = "DJS_RS_TEST_CRASH_ONCE";

/// The process side, started by the manager tests (a plain `cargo test` skips it; run
/// without a manager, it does nothing).
#[tokio::test]
#[ignore = "run by the manager tests, in the processes they start"]
async fn cluster_process() {
    let Some(assignment) = sharding::process().unwrap() else {
        return;
    };
    // No REST API: the gateway's details come from the manager (D-305).
    let client = Client::builder("test-token")
        .rest(
            Rest::builder()
                .token("test-token")
                .api("http://127.0.0.1:9/api")
                .build(),
        )
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
    let crash_once = std::env::var_os(CRASH_ONCE_ENV);
    client.on(Events::ClientReady, move |_ctx, _ready| {
        let crash_once = crash_once.clone();
        async move {
            if let Some(marker) = crash_once
                && std::fs::File::create_new(marker).is_ok()
            {
                // The first process crashes; the one started again runs.
                std::process::exit(1);
            }
            Ok(())
        }
    });
    client.login().await.unwrap();
}

/// The arguments that make this test binary run only [`cluster_process`].
fn child_args() -> [&'static str; 4] {
    ["cluster_process", "--exact", "--ignored", "--quiet"]
}

/// A REST client for a mock API that answers every request with a Get Gateway Bot response.
async fn mock_rest(gateway_url: &str, shards: u32, remaining: u32, max_concurrency: u32) -> Rest {
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
async fn mock_gateway(
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
    // Discord recommends 3 shards: 2 processes, of 2 and 1.
    let manager = ClusterManager::builder("test-token")
        .rest(mock_rest(&gateway_url, 3, 1000, 16).await)
        .shards_per_process(2)
        .executable(std::env::current_exe().unwrap())
        .args(child_args())
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
        shards.push(timeout(STEP, identified.recv()).await.unwrap().unwrap().0);
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
    let manager = ClusterManager::builder("test-token")
        .rest(mock_rest(&gateway_url, 1, 1000, 1).await)
        .shards(Shards::Count(1))
        .executable(std::env::current_exe().unwrap())
        .args(child_args())
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
        .rest(mock_rest("ws://127.0.0.1:9", 2, 1000, 1).await)
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
        .rest(mock_rest("ws://127.0.0.1:9", 1, 1000, 1).await)
        .shards(Shards::Count(1))
        .executable("/nonexistent/djs-rs-cluster")
        .build();
    let result = timeout(STEP, manager.spawn()).await.unwrap();
    assert!(matches!(result, Err(ClusterError::Io(_))), "{result:?}");
    // The manager can be spawned again after an error.
    let result = timeout(STEP, manager.spawn()).await.unwrap();
    assert!(matches!(result, Err(ClusterError::Io(_))), "{result:?}");
}

#[tokio::test]
async fn too_few_session_starts_start_nothing() {
    let manager = ClusterManager::builder("test-token")
        .rest(mock_rest("ws://127.0.0.1:9", 4, 3, 1).await)
        .executable("/nonexistent/djs-rs-cluster")
        .build();
    let mut events = manager.events();
    let result = timeout(STEP, manager.spawn()).await.unwrap();
    assert!(
        matches!(
            result,
            Err(ClusterError::SessionStartLimit {
                required: 4,
                remaining: 3,
                ..
            })
        ),
        "{result:?}"
    );
    // No process was started.
    assert!(
        timeout(Duration::from_millis(50), events.next())
            .await
            .is_err()
    );
}

/// D-299: the manager grants the IDENTIFY turns of every process, one per rate-limit bucket
/// every 5 seconds. With `max_concurrency` 2, shards 0 and 2 share bucket 0 and shards 1 and
/// 3 bucket 1, each pair split over the two processes.
#[tokio::test]
async fn processes_identify_in_turn_by_bucket() {
    let (gateway_url, mut identified) = mock_gateway(Vec::new()).await;
    let manager = ClusterManager::builder("test-token")
        .rest(mock_rest(&gateway_url, 4, 1000, 2).await)
        .shards_per_process(2)
        .executable(std::env::current_exe().unwrap())
        .args(child_args())
        .build();
    let running = tokio::spawn({
        let manager = manager.clone();
        async move { manager.spawn().await }
    });
    let mut at = [None; 4];
    for _ in 0..4 {
        let (shard, when) = timeout(STEP, identified.recv()).await.unwrap().unwrap();
        assert_eq!(shard[1], 4);
        let number = usize::try_from(shard[0].as_u64().unwrap()).unwrap();
        assert!(
            at[number].replace(when).is_none(),
            "shard {number} identified twice"
        );
    }
    let at = at.map(Option::unwrap);
    let apart = |a: usize, b: usize| {
        at[a]
            .checked_duration_since(at[b])
            .unwrap_or_else(|| at[b] - at[a])
    };
    // A bucket waits 5 s between its shards, whichever process they are in…
    assert!(apart(0, 2) >= Duration::from_secs(5), "{:?}", apart(0, 2));
    assert!(apart(1, 3) >= Duration::from_secs(5), "{:?}", apart(1, 3));
    // …and the buckets do not wait for each other.
    let first = |a: usize, b: usize| at[a].min(at[b]);
    let starts = first(0, 2)
        .checked_duration_since(first(1, 3))
        .unwrap_or_else(|| first(1, 3) - first(0, 2));
    assert!(starts < Duration::from_secs(2), "{starts:?}");

    manager.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
}

/// A manager running `sh -c <script>` as its only cluster process.
#[cfg(unix)]
async fn shell(script: &str, restart: Restart) -> ClusterManager {
    ClusterManager::builder("test-token")
        .rest(mock_rest("ws://127.0.0.1:9", 1, 1000, 1).await)
        .shards(Shards::Count(1))
        .executable("sh")
        .args(["-c", script])
        .restart(restart)
        .restart_delay(Duration::from_millis(50), Duration::from_millis(400))
        .build()
}

/// The events of a manager until `count` restarts, as (kind, exit code or delay in ms).
#[cfg(unix)]
async fn until_restarts(events: &mut ClusterEvents, count: usize) -> Vec<(&'static str, u128)> {
    let mut seen = Vec::new();
    let mut restarts = 0;
    while restarts < count {
        seen.push(match next_event(events).await {
            ClusterEvent::Spawned { cluster: 0, .. } => ("spawned", 0),
            ClusterEvent::Exited { cluster: 0, status } => {
                ("exited", status.code().unwrap_or(-1).try_into().unwrap())
            }
            ClusterEvent::Restarting { cluster: 0, delay } => {
                restarts += 1;
                ("restarting", delay.as_millis())
            }
            other => panic!("unexpected {other:?}"),
        });
    }
    seen
}

#[cfg(unix)]
#[tokio::test]
async fn failed_processes_are_restarted_with_growing_delays() {
    let manager = shell("exit 3", Restart::OnFailure).await;
    let mut events = manager.events();
    let running = tokio::spawn({
        let manager = manager.clone();
        async move { manager.spawn().await }
    });
    let seen = until_restarts(&mut events, 5).await;
    let mut expected = Vec::new();
    for delay in [50, 100, 200, 400, 400] {
        expected.extend([("spawned", 0), ("exited", 3), ("restarting", delay)]);
    }
    assert_eq!(seen, expected);
    // A shutdown cancels the restart still waiting, and `spawn` returns.
    manager.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn the_restart_policy_chooses_the_exits() {
    // A clean exit ends the cluster by default, and `spawn` returns by itself.
    let manager = shell("exit 0", Restart::OnFailure).await;
    let mut events = manager.events();
    timeout(STEP, manager.spawn()).await.unwrap().unwrap();
    let [(0, status)] = exits(&mut events, 1).await[..] else {
        panic!("one exit expected");
    };
    assert!(status.success());
    assert!(
        timeout(Duration::from_millis(50), events.next())
            .await
            .is_err()
    );

    // `Never` does not restart failures either.
    let manager = shell("exit 3", Restart::Never).await;
    let mut events = manager.events();
    timeout(STEP, manager.spawn()).await.unwrap().unwrap();
    assert_eq!(exits(&mut events, 1).await[0].1.code(), Some(3));

    // `Always` restarts clean exits too.
    let manager = shell("exit 0", Restart::Always).await;
    let mut events = manager.events();
    let running = tokio::spawn({
        let manager = manager.clone();
        async move { manager.spawn().await }
    });
    assert_eq!(
        until_restarts(&mut events, 1).await,
        [("spawned", 0), ("exited", 0), ("restarting", 50)]
    );
    manager.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
}

#[test]
#[should_panic(expected = "must not be zero")]
fn a_zero_restart_delay_is_rejected() {
    let _ =
        ClusterManager::builder("test-token").restart_delay(Duration::ZERO, Duration::from_secs(1));
}

#[test]
#[should_panic(expected = "must not be longer than the largest")]
fn a_first_delay_past_the_largest_is_rejected() {
    let _ = ClusterManager::builder("test-token")
        .restart_delay(Duration::from_secs(2), Duration::from_secs(1));
}

/// A crashed process gets the same shards again, connects again, and its shard identifies
/// again in its turn: the IDENTIFY queue outlives the processes (D-299).
#[tokio::test]
async fn a_crashed_process_is_started_again() {
    let (gateway_url, mut identified) = mock_gateway(Vec::new()).await;
    let marker = std::env::temp_dir().join(format!(
        "djs-rs-crash-once-{}-{:?}",
        std::process::id(),
        Instant::now()
    ));
    let manager = ClusterManager::builder("test-token")
        .rest(mock_rest(&gateway_url, 1, 1000, 1).await)
        .executable(std::env::current_exe().unwrap())
        .args(child_args())
        .env(CRASH_ONCE_ENV, &marker)
        .restart_delay(Duration::from_millis(50), Duration::from_secs(1))
        .build();
    let mut events = manager.events();
    let running = tokio::spawn({
        let manager = manager.clone();
        async move { manager.spawn().await }
    });

    let mut seen = Vec::new();
    while seen.len() < 6 {
        seen.push(match next_event(&mut events).await {
            ClusterEvent::Spawned { cluster: 0, .. } => "spawned",
            ClusterEvent::Connected { cluster: 0 } => "connected",
            ClusterEvent::Exited { cluster: 0, status } => {
                assert_eq!(status.code(), Some(1));
                "crashed"
            }
            ClusterEvent::Restarting { cluster: 0, delay } => {
                assert_eq!(delay, Duration::from_millis(50));
                "restarting"
            }
            other => panic!("unexpected {other:?}"),
        });
    }
    assert_eq!(
        seen,
        [
            "spawned",
            "connected",
            "crashed",
            "restarting",
            "spawned",
            "connected"
        ]
    );
    let (shard, first) = timeout(STEP, identified.recv()).await.unwrap().unwrap();
    assert_eq!(shard, json!([0, 1]));
    let (shard, second) = timeout(STEP, identified.recv()).await.unwrap().unwrap();
    assert_eq!(shard, json!([0, 1]));
    assert!(
        second - first >= Duration::from_secs(5),
        "{:?}",
        second - first
    );

    manager.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    assert!(exits(&mut events, 1).await[0].1.success());
    std::fs::remove_file(&marker).unwrap();
}
