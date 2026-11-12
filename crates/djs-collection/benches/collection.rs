//! Benchmarks of `Collection`. Run with `cargo bench -p djs-collection`.
//!
//! Keys are snowflake-like `u64`s. `Collection` (foldhash) is compared with `IndexMap` with
//! the same hasher (the wrapper's cost), `std::collections::HashMap` with the same hasher
//! (the cost of keeping the order) and `HashMap` with std's `SipHash` (the usual default).

use std::{
    collections::HashMap,
    hash::{BuildHasher, Hash, RandomState},
};

use divan::{Bencher, black_box};
use djs_collection::{Collection, DefaultHashBuilder};
use indexmap::IndexMap;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

const SIZES: [usize; 3] = [100, 10_000, 1_000_000];

/// `count` distinct snowflake-like keys in a scrambled order.
fn keys(count: usize) -> Vec<u64> {
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    (0..count as u64)
        .map(|index| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            // A creation time in the high bits, a unique counter in the low ones.
            ((1_200_000_000_000 + (state >> 40)) << 22)
                | (index & 0x3F_FFFF)
                | ((index >> 22) << 12)
        })
        .collect()
}

fn collection(count: usize) -> Collection<u64, u64> {
    keys(count).into_iter().map(|key| (key, key)).collect()
}

mod get {
    use super::{
        Bencher, BuildHasher, Collection, DefaultHashBuilder, Hash, HashMap, IndexMap, RandomState,
        SIZES, black_box, keys,
    };

    /// Looks up every key once per iteration.
    fn run<M>(
        bencher: Bencher<'_, '_>,
        count: usize,
        build: impl Fn(&[u64]) -> M,
        get: impl Fn(&M, &u64) -> Option<u64>,
    ) {
        let keys = keys(count);
        let map = build(&keys);
        let probes: Vec<u64> = keys.iter().step_by((count / 100).max(1)).copied().collect();
        bencher.counter(probes.len()).bench_local(|| {
            probes
                .iter()
                .map(|key| get(&map, black_box(key)).unwrap_or(0))
                .fold(0, u64::wrapping_add)
        });
    }

    fn std_map<S: BuildHasher + Default>(keys: &[u64]) -> HashMap<u64, u64, S>
    where
        u64: Hash,
    {
        keys.iter().map(|&key| (key, key)).collect()
    }

    #[divan::bench(args = SIZES)]
    fn collection(bencher: Bencher<'_, '_>, count: usize) {
        run(
            bencher,
            count,
            |keys| {
                keys.iter()
                    .map(|&key| (key, key))
                    .collect::<Collection<u64, u64>>()
            },
            |map, key| map.get(key).copied(),
        );
    }

    #[divan::bench(args = SIZES)]
    fn index_map_foldhash(bencher: Bencher<'_, '_>, count: usize) {
        run(
            bencher,
            count,
            |keys| {
                keys.iter()
                    .map(|&key| (key, key))
                    .collect::<IndexMap<u64, u64, DefaultHashBuilder>>()
            },
            |map, key| map.get(key).copied(),
        );
    }

    #[divan::bench(args = SIZES)]
    fn hash_map_foldhash(bencher: Bencher<'_, '_>, count: usize) {
        run(bencher, count, std_map::<DefaultHashBuilder>, |map, key| {
            map.get(key).copied()
        });
    }

    #[divan::bench(args = SIZES)]
    fn hash_map_siphash(bencher: Bencher<'_, '_>, count: usize) {
        run(bencher, count, std_map::<RandomState>, |map, key| {
            map.get(key).copied()
        });
    }
}

mod build {
    use super::{Bencher, Collection, DefaultHashBuilder, HashMap, SIZES, keys};

    #[divan::bench(args = SIZES)]
    fn collection(bencher: Bencher<'_, '_>, count: usize) {
        let keys = keys(count);
        bencher.counter(count).bench(|| {
            let mut collection = Collection::new();
            for &key in &keys {
                collection.insert(key, key);
            }
            collection
        });
    }

    #[divan::bench(args = SIZES)]
    fn collection_with_capacity(bencher: Bencher<'_, '_>, count: usize) {
        let keys = keys(count);
        bencher.counter(count).bench(|| {
            let mut collection = Collection::with_capacity(count);
            for &key in &keys {
                collection.insert(key, key);
            }
            collection
        });
    }

    #[divan::bench(args = SIZES)]
    fn hash_map_foldhash(bencher: Bencher<'_, '_>, count: usize) {
        let keys = keys(count);
        bencher.counter(count).bench(|| {
            let mut map = HashMap::with_hasher(DefaultHashBuilder::default());
            for &key in &keys {
                map.insert(key, key);
            }
            map
        });
    }
}

mod iterate {
    use super::{Bencher, DefaultHashBuilder, HashMap, SIZES, black_box, collection, keys};

    #[divan::bench(args = SIZES)]
    fn collection_values(bencher: Bencher<'_, '_>, count: usize) {
        let collection = collection(count);
        bencher.counter(count).bench(|| {
            black_box(&collection)
                .values()
                .fold(0, |sum, &value| sum ^ value)
        });
    }

