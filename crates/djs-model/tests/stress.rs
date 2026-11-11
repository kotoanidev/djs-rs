//! Extreme-load and robustness tests of gateway event decoding (D-041).
//!
//! The load tests are ignored by default because they take seconds and saturate the machine.
//! Run them in release mode and read the printed numbers:
//!
//! ```text
//! cargo test -p djs-model --release --test stress -- --ignored --nocapture --test-threads 1
//! ```
//!
//! The robustness tests (generated payloads, truncated/corrupted input, deep nesting) are not
//! ignored and run with the normal test suite.

#![expect(
    clippy::cast_precision_loss,
    reason = "throughput numbers are only printed"
)]

use std::{
    thread,
    time::{Duration, Instant},
};

use djs_model::gateway::{DispatchEvent, GatewayEvent, GatewayEventDeserializer, payload};
use serde::de::DeserializeSeed;

mod common;

use common::payloads::{self, GUILD};

/// Decodes one raw gateway payload; `None` if the top-level scan fails, `Err` if decoding does.
fn decode(json: &str) -> Option<Result<GatewayEvent, serde_json::Error>> {
    let seed = GatewayEventDeserializer::from_json(json)?;
    let mut deserializer = serde_json::Deserializer::from_str(json);
    Some(
        seed.deserialize(&mut deserializer)
            .and_then(|event| deserializer.end().map(|()| event)),
    )
}

fn dispatch(json: &str) -> DispatchEvent {
    match decode(json) {
        Some(Ok(GatewayEvent::Dispatch(_, event))) => event,
        other => panic!("expected a dispatch, got {other:?}"),
    }
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

fn memory_report(baseline: Option<(u64, u64)>) -> String {
    match (baseline, memory_kib()) {
        (Some((before, _)), Some((after, peak))) => format!(
            "rss {} → {} MiB, peak {} MiB",
            before / 1024,
            after / 1024,
            peak / 1024
        ),
        _ => "memory n/a".to_owned(),
    }
}

fn report(name: &str, events: u64, bytes: u64, elapsed: Duration, baseline: Option<(u64, u64)>) {
    let seconds = elapsed.as_secs_f64();
    println!(
        "[stress] {name}: {events} events in {elapsed:.2?} = {:.0} events/s, {:.1} MB/s; {}",
        events as f64 / seconds,
        bytes as f64 / seconds / 1e6,
        memory_report(baseline),
    );
}

/// 1 000 variants of every hot event (different IDs, users and sequences).
fn hot_event_corpus() -> Vec<String> {
    (0..1000)
        .flat_map(payloads::hot_events)
        .map(|(_, json)| json)
        .collect()
}

#[test]
fn generated_payloads_decode() {
    for (name, json) in payloads::hot_events(7) {
        let event = dispatch(&json);
        assert_eq!(event.name(), name);
        assert!(!matches!(event, DispatchEvent::Unknown(_)), "{name}");
    }
    let DispatchEvent::GuildCreate(guild) = dispatch(&payloads::guild_create(1, 100, 50, 20, 30))
    else {
        panic!("expected GUILD_CREATE");
    };
    let payload::GuildCreate::Available(guild) = *guild else {
        panic!("expected an available guild");
    };
    assert_eq!(guild.id.get(), GUILD);
    assert_eq!(
        (
            guild.members.len(),
            guild.presences.len(),
            guild.channels.len(),
            guild.roles.len(),
            guild.emojis.len(),
            guild.voice_states.len(),
        ),
        (100, 50, 20, 30, 50, 20)
    );
    assert_eq!(guild.member_count, 100);
}

/// Every prefix of real payloads, and copies with one byte replaced, must decode or fail —
/// never panic.
#[test]
fn truncated_and_corrupted_payloads_never_panic() {
    let mut corpus: Vec<String> = payloads::hot_events(3)
        .into_iter()
        // The fixture ends with a newline; without it, the last prefix would be complete.
        .map(|(_, json)| json.trim_end().to_owned())
        .collect();
    corpus.push(payloads::guild_create(1, 3, 3, 2, 2));
    let mut outcomes = [0_u32; 3];
    for json in &corpus {
        for end in 0..json.len() {
            if let Some(prefix) = json.get(..end) {
                let outcome = match decode(prefix) {
                    None => 0,
                    Some(Err(_)) => 1,
                    Some(Ok(_)) => 2,
                };
                outcomes[outcome] += 1;
            }
        }
    }
    // A strict prefix is never a complete payload.
    assert_eq!(outcomes[2], 0, "{outcomes:?}");

    for json in &corpus[..3] {
        let bytes = json.as_bytes();
        for at in 0..bytes.len() {
            for replacement in *b"\"}x" {
                let mut corrupted = bytes.to_vec();
                corrupted[at] = replacement;
                if let Ok(text) = std::str::from_utf8(&corrupted) {
                    let _ = decode(text);
                }
            }
        }
    }
}

/// Deeply nested JSON never overflows the stack: skipped data (unknown events and fields,
/// also through the extras adapter) is skipped iteratively by `serde_json` and decodes, while
/// arbitrary-JSON values hit `serde_json`'s recursion limit.
#[test]
fn deep_nesting_does_not_overflow_the_stack() {
    let deep = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
    let skipped = [
        // An unknown event, whose data is skipped.
        payloads::dispatch("FUTURE_EVENT", 1, &deep),
        // An unknown field of a known event.
        payloads::dispatch(
            "MESSAGE_DELETE",
            1,
            &format!(r#"{{"id":"1","channel_id":"2","future":{deep}}}"#),
        ),
        // An unknown field of an "object + extras" event, passed on to the object.
        payloads::dispatch(
            "THREAD_MEMBER_UPDATE",
            1,
            &format!(
                r#"{{"guild_id":"2","join_timestamp":"2026-11-11T12:00:00+00:00","flags":0,"future":{deep}}}"#
            ),
        ),
    ];
    for input in &skipped {
        let event = decode(input).unwrap().unwrap();
        assert!(matches!(event, GatewayEvent::Dispatch(1, _)));
    }

    // An arbitrary-JSON value (audit log change) is built recursively, so it is limited.
    let value = payloads::dispatch(
        "GUILD_AUDIT_LOG_ENTRY_CREATE",
        1,
        &format!(
            r#"{{"id":"1","action_type":1,"target_id":null,"user_id":null,"guild_id":"2","changes":[{{"key":"name","new_value":{deep}}}]}}"#
        ),
    );
    let error = decode(&value).unwrap().unwrap_err();
    assert!(error.to_string().contains("recursion limit"), "{error}");
}

/// Decodes the hot-event corpus `rounds` times on this thread; returns (events, bytes).
fn decode_corpus(corpus: &[String], rounds: u32) -> (u64, u64) {
    let mut events = 0;
    let mut bytes = 0;
    for _ in 0..rounds {
        for json in corpus {
            let event = decode(json).unwrap().unwrap();
            std::hint::black_box(event);
            events += 1;
            bytes += json.len() as u64;
        }
    }
    (events, bytes)
}

#[test]
#[ignore = "extreme load; run in release mode with --ignored --nocapture"]
fn flood_hot_events_single_thread() {
    let corpus = hot_event_corpus();
    reset_peak_memory();
    let baseline = memory_kib();
    let start = Instant::now();
    let (events, bytes) = decode_corpus(&corpus, 200);
    report(
        "2.2M hot events (11 kinds), 1 thread",
        events,
        bytes,
        start.elapsed(),
        baseline,
    );
    // Decoded events are dropped immediately: memory must not grow with the event count.
    if let (Some((before, _)), Some((after, _))) = (baseline, memory_kib()) {
        assert!(
            after <= before + 16 * 1024,
            "rss grew {before} → {after} KiB"
        );
    }
}

#[test]
#[ignore = "extreme load; run in release mode with --ignored --nocapture"]
fn flood_hot_events_all_threads() {
    let corpus = hot_event_corpus();
    let threads = thread::available_parallelism().map_or(4, usize::from);
    reset_peak_memory();
    let baseline = memory_kib();
    let start = Instant::now();
    let (events, bytes) = thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| scope.spawn(|| decode_corpus(&corpus, 100)))
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .fold((0, 0), |(events, bytes), (e, b)| (events + e, bytes + b))
    });
    report(
        &format!("{events} hot events, {threads} threads"),
        events,
        bytes,
        start.elapsed(),
        baseline,
    );
}

