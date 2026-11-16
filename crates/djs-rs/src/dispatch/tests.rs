//! Every gateway dispatch through [`convert`] (C-132): the payloads of djs-model's
//! `every_event`, kept in `tests/fixtures/dispatch_events.json`.

use djs_cache::{Cache, CacheConfig, Limit, Resource};
use djs_model::gateway::{DispatchEvent, GatewayEvent, GatewayEventDeserializer};
use serde::de::DeserializeSeed;

use super::{ShardTracker, convert};
use crate::event::{Dispatch, EventKind};

const EVENTS: &str = include_str!("../../tests/fixtures/dispatch_events.json");

/// Every dispatch of the fixture, with its Discord name.
fn events() -> Vec<(String, DispatchEvent)> {
    let list: Vec<serde_json::Value> = serde_json::from_str(EVENTS).unwrap();
    list.into_iter()
        .map(|event| {
            let name = event["t"].as_str().unwrap().to_owned();
            let json = serde_json::json!({"op": 0, "s": 1, "t": name, "d": event["d"]}).to_string();
            let seed = GatewayEventDeserializer::from_json(&json).unwrap();
            let mut deserializer = serde_json::Deserializer::from_str(&json);
            let Ok(GatewayEvent::Dispatch(_, dispatch)) = seed.deserialize(&mut deserializer)
            else {
                panic!("{name} is not a dispatch");
            };
            (name, dispatch)
        })
        .collect()
}

/// A cache of everything.
fn cache() -> Cache {
    let config = Resource::ALL
        .iter()
        .fold(CacheConfig::builder(), |config, resource| {
            config.limit(*resource, Limit::Unlimited)
        })
        .build();
    Cache::with_config(config)
}

/// The dispatch `name` of the fixture, with `change` applied to its data first.
fn event(name: &str, change: impl FnOnce(&mut serde_json::Value)) -> DispatchEvent {
    let list: Vec<serde_json::Value> = serde_json::from_str(EVENTS).unwrap();
    let mut data = list
        .into_iter()
        .find(|event| event["t"] == name)
        .unwrap_or_else(|| panic!("no {name}"))["d"]
        .take();
    change(&mut data);
    let json = serde_json::json!({"op": 0, "s": 1, "t": name, "d": data}).to_string();
    let seed = GatewayEventDeserializer::from_json(&json).unwrap();
    let mut deserializer = serde_json::Deserializer::from_str(&json);
    match seed.deserialize(&mut deserializer) {
        Ok(GatewayEvent::Dispatch(_, dispatch)) => dispatch,
        other => panic!("{name}: {other:?}"),
    }
}

/// The events `dispatch` becomes (all wanted), without [`Dispatch::Raw`].
fn converted(
    dispatch: DispatchEvent,
    cache: Option<&Cache>,
    tracker: &mut ShardTracker,
) -> Vec<String> {
    let mut out = Vec::new();
    convert(dispatch, cache, tracker, |_| true, &mut out);
    assert!(
        matches!(out.first(), Some(Dispatch::Raw(_))),
        "Raw comes first"
    );
    out.iter()
        .skip(1)
        .map(|event| event.name().to_owned())
        .collect()
}

/// discord.js's name of the event Discord's dispatch `name` becomes: its camelCase, but for
/// the ones discord.js names otherwise.
fn expected(name: &str) -> Vec<String> {
    let special: &[&str] = match name {
        // `ClientReady` comes from the client, after every shard's `READY`.
        "READY" => &["shardReady"],
        // The fixture's guild is unavailable.
        "GUILD_DELETE" => &["guildUnavailable"],
        "GUILD_ROLE_CREATE" => &["roleCreate"],
        "GUILD_ROLE_UPDATE" => &["roleUpdate"],
        "GUILD_ROLE_DELETE" => &["roleDelete"],
        // Then one event per new emoji or sticker (the cache had none).
        "GUILD_EMOJIS_UPDATE" => &["guildEmojisUpdate", "emojiCreate"],
        "GUILD_STICKERS_UPDATE" => &["guildStickersUpdate", "stickerCreate"],
        _ => &[],
    };
    if !special.is_empty() {
        return special.iter().map(|name| (*name).to_owned()).collect();
    }
    let mut camel = String::new();
    for (index, word) in name.split('_').enumerate() {
        let word = word.to_lowercase();
        if index == 0 {
            camel.push_str(&word);
        } else {
            let mut chars = word.chars();
            camel.extend(chars.next().map(|first| first.to_ascii_uppercase()));
            camel.push_str(chars.as_str());
        }
    }
    vec![camel]
}

