//! Extreme-load tests of HTTP interactions (D-041, D-398): bursts of signed requests through
//! `InteractionsEndpoint::handle`, all in flight at once, as a server spawning a task per
//! request would run them. No REST request is made.
//!
//! Ignored by default; run them in release mode, one at a time with pauses (they load every
//! core), and read the printed numbers:
//!
//! ```text
//! cargo test -p djs-rs --release --features http-interactions \
//!     --test stress_http_interactions -- --ignored --nocapture --test-threads 1
//! ```
#![expect(
    clippy::cast_precision_loss,
    reason = "throughput numbers are only printed"
)]

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use djs_rs::{
    Client, ext::InteractionExt, http_interactions::InteractionsEndpoint,
    model::message::MessageFlags,
};
use tokio::{sync::Semaphore, task::JoinSet};

#[path = "support/http_interactions.rs"]
mod support;

const REQUESTS: usize = 100_000;
/// Distinct signed requests, reused round-robin so that signing is not measured.
const POOL: u64 = 1_000;

/// `(VmRSS, VmHWM)` of this process in KiB (Linux only).
fn memory_kib() -> Option<(u64, u64)> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let field = |name: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    };
    Some((field("VmRSS:")?, field("VmHWM:")?))
}

fn memory(before: Option<(u64, u64)>) -> String {
    match (before, memory_kib()) {
        (Some((before, _)), Some((after, peak))) => format!(
            "rss {} → {} MiB, peak {} MiB",
            before / 1024,
            after / 1024,
            peak / 1024
        ),
        _ => "memory n/a".to_owned(),
    }
}

/// `POOL` requests `(signature, timestamp, body)`, each a distinct `/mod config set`; with
/// `forged`, every signature is of another body.
fn requests(forged: bool) -> Arc<Vec<(String, String, Vec<u8>)>> {
    let timestamp = support::now();
    let requests = (0..POOL)
        .map(|id| {
            let body = support::command(id);
            let signed: &[u8] = if forged { b"another body" } else { &body };
            (support::sign(&timestamp, signed), timestamp.clone(), body)
        })
        .collect();
    Arc::new(requests)
}

/// Handles `REQUESTS` requests, all spawned at once; each must be answered with `status`.
/// Calls `answered_all` once every response is known; returns when that was, and when every
/// background work had ended.
async fn burst(
    endpoint: &InteractionsEndpoint,
    forged: bool,
    status: u16,
    answered_all: impl FnOnce(),
) -> (Duration, Duration) {
    let requests = requests(forged);
    let answered = Arc::new(Semaphore::new(0));
    let start = Instant::now();
    let mut tasks = JoinSet::new();
    for index in 0..REQUESTS {
        let (endpoint, requests, answered) = (
            endpoint.clone(),
            Arc::clone(&requests),
            Arc::clone(&answered),
        );
        tasks.spawn(async move {
            let (signature, timestamp, body) = &requests[index % requests.len()];
            let (response, background) = endpoint.handle(signature, timestamp, body).await;
            assert_eq!(response.status, status);
            answered.add_permits(1);
            background.await;
        });
    }
    let all = u32::try_from(REQUESTS).unwrap();
    drop(answered.acquire_many(all).await.unwrap());
    let responses = start.elapsed();
    answered_all();
    while let Some(task) = tasks.join_next().await {
        task.unwrap();
    }
    (responses, start.elapsed())
}

fn report(name: &str, responses: Duration, done: Duration, memory: &str) {
    println!(
        "[stress] {name}: {REQUESTS} requests answered in {responses:.2?} = {:.0}/s \
         ({:.2} µs each), all work done in {done:.2?}; {memory}",
        REQUESTS as f64 / responses.as_secs_f64(),
        responses.as_secs_f64() * 1e6 / REQUESTS as f64,
    );
}

fn client() -> Client {
    Client::builder("stress-token").build()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn signed_commands_100k_replied_inline() {
    let client = client();
    client.command("mod config set", |ctx, cmd| async move {
        cmd.reply(&ctx, "Saved.").await?;
        Ok(())
    });
    let endpoint = client
        .interactions_endpoint(&support::public_key())
        .unwrap();
    let before = memory_kib();
    let _ = std::fs::write("/proc/self/clear_refs", "5");
    let (responses, done) = burst(&endpoint, false, 200, || ()).await;
    report("inline replies", responses, done, &memory(before));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn forged_requests_100k_refused() {
    let client = client();
    let endpoint = client
        .interactions_endpoint(&support::public_key())
        .unwrap();
    let before = memory_kib();
    let _ = std::fs::write("/proc/self/clear_refs", "5");
    let (responses, done) = burst(&endpoint, true, 401, || ()).await;
    report("forged signatures", responses, done, &memory(before));
}

/// Every handler defers, then waits until all `REQUESTS` deferrals are answered: the peak
/// memory is that of 100 000 handlers pending in the background at once.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn deferrals_100k_pending_at_once() {
    let client = client();
    let gate = Arc::new(Semaphore::new(0));
    let wait = Arc::clone(&gate);
    client.command("mod config set", move |ctx, cmd| {
        let wait = Arc::clone(&wait);
        async move {
            cmd.defer_reply(&ctx, MessageFlags::EPHEMERAL).await?;
            wait.acquire().await.unwrap().forget();
            Ok(())
        }
    });
    let endpoint = client
        .interactions_endpoint(&support::public_key())
        .unwrap();
    let before = memory_kib();
    let _ = std::fs::write("/proc/self/clear_refs", "5");
    // Every response is out, so every handler is pending: they may now end.
    let (responses, done) = burst(&endpoint, false, 200, || gate.add_permits(REQUESTS)).await;
    report("pending deferrals", responses, done, &memory(before));
}
