//! The serenity side of the djs-rs vs serenity comparison (`docs/VS_SERENITY.md`,
//! D-277–D-280).
//!
//! The djs-rs numbers come from the `vs-node` binary of `benches/vs-node` (the runners of
//! `docs/VS_NODE.md`, with `--cache serenity` for a cache equal to serenity's default one),
//! and both libraries connect to the same mock servers (`vs-node mock-gateway`,
//! `vs-node mock-rest`) and are measured by the same code (`djs_vs_node::metrics`). This
//! binary only runs serenity 0.12.5 with its default features and settings (D-280):
//!
//! | Subcommand | What it does |
//! |------------|--------------|
//! | `tls-api --dir D [--gateway URL]` | The HTTPS mock API serenity talks to (D-282), with a CA made for the run. |
//! | `decode --dir D` | Times decoding of the `vs-node fixtures` into serenity's `GatewayEvent`. |
//! | `flood --events N` | A serenity `Client` counting `message` handler calls. |
//! | `memory --guilds N` | A serenity `Client` caching the guilds it receives, then reporting RSS. |
//! | `rest --requests N --concurrency C` | Concurrent `send_message` calls through serenity's `Http`. |
//!
//! The clients take `--tls 127.0.0.1:<port> --ca <ca.pem>` (a running `tls-api`): serenity
//! with its default configuration, its HTTP client resolving `discord.com` to the mock.
//! `rest` also takes `--api http://127.0.0.1:<port>/api --limiter off` instead: serenity's
//! documented proxy setup, which needs its rate limiter disabled (the bound of D-282). Every
//! subcommand prints one JSON line with its results, in the format of the `vs-node` runners.
//! Runs happen in a network namespace without a route out (C-108).

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
use serde_json::json;
use serenity::{
    Client,
    all::{
        ChannelId, ClientBuilder, Context, EventHandler, GatewayIntents, Http, HttpBuilder,
        Message, Ready, async_trait,
    },
    model::{
        event::{Event, GatewayEvent},
        id::GuildId,
    },
};

mod tls;

/// The fixture files written by `vs-node fixtures`.
const FILES: [&str; 2] = ["message_create.json", "guild_create.json"];

/// The global allocator of this binary: the system one, serenity's default.
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
        "tls-api" => runtime().block_on(tls::serve(&options)),
        "decode" => decode(&options),
        "flood" => runtime().block_on(flood(&options)),
        "memory" => runtime().block_on(memory(&options)),
        "rest" => runtime().block_on(rest(&options)),
        _ => panic!("unknown subcommand `{command}` (tls-api, decode, flood, memory, rest)"),
    }
}

/// serenity's HTTP client and the variant it is: with `--tls` and `--ca`, its default
/// configuration given a client that reaches the HTTPS mock (D-282); with `--api` and
/// `--limiter off`, its proxy setup without the rate limiter, over plain HTTP.
fn http(options: &Options) -> (Http, &'static str) {
    if options.get("limiter", String::from("on")) == "off" {
        let api = options.required("api");
        // serenity's proxy replaces `https://discord.com` in its URLs (`…/api/v10/…`); its rate
        // limiter would not, so a proxy is only ever used with the limiter disabled.
        let origin = api.strip_suffix("/api").expect("--api ends with /api");
        assert!(
            origin.starts_with("http://127.0.0.1:"),
            "only a local mock API is allowed, got {origin}"
        );
        let http = HttpBuilder::new(TOKEN)
            .proxy(origin)
            .ratelimiter_disabled(true)
            .build();
        (http, "rate limiter disabled, plain HTTP proxy")
    } else {
        let http = HttpBuilder::new(TOKEN).client(tls::client(options)).build();
        (http, "default configuration")
    }
}

/// A serenity `Client` on the mock servers. serenity asks Get Gateway when it is built and
/// falls back to the real gateway if that fails, so the mock's answer is checked first: no
/// benchmark may ever reach Discord.
async fn client(
    options: &Options,
    intents: GatewayIntents,
    handler: impl EventHandler + 'static,
) -> Client {
    let (http, _) = http(options);
    let gateway = http
        .get_gateway()
        .await
        .expect("the mock answers Get Gateway");
    assert!(
        gateway.url.starts_with("ws://127.0.0.1:"),
        "only a local mock gateway is allowed, got {}",
        gateway.url
    );
    ClientBuilder::new_with_http(http, intents)
        .event_handler(handler)
        .await
        .expect("the client is built")
}

