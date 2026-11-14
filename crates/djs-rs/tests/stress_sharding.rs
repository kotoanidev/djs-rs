//! Extreme-load tests of multi-process sharding (D-041, Phase 9): many processes, command
//! floods, large payloads, a restart storm and a flood of bogus connections against the
//! manager. The manager starts this test binary again as its cluster processes
//! ([`cluster_process`]), against a mock REST API and gateway on `127.0.0.1`.
//!
//! Ignored by default; run them in release mode, one at a time, and read the printed numbers:
//!
//! ```text
//! cargo test -p djs-rs --features sharding --release --test stress_sharding -- \
//!     --ignored --nocapture --test-threads 1 --skip cluster_process
//! ```

#![cfg(feature = "sharding")]
#![expect(
    clippy::cast_precision_loss,
    reason = "throughput numbers are only printed"
)]

#[path = "support/sharding.rs"]
mod support;

use std::{
    ffi::OsString,
    sync::Arc,
    time::{Duration, Instant},
};

use djs_rs::{
    Shards,
    gateway::Compression,
    prelude::*,
    rest::Rest,
    sharding::{self, ClusterCommand, ClusterEvent, ClusterEvents, ClusterManager, ShardStatus},
};
use serde_json::Value;
use support::{mock_gateway, mock_rest};
use tokio::{io::AsyncWriteExt, net::TcpStream, task::JoinSet, time::timeout};

const STEP: Duration = Duration::from_secs(120);

/// Returns its input.
struct Echo;

impl ClusterCommand for Echo {
    const NAME: &'static str = "echo";
    type Input = u64;
    type Output = u64;
}

/// Returns its input, a string of any size.
struct Payload;

impl ClusterCommand for Payload {
    const NAME: &'static str = "payload";
    type Input = String;
    type Output = String;
}

/// From inside a cluster: `Input` broadcasts of [`Echo`] one after another; answers with
/// how long they took, in microseconds.
struct Relay;

impl ClusterCommand for Relay {
    const NAME: &'static str = "relay";
    type Input = u32;
    type Output = u64;
}

/// The process side (skipped by a plain run; without a manager it does nothing).
#[tokio::test]
#[ignore = "run by the manager tests, in the processes they start"]
async fn cluster_process() {
    let Some(assignment) = sharding::process().unwrap() else {
        return;
    };
    let client = Client::builder("stress-token")
        .rest(
            Rest::builder()
                .token("stress-token")
                .api("http://127.0.0.1:9/api")
                .build(),
        )
        .intents(Intents::GUILDS)
        .compression(Compression::None)
        .cluster(assignment)
        .build();
    client
        .cluster()
        .unwrap()
        .handle::<Echo, _, _>(|_, input| async move { Ok(input) })
        .handle::<Payload, _, _>(|_, input| async move { Ok(input) })
        .handle::<Relay, _, _>(|client, count| async move {
            let cluster = client.cluster().unwrap();
            let started = Instant::now();
            for n in 0..count {
                let replies = cluster.broadcast::<Echo>(u64::from(n)).await?;
                assert!(replies.iter().all(|reply| reply.result.is_ok()));
            }
            Ok(u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX))
        });
    client.login().await.unwrap();
}

fn child_args() -> [&'static str; 4] {
    ["cluster_process", "--exact", "--ignored", "--quiet"]
}

async fn next_event(events: &mut ClusterEvents) -> ClusterEvent {
    timeout(STEP, events.next())
        .await
        .expect("no cluster event in time")
        .expect("the manager is gone")
}

/// Starts a manager of `clusters` processes of `per_process` shards and waits until every
/// process is connected.
async fn running(
    clusters: u32,
    per_process: u32,
    max_concurrency: u32,
) -> (
    ClusterManager,
    tokio::task::JoinHandle<Result<(), sharding::ClusterError>>,
    Duration,
) {
    let (gateway_url, _identified) = mock_gateway(Vec::new()).await;
    let manager = ClusterManager::builder("stress-token")
        .rest(mock_rest(&gateway_url, clusters * per_process, 1000, max_concurrency).await)
        .shards_per_process(per_process)
        .executable(std::env::current_exe().unwrap())
        .args(child_args())
        .build();
    let mut events = manager.events();
    let started = Instant::now();
    let handle = tokio::spawn({
        let manager = manager.clone();
        async move { manager.spawn().await }
    });
    let mut connected = 0;
    while connected < clusters {
        if let ClusterEvent::Connected { .. } = next_event(&mut events).await {
            connected += 1;
        }
    }
    (manager, handle, started.elapsed())
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    let index = ((sorted.len() - 1) as f64 * p).round();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "an index"
    )]
    sorted[index as usize]
}

