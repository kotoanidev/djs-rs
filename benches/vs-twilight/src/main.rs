//! The twilight side of the djs-rs vs twilight comparison (`docs/VS_TWILIGHT.md`, D-385).
//!
//! The djs-rs numbers come from the `vs-node` binary of `benches/vs-node` (with
//! `--cache twilight` for a cache equal to twilight's default one), and both libraries connect
//! to the same mock servers (`vs-node mock-gateway`, `vs-node mock-rest`) and are measured by
//! the same code (`djs_vs_node::metrics`). This binary only runs twilight 0.18.0, as its
//! README writes a bot: a [`Shard`] whose events go through an [`InMemoryCache`] (its default
//! configuration) and to a spawned handler, and a twilight-http [`Client`]:
//!
//! | Subcommand | What it does |
//! |------------|--------------|
//! | `decode --dir D` | Times decoding of the `vs-node fixtures` into twilight's `GatewayEvent`. |
//! | `flood --gateway URL --events N [--cache none]` | A shard, the cache, a spawned handler per event counting `MESSAGE_CREATE`. |
//! | `memory --gateway URL --guilds N` | A shard caching the guilds it receives, then reporting RSS. |
//! | `rest --api URL --requests N --concurrency C [--limiter off]` | Concurrent `create_message` calls through twilight-http. |
//!
//! twilight is pointed at the mocks the way it documents for proxies: the gateway's
//! `proxy_url` and the HTTP client's `proxy(host, use_http)`, so it speaks plain WebSocket and
//! HTTP to `127.0.0.1` and keeps its rate limiter (its proactive global limit lifted, as
//! djs-rs's is). Its gateway is built without transport
//! compression (no `zstd` feature), as the mock gateway sends plain JSON to every library.
//! Every subcommand prints one JSON line with its results, in the format of the `vs-node`
//! runners. Runs happen in a network namespace without a route out (C-108).

use std::{
    fs,
    hint::black_box,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use djs_vs_node::{
    Options, TOKEN,
    metrics::{Window, cpu_ms, measure, percentile, status_kib},
    payloads::channel_id,
};
use serde::de::DeserializeSeed;
use serde_json::json;
use twilight_cache_inmemory::{DefaultInMemoryCache, ResourceType};
use twilight_gateway::{
    ConfigBuilder, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _,
};
use twilight_http::Client;
use twilight_http_ratelimiting::RateLimiter;
use twilight_model::{
    gateway::{
        event::{DispatchEvent, GatewayEvent, GatewayEventDeserializer},
        payload::incoming::GuildCreate,
    },
    id::Id,
};

/// The fixture files written by `vs-node fixtures`.
const FILES: [&str; 2] = ["message_create.json", "guild_create.json"];

/// The global allocator of this binary: the system one, twilight's default.
const ALLOCATOR: &str = "system";

fn main() {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let options = Options::parse(args);
    let runtime = || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("the tokio runtime starts")
    };
    match command.as_str() {
        "decode" => decode(&options),
        "flood" => runtime().block_on(flood(&options)),
        "memory" => runtime().block_on(memory(&options)),
        "rest" => runtime().block_on(rest(&options)),
        _ => panic!("unknown subcommand `{command}` (decode, flood, memory, rest)"),
    }
}

fn decode_once(json: &str) -> GatewayEvent {
    let seed = GatewayEventDeserializer::from_json(json).expect("a gateway payload");
    seed.deserialize(&mut serde_json::Deserializer::from_str(json))
        .expect("the payload decodes")
}

/// Times decoding of each fixture into twilight's `GatewayEvent`, as its shards do (the
/// same two steps as `twilight_gateway::parse`, without taking the `String`).
fn decode(options: &Options) {
    let dir = Path::new(options.required("dir"));
    for file in FILES {
        let json = fs::read_to_string(dir.join(file)).expect("run `vs-node fixtures` first");
        match decode_once(&json) {
            GatewayEvent::Dispatch(_, DispatchEvent::MessageCreate(message)) => {
                assert!(message.member.is_some());
            }
            GatewayEvent::Dispatch(_, DispatchEvent::GuildCreate(guild)) => {
                let GuildCreate::Available(guild) = *guild else {
                    panic!("an available guild")
                };
                assert_eq!(guild.members.len(), 1000);
            }
            other => panic!("unexpected event {other:?}"),
        }
        let timing = measure(|| drop(black_box(decode_once(black_box(&json)))));
        println!(
            "{}",
            json!({
                "scenario": "decode",
                "payload": file.trim_end_matches(".json").to_uppercase(),
                "bytes": json.len(),
                "library": "twilight",
                "variant": "twilight-model GatewayEvent",
                "allocator": ALLOCATOR,
                "median_ns": timing.median_ns,
                "min_ns": timing.min_ns,
            })
        );
    }
}

/// A shard on the mock gateway: its URL as twilight's `proxy_url`, checked to be local.
fn shard(options: &Options, intents: Intents) -> Shard {
    let gateway = options.required("gateway").trim_end_matches('/');
    assert!(
        gateway.starts_with("ws://127.0.0.1:"),
        "only a local mock gateway is allowed, got {gateway}"
    );
    let config = ConfigBuilder::new(TOKEN.to_owned(), intents)
        .proxy_url(gateway.to_owned())
        .build();
    Shard::with_config(ShardId::ONE, config)
}

