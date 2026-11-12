//! Extreme-load tests of `Shard` against a mock gateway that floods it with events (D-041).
//!
//! These tests are ignored by default because they take seconds and saturate the machine. Run
//! them in release mode and read the printed numbers:
//!
//! ```text
//! cargo test -p djs-gateway --release --test stress -- --ignored --nocapture --test-threads 1
//! ```
//!
//! The server runs on other runtime threads in the same process; the measured rate is therefore
//! a lower bound of what the shard alone can do.

#![expect(
    clippy::cast_precision_loss,
    reason = "throughput numbers are only printed"
)]

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use bytes::Bytes;
use djs_gateway::{Compression, Config, LocalIdentifyQueue, MessageSender, Shard, create_shards};
use djs_model::{
    gateway::{DispatchEvent, GatewayEvent, Intents, ShardId, payload::RequestGuildMembers},
    id::GuildId,
};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::TcpListener,
    sync::{mpsc, watch},
    time::timeout,
};
use tokio_websockets::{Message, ServerBuilder};

const STEP: Duration = Duration::from_secs(30);

/// A realistic `MESSAGE_CREATE` dispatch (≈ 700 bytes) with a varying sequence and message ID.
fn message_create(sequence: u64) -> String {
    format!(
        r#"{{"t":"MESSAGE_CREATE","s":{sequence},"op":0,"d":{{"type":0,"tts":false,"timestamp":"2026-11-10T19:42:07.123000+00:00","pinned":false,"nonce":"{sequence}","mentions":[],"mention_roles":[],"mention_everyone":false,"member":{{"roles":["1100000000000000001","1100000000000000002"],"premium_since":null,"pending":false,"nick":null,"mute":false,"joined_at":"2024-05-01T12:00:00.000000+00:00","flags":0,"deaf":false,"communication_disabled_until":null,"banner":null,"avatar":null}},"id":"{}","flags":0,"embeds":[],"edited_timestamp":null,"content":"stress message number {sequence}","components":[],"channel_id":"1200000000000000020","author":{{"username":"nelly","public_flags":0,"id":"1300000000000000003","global_name":"Nelly","discriminator":"0","clan":null,"avatar_decoration_data":null,"avatar":"a_1269e74af4df7417b13759eae50c83dc"}},"attachments":[],"guild_id":"1400000000000000002"}}}}"#,
        1_500_000_000_000_000_000_u64 + sequence
    )
}

/// A large dispatch the library does not know (shaped like a `GUILD_CREATE` with `members`
/// members, but with an unknown name), which the shard skips.
fn large_unknown_event(sequence: u64, members: u64) -> String {
    let members: Vec<String> = (0..members)
        .map(|i| {
            format!(
                r#"{{"user":{{"id":"{}","username":"member{i}","discriminator":"0","global_name":null,"avatar":null}},"roles":["1","2","3"],"joined_at":"2024-01-01T00:00:00.000000+00:00","deaf":false,"mute":false,"flags":0}}"#,
                1_000_000_000_000_000_000_u64 + i
            )
        })
        .collect();
    format!(
        r#"{{"t":"FUTURE_EVENT","s":{sequence},"op":0,"d":{{"id":"{sequence}","name":"big","members":[{}]}}}}"#,
        members.join(",")
    )
}

/// Realistic payloads shared with the `djs-model` benchmarks and stress tests.
#[allow(
    dead_code,
    reason = "shared with the djs-model tests, which use other helpers"
)]
#[path = "../../djs-model/tests/common/payloads.rs"]
mod payloads;

const HELLO: &str = r#"{"t":null,"s":null,"op":10,"d":{"heartbeat_interval":600000}}"#;
const READY: &str = r#"{"t":"READY","s":1,"op":0,"d":{"v":10,"user":{"id":"1","username":"stress-bot","discriminator":"0","global_name":null,"avatar":null,"bot":true},"guilds":[],"session_id":"s","resume_gateway_url":"ws://127.0.0.1:9","shard":[0,1],"application":{"id":"1","flags":0}}}"#;

