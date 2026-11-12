//! Extreme-load tests of the cache (D-041): memory at scale, concurrent readers and writers,
//! bounded churn, sweeping under load and repeated guild churn.
//!
//! The load tests are ignored by default because they take seconds and saturate the machine.
//! Run them in release mode and read the printed numbers:
//!
//! ```text
//! cargo test -p djs-cache --release --test stress -- --ignored --nocapture --test-threads 1
//! ```

#![expect(
    clippy::cast_precision_loss,
    reason = "throughput and memory numbers are only printed"
)]

#[expect(
    dead_code,
    reason = "shared with the djs-model tests, which use other helpers"
)]
#[path = "../../djs-model/tests/common/payloads.rs"]
mod payloads;

use std::{
    sync::{
        Barrier,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use djs_cache::{Cache, CacheConfig, CachedGuild, CachedMember, CachedMessage, CachedPresence};
use djs_model::{
    channel::Channel,
    gateway::{
        DispatchEvent, GatewayEvent, GatewayEventDeserializer,
        payload::{GuildCreate, GuildDelete},
    },
    guild::Role,
    id::{ChannelId, EmojiId, GuildId, MessageId, RoleId, UserId},
    user::User,
    voice::VoiceState,
};
use serde::de::DeserializeSeed;

const USER_BASE: u64 = 1_000_000_000_000_000_000;
/// The channel of the `MESSAGE_CREATE` fixture.
const CHANNEL: u64 = 1_200_000_000_000_000_020;

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

fn mib(kib: u64) -> f64 {
    kib as f64 / 1024.0
}

/// A small, fast pseudo-random sequence (xorshift64), enough to spread reads.
struct Rng(u64);

impl Rng {
    fn below(&mut self, bound: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % bound
    }
}

/// Generates `GUILD_CREATE`s like Discord sends at startup, each with unique IDs: its own
/// guild, channels, roles, emojis and `members` members (with as many unique users).
struct Guilds {
    template: GuildCreate,
    members: u64,
}

impl Guilds {
    fn new(members: u64, presences: u64) -> Self {
        let DispatchEvent::GuildCreate(template) =
            dispatch(&payloads::guild_create(1, members, presences, 20, 10))
        else {
            unreachable!()
        };
        Self {
            template: *template,
            members,
        }
    }

    const fn guild_id(index: u64) -> GuildId {
        GuildId::new(2_000_000 + index)
    }

    fn user_id(&self, guild: u64, member: u64) -> UserId {
        UserId::new(USER_BASE + guild * self.members + member)
    }

    fn get(&self, index: u64) -> GuildCreate {
        let mut event = self.template.clone();
        let GuildCreate::Available(guild) = &mut event else {
            unreachable!()
        };
        guild.id = Self::guild_id(index);
        let users = |id: UserId| self.user_id(index, id.get() - USER_BASE);
        for (i, channel) in (0..).zip(&mut guild.channels) {
            let Channel::Text(channel) = channel else {
                unreachable!("the payload has text channels only")
            };
            channel.id = ChannelId::new(3_000_000 + index * 1_000 + i);
        }
        for (i, role) in (0..).zip(&mut guild.roles) {
            role.id = RoleId::new(4_000_000 + index * 1_000 + i);
        }
        for (i, emoji) in (0..).zip(&mut guild.emojis) {
            emoji.id = EmojiId::new(5_000_000 + index * 1_000 + i);
        }
        for member in &mut guild.members {
            member.user.id = users(member.user.id);
        }
        for presence in &mut guild.presences {
            presence.user.id = users(presence.user.id);
        }
        for state in &mut guild.voice_states {
            state.user_id = users(state.user_id);
        }
        event
    }
}

#[test]
fn type_sizes() {
    use core::mem::size_of;
    println!(
        "[stress] sizes: CachedGuild {} B, CachedMember {} B, CachedMessage {} B, \
         CachedPresence {} B, User {} B, Channel {} B, Role {} B, VoiceState {} B, Cache {} B",
        size_of::<CachedGuild>(),
        size_of::<CachedMember>(),
        size_of::<CachedMessage>(),
        size_of::<CachedPresence>(),
        size_of::<User>(),
        size_of::<Channel>(),
        size_of::<Role>(),
        size_of::<VoiceState>(),
        size_of::<Cache>(),
    );
    // Regression guards (the sizes when measured, x86_64) for the types kept by the
    // thousand: a member shares its user, a presence keeps only its user's ID.
    if cfg!(target_pointer_width = "64") {
        assert!(
            size_of::<CachedMember>() <= 176,
            "{}",
            size_of::<CachedMember>()
        );
        assert!(
            size_of::<CachedPresence>() <= 152,
            "{}",
            size_of::<CachedPresence>()
        );
        assert!(
            size_of::<CachedMessage>() <= 512,
            "{}",
            size_of::<CachedMessage>()
        );
        assert!(
            size_of::<CachedGuild>() <= 440,
            "{}",
            size_of::<CachedGuild>()
        );
    }
}

/// The guilds a large bot receives at startup, cached with discord.js' defaults: time and
/// memory per member. Each event is built, applied and dropped, as on a real connection.
fn startup(name: &str, config: CacheConfig, guilds: u64, members: u64) {
    let generator = Guilds::new(members, 0);
    let cache = Cache::with_config(config);
    let before = memory_kib();
    reset_peak_memory();
    let mut applying = Duration::ZERO;
    for index in 0..guilds {
        let event = generator.get(index);
        let start = Instant::now();
        cache.update(&event);
        applying += start.elapsed();
    }
    let total = guilds * members;
    let memory = match (before, memory_kib()) {
        (Some((before, _)), Some((after, peak))) => format!(
            "rss {:.1} → {:.1} MiB (+{:.1} MiB, {:.0} B per member), peak {:.1} MiB",
            mib(before),
            mib(after),
            mib(after - before),
            (after - before) as f64 * 1024.0 / total as f64,
            mib(peak)
        ),
        _ => "memory n/a".to_owned(),
    };
    println!(
        "[stress] startup {name}: {guilds} guilds × {members} members in {:.0} ms \
         ({:.0} members/s, {:.2} ms per guild); {memory}; cached {} guilds, {} users, {} channels",
        applying.as_secs_f64() * 1e3,
        total as f64 / applying.as_secs_f64(),
        applying.as_secs_f64() * 1e3 / guilds as f64,
        cache.guilds().len(),
        cache.users().len(),
        cache.channels().len(),
    );
    assert_eq!(cache.guilds().len() as u64, guilds);
    if cache.config().is_enabled(djs_cache::Resource::Member) {
        let last = Guilds::guild_id(guilds - 1);
        assert_eq!(cache.members(last).len() as u64, members);
    }
}

/// The environment variable telling a child process which startup configuration to measure.
const STARTUP_CHILD: &str = "DJS_CACHE_STRESS_STARTUP";

/// The startup memory, measured for each configuration in a child process of this test
/// binary: a fresh heap, which memory freed by earlier tests in the same process would
/// otherwise fill first (and make the cache look smaller).
#[test]
#[ignore = "extreme load: run with --ignored in release mode"]
fn startup_500_guilds() {
    if let Ok(name) = std::env::var(STARTUP_CHILD) {
        let config = match name.as_str() {
            "discord.js config" => CacheConfig::discord_js(),
            "members + users" => CacheConfig::builder()
                .members(djs_cache::Limit::Unlimited)
                .users(djs_cache::Limit::Unlimited)
                .build(),
            "default config" => CacheConfig::default(),
            other => panic!("unknown configuration {other}"),
        };
        startup(&name, config, 500, 1_000);
        return;
    }
    for name in ["discord.js config", "members + users", "default config"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "startup_500_guilds", "--nocapture"])
            .env(STARTUP_CHILD, name)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "{name}: {stdout}");
        for line in stdout.lines().filter(|line| line.contains("[stress]")) {
            println!("{}", line.trim_start_matches(|c| c != '['));
        }
    }
}

