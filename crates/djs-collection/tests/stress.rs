//! Extreme-load tests of `Collection` (D-041): random operations checked against a simple
//! model, random picks checked for distinctness and fairness, millions of entries (time and
//! memory per entry against `HashMap`), adversarial removal patterns, and every core reading
//! one shared collection.
//!
//! The load tests are ignored by default because they take seconds and gigabytes. Run them in
//! release mode and read the printed numbers:
//!
//! ```text
//! cargo test -p djs-collection --release --test stress -- --ignored --nocapture --test-threads 1
//! ```

#![expect(
    clippy::cast_precision_loss,
    reason = "throughput and memory numbers are only printed"
)]

use std::{
    collections::{HashMap, HashSet},
    hint::black_box,
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use djs_collection::{Collection, DefaultHashBuilder};
use indexmap::IndexMap;

/// A small deterministic generator (xorshift64*), so failures can be replayed.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % bound as u64).unwrap()
    }
}

/// `count` distinct snowflake-like keys in a scrambled order: one creation time per key (so
/// they are distinct), random low bits, then shuffled.
fn keys(count: usize) -> Vec<u64> {
    let mut random = Random(0x9E37_79B9_7F4A_7C15);
    let mut keys: Vec<u64> = (0..count as u64)
        .map(|index| ((1_200_000_000_000 + index) << 22) | (random.next() >> 42))
        .collect();
    for at in (1..keys.len()).rev() {
        keys.swap(at, random.below(at + 1));
    }
    keys
}

/// The entries of `collection` in order.
fn entries<K: Copy, V: Copy, S>(collection: &Collection<K, V, S>) -> Vec<(K, V)> {
    collection
        .iter()
        .map(|(&key, &value)| (key, value))
        .collect()
}

#[test]
fn the_wrapper_costs_no_space() {
    assert_eq!(
        size_of::<Collection<u64, u64>>(),
        size_of::<IndexMap<u64, u64, DefaultHashBuilder>>()
    );
    assert_eq!(
        size_of::<Option<Collection<u64, u64>>>(),
        size_of::<Collection<u64, u64>>()
    );
}

/// Random inserts, removals, sweeps, sorts and reversals, each checked against a `Vec` of
/// entries that does the same thing the obvious way.
#[test]
fn random_operations_match_a_simple_model() {
    let mut random = Random(0xC011_EC71);
    let mut collection: Collection<u32, u32> = Collection::new();
    let mut model: Vec<(u32, u32)> = Vec::new();
    for step in 0..50_000 {
        // Small keys so that inserts often replace and removals often hit.
        let key = u32::try_from(random.below(300)).unwrap();
        let value = u32::try_from(random.below(1_000)).unwrap();
        let position = model.iter().position(|&(k, _)| k == key);
        match random.below(100) {
            0..50 => {
                let old = collection.insert(key, value);
                if let Some(at) = position {
                    assert_eq!(old, Some(std::mem::replace(&mut model[at].1, value)));
                } else {
                    assert_eq!(old, None);
                    model.push((key, value));
                }
            }
            50..70 => assert_eq!(
                collection.remove(&key),
                position.map(|at| model.remove(at).1)
            ),
            70..85 => assert_eq!(
                collection.swap_remove(&key),
                position.map(|at| model.swap_remove(at).1)
            ),
            85..90 => {
                let divisor = u32::try_from(2 + random.below(5)).unwrap();
                let before = model.len();
                model.retain(|&(_, v)| v % divisor != 0);
                assert_eq!(
                    collection.sweep(|&v, _| v % divisor == 0),
                    before - model.len()
                );
            }
            90..94 => {
                // A stable sort by value: equal values keep their order in both.
                collection.sort_by(|a, b, _, _| a.cmp(b));
                model.sort_by_key(|&(_, v)| v);
            }
            94..97 => {
                collection.reverse();
                model.reverse();
            }
            _ => {
                let limit = u32::try_from(random.below(1_000)).unwrap();
                collection.retain(|_, v| *v < limit);
                model.retain(|&(_, v)| v < limit);
            }
        }
        assert_eq!(entries(&collection), model, "after step {step}");
        if let Some(&(last_key, last_value)) = model.last() {
            assert_eq!(collection.last(), Some(&last_value));
            assert_eq!(collection.at(-1), Some(&last_value));
            assert_eq!(collection.index_of(&last_key), Some(model.len() - 1));
        }
        if let Some(&(_, first)) = model.first() {
            assert_eq!(collection.first(), Some(&first));
        }
    }
}