/// Compresses payloads like Discord's `zstd-stream`: one stream, flushed after every payload,
/// with the 64 KiB window (`windowLog` 16) read from the real gateway's first frame header.
#[cfg(feature = "zstd-stream")]
struct StreamCompressor(zstd::stream::raw::Encoder<'static>);

#[cfg(feature = "zstd-stream")]
impl StreamCompressor {
    fn new() -> Self {
        let mut encoder = zstd::stream::raw::Encoder::new(3).unwrap();
        encoder
            .set_parameter(zstd::stream::raw::CParameter::WindowLog(16))
            .unwrap();
        Self(encoder)
    }

    fn compress(&mut self, data: &[u8]) -> Vec<u8> {
        use zstd::stream::raw::{InBuffer, Operation, OutBuffer};
        let mut out = Vec::with_capacity(data.len() / 4 + 64);
        let mut input = InBuffer::around(data);
        while input.pos() < data.len() {
            out.reserve(4096);
            let position = out.len();
            self.0
                .run(&mut input, &mut OutBuffer::around_pos(&mut out, position))
                .unwrap();
        }
        loop {
            out.reserve(4096);
            let position = out.len();
            if self
                .0
                .flush(&mut OutBuffer::around_pos(&mut out, position))
                .unwrap()
                == 0
            {
                out.shrink_to_fit();
                return out;
            }
        }
    }
}

/// Compresses payloads like Discord's `zlib-stream`: one zlib stream, sync-flushed after every
/// payload, at the best level (the real gateway's zlib header says so) with a 32 KiB window.
#[cfg(feature = "zlib-stream")]
struct ZlibStreamCompressor(flate2::Compress);

#[cfg(feature = "zlib-stream")]
impl ZlibStreamCompressor {
    fn new() -> Self {
        Self(flate2::Compress::new(flate2::Compression::best(), true))
    }

    fn compress(&mut self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(data.len() / 3 + 64);
        let start = self.0.total_in();
        loop {
            out.reserve(4096);
            let read = usize::try_from(self.0.total_in() - start).unwrap();
            self.0
                .compress_vec(&data[read..], &mut out, flate2::FlushCompress::Sync)
                .unwrap();
            let read = usize::try_from(self.0.total_in() - start).unwrap();
            if read == data.len() && out.len() < out.capacity() {
                out.shrink_to_fit();
                return out;
            }
        }
    }
}

/// How the mock gateway encodes frames.
enum Frames {
    /// Text frames generated on the fly by the server task.
    Text(Box<dyn Iterator<Item = String> + Send>),
    /// Precomputed binary (compressed) frames, starting with Hello and READY.
    #[cfg_attr(
        not(any(feature = "zstd-stream", feature = "zlib-stream")),
        expect(dead_code, reason = "only compressed floods use binary frames")
    )]
    Binary(Vec<Vec<u8>>),
}

/// Serves one session: Hello → wait for Identify → READY → all `frames`, then waits for the
/// client to disconnect. Returns the number of payload bytes sent after READY.
async fn serve(listener: TcpListener, frames: Frames) -> u64 {
    let (stream, _) = timeout(STEP, listener.accept()).await.unwrap().unwrap();
    stream.set_nodelay(true).unwrap();
    let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();

    let mut sent = 0_u64;
    match frames {
        Frames::Text(payloads) => {
            ws.send(Message::text(HELLO.to_owned())).await.unwrap();
            wait_for_identify(&mut ws).await;
            ws.send(Message::text(READY.to_owned())).await.unwrap();
            for (index, payload) in payloads.enumerate() {
                sent += payload.len() as u64;
                ws.feed(Message::text(payload)).await.unwrap();
                if index % 64 == 63 {
                    ws.flush().await.unwrap();
                }
            }
        }
        Frames::Binary(frames) => {
            let mut frames = frames.into_iter();
            ws.send(Message::binary(frames.next().unwrap()))
                .await
                .unwrap();
            wait_for_identify(&mut ws).await;
            ws.send(Message::binary(frames.next().unwrap()))
                .await
                .unwrap();
            for (index, frame) in frames.enumerate() {
                sent += frame.len() as u64;
                ws.feed(Message::binary(frame)).await.unwrap();
                if index % 64 == 63 {
                    ws.flush().await.unwrap();
                }
            }
        }
    }
    ws.flush().await.unwrap();
    while let Ok(Some(Ok(_))) = timeout(STEP, ws.next()).await {}
    sent
}