/// Starts `client` on a task; a failure ends the process instead of leaving the scenario
/// waiting forever.
fn start(mut client: Client) {
    tokio::spawn(async move {
        if let Err(error) = client.start().await {
            eprintln!("serenity client failed: {error}");
            std::process::exit(1);
        }
    });
}

fn decode_once(json: &str) -> GatewayEvent {
    serde_json::from_str(json).expect("the payload decodes")
}

/// Times decoding of each fixture into serenity's `GatewayEvent`, as its shards do.
fn decode(options: &Options) {
    let dir = Path::new(options.required("dir"));
    for file in FILES {
        let json = fs::read_to_string(dir.join(file)).expect("run `vs-node fixtures` first");
        match decode_once(&json) {
            GatewayEvent::Dispatch(_, Event::MessageCreate(event)) => {
                assert!(event.message.member.is_some());
            }
            GatewayEvent::Dispatch(_, Event::GuildCreate(event)) => {
                assert_eq!(event.guild.members.len(), 1000);
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
                "library": "serenity",
                "variant": "serenity GatewayEvent",
                "allocator": ALLOCATOR,
                "median_ns": timing.median_ns,
                "min_ns": timing.min_ns,
            })
        );
    }
}

/// Counts `message` calls in a [`Window`].
struct Flood(Arc<Window>);

#[async_trait]
impl EventHandler for Flood {
    async fn message(&self, _ctx: Context, message: Message) {
        black_box(&message.content);
        self.0.hit();
    }
}

/// A serenity `Client` with one `message` handler and its default cache, through the mock
/// gateway's flood.
async fn flood(options: &Options) {
    let window = Arc::new(Window::new(options.get("events", 1_000_000)));
    let intents =
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    start(client(options, intents, Flood(Arc::clone(&window))).await);
    window.wait().await;
    println!(
        "{}",
        window.result(
            "serenity",
            "Client + message handler, default cache",
            ALLOCATOR
        )
    );
    std::process::exit(0);
}

/// Reports the memory once the guilds are cached: at `ready` without guilds, else at
/// `cache_ready` (every guild of `READY` received and cached).
struct Memory {
    guilds: usize,
    connected_rss: AtomicU64,
}

impl Memory {
    fn report(&self, ctx: &Context) -> ! {
        let guilds = ctx.cache.guilds();
        let members: usize = guilds
            .iter()
            .map(|&id| ctx.cache.guild(id).map_or(0, |guild| guild.members.len()))
            .sum();
        println!(
            "{}",
            json!({
                "scenario": "memory",
                "library": "serenity",
                "variant": "Client + default cache",
                "allocator": ALLOCATOR,
                "guilds": guilds.len(),
                "members": members,
                "connected_rss_kib": self.connected_rss.load(Ordering::Relaxed),
                "rss_kib": status_kib("VmRSS"),
                "peak_rss_kib": status_kib("VmHWM"),
            })
        );
        std::process::exit(0);
    }
}

#[async_trait]
impl EventHandler for Memory {
    async fn ready(&self, ctx: Context, _ready: Ready) {
        self.connected_rss
            .store(status_kib("VmRSS"), Ordering::Relaxed);
        if self.guilds == 0 {
            self.report(&ctx);
        }
    }

    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        self.report(&ctx);
    }
}

/// Connects to the mock gateway, caches the `--guilds` guilds it sends and prints the
/// resident memory before and after.
async fn memory(options: &Options) {
    let handler = Memory {
        guilds: options.get("guilds", 100),
        connected_rss: AtomicU64::new(0),
    };
    start(
        client(
            options,
            GatewayIntents::GUILDS | GatewayIntents::GUILD_MEMBERS,
            handler,
        )
        .await,
    );
    std::future::pending::<()>().await;
}

/// Sends `requests` requests from `concurrency` workers and returns every latency in µs.
async fn round(http: &Arc<Http>, requests: u64, concurrency: u64) -> Vec<u32> {
    let issued = Arc::new(AtomicU64::new(0));
    let workers: Vec<_> = (0..concurrency)
        .map(|worker| {
            let http = Arc::clone(http);
            let issued = Arc::clone(&issued);
            tokio::spawn(async move {
                let channel = ChannelId::new(channel_id(0, worker));
                let body = json!({ "content": "benchmark" });
                let mut latencies = Vec::new();
                while issued.fetch_add(1, Ordering::Relaxed) < requests {
                    let started = Instant::now();
                    let message = http
                        .send_message(channel, Vec::new(), &body)
                        .await
                        .expect("the mock API answers");
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
            "library": "serenity",
            "variant": format!("Http::send_message, {variant}"),
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
