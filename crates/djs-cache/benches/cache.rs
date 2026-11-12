//! Benchmarks of djs-cache hot paths. Run with `cargo bench -p djs-cache`.
//!
//! The global `AllocProfiler` reports allocations per operation next to timings. Events are
//! decoded before the timed part: what is measured is applying them to the cache.

use std::{
    hash::{Hash, RandomState},
    sync::{Arc, LazyLock},
    time::Duration,
};

use dashmap::DashMap;
use divan::{Bencher, black_box};
use djs_cache::{Backend, Cache, CacheConfig, Limit, Resource, Store};
use djs_model::{
    gateway::{DispatchEvent, GatewayEvent, GatewayEventDeserializer},
    id::{ChannelId, GuildId, MessageId, UserId},
};
use serde::de::DeserializeSeed;

#[allow(
    dead_code,
    reason = "shared with the djs-model tests, which use other helpers"
)]
#[path = "../../djs-model/tests/common/payloads.rs"]
mod payloads;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

const GUILD: GuildId = GuildId::new(payloads::GUILD);
/// The channel of the `MESSAGE_CREATE` fixture and of the hot events.
const CHANNEL: ChannelId = ChannelId::new(1_200_000_000_000_000_020);
const FIRST_USER: UserId = UserId::new(1_000_000_000_000_000_000);

fn dispatch(json: &str) -> DispatchEvent {
    let seed = GatewayEventDeserializer::from_json(json).unwrap();
    match seed
        .deserialize(&mut serde_json::Deserializer::from_str(json))
        .unwrap()
    {
        GatewayEvent::Dispatch(_, event) => event,
        other => panic!("expected a dispatch, got {other:?}"),
    }
}

/// The `GUILD_CREATE` of a guild with 1000 members and presences, 20 channels and 10 roles.
static GUILD_CREATE: LazyLock<DispatchEvent> =
    LazyLock::new(|| dispatch(&payloads::guild_create(1, 1000, 1000, 20, 10)));

/// The hot events of `payloads::hot_events`, decoded.
static HOT_EVENTS: LazyLock<Vec<(&'static str, DispatchEvent)>> = LazyLock::new(|| {
    payloads::hot_events(7)
        .into_iter()
        .map(|(name, json)| (name, dispatch(&json)))
        .collect()
});

fn hot_event(name: &str) -> &'static DispatchEvent {
    &HOT_EVENTS
        .iter()
        .find(|(event, _)| *event == name)
        .unwrap()
        .1
}

/// A `MESSAGE_CREATE` with the given ID, in the fixture's channel.
fn message(id: u64) -> DispatchEvent {
    let mut event = hot_event("MESSAGE_CREATE").clone();
    if let DispatchEvent::MessageCreate(message) = &mut event {
        message.id = MessageId::new(id);
    }
    event
}

/// A cache configured like discord.js' defaults, holding [`GUILD_CREATE`] and a full message
/// channel.
fn discord_js_cache<B: Backend>(backend: B) -> Cache<B> {
    let cache = Cache::with_backend(CacheConfig::discord_js(), backend);
    cache.update_dispatch(&GUILD_CREATE);
    for id in 1..=200 {
        cache.update_dispatch(&message(id));
    }
    cache
}

/// Storing objects received from the REST API (D-230): what a manager's `fetch` or `send`
/// adds to the request.
mod store {
    use super::{Bencher, Cache, CacheConfig, GUILD, Limit, black_box, payloads};
    use djs_model::{guild::Member, message::Message, user::User};
    use serde_json::{Value, json};

    fn member(id: u64) -> Member {
        serde_json::from_value(json!({
            "user": {"id": id.to_string(), "username": "nelly", "discriminator": "0",
                     "global_name": null, "avatar": null},
            "nick": "nick", "roles": ["11", "12"], "joined_at": "2026-01-01T00:00:00.000000+00:00",
            "deaf": false, "mute": false, "flags": 0, "pending": false
        }))
        .unwrap()
    }

    fn members_cache() -> Cache {
        Cache::with_config(
            CacheConfig::builder()
                .members(Limit::Unlimited)
                .users(Limit::Unlimited)
                .messages(Limit::Max(200))
                .build(),
        )
    }

