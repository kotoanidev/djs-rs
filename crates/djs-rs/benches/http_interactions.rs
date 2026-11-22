//! Benchmarks of HTTP interactions (D-041, D-398). Run with
//! `cargo bench -p djs-rs --features http-interactions --bench http_interactions`.
//!
//! Each iteration is one signed request through `InteractionsEndpoint::handle`, and the
//! background work it returns, on a current-thread runtime: the timestamp and signature
//! checks, decoding, routing, the handler and the inline response. No REST request is made.
//! The global `AllocProfiler` reports allocations per request next to timings.

use std::sync::LazyLock;

use divan::{Bencher, black_box};
use djs_rs::{
    Client, ext::InteractionExt, http_interactions::InteractionsEndpoint,
    model::message::MessageFlags,
};
use tokio::runtime::Runtime;

#[path = "../tests/support/http_interactions.rs"]
mod support;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

/// An endpoint whose `/mod config set` replies inline and whose `/defer config set` defers.
static ENDPOINT: LazyLock<(Runtime, InteractionsEndpoint)> = LazyLock::new(|| {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    // The REST client is never used: every response here is inline.
    let client = runtime.block_on(async { Client::builder("bench-token").build() });
    client.command("mod config set", |ctx, cmd| async move {
        cmd.reply(&ctx, "Saved.").await?;
        Ok(())
    });
    client.command("defer config set", |ctx, cmd| async move {
        cmd.defer_reply(&ctx, MessageFlags::EPHEMERAL).await?;
        Ok(())
    });
    let endpoint = client
        .interactions_endpoint(&support::public_key())
        .unwrap();
    (runtime, endpoint)
});

/// Handles one request and runs its background work; returns the status.
fn handle(signature: &str, timestamp: &str, body: &[u8]) -> u16 {
    let (runtime, endpoint) = &*ENDPOINT;
    runtime.block_on(async {
        let (response, background) = endpoint.handle(signature, timestamp, body).await;
        background.await;
        response.status
    })
}

/// Measures `body` signed now; the request must be answered with `status`.
fn bench_signed(bencher: Bencher<'_, '_>, body: &[u8], status: u16) {
    let timestamp = support::now();
    let signature = support::sign(&timestamp, body);
    assert_eq!(handle(&signature, &timestamp, body), status);
    bencher
        .counter(divan::counter::BytesCount::new(body.len()))
        .bench(|| {
            handle(
                black_box(&signature),
                black_box(&timestamp),
                black_box(body),
            )
        });
}

/// A `PING`: the signature check, decoding and the `PONG`.
#[divan::bench]
fn ping(bencher: Bencher<'_, '_>) {
    bench_signed(bencher, support::PING, 200);
}

/// A realistic 2.4 KB slash command whose handler replies inline.
#[divan::bench]
fn reply(bencher: Bencher<'_, '_>) {
    bench_signed(bencher, &support::command(1), 200);
}

/// The same command deferred: the deferral is the HTTP body, the background work ends.
#[divan::bench]
fn deferral(bencher: Bencher<'_, '_>) {
    let body = String::from_utf8(support::command(1)).unwrap().replacen(
        r#""name":"mod""#,
        r#""name":"defer""#,
        1,
    );
    bench_signed(bencher, body.as_bytes(), 200);
}

/// A forged request: the signature check fails (a full Ed25519 verification).
#[divan::bench]
fn bad_signature(bencher: Bencher<'_, '_>) {
    let body = support::command(1);
    let timestamp = support::now();
    let signature = support::sign(&timestamp, b"another body");
    assert_eq!(handle(&signature, &timestamp, &body), 401);
    bencher.bench(|| {
        handle(
            black_box(&signature),
            black_box(&timestamp),
            black_box(&body),
        )
    });
}

/// A replayed request, older than five minutes: refused before any cryptography.
#[divan::bench]
fn stale_timestamp(bencher: Bencher<'_, '_>) {
    let body = support::command(1);
    let timestamp = "1000000000";
    let signature = support::sign(timestamp, &body);
    assert_eq!(handle(&signature, timestamp, &body), 401);
    bencher.bench(|| {
        handle(
            black_box(&signature),
            black_box(timestamp),
            black_box(&body),
        )
    });
}