#[test]
fn every_dispatch_becomes_its_events() {
    let events = events();
    assert_eq!(events.len(), 78, "every typed dispatch");
    let cache = cache();
    for (name, dispatch) in events {
        let names = converted(dispatch, Some(&cache), &mut ShardTracker::default());
        assert_eq!(names, expected(&name), "{name}");
    }
}

#[test]
fn without_a_cache_every_dispatch_still_arrives() {
    for (name, dispatch) in events() {
        let names = converted(dispatch, None, &mut ShardTracker::default());
        let mut expected = expected(&name);
        // The events per emoji and sticker need the cache's old list.
        if name == "GUILD_EMOJIS_UPDATE" || name == "GUILD_STICKERS_UPDATE" {
            expected.truncate(1);
        }
        assert_eq!(names, expected, "{name}");
    }
}

#[test]
fn only_wanted_events_are_built() {
    let mut tracker = ShardTracker::default();
    for (name, dispatch) in events() {
        let mut out = Vec::new();
        convert(
            dispatch,
            None::<&Cache>,
            &mut tracker,
            |kind| kind == EventKind::MessageCreate,
            &mut out,
        );
        let names: Vec<&str> = out.iter().map(Dispatch::name).collect();
        if name == "MESSAGE_CREATE" {
            assert_eq!(names, ["messageCreate"]);
        } else {
            assert!(names.is_empty(), "{name}: {names:?}");
        }
    }
}

#[test]
fn guilds_become_available_unavailable_and_deleted() {
    const GUILD: &str = "1300000000000000001";
    let cache = cache();
    let mut tracker = ShardTracker::default();
    // `READY` lists the guild as unavailable: its `GUILD_CREATE` makes it available.
    assert_eq!(
        converted(event("READY", |_| {}), Some(&cache), &mut tracker),
        ["shardReady"]
    );
    let ready = tracker.take_ready().expect("a READY was seen");
    assert!(ready.ready.is_some(), "built, as it was wanted");
    let create = || event("GUILD_CREATE", |data| data["id"] = GUILD.into());
    assert_eq!(
        converted(create(), Some(&cache), &mut tracker),
        ["guildAvailable"]
    );
    // An outage, then the guild back.
    assert_eq!(
        converted(event("GUILD_DELETE", |_| {}), Some(&cache), &mut tracker),
        ["guildUnavailable"]
    );
    assert_eq!(
        converted(create(), Some(&cache), &mut tracker),
        ["guildAvailable"]
    );
    // Already available: a new `GUILD_CREATE` is a guild the bot joined.
    assert_eq!(
        converted(create(), Some(&cache), &mut tracker),
        ["guildCreate"]
    );
    // Removed (kicked, left): `guildDelete`, with the guild the cache held.
    let mut out = Vec::new();
    convert(
        event("GUILD_DELETE", |data| data["unavailable"] = false.into()),
        Some(&cache),
        &mut tracker,
        |_| true,
        &mut out,
    );
    let Some(Dispatch::GuildDelete(deleted)) = out.get(1) else {
        panic!("{out:?}");
    };
    assert!(deleted.old.is_some(), "the cached guild");
}

#[test]
fn emoji_changes_become_one_event_each() {
    let cache = cache();
    let mut tracker = ShardTracker::default();
    let emoji = |id: &str, name: &str| serde_json::json!({"id": id, "name": name, "roles": [], "require_colons": true, "managed": false, "animated": false, "available": true});
    let update = |emojis: Vec<serde_json::Value>| {
        event("GUILD_EMOJIS_UPDATE", move |data| {
            data["emojis"] = emojis.into();
        })
    };
    let first = update(vec![
        emoji("1300000000000000101", "a"),
        emoji("1300000000000000102", "b"),
    ]);
    assert_eq!(
        converted(first, Some(&cache), &mut tracker),
        ["guildEmojisUpdate", "emojiCreate", "emojiCreate"]
    );
    // `a` renamed, `b` removed, `c` added.
    let second = update(vec![
        emoji("1300000000000000101", "a2"),
        emoji("1300000000000000103", "c"),
    ]);
    let mut names = converted(second, Some(&cache), &mut tracker);
    names[1..].sort_unstable();
    assert_eq!(
        names,
        [
            "guildEmojisUpdate",
            "emojiCreate",
            "emojiDelete",
            "emojiUpdate"
        ]
    );
}