    /// The same user again: the stored one is equal and reused.
    #[divan::bench]
    fn user_unchanged(bencher: Bencher<'_, '_>) {
        let cache = members_cache();
        let user: User = member(7).user;
        cache.store_user(&user);
        bencher.bench(|| cache.store_user(black_box(&user)));
    }

    /// A member already cached, replaced by the same one (a forced `fetch`).
    #[divan::bench]
    fn member_replaced(bencher: Bencher<'_, '_>) {
        let cache = members_cache();
        let member = member(7);
        cache.store_member(GUILD, &member);
        bencher.bench(|| cache.store_member(GUILD, black_box(&member)));
    }

    /// A new member each time (a `fetch` that missed): member and user inserted.
    #[divan::bench]
    fn member_new(bencher: Bencher<'_, '_>) {
        let cache = members_cache();
        let mut next = 1_000;
        bencher
            .with_inputs(|| {
                next += 1;
                member(next)
            })
            .bench_local_refs(|member| cache.store_member(GUILD, member));
    }

    /// A sent message stored in a channel at its limit of 200 (a manager's `send`).
    #[divan::bench]
    fn message_sent(bencher: Bencher<'_, '_>) {
        let cache = members_cache();
        let create: Value = serde_json::from_str(payloads::MESSAGE_CREATE).unwrap();
        let template: Message = serde_json::from_value(create["d"].clone()).unwrap();
        let mut next = 1_000;
        bencher
            .with_inputs(|| {
                next += 1;
                let mut message = template.clone();
                message.id = djs_model::id::MessageId::new(next);
                message
            })
            .bench_local_refs(|message| cache.store_message(message));
    }

    /// Converting without storing: the resource is disabled (or the client has no cache).
    #[divan::bench]
    fn member_not_cached(bencher: Bencher<'_, '_>) {
        let cache = Cache::new();
        let member = member(7);
        bencher.bench(|| cache.store_member(GUILD, black_box(&member)));
    }
}

/// Applying each hot event to a cache configured like discord.js (everything, 200 messages per
/// channel) that already holds the event's guild.
mod update {
    use super::{
        Arc, Bencher, Cache, CacheConfig, DashMapBackend, GUILD_CREATE, black_box,
        discord_js_cache, hot_event, message,
    };

    const HOT: &[&str] = &[
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

    /// The same event over and over: an update of state the cache already has.
    #[divan::bench(args = HOT)]
    fn hot(bencher: Bencher<'_, '_>, name: &str) {
        let cache = discord_js_cache(DashMapBackend::new());
        let event = hot_event(name);
        bencher.bench(|| cache.update_dispatch(black_box(event)));
    }

    /// A new message in a channel at its limit of 200: inserted, the oldest evicted.
    #[divan::bench]
    fn message_create_at_the_limit(bencher: Bencher<'_, '_>) {
        let cache = discord_js_cache(DashMapBackend::new());
        let mut next = 1_000;
        bencher
            .with_inputs(|| {
                next += 1;
                message(next)
            })
            .bench_local_refs(|event| cache.update_dispatch(event));
    }

    /// A `MESSAGE_CREATE` when messages are not cached (the default configuration): only the
    /// configuration is consulted.
    #[divan::bench]
    fn message_create_not_cached(bencher: Bencher<'_, '_>) {
        let cache = Cache::new();
        let event = hot_event("MESSAGE_CREATE");
        bencher.bench(|| cache.update_dispatch(black_box(event)));
    }

    /// The first `GUILD_CREATE` of a 1000-member guild into an empty cache.
    #[divan::bench(sample_count = 30, sample_size = 3)]
    fn guild_create_1000_members(bencher: Bencher<'_, '_>) {
        bencher
            .with_inputs(|| Cache::with_config(CacheConfig::discord_js()))
            .bench_local_refs(|cache| cache.update_dispatch(&GUILD_CREATE));
    }

    /// The same `GUILD_CREATE` with the default configuration (guilds, channels, roles).
    #[divan::bench(sample_count = 30, sample_size = 3)]
    fn guild_create_1000_members_default_config(bencher: Bencher<'_, '_>) {
        bencher
            .with_inputs(Cache::new)
            .bench_local_refs(|cache| cache.update_dispatch(&GUILD_CREATE));
    }

    /// For comparison with the updates (D-161): cloning a decoded event, the cost of copying
    /// all of its data.
    #[divan::bench(args = ["MESSAGE_CREATE", "PRESENCE_UPDATE", "GUILD_MEMBER_UPDATE"])]
    fn clone_event(bencher: Bencher<'_, '_>, name: &str) {
        let event = hot_event(name);
        bencher.bench(|| black_box(event).clone());
    }

    #[divan::bench(sample_count = 30, sample_size = 3)]
    fn clone_guild_create_1000_members(bencher: Bencher<'_, '_>) {
        bencher.bench(|| Arc::new(black_box(&*GUILD_CREATE).clone()));
    }
}

/// Reads of a cache holding a 1000-member guild.
mod read {
    use super::{Bencher, CHANNEL, DashMapBackend, FIRST_USER, GUILD, black_box, discord_js_cache};
    use djs_model::id::{MessageId, UserId};

