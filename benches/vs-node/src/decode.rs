//! Scenario 1: decoding gateway payloads into the library's types.

use std::{fs, hint::black_box, path::Path};

use djs_model::gateway::{DispatchEvent, GatewayEvent, GatewayEventDeserializer, payload};
use serde::de::DeserializeSeed;
use serde_json::json;

use crate::{
    Options,
    metrics::measure,
    payloads::{GuildShape, guild_create, message_create},
};

/// The fixture files, written by `fixtures` and read by both `decode` runners.
const FILES: [&str; 2] = ["message_create.json", "guild_create.json"];

/// Writes the decode fixtures to `--dir`: a typical `MESSAGE_CREATE` and the `GUILD_CREATE` of
/// a 1000-member guild that contains its channel and author.
pub(crate) fn write_fixtures(options: &Options) {
    let dir = Path::new(options.required("dir"));
    fs::create_dir_all(dir).expect("the fixture directory can be created");
    fs::write(dir.join(FILES[0]), message_create(2, 7)).expect("fixture written");
    fs::write(
        dir.join(FILES[1]),
        guild_create(1, 0, &GuildShape::default()),
    )
    .expect("fixture written");
}

fn decode(json: &str) -> GatewayEvent {
    let seed = GatewayEventDeserializer::from_json(json).expect("a gateway payload");
    seed.deserialize(&mut serde_json::Deserializer::from_str(json))
        .expect("the payload decodes")
}

/// Times decoding of each fixture in `--dir` and prints one result line per fixture.
pub(crate) fn run(options: &Options, allocator: &str) {
    let dir = Path::new(options.required("dir"));
    for file in FILES {
        let json = fs::read_to_string(dir.join(file)).expect("run `fixtures` first");
        // Check once that the payload decodes into the expected type, not `Unknown`.
        match decode(&json) {
            GatewayEvent::Dispatch(_, DispatchEvent::MessageCreate(message)) => {
                assert!(message.member.is_some());
            }
            GatewayEvent::Dispatch(_, DispatchEvent::GuildCreate(guild)) => {
                let payload::GuildCreate::Available(guild) = *guild else {
                    panic!("an available guild")
                };
                assert_eq!(guild.members.len(), 1000);
            }
            other => panic!("unexpected event {other:?}"),
        }
        let timing = measure(|| drop(black_box(decode(black_box(&json)))));
        println!(
            "{}",
            json!({
                "scenario": "decode",
                "payload": file.trim_end_matches(".json").to_uppercase(),
                "bytes": json.len(),
                "library": "djs-rs",
                "variant": "djs-model typed decode",
                "allocator": allocator,
                "median_ns": timing.median_ns,
                "min_ns": timing.min_ns,
            })
        );
    }
}