#[test]
fn random_picks_are_distinct_and_cover_everything() {
    // Every length and count around both sampling methods (shuffle and Floyd's).
    for len in 0..64_usize {
        let collection: Collection<u64, u64> = (0..len as u64).map(|key| (key, key)).collect();
        for count in 0..=len + 2 {
            let picks = collection.random_n(count);
            assert_eq!(picks.len(), count.min(collection.len()), "{len} {count}");
            let distinct: HashSet<u64> = picks.iter().map(|&&value| value).collect();
            assert_eq!(distinct.len(), picks.len(), "{len} {count}: {picks:?}");
        }
    }
    let large: Collection<u64, u64> = (0..100_000).map(|key| (key, key)).collect();
    for count in [1, 10, 1_000, 24_999, 25_000, 50_000, 100_000] {
        let picks = large.random_n(count);
        let distinct: HashSet<u64> = picks.iter().map(|&&value| value).collect();
        assert_eq!(distinct.len(), count);
    }
}

#[test]
fn random_picks_are_fair() {
    const DRAWS: usize = 200_000;
    let collection: Collection<usize, usize> = (0..10).map(|key| (key, key)).collect();
    let mut seen = [0_usize; 10];
    for _ in 0..DRAWS {
        seen[*collection.random().unwrap()] += 1;
    }
    // 20 000 expected each, with a standard deviation of 134: a 10% miss is 15 of them.
    for count in seen {
        assert!(count.abs_diff(DRAWS / 10) < DRAWS / 100, "{seen:?}");
    }
    // random_n: every value equally likely to be picked, and to come first, with both
    // sampling methods (3 of 10 shuffles every index; 2 of 20 uses Floyd's).
    for (len, count) in [(10, 3), (20, 2)] {
        let collection: Collection<usize, usize> = (0..len).map(|key| (key, key)).collect();
        let mut picked = vec![0_usize; len];
        let mut first = vec![0_usize; len];
        let rounds = 20_000 * len / count;
        for _ in 0..rounds {
            let picks = collection.random_n(count);
            first[*picks[0]] += 1;
            for &&value in &picks {
                picked[value] += 1;
            }
        }
        // 20 000 picks and 20 000 / count first places expected for each value: a 10% miss
        // is more than 6 standard deviations.
        let expected = 20_000 / count;
        for (&times, &firsts) in picked.iter().zip(&first) {
            assert!(times.abs_diff(20_000) < 2_000, "{len} {count}: {picked:?}");
            assert!(
                firsts.abs_diff(expected) < expected / 10,
                "{len} {count}: {first:?}"
            );
        }
    }
}

/// `(VmRSS, VmHWM)` of this process in KiB, from `/proc/self/status` (Linux only).
fn memory_kib() -> Option<(u64, u64)> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let field = |name: &str| {
        status
            .lines()
            .find(|line| line.starts_with(name))?
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()
    };
    Some((field("VmRSS:")?, field("VmHWM:")?))
}

/// Resets the peak (`VmHWM`) to the current resident size (Linux only; ignored elsewhere).
fn reset_peak_memory() {
    let _ = std::fs::write("/proc/self/clear_refs", "5");
}

/// Builds a map with `build`, prints the time and the memory per entry, looks every key up and
/// returns the map's resident bytes per entry. Large tables live in their own memory mappings,
/// so the resident size before and after measures them alone.
fn measure<M>(
    name: &str,
    keys: &[u64],
    build: impl Fn(&[u64]) -> M,
    get: impl Fn(&M, u64) -> Option<u64>,
) -> Option<f64> {
    reset_peak_memory();
    let before = memory_kib();
    let began = Instant::now();
    let map = build(keys);
    let build_time = began.elapsed();
    let after = memory_kib();
    let began = Instant::now();
    let found = keys
        .iter()
        .filter(|&&key| get(&map, key) == Some(key))
        .count();
    let looked_up = began.elapsed();
    assert_eq!(found, keys.len(), "{name}");
    drop(map);
    let per_entry = |kib: u64| kib as f64 * 1024.0 / keys.len() as f64;
    let memory = before.zip(after).map(|((rss, _), (rss_after, peak))| {
        (
            per_entry(rss_after.saturating_sub(rss)),
            per_entry(peak.saturating_sub(rss)),
        )
    });
    let (resident, peak) = memory.unwrap_or((f64::NAN, f64::NAN));
    println!(
        "{name}: built in {build_time:?} ({:.1} ns/entry), {:.1} ns/lookup, \
         {resident:.1} B/entry resident, {peak:.1} B/entry at the peak",
        build_time.as_nanos() as f64 / keys.len() as f64,
        looked_up.as_nanos() as f64 / keys.len() as f64,
    );
    memory.map(|(resident, _)| resident)
}