async fn wait_for_identify(ws: &mut tokio_websockets::WebSocketStream<tokio::net::TcpStream>) {
    loop {
        let message = timeout(STEP, ws.next()).await.unwrap().unwrap().unwrap();
        if message
            .as_text()
            .is_some_and(|text| text.contains(r#""op":2"#))
        {
            return;
        }
    }
}

/// Connects a shard, receives READY and then `expected` further dispatches, checking that
/// every sequence number arrives in order. Returns the time from READY to the last dispatch.
async fn drain(url: &str, compression: Compression, expected: u64) -> Duration {
    let config = Config::builder("stress-token", Intents::GUILD_MESSAGES)
        .gateway_url(url)
        .compression(compression)
        .build();
    let mut shard = Shard::with_config(ShardId::ONE, config);
    let mut started = None;
    let mut received = 0;
    let mut cpu_at_ready = 0_u64;
    while received < expected {
        let event = timeout(STEP, shard.next_event())
            .await
            .expect("no event in time")
            .expect("shard ended")
            .expect("shard error");
        match event {
            GatewayEvent::Dispatch(_, DispatchEvent::Ready(_)) => {
                started = Some(Instant::now());
                cpu_at_ready = thread_cpu_ns();
            }
            GatewayEvent::Dispatch(sequence, _) => {
                received += 1;
                assert_eq!(sequence, received + 1, "dispatch out of order");
            }
            _ => {}
        }
    }
    let elapsed = started.expect("no READY").elapsed();
    let cpu = thread_cpu_ns() - cpu_at_ready;
    println!(
        "[stress]   shard thread CPU: {:.1} ns per event",
        cpu as f64 / expected as f64
    );
    shard.close().await;
    elapsed
}

/// CPU time of the current thread in nanoseconds (Linux `schedstat`).
fn thread_cpu_ns() -> u64 {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

/// `(VmRSS, VmHWM)` of this process in KiB, from `/proc/self/status` (Linux only).
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

/// Resets `VmHWM` to the current RSS so the next reading shows the peak of the measured part.
fn reset_peak_memory() {
    let _ = std::fs::write("/proc/self/clear_refs", "5");
}

fn report(
    name: &str,
    events: u64,
    wire_bytes: u64,
    elapsed: Duration,
    baseline: Option<(u64, u64)>,
) {
    let seconds = elapsed.as_secs_f64();
    let memory = match (baseline, memory_kib()) {
        (Some((before, _)), Some((after, peak))) => format!(
            "rss {} → {} MiB, peak {} MiB",
            before / 1024,
            after / 1024,
            peak / 1024
        ),
        _ => "memory n/a".to_owned(),
    };
    println!(
        "[stress] {name}: {events} events in {elapsed:.2?} = {:.0} events/s, \
         {:.1} MB/s on the wire; {memory}",
        events as f64 / seconds,
        wire_bytes as f64 / seconds / 1e6,
    );
}

async fn listener() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    (listener, url)
}

/// One million `MESSAGE_CREATE` text frames through one shard.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_1m_message_creates_plain() {
    const EVENTS: u64 = 1_000_000;
    let (listener, url) = listener().await;
    let payloads = (2..EVENTS + 2).map(message_create);
    let server = tokio::spawn(serve(listener, Frames::Text(Box::new(payloads))));
    let baseline = memory_kib();
    reset_peak_memory();
    let elapsed = drain(&url, Compression::None, EVENTS).await;
    let sent = server.await.unwrap();
    report("1M MESSAGE_CREATE, plain", EVENTS, sent, elapsed, baseline);
}

/// One million `MESSAGE_CREATE` frames compressed with `zstd-stream`.
#[cfg(feature = "zstd-stream")]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_1m_message_creates_zstd() {
    const EVENTS: u64 = 1_000_000;
    let mut compressor = StreamCompressor::new();
    let mut frames = vec![
        compressor.compress(HELLO.as_bytes()),
        compressor.compress(READY.as_bytes()),
    ];
    frames.extend(
        (2..EVENTS + 2).map(|sequence| compressor.compress(message_create(sequence).as_bytes())),
    );
    let raw: u64 = (2..EVENTS + 2)
        .map(|sequence| message_create(sequence).len() as u64)
        .sum();

    let (listener, url) = listener().await;
    let server = tokio::spawn(serve(listener, Frames::Binary(frames)));
    let baseline = memory_kib();
    reset_peak_memory();
    let elapsed = drain(&url, Compression::ZstdStream, EVENTS).await;
    let sent = server.await.unwrap();
    report(
        "1M MESSAGE_CREATE, zstd-stream",
        EVENTS,
        sent,
        elapsed,
        baseline,
    );
    println!(
        "[stress]   zstd ratio: {raw} → {sent} bytes ({:.1}x); decompressed {:.1} MB/s",
        raw as f64 / sent as f64,
        raw as f64 / elapsed.as_secs_f64() / 1e6
    );
}