    #[divan::bench]
    fn guild(bencher: Bencher<'_, '_>) {
        let cache = discord_js_cache(DashMapBackend::new());
        bencher.bench(|| cache.guilds().get(black_box(GUILD)));
    }

    #[divan::bench]
    fn member(bencher: Bencher<'_, '_>) {
        let cache = discord_js_cache(DashMapBackend::new());
        let user = UserId::new(FIRST_USER.get() + 500);
        bencher.bench(|| cache.members(black_box(GUILD)).get(black_box(user)));
    }

    #[divan::bench]
    fn member_missing(bencher: Bencher<'_, '_>) {
        let cache = discord_js_cache(DashMapBackend::new());
        bencher.bench(|| {
            cache
                .members(black_box(GUILD))
                .get(black_box(UserId::new(3)))
        });
    }

    #[divan::bench]
    fn presence(bencher: Bencher<'_, '_>) {
        let cache = discord_js_cache(DashMapBackend::new());
        let user = UserId::new(FIRST_USER.get() + 500);
        bencher.bench(|| cache.presences(black_box(GUILD)).get(black_box(user)));
    }

    #[divan::bench]
    fn message(bencher: Bencher<'_, '_>) {
        let cache = discord_js_cache(DashMapBackend::new());
        bencher.bench(|| {
            cache
                .messages(black_box(CHANNEL))
                .get(black_box(MessageId::new(100)))
        });
    }

    /// Every member of the guild, as `Arc`s.
    #[divan::bench]
    fn all_members(bencher: Bencher<'_, '_>) {
        let cache = discord_js_cache(DashMapBackend::new());
        bencher.bench(|| cache.members(black_box(GUILD)).values());
    }

    /// The channel IDs of the guild (from the guild → channels index).
    #[divan::bench]
    fn channels_in_guild(bencher: Bencher<'_, '_>) {
        let cache = discord_js_cache(DashMapBackend::new());
        bencher.bench(|| cache.channels().in_guild(black_box(GUILD)));
    }
}

/// Sweeping: scanning without removing anything (repeatable), and removing everything.
mod sweep {
    use super::{Bencher, CacheConfig, DashMapBackend, Duration, discord_js_cache, message};
    use djs_cache::Cache;

    /// 200 messages scanned, none old enough.
    #[divan::bench]
    fn messages_none_old(bencher: Bencher<'_, '_>) {
        let cache = discord_js_cache(DashMapBackend::new());
        let century = Duration::from_secs(100 * 365 * 24 * 3_600);
        bencher.bench(|| cache.sweep_messages_older_than(century));
    }

    /// 1000 members scanned, none removed.
    #[divan::bench]
    fn members_none(bencher: Bencher<'_, '_>) {
        let cache = discord_js_cache(DashMapBackend::new());
        bencher.bench(|| cache.sweep_members(|_, member| member.user.bot));
    }

