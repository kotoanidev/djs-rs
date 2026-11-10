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

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

const MESSAGE_CREATE: &str = include_str!("../tests/fixtures/message_create.json");
const READY: &str = include_str!("../tests/fixtures/ready.json");

/// A large dispatch the library does not decode (e.g. `GUILD_CREATE` with 1000 members),
/// which must be skipped cheaply.
fn large_unknown_event() -> String {
    let members: Vec<String> = (0..1000)
        .map(|i| {
            format!(
                r#"{{"user":{{"id":"{}","username":"member{i}","discriminator":"0","global_name":null,"avatar":null}},"roles":["1","2","3"],"joined_at":"2024-01-01T00:00:00.000000+00:00","deaf":false,"mute":false,"flags":0}}"#,
                1_000_000_000_000_000_000_u64 + i
            )
        })
        .collect();
    format!(
        r#"{{"t":"GUILD_CREATE","s":5,"op":0,"d":{{"id":"1","name":"big","members":[{}]}}}}"#,
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