    #[divan::bench(args = SIZES)]
    fn hash_map_foldhash_values(bencher: Bencher<'_, '_>, count: usize) {
        let map: HashMap<u64, u64, DefaultHashBuilder> =
            keys(count).into_iter().map(|key| (key, key)).collect();
        bencher
            .counter(count)
            .bench(|| black_box(&map).values().fold(0, |sum, &value| sum ^ value));
    }
}

mod remove {
    use super::{Bencher, collection, keys};

    /// Removes the first entry `count / 10` times, then puts them back.
    fn front(bencher: Bencher<'_, '_>, count: usize, shift: bool) {
        let keys = keys(count);
        let removed = (count / 10).max(1);
        bencher
            .with_inputs(|| collection(count))
            .bench_local_values(|mut collection| {
                for key in &keys[..removed] {
                    if shift {
                        collection.remove(key);
                    } else {
                        collection.swap_remove(key);
                    }
                }
                collection
            });
    }

    /// `remove` keeps the order: every later entry moves up (D-215).
    #[divan::bench(args = [100, 10_000])]
    fn remove_front_tenth(bencher: Bencher<'_, '_>, count: usize) {
        front(bencher, count, true);
    }

    #[divan::bench(args = [100, 10_000])]
    fn swap_remove_front_tenth(bencher: Bencher<'_, '_>, count: usize) {
        front(bencher, count, false);
    }

    /// The same tenth removed with one `sweep`.
    #[divan::bench(args = [100, 10_000])]
    fn sweep_front_tenth(bencher: Bencher<'_, '_>, count: usize) {
        let removed = (count / 10).max(1);
        bencher
            .with_inputs(|| collection(count))
            .bench_local_values(|mut collection| {
                let mut seen = 0;
                collection.sweep(|_, _| {
                    seen += 1;
                    seen <= removed
                });
                collection
            });
    }
}

mod helpers {
    use super::{Bencher, Collection, black_box, collection};

    const COUNT: usize = 10_000;

    #[divan::bench]
    fn filter_half(bencher: Bencher<'_, '_>) {
        bencher
            .with_inputs(|| collection(COUNT))
            .bench_local_values(|collection| collection.filter(|value, _| value % 2 == 0));
    }

    #[divan::bench]
    fn partition_half(bencher: Bencher<'_, '_>) {
        bencher
            .with_inputs(|| collection(COUNT))
            .bench_local_values(|collection| collection.partition(|value, _| value % 2 == 0));
    }

    #[divan::bench]
    fn map_values(bencher: Bencher<'_, '_>) {
        bencher
            .with_inputs(|| collection(COUNT))
            .bench_local_values(|collection| {
                collection.map_values(|value, _| value.to_le_bytes()[0])
            });
    }

    #[divan::bench]
    fn sort_by_value(bencher: Bencher<'_, '_>) {
        bencher
            .with_inputs(|| collection(COUNT))
            .bench_local_values(|mut collection| {
                collection.sort_by(|a, b, _, _| a.cmp(b));
                collection
            });
    }

    #[divan::bench]
    fn find_last_match(bencher: Bencher<'_, '_>) {
        let collection = collection(COUNT);
        let target = *collection.last().unwrap();
        bencher.bench(|| {
            black_box(&collection)
                .find(|&value, _| value == target)
                .copied()
        });
    }

    #[divan::bench]
    fn random(bencher: Bencher<'_, '_>) {
        let collection = collection(COUNT);
        bencher.bench(|| black_box(&collection).random().copied());
    }

    #[divan::bench(args = [1, 10, 1000, 5000])]
    fn random_n(bencher: Bencher<'_, '_>, count: usize) {
        let collection = collection(COUNT);
        bencher.bench(|| black_box(&collection).random_n(count).len());
    }

    /// Two collections sharing half their keys.
    fn halves() -> (Collection<u64, u64>, Collection<u64, u64>) {
        let all = collection(COUNT + COUNT / 2);
        let left: Collection<u64, u64> = all.iter().take(COUNT).map(|(&k, &v)| (k, v)).collect();
        let right = all.iter().skip(COUNT / 2).map(|(&k, &v)| (k, v)).collect();
        (left, right)
    }

    #[divan::bench]
    fn merge(bencher: Bencher<'_, '_>) {
        bencher
            .with_inputs(halves)
            .bench_local_values(|(left, right)| {
                left.merge(right, |v, _| Some(v), |v, _| Some(v), |a, b, _| Some(a ^ b))
            });
    }

    #[divan::bench]
    fn union(bencher: Bencher<'_, '_>) {
        bencher
            .with_inputs(halves)
            .bench_local_values(|(left, right)| left.union(right));
    }

    #[divan::bench]
    fn symmetric_difference(bencher: Bencher<'_, '_>) {
        bencher
            .with_inputs(halves)
            .bench_local_values(|(left, right)| left.symmetric_difference(right));
    }

    #[divan::bench]
    fn intersection(bencher: Bencher<'_, '_>) {
        bencher
            .with_inputs(halves)
            .bench_local_values(|(left, right)| left.intersection(&right));
    }
}