#[test]
#[ignore = "a load test: run with --release --ignored --nocapture"]
fn ten_million_entries() {
    const COUNT: usize = 10_000_000;
    let keys = keys(COUNT);
    println!("{COUNT} entries of (u64, u64), inserted one by one:");
    let collection = measure(
        "Collection",
        &keys,
        |keys| {
            let mut map = Collection::new();
            for &key in keys {
                map.insert(key, key);
            }
            map
        },
        |map, key| map.get(&key).copied(),
    );
    measure(
        "Collection::with_capacity",
        &keys,
        |keys| {
            let mut map = Collection::with_capacity(keys.len());
            for &key in keys {
                map.insert(key, key);
            }
            map
        },
        |map, key| map.get(&key).copied(),
    );
    let hash_map = measure(
        "HashMap + foldhash",
        &keys,
        |keys| {
            let mut map = HashMap::with_hasher(DefaultHashBuilder::default());
            for &key in keys {
                map.insert(key, key);
            }
            map
        },
        |map, key| map.get(&key).copied(),
    );
    measure(
        "HashMap + SipHash",
        &keys,
        |keys| {
            let mut map = HashMap::new();
            for &key in keys {
                map.insert(key, key);
            }
            map
        },
        |map, key| map.get(&key).copied(),
    );
    // The order costs a stored hash (8 bytes) and an index (8 bytes) per entry, and a second
    // growable table: more than a plain map, but well under twice as much.
    if let (Some(collection), Some(hash_map)) = (collection, hash_map) {
        assert!(collection < hash_map * 2.0, "{collection} vs {hash_map}");
    }

    // Order-keeping bulk work on all ten million.
    let mut collection: Collection<u64, u64> = keys.iter().map(|&key| (key, key)).collect();
    let (removed, _) = timed("sweep half of 10 000 000", || {
        collection.sweep(|&value, _| value % 2 == 0)
    });
    assert_eq!(removed + collection.len(), COUNT);
    timed("sort_by value, 5 000 000 entries", || {
        collection.sort_by(|a, b, _, _| a.cmp(b));
    });
    let (sum, _) = timed("iterate 5 000 000 values", || {
        collection.values().fold(0_u64, |sum, &value| sum ^ value)
    });
    black_box(sum);
    let (halves, _) = timed("partition 5 000 000 in two", || {
        collection.partition(|&value, _| value % 4 == 1)
    });
    assert_eq!(halves.0.len() + halves.1.len(), COUNT - removed);
}

/// Times `work` and prints it with `name`.
fn timed<T>(name: &str, work: impl FnOnce() -> T) -> (T, Duration) {
    let began = Instant::now();
    let result = work();
    let elapsed = began.elapsed();
    println!("{name}: {elapsed:?}");
    (result, elapsed)
}

#[test]
#[ignore = "a load test: run with --release --ignored --nocapture"]
fn adversarial_removal_patterns() {
    const COUNT: usize = 1_000_000;
    let keys = keys(COUNT);
    let fresh = || -> Collection<u64, u64> { keys.iter().map(|&key| (key, key)).collect() };

    // Linear patterns: from the back, or by swapping, or in one sweep — each on a million.
    let mut collection = fresh();
    let ((), back) = timed("remove all 1 000 000 from the back", || {
        for key in keys.iter().rev() {
            collection.remove(key);
        }
    });
    assert!(collection.is_empty());
    let mut collection = fresh();
    let ((), swap) = timed("swap_remove all 1 000 000 from the front", || {
        for key in &keys {
            collection.swap_remove(key);
        }
    });
    assert!(collection.is_empty());
    let mut collection = fresh();
    let (_, sweep) = timed("sweep all 1 000 000 in one pass", || {
        collection.sweep(|_, _| true)
    });
    assert!(collection.is_empty());
    for elapsed in [back, swap, sweep] {
        assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
    }

    // The quadratic one: each order-keeping removal near the front moves every later entry.
    let mut collection = fresh();
    let ((), front) = timed("remove 1 000 from the front of 1 000 000", || {
        for key in &keys[..1_000] {
            collection.remove(key);
        }
    });
    let mut collection = fresh();
    let (_, swept) = timed("the same 1 000 with one sweep", || {
        let mut seen = 0;
        collection.sweep(|_, _| {
            seen += 1;
            seen <= 1_000
        })
    });
    assert_eq!(collection.len(), COUNT - 1_000);
    println!(
        "  -> one sweep is {:.0}× faster than 1 000 front removals",
        front.as_secs_f64() / swept.as_secs_f64()
    );

    // Random positions: half the cost of the front on average.
    let mut collection = fresh();
    let mut random = Random(99);
    timed("remove 1 000 at random positions of 1 000 000", || {
        for _ in 0..1_000 {
            let key = *collection
                .key_at(random.below(collection.len()).cast_signed())
                .unwrap();
            collection.remove(&key);
        }
    });

    // Insert / remove churn at the back, as a message cache does: stays linear.
    let mut collection = fresh();
    timed(
        "1 000 000 × (insert a new key, remove the oldest with sweep every 10 000)",
        || {
            for (round, &key) in keys.iter().enumerate() {
                collection.insert(!key, key);
                if round % 10_000 == 9_999 {
                    let mut left = 10_000;
                    collection.sweep(|_, _| {
                        left -= 1;
                        left >= 0
                    });
                }
            }
        },
    );
    assert_eq!(collection.len(), COUNT);
}

