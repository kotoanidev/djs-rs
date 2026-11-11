//! Benchmarks of djs-model hot paths. Run with `cargo bench -p djs-model`.
//!
//! The global `AllocProfiler` reports allocations per operation next to timings.

use divan::{Bencher, black_box, counter::BytesCount};
use djs_model::{
    gateway::{GatewayEvent, GatewayEventDeserializer},
    id::UserId,
    util::{ImageHash, Timestamp},
};
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

/// A large dispatch the library does not know (shaped like a 1000-member `GUILD_CREATE`, but
/// with an unknown name), which must be skipped cheaply.
fn large_unknown_event() -> String {
    let members: Vec<String> = (0..1000)
        .map(|i| {
            format!(
                r#"{{"user":{{"id":"{}","username":"member{i}","discriminator":"0","global_name":null,"avatar":null}},"roles":["1","2","3"],"joined_at":"2024-01-01T00:00:00.000000+00:00","deaf":false,"mute":false,"flags":0}}"#,
                1_000_000_000_000_000_000_u64 + i
            )
        })
        .collect();
    // Same length as the Phase 1 payload (`GUILD_CREATE` was not decoded then), for A/B runs.
    format!(
        r#"{{"t":"FUTURE_EVENT","s":5,"op":0,"d":{{"id":"1","name":"big","members":[{}]}}}}"#,
        members.join(",")
    )
}

fn decode(json: &str) -> GatewayEvent {
    let seed = GatewayEventDeserializer::from_json(json).unwrap();
    seed.deserialize(&mut serde_json::Deserializer::from_str(json))
        .unwrap()
}

mod gateway {
    use super::{
        Bencher, BytesCount, GatewayEventDeserializer, MESSAGE_CREATE, READY, black_box, decode,
        large_unknown_event,
    };

    #[divan::bench]
    fn message_create(bencher: Bencher<'_, '_>) {
        bencher
            .counter(BytesCount::of_str(MESSAGE_CREATE))
            .bench(|| decode(black_box(MESSAGE_CREATE)));
    }

    #[divan::bench]
    fn ready(bencher: Bencher<'_, '_>) {
        bencher
            .counter(BytesCount::of_str(READY))
            .bench(|| decode(black_box(READY)));
    }

    #[divan::bench]
    fn skip_large_unknown_event(bencher: Bencher<'_, '_>) {
        let json = large_unknown_event();
        bencher
            .counter(BytesCount::of_str(&json))
            .bench(|| decode(black_box(&json)));
    }

    /// The top-level `op`/`s`/`t` scan alone.
    #[divan::bench]
    fn scan_only(bencher: Bencher<'_, '_>) {
        bencher.bench(|| GatewayEventDeserializer::from_json(black_box(MESSAGE_CREATE)));
    }

    /// Baseline: what a naive implementation does — parse into a generic JSON value first.
    #[divan::bench]
    fn baseline_serde_json_value(bencher: Bencher<'_, '_>) {
        bencher
            .counter(BytesCount::of_str(MESSAGE_CREATE))
            .bench(|| {
                serde_json::from_str::<serde_json::Value>(black_box(MESSAGE_CREATE)).unwrap()
            });
    }
}

/// The frequent events a bot receives (`tests/common/payloads.rs`).
const HOT_EVENTS: &[&str] = &[
    "MESSAGE_CREATE",
    "MESSAGE_UPDATE",
    "INTERACTION_CREATE",
    "PRESENCE_UPDATE",
    "TYPING_START",
    "MESSAGE_REACTION_ADD",
    "MESSAGE_DELETE",
    "GUILD_MEMBER_ADD",
    "GUILD_MEMBER_UPDATE",
    "VOICE_STATE_UPDATE",
    "CHANNEL_UPDATE",
];

mod events {
    use super::{Bencher, BytesCount, HOT_EVENTS, black_box, decode, payloads};

    fn payload(name: &str) -> String {
        payloads::hot_events(1)
            .into_iter()
            .find(|(event, _)| *event == name)
            .unwrap()
            .1
    }