/// Writers apply member and presence updates while readers look members and presences up,
/// all on one 10 000-member guild.
#[test]
#[ignore = "extreme load: run with --ignored in release mode"]
fn concurrent_readers_and_writers() {
    const MEMBERS: u64 = 10_000;
    const RUN: Duration = Duration::from_secs(2);
    let generator = Guilds::new(MEMBERS, MEMBERS);
    let guild = Guilds::guild_id(0);
    let cache = Cache::with_config(CacheConfig::discord_js());
    cache.update(&generator.get(0));
    assert_eq!(cache.presences(guild).len() as u64, MEMBERS);

    // One member update and one presence update per member, decoded up front.
    let updates: Vec<DispatchEvent> = (0..MEMBERS)
        .flat_map(|i| {
            let user = generator.user_id(0, i);
            let member = payloads::member(i).replacen(
                '{',
                &format!(r#"{{"guild_id":"{guild}","nick":"nick {i}","#),
                1,
            );
            let member = member.replacen(r#""nick":null,"#, "", 1);
            let presence = payloads::presence(i, Some(guild.get()));
            [
                dispatch(&payloads::dispatch("GUILD_MEMBER_UPDATE", 1, &member)),
                dispatch(&payloads::dispatch("PRESENCE_UPDATE", 1, &presence)),
            ]
            .map(|mut event| {
                // The payload helpers number users from the base; shift them to the guild's.
                match &mut event {
                    DispatchEvent::GuildMemberUpdate(update) => update.user.id = user,
                    DispatchEvent::PresenceUpdate(presence) => presence.user.id = user,
                    _ => unreachable!(),
                }
                event
            })
        })
        .collect();

    for (writers, readers) in [(1, 0), (0, 1), (1, 1), (2, 2), (4, 4), (2, 8), (8, 2)] {
        let stop = AtomicBool::new(false);
        let (written, read, found) = (AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0));
        let barrier = Barrier::new(writers + readers + 1);
        thread::scope(|scope| {
            for writer in 0..writers {
                let (cache, updates, stop, written, barrier) =
                    (&cache, &updates, &stop, &written, &barrier);
                scope.spawn(move || {
                    barrier.wait();
                    let mut count = 0_u64;
                    let mut index = writer * updates.len() / writers.max(1);
                    while !stop.load(Ordering::Relaxed) {
                        for _ in 0..64 {
                            cache.update_dispatch(&updates[index]);
                            index = (index + 1) % updates.len();
                        }
                        count += 64;
                    }
                    written.fetch_add(count, Ordering::Relaxed);
                });
            }
            for reader in 0..readers {
                let (cache, generator, stop, read, found, barrier) =
                    (&cache, &generator, &stop, &read, &found, &barrier);
                scope.spawn(move || {
                    barrier.wait();
                    let mut rng = Rng(0x9e37_79b9_7f4a_7c15 ^ (reader as u64 + 1));
                    let (mut count, mut hits) = (0_u64, 0_u64);
                    while !stop.load(Ordering::Relaxed) {
                        for _ in 0..64 {
                            let user = generator.user_id(0, rng.below(MEMBERS));
                            hits += u64::from(cache.members(guild).get(user).is_some());
                            hits += u64::from(cache.presences(guild).get(user).is_some());
                        }
                        count += 128;
                    }
                    read.fetch_add(count, Ordering::Relaxed);
                    found.fetch_add(hits, Ordering::Relaxed);
                });
            }
            barrier.wait();
            thread::sleep(RUN);
            stop.store(true, Ordering::Relaxed);
        });
        let seconds = RUN.as_secs_f64();
        let (written, read, found) = (
            written.into_inner() as f64,
            read.into_inner() as f64,
            found.into_inner(),
        );
        println!(
            "[stress] concurrent: {writers} writers, {readers} readers → \
             {:.2} M updates/s, {:.2} M reads/s",
            written / seconds / 1e6,
            read / seconds / 1e6,
        );
        // Nothing is removed by these updates, so every read finds its entry.
        assert_eq!(found as f64, read);
    }
    assert_eq!(cache.members(guild).len() as u64, MEMBERS);
    assert_eq!(cache.presences(guild).len() as u64, MEMBERS);
}

/// A `MESSAGE_CREATE` with the given ID and channel.
fn message(template: &DispatchEvent, id: u64, channel: u64) -> DispatchEvent {
    let mut event = template.clone();
    if let DispatchEvent::MessageCreate(message) = &mut event {
        message.id = MessageId::new(id);
        message.channel_id = ChannelId::new(channel);
    }
    event
}

/// Millions of messages over 1000 channels with discord.js' limit of 200 per channel: the
/// cache stays at 200 000 messages and its memory stops growing once full.
#[test]
#[ignore = "extreme load: run with --ignored in release mode"]
fn message_churn_stays_bounded() {
    const CHANNELS: u64 = 1_000;
    const ROUNDS: u64 = 4;
    const PER_ROUND: u64 = 500_000;
    let template = dispatch(payloads::MESSAGE_CREATE);
    for threads in [1_u64, 4] {
        let cache = Cache::with_config(CacheConfig::discord_js());
        let before = memory_kib();
        let mut id = 0;
        for round in 0..ROUNDS {
            let start = Instant::now();
            thread::scope(|scope| {
                for thread in 0..threads {
                    let (cache, template) = (&cache, &template);
                    let first = id + thread * PER_ROUND / threads;
                    scope.spawn(move || {
                        for id in first..first + PER_ROUND / threads {
                            // Each thread writes its own channels, as shards would.
                            let channel =
                                1 + thread * CHANNELS / threads + id % (CHANNELS / threads);
                            cache.update_dispatch(&message(template, id + 1, channel));
                        }
                    });
                }
            });
            id += PER_ROUND;
            let elapsed = start.elapsed().as_secs_f64();
            let memory = match (before, memory_kib()) {
                (Some((before, _)), Some((after, _))) => {
                    format!("rss +{:.1} MiB", mib(after.saturating_sub(before)))
                }
                _ => "memory n/a".to_owned(),
            };
            let cached: usize = (1..=CHANNELS)
                .map(|channel| cache.messages(ChannelId::new(channel)).len())
                .sum();
            println!(
                "[stress] message churn ({threads} threads), round {}: {:.2} M messages/s \
                 (with the event clone), {cached} cached, {memory}",
                round + 1,
                PER_ROUND as f64 / elapsed / 1e6,
            );
            assert_eq!(cached as u64, CHANNELS * 200);
        }
    }
}

/// Writers insert messages while another thread sweeps them: no deadlock, and a final sweep
/// leaves nothing.
#[test]
#[ignore = "extreme load: run with --ignored in release mode"]
fn sweeping_under_load() {
    const RUN: Duration = Duration::from_secs(2);
    let template = dispatch(payloads::MESSAGE_CREATE);
    let config = CacheConfig::builder()
        .messages(djs_cache::Limit::Max(200))
        .members(djs_cache::Limit::Unlimited)
        .users(djs_cache::Limit::Unlimited)
        .build();
    let cache = Cache::with_config(config);
    let stop = AtomicBool::new(false);
    let (inserted, swept, sweeps) = (AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0));
    thread::scope(|scope| {
        for writer in 0..4_u64 {
            let (cache, template, stop, inserted) = (&cache, &template, &stop, &inserted);
            scope.spawn(move || {
                let mut id = writer << 40;
                while !stop.load(Ordering::Relaxed) {
                    id += 1;
                    cache.update_dispatch(&message(template, id, CHANNEL + id % 100));
                }
                inserted.fetch_add(id - (writer << 40), Ordering::Relaxed);
            });
        }
        let (cache, stop, swept, sweeps) = (&cache, &stop, &swept, &sweeps);
        scope.spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                // The fixture is from 2026-11-10: every message is old enough.
                swept.fetch_add(
                    cache.sweep_messages_older_than(Duration::ZERO) as u64,
                    Ordering::Relaxed,
                );
                sweeps.fetch_add(1, Ordering::Relaxed);
                thread::sleep(Duration::from_millis(1));
            }
        });
        thread::sleep(RUN);
        stop.store(true, Ordering::Relaxed);
    });
    let last = cache.sweep_messages_older_than(Duration::ZERO) as u64;
    let left: usize = (0..100)
        .map(|offset| cache.messages(ChannelId::new(CHANNEL + offset)).len())
        .sum();
    let (inserted, swept, sweeps) = (
        inserted.into_inner(),
        swept.into_inner() + last,
        sweeps.into_inner(),
    );
    println!(
        "[stress] sweeping under load: {:.2} M inserts/s by 4 writers, {sweeps} sweeps \
         removed {swept} messages, {left} left",
        inserted as f64 / RUN.as_secs_f64() / 1e6,
    );
    assert_eq!(left, 0);
    assert!(swept <= inserted);
}

