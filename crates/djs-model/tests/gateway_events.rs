//! Decodes complete, realistic gateway payloads — including many fields that djs-model does not
//! model yet — to make sure deserialization is forward compatible.

use djs_model::{
    gateway::{DispatchEvent, GatewayEvent, GatewayEventDeserializer, ShardId},
    message::MessageType,
};
use serde::de::DeserializeSeed;

fn decode(json: &str) -> GatewayEvent {
    let seed = GatewayEventDeserializer::from_json(json).expect("top-level scan failed");
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let event = seed
        .deserialize(&mut deserializer)
        .expect("deserialization failed");
    deserializer.end().expect("trailing characters");
    event
}

#[test]
fn ready() {
    let GatewayEvent::Dispatch(sequence, DispatchEvent::Ready(ready)) =
        decode(include_str!("fixtures/ready.json"))
    else {
        panic!("expected a READY dispatch");
    };
    assert_eq!(sequence, 1);
    assert_eq!(ready.version, 10);
    assert_eq!(ready.user.username, "djs-test-bot");
    assert_eq!(ready.user.discriminator, 4821);
    assert!(ready.user.bot);
    assert!(ready.user.avatar.unwrap().is_animated());
    assert_eq!(ready.guilds.len(), 2);
    assert!(ready.guilds.iter().all(|guild| guild.unavailable));
    assert_eq!(ready.session_id, "0123456789abcdef0123456789abcdef");
    assert_eq!(
        ready.resume_gateway_url,
        "wss://gateway-us-east1-b.discord.gg"
    );
    assert_eq!(ready.shard, Some(ShardId::ONE));
    assert_eq!(ready.application.id.get(), 1_437_000_000_000_000_001);
}

#[test]
fn message_create() {
    let GatewayEvent::Dispatch(sequence, DispatchEvent::MessageCreate(message)) =
        decode(include_str!("fixtures/message_create.json"))
    else {
        panic!("expected a MESSAGE_CREATE dispatch");
    };
    assert_eq!(sequence, 42);
    assert_eq!(message.kind, MessageType::Reply);
    assert!(message.content.starts_with("!ping — zażółć gęślą jaźń 🦀"));
    assert_eq!(message.author.display_name(), "Nelly");
    assert_eq!(message.author.tag(), "nelly");
    assert_eq!(
        message.guild_id.map(djs_model::id::Id::get),
        Some(1_437_000_000_000_000_100)
    );
    assert_eq!(message.timestamp.as_unix_millis(), 1_794_339_727_123);
    assert!(message.edited_timestamp.is_none());
}

#[test]
fn message_round_trips_through_serialize() {
    let GatewayEvent::Dispatch(_, DispatchEvent::MessageCreate(message)) =
        decode(include_str!("fixtures/message_create.json"))
    else {
        panic!("expected a MESSAGE_CREATE dispatch");
    };
    let json = serde_json::to_string(&*message).unwrap();
    let again: djs_model::message::Message = serde_json::from_str(&json).unwrap();
    assert_eq!(again, message.0);
}