/// 2 000 unknown events of ~200 KB each (shaped like 1 000-member `GUILD_CREATE`s), which the
/// shard skips; memory must stay bounded.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_2k_large_unknown_events_plain() {
    const EVENTS: u64 = 2_000;
    let template = large_unknown_event(0, 1_000);
    let (listener, url) = listener().await;
    let payloads = (2..EVENTS + 2)
        .map(move |sequence| template.replacen(r#""s":0"#, &format!(r#""s":{sequence}"#), 1));
    let server = tokio::spawn(serve(listener, Frames::Text(Box::new(payloads))));
    let baseline = memory_kib();
    reset_peak_memory();
    let elapsed = drain(&url, Compression::None, EVENTS).await;
    let sent = server.await.unwrap();
    report(
        "2k large unknown events (1000 members), plain",
        EVENTS,
        sent,
        elapsed,
        baseline,
    );
}

/// 2 000 full `GUILD_CREATE` payloads of ~700 KB each (1 000 members, 250 presences, 100
/// channels, 50 roles), decoded and dropped; memory must stay bounded.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_2k_full_guild_creates_plain() {
    const EVENTS: u64 = 2_000;
    let (listener, url) = listener().await;
    // One template, so that the server only patches the sequence instead of generating 700 KB.
    let template = payloads::guild_create(0, 1_000, 250, 100, 50);
    let payloads = (2..EVENTS + 2)
        .map(move |sequence| template.replacen(r#""s":0"#, &format!(r#""s":{sequence}"#), 1));
    let server = tokio::spawn(serve(listener, Frames::Text(Box::new(payloads))));
    let baseline = memory_kib();
    reset_peak_memory();
    let elapsed = drain(&url, Compression::None, EVENTS).await;
    let sent = server.await.unwrap();
    report(
        "2k full GUILD_CREATE (1000 members), plain, decoded",
        EVENTS,
        sent,
        elapsed,
        baseline,
    );
}

/// The same large payloads compressed with `zstd-stream`.
#[cfg(feature = "zstd-stream")]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_2k_large_unknown_events_zstd() {
    const EVENTS: u64 = 2_000;
    let mut compressor = StreamCompressor::new();
    let mut frames = vec![
        compressor.compress(HELLO.as_bytes()),
        compressor.compress(READY.as_bytes()),
    ];
    // Different member IDs per guild so that zstd cannot just reference the previous payload.
    let mut raw = 0;
    for sequence in 2..EVENTS + 2 {
        let payload = large_unknown_event(sequence, 1_000).replace(
            "1000000000000000",
            &format!("{:016}", 2_000_000_000_000_000 + sequence * 1_000),
        );
        raw += payload.len() as u64;
        frames.push(compressor.compress(payload.as_bytes()));
    }

    let (listener, url) = listener().await;
    let server = tokio::spawn(serve(listener, Frames::Binary(frames)));
    let baseline = memory_kib();
    reset_peak_memory();
    let elapsed = drain(&url, Compression::ZstdStream, EVENTS).await;
    let sent = server.await.unwrap();
    report(
        "2k large unknown events (1000 members), zstd-stream",
        EVENTS,
        sent,
        elapsed,
        baseline,
    );
    println!(
        "[stress]   zstd ratio: {raw} → {sent} bytes ({:.1}x); decompressed {:.1} MB/s",
        raw as f64 / sent as f64,
        raw as f64 / elapsed.as_secs_f64() / 1e6
    );
}

/// One million `MESSAGE_CREATE` frames compressed with `zlib-stream`.
#[cfg(feature = "zlib-stream")]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_1m_message_creates_zlib() {
    const EVENTS: u64 = 1_000_000;
    let mut compressor = ZlibStreamCompressor::new();
    let mut frames = vec![
        compressor.compress(HELLO.as_bytes()),
        compressor.compress(READY.as_bytes()),
    ];
    frames.extend(
        (2..EVENTS + 2).map(|sequence| compressor.compress(message_create(sequence).as_bytes())),
    );
    let raw: u64 = (2..EVENTS + 2)
        .map(|sequence| message_create(sequence).len() as u64)
        .sum();

    let (listener, url) = listener().await;
    let server = tokio::spawn(serve(listener, Frames::Binary(frames)));
    let baseline = memory_kib();
    reset_peak_memory();
    let elapsed = drain(&url, Compression::ZlibStream, EVENTS).await;
    let sent = server.await.unwrap();
    report(
        "1M MESSAGE_CREATE, zlib-stream",
        EVENTS,
        sent,
        elapsed,
        baseline,
    );
    println!(
        "[stress]   zlib ratio: {raw} → {sent} bytes ({:.1}x); decompressed {:.1} MB/s",
        raw as f64 / sent as f64,
        raw as f64 / elapsed.as_secs_f64() / 1e6
    );
}