    /// 10 channels × 200 messages, all removed.
    #[divan::bench(sample_count = 30, sample_size = 3)]
    fn messages_all_2000(bencher: Bencher<'_, '_>) {
        bencher
            .with_inputs(|| {
                let cache = Cache::with_config(CacheConfig::discord_js());
                for id in 1..=2_000_u64 {
                    let mut event = message(id);
                    if let djs_model::gateway::DispatchEvent::MessageCreate(message) = &mut event {
                        message.channel_id = djs_model::id::ChannelId::new(id % 10 + 1);
                    }
                    cache.update_dispatch(&event);
                }
                cache
            })
            .bench_local_refs(|cache| cache.sweep_messages_older_than(Duration::ZERO));
    }
}

use djs_cache::DashMapBackend;

/// D-158: the default stores hash keys with foldhash. The same cache with stores hashing
/// with the standard library's `SipHash` (`RandomState`), through the public [`Backend`] trait.
mod hasher {
    use super::{
        Bencher, DashMapBackend, FIRST_USER, GUILD, SipBackend, black_box, discord_js_cache,
        hot_event,
    };
    use djs_model::id::UserId;

    #[divan::bench(types = [DashMapBackend, SipBackend])]
    fn member_get<B: djs_cache::Backend + Default>(bencher: Bencher<'_, '_>) {
        let cache = discord_js_cache(B::default());
        let user = UserId::new(FIRST_USER.get() + 500);
        bencher.bench(|| cache.members(black_box(GUILD)).get(black_box(user)));
    }

    #[divan::bench(types = [DashMapBackend, SipBackend])]
    fn presence_update<B: djs_cache::Backend + Default>(bencher: Bencher<'_, '_>) {
        let cache = discord_js_cache(B::default());
        let event = hot_event("PRESENCE_UPDATE");
        bencher.bench(|| cache.update_dispatch(black_box(event)));
    }

    #[divan::bench(types = [DashMapBackend, SipBackend])]
    fn guild_member_update<B: djs_cache::Backend + Default>(bencher: Bencher<'_, '_>) {
        let cache = discord_js_cache(B::default());
        let event = hot_event("GUILD_MEMBER_UPDATE");
        bencher.bench(|| cache.update_dispatch(black_box(event)));
    }

    #[divan::bench(types = [DashMapBackend, SipBackend], sample_count = 30, sample_size = 3)]
    fn guild_create_1000_members<B: djs_cache::Backend + Default>(bencher: Bencher<'_, '_>) {
        bencher
            .with_inputs(|| {
                djs_cache::Cache::with_backend(djs_cache::CacheConfig::discord_js(), B::default())
            })
            .bench_local_refs(|cache| cache.update_dispatch(&super::GUILD_CREATE));
    }
}

/// A [`Backend`] like the default, but hashing with `SipHash`-1-3 (`std::hash::RandomState`).
#[derive(Debug, Default)]
struct SipBackend;

struct SipStore<K, V> {
    map: DashMap<K, Arc<V>, RandomState>,
}

impl<K, V> Store<K, V> for SipStore<K, V>
where
    K: Eq + Hash + Copy + Send + Sync,
    V: Send + Sync,
{
    fn get(&self, key: &K) -> Option<Arc<V>> {
        self.map.get(key).map(|value| Arc::clone(&value))
    }

    fn insert(&self, key: K, value: Arc<V>) -> Option<Arc<V>> {
        self.map.insert(key, value)
    }

    fn remove(&self, key: &K) -> Option<Arc<V>> {
        self.map.remove(key).map(|(_, value)| value)
    }

    fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn retain(&self, keep: &mut dyn FnMut(&K, &V) -> bool) {
        self.map.retain(|key, value| keep(key, value));
    }

    fn keys(&self) -> Vec<K> {
        self.map.iter().map(|entry| *entry.key()).collect()
    }

    fn values(&self) -> Vec<Arc<V>> {
        self.map
            .iter()
            .map(|entry| Arc::clone(entry.value()))
            .collect()
    }
}

impl Backend for SipBackend {
    type Store<K, V>
        = SipStore<K, V>
    where
        K: Eq + Hash + Copy + Send + Sync + 'static,
        V: Send + Sync + 'static;

    fn store<K, V>(&self, _resource: Resource, _limit: Limit) -> SipStore<K, V>
    where
        K: Eq + Hash + Copy + Send + Sync + 'static,
        V: Send + Sync + 'static,
    {
        SipStore {
            map: DashMap::with_hasher(RandomState::new()),
        }
    }
}