    /// Every hot event, decoded into its typed payload.
    #[divan::bench(args = HOT_EVENTS)]
    fn hot(bencher: Bencher<'_, '_>, name: &str) {
        let json = payload(name);
        bencher
            .counter(BytesCount::of_str(&json))
            .bench(|| decode(black_box(&json)));
    }

    /// A full `GUILD_CREATE` (1 000 members, 250 presences, 100 channels, 50 roles).
    #[divan::bench]
    fn guild_create_1000_members(bencher: Bencher<'_, '_>) {
        let json = payloads::guild_create(1, 1000, 250, 100, 50);
        bencher
            .counter(BytesCount::of_str(&json))
            .bench(|| decode(black_box(&json)));
    }

    /// The same `GUILD_CREATE` into a `serde_json::Value`, for comparison.
    #[divan::bench]
    fn guild_create_baseline_serde_json_value(bencher: Bencher<'_, '_>) {
        let json = payloads::guild_create(1, 1000, 250, 100, 50);
        bencher
            .counter(BytesCount::of_str(&json))
            .bench(|| serde_json::from_str::<serde_json::Value>(black_box(&json)).unwrap());
    }

    /// An unavailable guild (outage), recognized after the guild decode fails.
    #[divan::bench]
    fn guild_create_unavailable(bencher: Bencher<'_, '_>) {
        let json = r#"{"t":"GUILD_CREATE","s":2,"op":0,"d":{"id":"1400000000000000002","unavailable":true}}"#;
        bencher.bench(|| decode(black_box(json)));
    }
}

mod commands {
    use djs_model::{
        gateway::{
            Activity, ActivityType, Intents, ShardId, Status,
            payload::{
                CommandPayload, GatewayCapabilities, Identify, IdentifyProperties, UpdatePresence,
            },
        },
        util::Timestamp,
    };

    use super::black_box;

    fn presence() -> UpdatePresence {
        let mut presence = UpdatePresence::new(
            Status::Online,
            [Activity::new(ActivityType::Custom, "Custom Status")],
        );
        presence.since = Timestamp::from_unix_millis(1_794_398_400_000);
        presence
    }

    #[divan::bench]
    fn identify_to_json(bencher: divan::Bencher<'_, '_>) {
        let identify = Identify {
            token: "not-a-real-token-only-for-benchmarking-0123456789".into(),
            properties: IdentifyProperties::new("djs-rs"),
            large_threshold: Some(250),
            shard: Some(ShardId::new(0, 1)),
            intents: Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT,
            presence: Some(presence()),
            capabilities: GatewayCapabilities::empty(),
        };
        bencher
            .bench(|| serde_json::to_string(&CommandPayload::new(black_box(&identify))).unwrap());
    }

    #[divan::bench]
    fn update_presence_to_json(bencher: divan::Bencher<'_, '_>) {
        let presence = presence();
        bencher
            .bench(|| serde_json::to_string(&CommandPayload::new(black_box(&presence))).unwrap());
    }
}

mod values {
    use super::{ImageHash, Timestamp, UserId, black_box};

    #[divan::bench]
    fn id_parse() -> UserId {
        black_box("80351110224678912").parse().unwrap()
    }

    #[divan::bench]
    fn id_to_json() -> String {
        serde_json::to_string(&black_box(UserId::new(80_351_110_224_678_912))).unwrap()
    }

    #[divan::bench]
    fn timestamp_parse() -> Timestamp {
        black_box("2026-11-10T19:42:07.123000+00:00")
            .parse()
            .unwrap()
    }

    #[divan::bench]
    fn timestamp_format() -> String {
        black_box(Timestamp::from_unix_micros(1_794_339_727_123_000)).to_string()
    }

    #[divan::bench]
    fn image_hash_parse() -> ImageHash {
        black_box("a_1269e74af4df7417b13759eae50c83dc")
            .parse()
            .unwrap()
    }
}