/// A bot's event handler, spawned for each event as twilight's README does.
#[expect(
    clippy::unused_async,
    reason = "a task per event, as twilight's README spawns its handler"
)]
async fn handle(event: Event, window: Arc<Window>) {
    if let Event::MessageCreate(message) = event {
        black_box(&message.content);
        window.hit();
    }
}

/// A shard with the default cache (or none with `--cache none`) and a handler task per event,
/// through the mock gateway's flood.
async fn flood(options: &Options) {
    let window = Arc::new(Window::new(options.get("events", 1_000_000)));
    let cached = match options.get("cache", String::from("default")).as_str() {
        "default" => true,
        "none" => false,
        other => panic!("unknown --cache {other} (default, none)"),
    };
    let intents = Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT;
    let mut shard = shard(options, intents);
    let cache = DefaultInMemoryCache::new();
    let counter = Arc::clone(&window);
    tokio::spawn(async move {
        while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
            let Ok(event) = item else {
                continue;
            };
            if cached {
                cache.update(&event);
            }
            tokio::spawn(handle(event, Arc::clone(&counter)));
        }
        eprintln!("the twilight shard ended");
        std::process::exit(1);
    });
    window.wait().await;
    let variant = if cached {
        "Shard + InMemoryCache + a task per event, default cache"
    } else {
        "Shard + a task per event, no cache"
    };
    println!("{}", window.result("twilight", variant, ALLOCATOR));
    std::process::exit(0);
}

/// Connects to the mock gateway, caches the `--guilds` guilds it sends and prints the
/// resident memory before and after.
async fn memory(options: &Options) {
    let expected: usize = options.get("guilds", 100);
    let mut shard = shard(options, Intents::GUILDS | Intents::GUILD_MEMBERS);
    let cache = DefaultInMemoryCache::new();
    let mut connected_rss = 0;
    let mut guilds = Vec::new();
    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let Ok(event) = item else {
            continue;
        };
        cache.update(&event);
        match &event {
            Event::Ready(_) => {
                connected_rss = status_kib("VmRSS");
                if expected == 0 {
                    break;
                }
            }
            Event::GuildCreate(guild) => {
                guilds.push(guild.id());
                if guilds.len() == expected {
                    break;
                }
            }
            _ => {}
        }
    }
    let stats = cache.stats();
    let members: usize = guilds
        .iter()
        .map(|&id| stats.guild_members(id).unwrap_or(0))
        .sum();
    assert!(
        cache.config().resource_types() == ResourceType::all(),
        "the default cache"
    );
    println!(
        "{}",
        json!({
            "scenario": "memory",
            "library": "twilight",
            "variant": "Shard + default InMemoryCache",
            "allocator": ALLOCATOR,
            "guilds": stats.guilds(),
            "members": members,
            "connected_rss_kib": connected_rss,
            "rss_kib": status_kib("VmRSS"),
            "peak_rss_kib": status_kib("VmHWM"),
        })
    );
    std::process::exit(0);
}

/// twilight-http on the mock API (`--api http://127.0.0.1:<port>/api`) as its proxy: its
/// rate limiter with the proactive global limit lifted, as `vs-node rest` lifts djs-rs's
/// (`RateLimiter::new(u16::MAX)`, the way twilight documents to disable it; the default is 50
/// requests per second), or (`--limiter off`) no rate limiter at all.
fn http(options: &Options) -> (Client, &'static str) {
    let api = options.required("api");
    let host = api
        .strip_prefix("http://")
        .and_then(|rest| rest.strip_suffix("/api"))
        .expect("--api is http://<host>/api");
    assert!(
        host.starts_with("127.0.0.1:"),
        "only a local mock API is allowed, got {host}"
    );
    let builder = Client::builder()
        .token(TOKEN.to_owned())
        .proxy(host.to_owned(), true);
    if options.get("limiter", String::from("on")) == "off" {
        (builder.ratelimiter(None).build(), "rate limiter disabled")
    } else {
        (
            builder
                .ratelimiter(Some(RateLimiter::new(u16::MAX)))
                .build(),
            "rate limiter, global limit lifted",
        )
    }
}

/// Sends `requests` requests from `concurrency` workers and returns every latency in µs.
async fn round(http: &Arc<Client>, requests: u64, concurrency: u64) -> Vec<u32> {
    let issued = Arc::new(AtomicU64::new(0));
    let workers: Vec<_> = (0..concurrency)
        .map(|worker| {
            let http = Arc::clone(http);
            let issued = Arc::clone(&issued);
            tokio::spawn(async move {
                let channel = Id::new(channel_id(0, worker));
                let mut latencies = Vec::new();
                while issued.fetch_add(1, Ordering::Relaxed) < requests {
                    let started = Instant::now();
                    let message = http
                        .create_message(channel)
                        .content("benchmark")
                        .await
                        .expect("the mock API answers")
                        .model()
                        .await
                        .expect("the message decodes");
                    black_box(message);
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

/// A warm-up round (a tenth of the requests), then the measured round, as `vs-node rest`.
async fn rest(options: &Options) {
    let requests: u64 = options.get("requests", 200_000);
    let concurrency: u64 = options.get("concurrency", 64);
    let (http, variant) = http(options);
    let http = Arc::new(http);
    round(&http, requests / 10, concurrency).await;

    let cpu_before = cpu_ms();
    let started = Instant::now();
    let mut latencies = round(&http, requests, concurrency).await;
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
            "library": "twilight",
            "variant": format!("Client::create_message, {variant}"),
            "allocator": ALLOCATOR,
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