/// A 1000-member guild joined and left over and over: nothing of it stays behind, and the
/// memory does not grow from round to round.
#[test]
#[ignore = "extreme load: run with --ignored in release mode"]
fn guild_churn_leaves_nothing_behind() {
    const ROUNDS: u64 = 300;
    let generator = Guilds::new(1_000, 1_000);
    let mut config = djs_cache::CacheConfigBuilder::from(CacheConfig::discord_js());
    // Users outlive the guilds they were seen in (as in discord.js); leave them out here.
    config = config.users(djs_cache::Limit::Disabled);
    let cache = Cache::with_config(config.build());
    let mut rss = Vec::new();
    let start = Instant::now();
    for round in 0..ROUNDS {
        let index = round % 10;
        cache.update(&generator.get(index));
        let delete: GuildDelete = serde_json::from_value(serde_json::json!({
            "id": Guilds::guild_id(index).to_string()
        }))
        .unwrap();
        cache.update(&delete);
        if round % 50 == 49 {
            rss.push(memory_kib().map_or(0, |(rss, _)| rss));
        }
    }
    let elapsed = start.elapsed();
    println!(
        "[stress] guild churn: {ROUNDS} × join + leave of a 1000-member guild in {:.0} ms; \
         rss every 50 rounds: {:?} MiB",
        elapsed.as_secs_f64() * 1e3,
        rss.iter()
            .map(|&kib| (mib(kib) * 10.0).round() / 10.0)
            .collect::<Vec<_>>(),
    );
    assert_eq!(cache.guilds().len(), 0);
    assert_eq!(cache.channels().len(), 0);
    for index in 0..10 {
        let guild = Guilds::guild_id(index);
        assert_eq!(cache.members(guild).len(), 0);
        assert_eq!(cache.presences(guild).len(), 0);
        assert_eq!(cache.roles(guild).len(), 0);
        assert_eq!(cache.emojis(guild).len(), 0);
        assert_eq!(cache.voice_states(guild).len(), 0);
    }
}