/// Runs `work` on `threads` threads for `duration` and returns the runs per second.
fn throughput(threads: usize, duration: Duration, work: &(dyn Fn(u64) -> u64 + Sync)) -> f64 {
    let stop = AtomicBool::new(false);
    let runs = AtomicU64::new(0);
    let start = Barrier::new(threads + 1);
    let elapsed = thread::scope(|scope| {
        for thread in 0..threads as u64 {
            let (stop, runs, start) = (&stop, &runs, &start);
            scope.spawn(move || {
                let mut done = 0_u64;
                let mut sink = 0;
                let mut seed = 0x1234_5678 ^ thread;
                start.wait();
                while !stop.load(Ordering::Relaxed) {
                    seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                    sink ^= work(seed);
                    done += 1;
                }
                black_box(sink);
                runs.fetch_add(done, Ordering::Relaxed);
            });
        }
        start.wait();
        let began = Instant::now();
        thread::sleep(duration);
        stop.store(true, Ordering::Relaxed);
        began.elapsed()
    });
    runs.into_inner() as f64 / elapsed.as_secs_f64()
}

#[test]
#[ignore = "a load test: run with --release --ignored --nocapture"]
fn every_core_reads_a_shared_collection() {
    const COUNT: usize = 1_000_000;
    let cores = thread::available_parallelism().map_or(4, usize::from);
    let duration = Duration::from_secs(2);
    let keys = Arc::new(keys(COUNT));
    let collection: Arc<Collection<u64, u64>> =
        Arc::new(keys.iter().map(|&key| (key, key)).collect());
    let small: Arc<Collection<u64, u64>> =
        Arc::new(keys[..1_000].iter().map(|&key| (key, key)).collect());
    let index = |seed: u64, len: usize| usize::try_from(seed >> 32).unwrap() % len;

    let get = |seed: u64| {
        let key = keys[index(seed, COUNT)];
        collection.get(&key).copied().unwrap_or(0)
    };
    let miss = |seed: u64| collection.get(&seed).copied().unwrap_or(1);
    let at = |seed: u64| *collection.at(index(seed, COUNT).cast_signed()).unwrap();
    let random = |_: u64| *collection.random().unwrap();
    let scan = |seed: u64| {
        let target = seed % 1_000;
        small
            .find(|&value, _| value % 1_000 == target)
            .copied()
            .unwrap_or(0)
    };
    let pick = |_: u64| {
        small
            .random_n(10)
            .into_iter()
            .fold(0, |sum, &value| sum ^ value)
    };
    let cases: [(&str, &(dyn Fn(u64) -> u64 + Sync)); 6] = [
        ("get a present key of 1 000 000", &get),
        ("get a missing key of 1 000 000", &miss),
        ("at(index) of 1 000 000", &at),
        ("random() of 1 000 000", &random),
        ("find in 1 000 entries (half a scan on average)", &scan),
        ("random_n(10) of 1 000 entries", &pick),
    ];
    for (name, work) in cases {
        let one = throughput(1, duration, work);
        let all = throughput(cores, duration, work);
        println!(
            "{name}: {one:.0}/s on 1 thread, {all:.0}/s on {cores} threads ({:.1}×)",
            all / one
        );
        assert!(all > one, "{name}: reading did not scale");
    }
}