fn latencies(label: &str, mut samples: Vec<Duration>) {
    samples.sort_unstable();
    let total: Duration = samples.iter().sum();
    println!(
        "{label}: {} ops, {:.0} ops/s, p50 {:?}, p99 {:?}, max {:?}",
        samples.len(),
        samples.len() as f64 / total.as_secs_f64(),
        percentile(&samples, 0.5),
        percentile(&samples, 0.99),
        samples.last().unwrap()
    );
}

/// This process's resident set size in MiB, and its open file descriptors.
fn own_usage() -> (f64, usize) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let kib: f64 = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .unwrap()
        .trim()
        .trim_end_matches("kB")
        .trim()
        .parse()
        .unwrap();
    let fds = std::fs::read_dir("/proc/self/fd").unwrap().count();
    (kib / 1024.0, fds)
}

/// This process's open file descriptors by kind (`socket`, `pipe`, `anon_inode:[eventfd]`, …).
fn fd_kinds() -> std::collections::BTreeMap<String, usize> {
    let mut kinds = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir("/proc/self/fd").unwrap().flatten() {
        let Ok(target) = std::fs::read_link(entry.path()) else {
            continue;
        };
        let target = target.to_string_lossy().into_owned();
        let kind = target.split(['[', ':']).next().unwrap_or("?").to_owned();
        let kind = if target.starts_with("anon_inode") {
            target
        } else if kind.starts_with('/') {
            "file".into()
        } else {
            kind
        };
        *kinds.entry(kind).or_default() += 1;
    }
    kinds
}