/// 2 000 ~200 KB unknown events compressed with `zlib-stream`; memory must stay bounded.
#[cfg(feature = "zlib-stream")]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_2k_large_unknown_events_zlib() {
    const EVENTS: u64 = 2_000;
    let mut compressor = ZlibStreamCompressor::new();
    let mut frames = vec![
        compressor.compress(HELLO.as_bytes()),
        compressor.compress(READY.as_bytes()),
    ];
    // The same payloads as the zstd variant.
    let mut raw = 0;
    for sequence in 2..EVENTS + 2 {
        let payload = large_unknown_event(sequence, 1_000).replace(
            "1000000000000000",
            &format!("{:016}", 2_000_000_000_000_000 + sequence * 1_000),
        );
        raw += payload.len() as u64;
        frames.push(compressor.compress(payload.as_bytes()));
    }

    let (listener, url) = listener().await;
    let server = tokio::spawn(serve(listener, Frames::Binary(frames)));
    let baseline = memory_kib();
    reset_peak_memory();
    let elapsed = drain(&url, Compression::ZlibStream, EVENTS).await;
    let sent = server.await.unwrap();
    report(
        "2k large unknown events (1000 members), zlib-stream",
        EVENTS,
        sent,
        elapsed,
        baseline,
    );
    println!(
        "[stress]   zlib ratio: {raw} → {sent} bytes ({:.1}x); decompressed {:.1} MB/s",
        raw as f64 / sent as f64,
        raw as f64 / elapsed.as_secs_f64() / 1e6
    );
}

