//! Scenario 4: REST throughput — concurrent Create Message calls against the mock API.
//!
//! Each worker posts to its own channel, so every request goes through a different rate-limit
//! bucket and neither library serializes requests within a bucket. The proactive global limit
//! (50/s by default in both libraries) is disabled so the client overhead is what is measured.

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use djs_model::{id::ChannelId, rest::CreateMessage};
use djs_rest::Rest;
use serde_json::json;

use crate::{
    Options, TOKEN,
    metrics::{cpu_ms, percentile, status_kib},
    payloads::channel_id,
};

/// Sends `requests` requests from `concurrency` workers and returns every latency in µs.
async fn round(rest: &Rest, requests: u64, concurrency: u64) -> Vec<u32> {
    let issued = Arc::new(AtomicU64::new(0));
    let workers: Vec<_> = (0..concurrency)
        .map(|worker| {
            let rest = rest.clone();
            let issued = Arc::clone(&issued);
            tokio::spawn(async move {
                let channel = ChannelId::new(channel_id(0, worker));
                let body = CreateMessage::from("benchmark");
                let mut latencies = Vec::new();
                while issued.fetch_add(1, Ordering::Relaxed) < requests {
                    let started = Instant::now();
                    let message = rest
                        .create_message(channel, &body)
                        .await
                        .expect("the mock API answers");
                    std::hint::black_box(message);
                    latencies
                        .push(u32::try_from(started.elapsed().as_micros()).unwrap_or(u32::MAX));
                }
                latencies
            })
        })
        .collect();
    let mut latencies = Vec::new();
    for worker in workers {
        latencies.extend(worker.await.expect("a worker finished"));
    }
    latencies
}

/// Runs a warm-up round (a tenth of the requests, which also opens the connections), then the
/// measured round, and prints the result.
pub(crate) async fn run(options: &Options, allocator: &str) {
    let requests: u64 = options.get("requests", 200_000);
    let concurrency: u64 = options.get("concurrency", 64);
    let rest = Rest::builder()
        .token(TOKEN)
        .api(options.required("api"))
        .global_requests_per_second(0)
        .build();
    round(&rest, requests / 10, concurrency).await;

    let cpu_before = cpu_ms();
    let started = Instant::now();
    let mut latencies = round(&rest, requests, concurrency).await;
    let seconds = started.elapsed().as_secs_f64();
    let cpu = cpu_ms() - cpu_before;
    latencies.sort_unstable();
    #[expect(
        clippy::cast_precision_loss,
        reason = "request counts are far below 2^52"
    )]
    let count = requests as f64;
    println!(
        "{}",
        json!({
            "scenario": "rest",
            "library": "djs-rs",
            "variant": "djs-rest create_message",
            "allocator": allocator,
            "requests": requests,
            "concurrency": concurrency,
            "seconds": seconds,
            "requests_per_sec": count / seconds,
            "p50_us": percentile(&latencies, 50.0),
            "p99_us": percentile(&latencies, 99.0),
            "cpu_ms": cpu,
            "cpu_us_per_request": cpu * 1000.0 / count,
            "peak_rss_kib": status_kib("VmHWM"),
        })
    );
}