/// 64 processes of one shard: the time to start and connect them, the IDENTIFY schedule
/// over all of them (`max_concurrency` 16: four rounds 5 s apart), their memory, and the
/// time to stop them.
#[tokio::test]
#[ignore = "extreme-load test; run in release, see the module docs"]
async fn sixty_four_processes() {
    let (manager_mib, manager_fds) = own_usage();
    println!("file descriptors before: {manager_fds}, {:?}", fd_kinds());
    let (gateway_url, mut identified) = mock_gateway(Vec::new()).await;
    let manager = ClusterManager::builder("stress-token")
        .rest(mock_rest(&gateway_url, 64, 1000, 16).await)
        .executable(std::env::current_exe().unwrap())
        .args(child_args())
        .build();
    let mut events = manager.events();
    let started = Instant::now();
    let handle = tokio::spawn({
        let manager = manager.clone();
        async move { manager.spawn().await }
    });
    let mut connected = 0;
    while connected < 64 {
        if let ClusterEvent::Connected { .. } = next_event(&mut events).await {
            connected += 1;
        }
    }
    let connect_time = started.elapsed();

    let mut at = vec![None; 64];
    for _ in 0..64 {
        let (shard, when) = timeout(STEP, identified.recv()).await.unwrap().unwrap();
        let number = usize::try_from(shard[0].as_u64().unwrap()).unwrap();
        assert!(at[number].replace(when).is_none(), "shard {number} twice");
    }
    let at: Vec<Instant> = at.into_iter().map(Option::unwrap).collect();
    let first = *at.iter().min().unwrap();
    let identify_time = at.iter().max().unwrap().duration_since(first);
    // Each bucket's four shards at least 5 s apart.
    let mut closest = Duration::MAX;
    for bucket in 0..16 {
        let mut times: Vec<Instant> = (bucket..64).step_by(16).map(|shard| at[shard]).collect();
        times.sort_unstable();
        for pair in times.windows(2) {
            closest = closest.min(pair[1] - pair[0]);
        }
    }
    assert!(closest >= Duration::from_secs(5), "{closest:?}");

    // Until every shard reports itself active.
    let stats = timeout(STEP, async {
        loop {
            let stats: Vec<_> = manager
                .stats()
                .await
                .unwrap()
                .into_iter()
                .filter_map(|reply| reply.result.ok())
                .collect();
            if stats.len() == 64
                && stats
                    .iter()
                    .flat_map(|stats| &stats.shards)
                    .all(|shard| shard.status == ShardStatus::Active)
            {
                break stats;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    let mut memory: Vec<f64> = stats
        .iter()
        .map(|stats| stats.memory.unwrap() as f64 / (1024.0 * 1024.0))
        .collect();
    memory.sort_by(f64::total_cmp);
    let (with_clusters_mib, with_clusters_fds) = own_usage();

    let stopping = Instant::now();
    manager.shutdown();
    timeout(STEP, handle).await.unwrap().unwrap().unwrap();
    let stop_time = stopping.elapsed();
    let (after_mib, after_fds) = own_usage();
    // Once the connections' tasks have seen their ends.
    tokio::time::sleep(Duration::from_secs(1)).await;
    let (settled_mib, settled_fds) = own_usage();
    println!(
        "file descriptors 1 s after: {settled_fds}, {:?}",
        fd_kinds()
    );
    println!(
        "64 processes: connected in {connect_time:?}; identified over {identify_time:?} \
         (closest in a bucket {closest:?}); RSS per process min {:.1} / median {:.1} / max \
         {:.1} MiB, total {:.0} MiB; stopped in {stop_time:?}",
        memory[0],
        memory[32],
        memory[63],
        memory.iter().sum::<f64>()
    );
    println!(
        "the manager's process: {manager_mib:.1} MiB, {manager_fds} fds before; \
         {with_clusters_mib:.1} MiB, {with_clusters_fds} fds with 64 clusters; \
         {after_mib:.1} MiB, {after_fds} fds after; {settled_mib:.1} MiB, {settled_fds} fds \
         1 s later"
    );
}

/// Commands at full speed over 16 processes: broadcasts and single sends one after another
/// (latency) and 64 in flight (throughput), and broadcasts sent from inside a cluster.
#[tokio::test]
#[ignore = "extreme-load test; run in release, see the module docs"]
async fn command_floods() {
    let (manager, handle, connect_time) = running(16, 1, 16).await;
    println!("16 processes connected in {connect_time:?}");
    let (before_mib, _) = own_usage();

    let mut samples = Vec::new();
    for n in 0..5_000 {
        let started = Instant::now();
        let replies = manager.broadcast::<Echo>(n).await.unwrap();
        samples.push(started.elapsed());
        assert!(replies.iter().all(|reply| reply.result.is_ok()));
    }
    latencies("broadcast to 16, one at a time", samples);

    let mut samples = Vec::new();
    for n in 0..20_000_u64 {
        let started = Instant::now();
        let cluster = u32::try_from(n % 16).unwrap();
        assert_eq!(manager.send::<Echo>(cluster, n).await.unwrap(), n);
        samples.push(started.elapsed());
    }
    latencies("send to one of 16, one at a time", samples);

    for (label, total, broadcast) in [
        ("broadcasts to 16, 64 in flight", 20_000_u64, true),
        ("sends, 64 in flight", 200_000, false),
    ] {
        let manager = Arc::new(manager.clone());
        let started = Instant::now();
        let mut tasks = JoinSet::new();
        for worker in 0..64 {
            let manager = Arc::clone(&manager);
            tasks.spawn(async move {
                let mut n = worker;
                while n < total {
                    if broadcast {
                        let replies = manager.broadcast::<Echo>(n).await.unwrap();
                        assert!(replies.iter().all(|reply| reply.result.is_ok()));
                    } else {
                        let cluster = u32::try_from(n % 16).unwrap();
                        assert_eq!(manager.send::<Echo>(cluster, n).await.unwrap(), n);
                    }
                    n += 64;
                }
            });
        }
        while let Some(done) = tasks.join_next().await {
            done.unwrap();
        }
        let elapsed = started.elapsed();
        let messages = if broadcast { total * 16 } else { total };
        println!(
            "{label}: {total} in {elapsed:?}: {:.0} commands/s, {:.0} cluster answers/s",
            total as f64 / elapsed.as_secs_f64(),
            messages as f64 / elapsed.as_secs_f64()
        );
    }

    // From inside every cluster at once: 1000 broadcasts each, through the manager.
    let started = Instant::now();
    let replies = manager.broadcast::<Relay>(1_000).await.unwrap();
    let elapsed = started.elapsed();
    let mut per_cluster: Vec<Duration> = replies
        .into_iter()
        .map(|reply| Duration::from_micros(reply.result.unwrap()))
        .collect();
    per_cluster.sort_unstable();
    println!(
        "16 clusters broadcasting 1000 times each at once: {elapsed:?} ({:.0} broadcasts/s, \
         {:.0} cluster answers/s); slowest cluster {:?}",
        16_000.0 / elapsed.as_secs_f64(),
        256_000.0 / elapsed.as_secs_f64(),
        per_cluster.last().unwrap()
    );

    let (after_mib, _) = own_usage();
    println!("the manager's process: {before_mib:.1} MiB before the floods, {after_mib:.1} after");
    manager.shutdown();
    timeout(STEP, handle).await.unwrap().unwrap().unwrap();
}

/// Large inputs and results through one process, up to the 16 MiB frame.
#[tokio::test]
#[ignore = "extreme-load test; run in release, see the module docs"]
async fn large_payloads() {
    let (manager, handle, _) = running(2, 1, 2).await;
    for mib in [1, 4, 8, 15] {
        let input = "x".repeat(mib << 20);
        let rounds = 20;
        let started = Instant::now();
        for _ in 0..rounds {
            let output = manager.send::<Payload>(0, input.clone()).await.unwrap();
            assert_eq!(output.len(), input.len());
        }
        let elapsed = started.elapsed() / rounds;
        println!(
            "{mib} MiB there and back: {elapsed:?} per round trip, {:.0} MiB/s both ways \
             together",
            mib as f64 / elapsed.as_secs_f64() * 2.0
        );
    }
    let failed = manager.send::<Payload>(0, "x".repeat(17 << 20)).await;
    assert!(
        matches!(failed, Err(sharding::ClusterError::Encoding(_))),
        "{failed:?}"
    );
    assert_eq!(manager.send::<Echo>(1, 7).await.unwrap(), 7);
    manager.shutdown();
    timeout(STEP, handle).await.unwrap().unwrap().unwrap();
}

/// 8 processes that fail at once, restarted every 1–4 ms for 10 seconds: the manager's
/// memory and file descriptors before and after.
#[cfg(unix)]
#[tokio::test]
#[ignore = "extreme-load test; run in release, see the module docs"]
async fn restart_storm() {
    let (before_mib, before_fds) = own_usage();
    let manager = ClusterManager::builder("stress-token")
        .rest(mock_rest("ws://127.0.0.1:9", 8, 1000, 1).await)
        .shards(Shards::Count(8))
        .executable("sh")
        .args([OsString::from("-c"), OsString::from("exit 1")])
        .restart_delay(Duration::from_millis(1), Duration::from_millis(4))
        .build();
    let mut events = manager.events();
    let handle = tokio::spawn({
        let manager = manager.clone();
        async move { manager.spawn().await }
    });
    let started = Instant::now();
    let (mut spawned, mut exited, mut restarts) = (0_u64, 0_u64, 0_u64);
    while started.elapsed() < Duration::from_secs(10) {
        match next_event(&mut events).await {
            ClusterEvent::Spawned { .. } => spawned += 1,
            ClusterEvent::Exited { .. } => exited += 1,
            ClusterEvent::Restarting { .. } => restarts += 1,
            _ => {}
        }
    }
    let (during_mib, during_fds) = own_usage();
    manager.shutdown();
    timeout(STEP, handle).await.unwrap().unwrap().unwrap();
    drop(events);
    let (after_mib, after_fds) = own_usage();
    println!(
        "restart storm, 10 s: {spawned} processes started ({:.0}/s), {exited} exits, \
         {restarts} restarts; the manager's process: {before_mib:.1} MiB / {before_fds} fds \
         before, {during_mib:.1} MiB / {during_fds} fds during, {after_mib:.1} MiB / \
         {after_fds} fds after",
        spawned as f64 / 10.0
    );
    assert!(after_fds <= before_fds + 4, "{before_fds} → {after_fds}");
}

/// The manager's address and token, read from a cluster process's environment (Linux).
fn manager_of(pid: u32) -> (String, String) {
    let environ = std::fs::read(format!("/proc/{pid}/environ")).unwrap();
    let value = environ
        .split(|byte| *byte == 0)
        .find_map(|entry| entry.strip_prefix(b"DJS_RS_CLUSTER="))
        .unwrap();
    let assignment: Value = serde_json::from_slice(value).unwrap();
    (
        assignment["address"].as_str().unwrap().to_owned(),
        assignment["token"].as_str().unwrap().to_owned(),
    )
}

fn frame(json: &str) -> Vec<u8> {
    let mut frame = u32::try_from(json.len()).unwrap().to_be_bytes().to_vec();
    frame.extend_from_slice(json.as_bytes());
    frame
}

/// 1200 bogus connections to the manager's port (garbage, a frame announced too large, a
/// wrong token, a second connection for a connected cluster, silence) while commands go on.
#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "extreme-load test; run in release, see the module docs"]
async fn bogus_connections() {
    let (manager, handle, _) = running(4, 1, 4).await;
    let pid = manager.clusters()[0].pid.unwrap();
    let (address, token) = manager_of(pid);
    let (before_mib, before_fds) = own_usage();

    // Commands go on until the flood is over.
    let flooding = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let commands = tokio::spawn({
        let (manager, flooding) = (manager.clone(), Arc::clone(&flooding));
        async move {
            let mut samples = Vec::new();
            let mut n = 0;
            while flooding.load(std::sync::atomic::Ordering::Relaxed) || samples.len() < 100 {
                let started = Instant::now();
                let replies = manager.broadcast::<Echo>(n).await.unwrap();
                samples.push(started.elapsed());
                assert!(replies.iter().all(|reply| reply.result.is_ok()));
                n += 1;
            }
            samples
        }
    });
    let flood = Instant::now();
    let mut silent = Vec::new();
    let mut attacks = JoinSet::new();
    for n in 0..1_200 {
        let stream = TcpStream::connect(&address).await.unwrap();
        let payload = match n % 5 {
            0 => b"GET / HTTP/1.1\r\nHost: x\r\n\r\n".to_vec(),
            1 => (16_u32 << 20).to_be_bytes().to_vec(),
            2 => frame(r#"{"op":"hello","cluster":0,"token":"wrong"}"#),
            3 => frame(&format!(
                r#"{{"op":"hello","cluster":0,"token":"{token}"}}"#
            )),
            _ => {
                silent.push(stream);
                continue;
            }
        };
        attacks.spawn(async move {
            let mut stream = stream;
            let _ = stream.write_all(&payload).await;
            // Held until the manager closes it.
            let mut buffer = [0; 64];
            let _ = timeout(
                Duration::from_secs(15),
                tokio::io::AsyncReadExt::read(&mut stream, &mut buffer),
            )
            .await;
        });
    }
    while let Some(done) = attacks.join_next().await {
        done.unwrap();
    }
    let refused = flood.elapsed();
    flooding.store(false, std::sync::atomic::Ordering::Relaxed);
    let (during_mib, during_fds) = own_usage();
    latencies(
        "broadcast to 4 during the flood",
        timeout(STEP, commands).await.unwrap().unwrap(),
    );
    assert!(manager.clusters().iter().all(|cluster| cluster.connected));
    drop(silent);
    // The manager drops silent connections after 10 s; here they are closed sooner.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (after_mib, after_fds) = own_usage();
    println!(
        "1200 connections opened and the 960 bogus ones closed by the manager in {refused:?} \
         (240 silent ones held); \
         this process: {before_mib:.1} MiB / {before_fds} fds before, {during_mib:.1} MiB / \
         {during_fds} fds during, {after_mib:.1} MiB / {after_fds} fds after"
    );
    assert_eq!(manager.send::<Echo>(3, 3).await.unwrap(), 3);
    manager.shutdown();
    timeout(STEP, handle).await.unwrap().unwrap().unwrap();
}