/// Serves `count` sessions at once, replaying the same precomputed `frames` (Hello, READY,
/// then the flood) to every connection; the flood starts once `go` turns true.
fn serve_many(
    listener: TcpListener,
    count: usize,
    frames: Arc<[Bytes]>,
    binary: bool,
    go: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut sessions = Vec::with_capacity(count);
        for _ in 0..count {
            let (stream, _) = timeout(STEP, listener.accept()).await.unwrap().unwrap();
            let (frames, mut go) = (Arc::clone(&frames), go.clone());
            sessions.push(tokio::spawn(async move {
                stream.set_nodelay(true).unwrap();
                let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();
                let message = |frame: &Bytes| {
                    if binary {
                        Message::binary(frame.clone())
                    } else {
                        Message::text(frame.clone())
                    }
                };
                ws.send(message(&frames[0])).await.unwrap();
                wait_for_identify(&mut ws).await;
                ws.send(message(&frames[1])).await.unwrap();
                go.wait_for(|go| *go).await.unwrap();
                for (index, frame) in frames[2..].iter().enumerate() {
                    ws.feed(message(frame)).await.unwrap();
                    if index % 64 == 63 {
                        ws.flush().await.unwrap();
                    }
                }
                ws.flush().await.unwrap();
                while let Ok(Some(Ok(_))) = timeout(STEP, ws.next()).await {}
            }));
        }
        for session in sessions {
            session.await.unwrap();
        }
    })
}

/// Runs `count` shards of one bot in this process (one task each, all identifying at once),
/// measures the time until all are ready and the memory they hold, then floods every shard
/// with `per_shard` dispatches.
async fn many_shards(name: &str, count: u32, per_shard: u64, compression: Compression) {
    let raw: Vec<String> = [HELLO.to_owned(), READY.to_owned()]
        .into_iter()
        .chain((2..per_shard + 2).map(message_create))
        .collect();
    let binary = compression != Compression::None;
    let frames: Arc<[Bytes]> = match compression {
        Compression::None => raw.into_iter().map(Bytes::from).collect(),
        #[cfg(feature = "zstd-stream")]
        Compression::ZstdStream => {
            let mut compressor = StreamCompressor::new();
            raw.iter()
                .map(|payload| Bytes::from(compressor.compress(payload.as_bytes())))
                .collect()
        }
        #[cfg(feature = "zlib-stream")]
        Compression::ZlibStream => {
            let mut compressor = ZlibStreamCompressor::new();
            raw.iter()
                .map(|payload| Bytes::from(compressor.compress(payload.as_bytes())))
                .collect()
        }
        _ => unreachable!("no other compression"),
    };
    let wire: u64 = frames[2..].iter().map(|frame| frame.len() as u64).sum();

    let baseline = memory_kib();
    reset_peak_memory();
    let (listener, url) = listener().await;
    let (go_tx, go_rx) = watch::channel(false);
    let server = serve_many(listener, count as usize, frames, binary, go_rx);

    // Every shard has its own bucket, so all of them identify at once.
    let config = Config::builder("stress-token", Intents::GUILD_MESSAGES)
        .gateway_url(url)
        .compression(compression)
        .identify_queue(Arc::new(LocalIdentifyQueue::new(count)))
        .build();
    let (ready_tx, mut ready_rx) = mpsc::unbounded_channel();
    let started = Instant::now();
    let mut tasks = Vec::new();
    for mut shard in create_shards(0..count, count, config) {
        let ready_tx = ready_tx.clone();
        tasks.push(tokio::spawn(async move {
            let mut received = 0;
            let mut finished = None;
            while received < per_shard {
                let event = timeout(STEP, shard.next_event())
                    .await
                    .expect("no event in time")
                    .expect("shard ended")
                    .expect("shard error");
                match event {
                    GatewayEvent::Dispatch(_, DispatchEvent::Ready(_)) => {
                        ready_tx.send(()).unwrap();
                    }
                    GatewayEvent::Dispatch(sequence, _) => {
                        received += 1;
                        assert_eq!(sequence, received + 1, "dispatch out of order");
                        if received == per_shard {
                            finished = Some(Instant::now());
                        }
                    }
                    _ => {}
                }
            }
            shard.close().await;
            finished.unwrap()
        }));
    }
    for _ in 0..count {
        timeout(STEP, ready_rx.recv()).await.unwrap().unwrap();
    }
    let connected = started.elapsed();
    let idle = memory_kib();

    let flood = Instant::now();
    go_tx.send_replace(true);
    let mut last = flood;
    for task in tasks {
        last = last.max(task.await.unwrap());
    }
    let elapsed = last - flood;
    server.await.unwrap();

    let events = u64::from(count) * per_shard;
    let per_shard_kib = match (baseline, idle) {
        (Some((before, _)), Some((after, _))) => {
            format!(
                "{:.0}",
                (after.saturating_sub(before)) as f64 / f64::from(count)
            )
        }
        _ => "n/a".to_owned(),
    };
    println!(
        "[stress] {name}: {count} shards ready in {connected:.2?}, {per_shard_kib} KiB per \
         connected shard (client + mock server side)"
    );
    report(name, events, wire * u64::from(count), elapsed, baseline);
}

