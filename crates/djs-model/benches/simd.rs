//! Phase 4, step 5 (D-013, D-143): does `simd-json` decode gateway payloads faster than
//! `serde_json`? Run with `cargo bench -p djs-model --bench simd`.
//!
//! Both sides decode with the same `GatewayEventDeserializer` seed. The `simd_json` side
//! includes copying the payload into a reused mutable buffer (simd-json parses in place, and
//! text frames arrive in an immutable buffer) and reuses simd-json's internal buffers.

use divan::{Bencher, black_box, counter::BytesCount};
use djs_model::gateway::{GatewayEvent, GatewayEventDeserializer};
use serde::de::DeserializeSeed;

#[allow(dead_code, reason = "shared with the tests, which use other helpers")]
#[path = "../tests/common/payloads.rs"]
mod payloads;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

const MESSAGE_CREATE: &str = include_str!("../tests/fixtures/message_create.json");
const READY: &str = include_str!("../tests/fixtures/ready.json");

const CASES: &[&str] = &[
    "MESSAGE_CREATE (fixture)",
    "MESSAGE_CREATE (2000 chars, 3 embeds)",
    "MESSAGE_UPDATE",
    "MESSAGE_DELETE",
    "MESSAGE_REACTION_ADD",
    "INTERACTION_CREATE",
    "PRESENCE_UPDATE",
    "TYPING_START",
    "GUILD_MEMBER_ADD",
    "GUILD_MEMBER_UPDATE",
    "VOICE_STATE_UPDATE",
    "CHANNEL_UPDATE",
    "READY (fixture)",
    "GUILD_CREATE (1000 members)",
    "unknown 200 KB event (skipped)",
];

/// A long message: 2 000 characters of content (with escapes and non-ASCII text) and three
/// embeds with 1 000-character descriptions.
fn long_message() -> String {
    let mut payload: serde_json::Value = serde_json::from_str(MESSAGE_CREATE).unwrap();
    let text = "Zażółć gęślą jaźń — \"quoted\" text\nwith lines and emoji 🦀. ".repeat(40);
    let content: String = text.chars().take(2000).collect();
    let description: String = text.chars().take(1000).collect();
    payload["d"]["content"] = content.into();
    payload["d"]["embeds"] = (0..3)
        .map(|i| {
            serde_json::json!({
                "type": "rich", "title": format!("Embed {i}"), "description": description,
                "color": 5_814_783,
                "fields": [{"name": "Field", "value": "Value", "inline": true}],
                "footer": {"text": "footer text"}
            })
        })
        .collect();
    payload.to_string()
}

fn payload(case: &str) -> String {
    match case {
        "MESSAGE_CREATE (fixture)" => MESSAGE_CREATE.to_owned(),
        "READY (fixture)" => READY.to_owned(),
        "MESSAGE_CREATE (2000 chars, 3 embeds)" => long_message(),
        "GUILD_CREATE (1000 members)" => payloads::guild_create(1, 1000, 250, 100, 50),
        "unknown 200 KB event (skipped)" => {
            let members: Vec<String> = (0..1000)
                .map(|i| {
                    format!(
                        r#"{{"user":{{"id":"{}","username":"member{i}","discriminator":"0","global_name":null,"avatar":null}},"roles":["1","2","3"],"joined_at":"2024-01-01T00:00:00.000000+00:00","deaf":false,"mute":false,"flags":0}}"#,
                        1_000_000_000_000_000_000_u64 + i
                    )
                })
                .collect();
            format!(
                r#"{{"t":"FUTURE_EVENT","s":5,"op":0,"d":{{"id":"1","name":"big","members":[{}]}}}}"#,
                members.join(",")
            )
        }
        name => {
            payloads::hot_events(1)
                .into_iter()
                .find(|(event, _)| *event == name)
                .unwrap_or_else(|| panic!("no payload for {name}"))
                .1
        }
    }
}

#[divan::bench(args = CASES)]
fn serde_json(bencher: Bencher<'_, '_>, case: &str) {
    let json = payload(case);
    bencher
        .counter(BytesCount::of_str(&json))
        .bench(|| -> GatewayEvent {
            let json = black_box(json.as_str());
            let seed = GatewayEventDeserializer::from_json(json).unwrap();
            seed.deserialize(&mut serde_json::Deserializer::from_str(json))
                .unwrap()
        });
}

#[divan::bench(args = CASES)]
fn simd_json(bencher: Bencher<'_, '_>, case: &str) {
    let json = payload(case);
    let mut input = Vec::with_capacity(json.len());
    let mut buffers = simd_json::Buffers::new(json.len());
    bencher
        .counter(BytesCount::of_str(&json))
        .bench_local(|| -> GatewayEvent {
            let json = black_box(json.as_str());
            let seed = GatewayEventDeserializer::from_json(json).unwrap();
            input.clear();
            input.extend_from_slice(json.as_bytes());
            let mut deserializer =
                simd_json::Deserializer::from_slice_with_buffers(&mut input, &mut buffers).unwrap();
            seed.deserialize(&mut deserializer).unwrap()
        });
}
