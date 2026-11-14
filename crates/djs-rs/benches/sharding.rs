//! Benchmarks of multi-process sharding (D-041, Phase 9): a command's round trip from the
//! manager to 1, 4 and 16 cluster processes and back, payload sizes, and the built-in stats.
//! Run with `cargo bench -p djs-rs --features sharding --bench sharding`.
//!
//! This benchmark binary is also the cluster processes: started by the manager (the
//! assignment in its environment), it runs a client that answers the commands instead of
//! the benchmarks. The global `AllocProfiler` counts the manager's allocations per command.

#[path = "../tests/support/sharding.rs"]
mod support;

use std::{
    ffi::OsString,
    sync::{LazyLock, OnceLock},
};

use divan::{Bencher, black_box};
use djs_rs::{
    gateway::Compression,
    prelude::*,
    rest::Rest,
    sharding::{self, ClusterAssignment, ClusterCommand, ClusterEvent, ClusterManager},
};
use support::{mock_gateway, mock_rest};
use tokio::runtime::Runtime;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

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

fn main() {
    match sharding::process().unwrap() {
        Some(assignment) => cluster(assignment),
        None => divan::main(),
    }
}

/// A cluster process: answers the commands until the manager goes away.
fn cluster(assignment: ClusterAssignment) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let client = Client::builder("bench-token")
            .rest(
                Rest::builder()
                    .token("bench-token")
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
            .handle::<Payload, _, _>(|_, input| async move { Ok(input) });
        let _ = client.login().await;
    });
}

/// The manager's runtime.
static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
});

/// A running manager of `clusters` processes, all connected. Its processes exit when this
/// benchmark process does (their connection to it closes).
fn manager(clusters: u32) -> &'static ClusterManager {
    static MANAGERS: [OnceLock<ClusterManager>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let slot = match clusters {
        1 => &MANAGERS[0],
        4 => &MANAGERS[1],
        _ => &MANAGERS[2],
    };
    slot.get_or_init(|| {
        RUNTIME.block_on(async {
            let (gateway_url, _identified) = mock_gateway(Vec::new()).await;
            let manager = ClusterManager::builder("bench-token")
                .rest(mock_rest(&gateway_url, clusters, 1000, 16).await)
                .executable(std::env::current_exe().unwrap())
                .args(Vec::<OsString>::new())
                .build();
            let mut events = manager.events();
            tokio::spawn({
                let manager = manager.clone();
                async move { manager.spawn().await }
            });
            let mut connected = 0;
            while connected < clusters {
                if let Some(ClusterEvent::Connected { .. }) = events.next().await {
                    connected += 1;
                }
            }
            manager
        })
    })
}

/// A broadcast of a small command to every cluster, and every answer back.
#[divan::bench(args = [1, 4, 16])]
fn broadcast(bencher: Bencher<'_, '_>, clusters: u32) {
    let manager = manager(clusters);
    bencher.bench(|| {
        let replies = RUNTIME
            .block_on(manager.broadcast::<Echo>(black_box(7)))
            .unwrap();
        assert_eq!(replies.len(), clusters as usize);
    });
}

/// A small command to one cluster.
#[divan::bench]
fn send(bencher: Bencher<'_, '_>) {
    let manager = manager(4);
    bencher.bench(|| {
        RUNTIME
            .block_on(manager.send::<Echo>(2, black_box(7)))
            .unwrap()
    });
}

/// A string of `size` bytes to one cluster and back.
#[divan::bench(args = [1 << 10, 64 << 10, 1 << 20])]
fn payload(bencher: Bencher<'_, '_>, size: usize) {
    let manager = manager(1);
    let input = "x".repeat(size);
    bencher
        .counter(divan::counter::BytesCount::new(size * 2))
        .bench(|| {
            RUNTIME
                .block_on(manager.send::<Payload>(0, input.clone()))
                .unwrap()
        });
}

/// The built-in stats of 4 clusters.
#[divan::bench]
fn stats(bencher: Bencher<'_, '_>) {
    let manager = manager(4);
    bencher.bench(|| RUNTIME.block_on(manager.stats()).unwrap());
}