/// 1 000 shards in one process, then 1 000 `MESSAGE_CREATE`s to each (1M events).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn thousand_shards_plain() {
    many_shards(
        "1000 shards × 1000 MESSAGE_CREATE, plain",
        1_000,
        1_000,
        Compression::None,
    )
    .await;
}

/// The same with `zstd-stream`: every shard keeps its own decompression context.
#[cfg(feature = "zstd-stream")]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn thousand_shards_zstd() {
    many_shards(
        "1000 shards × 1000 MESSAGE_CREATE, zstd-stream",
        1_000,
        1_000,
        Compression::ZstdStream,
    )
    .await;
}

/// The send-limit server: Hello, READY, then a `MESSAGE_CREATE` every 100 ms (recording each
/// send time in `sent_at`) while it records when each of `commands` member requests arrives.
fn send_limit_server(
    listener: TcpListener,
    sent_at: Arc<Mutex<Vec<Instant>>>,
    commands: usize,
) -> tokio::task::JoinHandle<(Instant, Vec<Instant>)> {
    tokio::spawn(async move {
        let (stream, _) = timeout(STEP, listener.accept()).await.unwrap().unwrap();
        let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();
        ws.send(Message::text(HELLO.to_owned())).await.unwrap();
        wait_for_identify(&mut ws).await;
        ws.send(Message::text(READY.to_owned())).await.unwrap();
        let ready_at = Instant::now();
        let (mut sink, mut stream) = ws.split();
        let ticker = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            for sequence in 2.. {
                interval.tick().await;
                sent_at.lock().unwrap().push(Instant::now());
                if sink
                    .send(Message::text(message_create(sequence)))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
        let mut arrived = Vec::new();
        while arrived.len() < commands {
            let message = timeout(Duration::from_secs(90), stream.next())
                .await
                .expect("commands stopped coming")
                .unwrap()
                .unwrap();
            if message
                .as_text()
                .is_some_and(|text| text.contains(r#""op":8"#))
            {
                arrived.push(Instant::now());
            }
        }
        ticker.abort();
        (ready_at, arrived)
    })
}

/// The same with `zlib-stream` (32 KiB window per shard).
#[cfg(feature = "zlib-stream")]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn thousand_shards_zlib() {
    many_shards(
        "1000 shards × 1000 MESSAGE_CREATE, zlib-stream",
        1_000,
        1_000,
        Compression::ZlibStream,
    )
    .await;
}

/// Longer streams (10 000 events, 8.8 MB per shard): how far each shard's decompression memory
/// grows once its window has filled.
#[cfg(feature = "zstd-stream")]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn hundred_shards_long_streams_zstd() {
    many_shards(
        "100 shards × 10000 MESSAGE_CREATE, zstd-stream",
        100,
        10_000,
        Compression::ZstdStream,
    )
    .await;
}

/// Longer streams with `zlib-stream`.
#[cfg(feature = "zlib-stream")]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn hundred_shards_long_streams_zlib() {
    many_shards(
        "100 shards × 10000 MESSAGE_CREATE, zlib-stream",
        100,
        10_000,
        Compression::ZlibStream,
    )
    .await;
}