#[test]
#[ignore = "extreme load; run in release mode with --ignored --nocapture"]
fn huge_guild_create() {
    // A 100k-member guild with 25k online members (presences), 500 channels and 250 roles.
    let json = payloads::guild_create(1, 100_000, 25_000, 500, 250);
    let size = json.len() as u64;
    for round in 1..=3 {
        reset_peak_memory();
        let baseline = memory_kib();
        let start = Instant::now();
        let event = dispatch(&json);
        let elapsed = start.elapsed();
        let DispatchEvent::GuildCreate(guild) = &event else {
            panic!("expected GUILD_CREATE");
        };
        let payload::GuildCreate::Available(guild) = &**guild else {
            panic!("expected an available guild");
        };
        assert_eq!(guild.members.len(), 100_000);
        println!(
            "[stress] GUILD_CREATE 100k members / 25k presences ({:.1} MB), round {round}: \
             decoded in {elapsed:.2?} = {:.0} MB/s; {} (decoded value alive)",
            size as f64 / 1e6,
            size as f64 / elapsed.as_secs_f64() / 1e6,
            memory_report(baseline),
        );
        drop(event);
    }
}

#[test]
#[ignore = "extreme load; run in release mode with --ignored --nocapture"]
fn many_guild_creates_kept_alive() {
    // Startup of a bot in 2 500 guilds of 200 members each, keeping every decoded guild alive
    // (as a cache would): how much memory does the decoded model take compared to the JSON?
    let guilds = 2_500;
    let payloads: Vec<String> = (0..guilds)
        .map(|i| payloads::guild_create(i, 200, 50, 30, 20))
        .collect();
    let bytes: u64 = payloads.iter().map(|json| json.len() as u64).sum();
    reset_peak_memory();
    let baseline = memory_kib();
    let start = Instant::now();
    let decoded: Vec<DispatchEvent> = payloads.iter().map(|json| dispatch(json)).collect();
    let elapsed = start.elapsed();
    report(
        "2.5k GUILD_CREATE (200 members), all kept",
        guilds,
        bytes,
        elapsed,
        baseline,
    );
    if let (Some((before, _)), Some((after, _))) = (baseline, memory_kib()) {
        println!(
            "[stress]   decoded model: {:.1} MiB for {:.1} MiB of JSON ({:.2}×)",
            (after - before) as f64 / 1024.0,
            bytes as f64 / 1_048_576.0,
            (after - before) as f64 * 1024.0 / bytes as f64,
        );
    }
    assert_eq!(decoded.len(), 2_500);
}