/// Longer streams without compression.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn hundred_shards_long_streams_plain() {
    many_shards(
        "100 shards × 10000 MESSAGE_CREATE, plain",
        100,
        10_000,
        Compression::None,
    )
    .await;
}

/// The send limit in real time (D-142, takes about a minute): 230 queued commands — two
/// windows' worth — while the server keeps sending events every 100 ms. Commands must never
/// exceed 115 in any 61 s, and events must keep flowing while commands wait.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored (about 62 s)"]
async fn send_limit_in_real_time() {
    const COMMANDS: u32 = 230;
    const LIMIT: usize = 115;
    /// The shard's window; arrivals are checked against Discord's 60 s (as the server sees
    /// them, so jitter of a millisecond cannot fail the check).
    const WINDOW: Duration = Duration::from_secs(61);
    const DISCORD_WINDOW: Duration = Duration::from_secs(60);

    let (listener, url) = listener().await;
    let sent_at: Arc<Mutex<Vec<Instant>>> = Arc::default();
    let server = send_limit_server(listener, Arc::clone(&sent_at), COMMANDS as usize);

    let mut shard = Shard::with_config(
        ShardId::ONE,
        Config::builder("stress-token", Intents::GUILD_MESSAGES)
            .gateway_url(url)
            .compression(Compression::None)
            .build(),
    );
    let sender: MessageSender = shard.sender();
    for n in 1..=COMMANDS {
        sender
            .command(&RequestGuildMembers::query(GuildId::new(2), "", n))
            .unwrap();
    }
    let mut received_at = Vec::new();
    tokio::pin!(server);
    let (ready_at, commands) = loop {
        tokio::select! {
            result = &mut server => break result.unwrap(),
            event = shard.next_event() => {
                if let Some(Ok(GatewayEvent::Dispatch(sequence, DispatchEvent::MessageCreate(_)))) = event {
                    received_at.push((sequence, Instant::now()));
                }
            }
        }
    };

    let max_in_window = (0..commands.len())
        .map(|i| {
            commands[i..]
                .iter()
                .take_while(|&&at| at < commands[i] + DISCORD_WINDOW)
                .count()
        })
        .max()
        .unwrap();
    let first_batch = commands[LIMIT - 1] - ready_at;
    let second_start = commands[LIMIT] - ready_at;
    let sent_at = sent_at.lock().unwrap();
    let mut lags: Vec<Duration> = received_at
        .iter()
        .map(|&(sequence, at)| at - sent_at[usize::try_from(sequence - 2).unwrap()])
        .collect();
    lags.sort_unstable();
    println!(
        "[stress] send limit: {COMMANDS} commands, max {max_in_window} in any 60 s; first {LIMIT} \
         within {first_batch:.2?} of READY, command {} at {second_start:.2?}; {} events received \
         meanwhile, lag p50 {:.2?}, p99 {:.2?}, max {:.2?}",
        LIMIT + 1,
        lags.len(),
        lags[lags.len() / 2],
        lags[lags.len() * 99 / 100],
        lags[lags.len() - 1],
    );
    assert!(max_in_window <= LIMIT);
    assert!(first_batch < Duration::from_secs(1));
    assert!(second_start + Duration::from_millis(5) >= WINDOW);
    assert!(
        lags[lags.len() - 1] < Duration::from_millis(100),
        "events were held back"
    );
}

/// Sizes of the types and futures a bot holds per shard.
#[test]
#[ignore = "prints numbers; run with --ignored --nocapture"]
fn type_sizes() {
    let mut shard = Shard::new(ShardId::ONE, "stress-token", Intents::GUILDS);
    let next_event = core::mem::size_of_val(&shard.next_event());
    println!(
        "[stress] sizes: Shard {} B, MessageSender {} B, Config {} B, next_event future {} B",
        core::mem::size_of::<Shard>(),
        core::mem::size_of::<MessageSender>(),
        core::mem::size_of::<Config>(),
        next_event,
    );
}
