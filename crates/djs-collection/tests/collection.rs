//! `Collection`: insertion order (D-209, D-215), the discord.js helpers (D-210) with
//! consuming transformations (D-214), and the map basics (D-217).

use std::{cell::Cell, collections::HashSet, hash::RandomState};

use djs_collection::{Collection, DefaultHashBuilder};
use indexmap::IndexMap;

fn letters() -> Collection<u32, char> {
    [(1, 'a'), (2, 'b'), (3, 'c'), (4, 'd')].into()
}

fn keys<K: Copy, V, S>(collection: &Collection<K, V, S>) -> Vec<K> {
    collection.keys().copied().collect()
}

#[test]
fn insertion_order_is_kept() {
    let mut collection = letters();
    // Replacing a value keeps the position.
    assert_eq!(collection.insert(2, 'B'), Some('b'));
    assert_eq!(keys(&collection), [1, 2, 3, 4]);
    // Removing keeps the order of the rest (D-215).
    assert_eq!(collection.remove(&2), Some('B'));
    assert_eq!(keys(&collection), [1, 3, 4]);
    // A removed key comes back last.
    collection.insert(2, 'b');
    assert_eq!(keys(&collection), [1, 3, 4, 2]);
    // swap_remove moves the last entry into the gap.
    assert_eq!(collection.swap_remove(&1), Some('a'));
    assert_eq!(keys(&collection), [2, 3, 4]);
    assert_eq!(collection.remove_entry(&3), Some((3, 'c')));
    assert_eq!(collection.remove(&99), None);
}

#[test]
fn map_basics() {
    let mut collection = letters();
    assert_eq!(collection.len(), 4);
    assert!(!collection.is_empty());
    assert_eq!(collection.get(&3), Some(&'c'));
    assert_eq!(collection[&3], 'c');
    assert_eq!(collection.get_key_value(&4), Some((&4, &'d')));
    assert!(collection.contains_key(&1));
    assert_eq!(collection.index_of(&3), Some(2));
    *collection.get_mut(&1).unwrap() = 'A';
    for value in collection.values_mut() {
        *value = value.to_ascii_uppercase();
    }
    for (_, value) in &mut collection {
        value.make_ascii_lowercase();
    }
    assert_eq!(collection.values().collect::<String>(), "abcd");
    assert_eq!(collection.iter().next_back(), Some((&4, &'d')));
    assert_eq!(collection.iter().len(), 4);
    let owned: Vec<(u32, char)> = collection.clone().into_iter().collect();
    assert_eq!(owned[0], (1, 'a'));
    assert_eq!(collection.clone().into_keys().sum::<u32>(), 10);
    assert_eq!(collection.clone().into_values().last(), Some('d'));
    collection.extend([(5, 'e')]);
    assert_eq!(collection.last(), Some(&'e'));
    collection.clear();
    assert!(collection.is_empty());
    // String keys are looked up by &str.
    let names: Collection<String, u8> = [("ada".to_owned(), 1)].into_iter().collect();
    assert_eq!(names.get("ada"), Some(&1));
}

#[test]
#[should_panic(expected = "no entry for this key")]
fn indexing_a_missing_key_panics() {
    let _ = letters()[&99];
}

#[test]
fn positions() {
    let collection = letters();
    assert_eq!(
        (collection.first(), collection.first_key()),
        (Some(&'a'), Some(&1))
    );
    assert_eq!(
        (collection.last(), collection.last_key()),
        (Some(&'d'), Some(&4))
    );
    assert_eq!(collection.at(1), Some(&'b'));
    assert_eq!(collection.at(-1), Some(&'d'));
    assert_eq!(collection.at(-4), Some(&'a'));
    assert_eq!(collection.at(-5), None);
    assert_eq!(collection.at(4), None);
    assert_eq!(collection.at(isize::MIN), None);
    assert_eq!(collection.key_at(-2), Some(&3));
    let empty: Collection<u32, char> = Collection::new();
    assert_eq!(
        (empty.first(), empty.last(), empty.at(0), empty.random()),
        (None, None, None, None)
    );
    assert_eq!(empty.random_n(3).len(), 0);
}

#[test]
fn finding() {
    let collection = letters();
    assert_eq!(collection.find(|&value, _| value > 'b'), Some(&'c'));
    assert_eq!(collection.find_key(|&value, _| value > 'b'), Some(&3));
    assert_eq!(collection.find_last(|_, &key| key < 3), Some(&'b'));
    assert_eq!(collection.find_last_key(|_, &key| key < 3), Some(&2));
    assert_eq!(collection.find(|_, _| false), None);
    assert!(collection.has_all(&[1, 2]));
    assert!(!collection.has_all(&[1, 9]));
    assert!(collection.has_any(&[9, 4]));
    assert!(!collection.has_any(&[]));
    assert_eq!(
        collection.map(|value, key| format!("{key}{value}")),
        ["1a", "2b", "3c", "4d"]
    );
}

#[test]
fn changing_in_place() {
    let mut collection = letters();
    assert_eq!(collection.sweep(|_, &key| key % 2 == 0), 2);
    assert_eq!(keys(&collection), [1, 3]);
    collection.retain(|&key, _| key > 1);
    assert_eq!(keys(&collection), [3]);

    let mut collection = letters();
    collection.reverse();
    assert_eq!(keys(&collection), [4, 3, 2, 1]);
    collection.sort();
    assert_eq!(keys(&collection), [1, 2, 3, 4]);
    // Stable: equal values keep their order.
    let mut parity: Collection<u32, u32> = (0..6).map(|key| (key, key % 2)).collect();
    parity.sort_by(|a, b, _, _| a.cmp(b));
    assert_eq!(keys(&parity), [0, 2, 4, 1, 3, 5]);
    parity.sort_by(|_, _, a, b| b.cmp(a));
    assert_eq!(keys(&parity), [5, 4, 3, 2, 1, 0]);

    let mut counters: Collection<&str, u32> = Collection::new();
    let calls = Cell::new(0);
    for _ in 0..3 {
        *counters.ensure("a", |_| {
            calls.set(calls.get() + 1);
            0
        }) += 1;
    }
    assert_eq!((counters["a"], calls.get()), (3, 1));
}

#[test]
fn transformations_consume_the_collection() {
    // D-214: nothing is cloned; the values here are not even Clone.
    struct NotClone(u32);
    let make =
        || -> Collection<u32, NotClone> { (1..=6).map(|key| (key, NotClone(key * 10))).collect() };

    let even = make().filter(|value, _| value.0 % 20 == 0);
    assert_eq!(keys(&even), [2, 4, 6]);
    let (small, large) = make().partition(|value, _| value.0 <= 30);
    assert_eq!((keys(&small), keys(&large)), (vec![1, 2, 3], vec![4, 5, 6]));
    let doubled = make().map_values(|value, &key| value.0 + key);
    assert_eq!(
        doubled.values().copied().collect::<Vec<_>>(),
        [11, 22, 33, 44, 55, 66]
    );
    assert_eq!(keys(&make().to_reversed()), [6, 5, 4, 3, 2, 1]);
    assert_eq!(
        keys(&make().to_sorted_by(|a, b, _, _| b.0.cmp(&a.0))),
        [6, 5, 4, 3, 2, 1]
    );
    assert_eq!(keys(&letters().to_reversed().to_sorted()), [1, 2, 3, 4]);
    let spread = make().flat_map(|value, key| -> Collection<u32, u32> {
        [(key, value.0), (key + 100, value.0)].into()
    });
    assert_eq!(spread.len(), 12);
    assert_eq!(spread.first_key(), Some(&1));
    assert_eq!(spread.key_at(1), Some(&101));
}

#[test]
fn set_operations() {
    let left: Collection<u32, &str> = [(1, "l1"), (2, "l2"), (3, "l3")].into();
    let right: Collection<u32, &str> = [(3, "r3"), (4, "r4"), (2, "r2"), (5, "r5")].into();
    let pairs =
        |collection: Collection<u32, &'static str>| collection.into_iter().collect::<Vec<_>>();

    // This collection's values win, in its order, then the other's new keys.
    assert_eq!(
        pairs(left.clone().union(right.clone())),
        [(1, "l1"), (2, "l2"), (3, "l3"), (4, "r4"), (5, "r5")]
    );
    assert_eq!(
        pairs(left.clone().intersection(&right)),
        [(2, "l2"), (3, "l3")]
    );
    assert_eq!(pairs(left.clone().difference(&right)), [(1, "l1")]);
    assert_eq!(
        pairs(left.clone().symmetric_difference(right.clone())),
        [(1, "l1"), (4, "r4"), (5, "r5")]
    );
    // Later values replace earlier ones in place.
    assert_eq!(
        pairs(left.clone().concat([right.clone(), [(1, "x")].into()])),
        [(1, "x"), (2, "r2"), (3, "r3"), (4, "r4"), (5, "r5")]
    );
    // Other value types work where the values are not used.
    let numbers: Collection<u32, f64> = [(2, 0.5)].into();
    assert_eq!(pairs(left.clone().intersection(&numbers)), [(2, "l2")]);

    let merged = left.merge(
        right,
        |value, _| Some(format!("only left {value}")),
        |value, &key| (key != 5).then(|| format!("only right {value}")),
        |a, b, _| Some(format!("{a}+{b}")),
    );
    assert_eq!(
        merged.into_iter().collect::<Vec<_>>(),
        [
            (1, "only left l1".to_owned()),
            (2, "l2+r2".to_owned()),
            (3, "l3+r3".to_owned()),
            (4, "only right r4".to_owned()),
        ]
    );
}

#[test]
fn combining_entries() {
    let words = ["a", "b", "a", "c", "a", "b"];
    let counts: Collection<&str, usize> =
        Collection::combine_entries(words.iter().map(|&word| (word, 1)), |a, b, _| a + b);
    assert_eq!(
        counts.into_iter().collect::<Vec<_>>(),
        [("a", 3), ("b", 2), ("c", 1)]
    );
}

#[test]
fn random_picks() {
    let collection: Collection<u32, u32> = (0..10).map(|key| (key, key)).collect();
    let mut seen = HashSet::new();
    for _ in 0..1000 {
        let value = *collection.random().unwrap();
        assert!(value < 10);
        seen.insert(value);
        assert!(collection.random_key().is_some());
    }
    // 1000 draws from 10 values miss one with probability ≈ 10 × 0.9^1000.
    assert_eq!(seen.len(), 10);
    for count in [0, 1, 3, 9, 10, 15] {
        let picks = collection.random_n(count);
        assert_eq!(picks.len(), count.min(10));
        let distinct: HashSet<_> = picks.iter().collect();
        assert_eq!(distinct.len(), picks.len(), "repeated pick for {count}");
    }
    // Large collections use Floyd's sampling.
    let large: Collection<u32, u32> = (0..100_000).map(|key| (key, key)).collect();
    let picks = large.random_n(50);
    assert_eq!(picks.iter().collect::<HashSet<_>>().len(), 50);
}

#[test]
fn equality_ignores_order() {
    let forward = letters();
    let backward = letters().to_reversed();
    assert_eq!(forward, backward);
    assert_ne!(forward, letters().filter(|_, &key| key != 1));
    let other_hasher: Collection<u32, char, RandomState> = letters().into_iter().collect();
    assert_eq!(forward, other_hasher);
}

#[test]
fn conversions_and_formatting() {
    let map: IndexMap<u32, char, DefaultHashBuilder> = letters().into_index_map();
    let collection = Collection::from(map);
    assert_eq!(collection.as_index_map().len(), 4);
    let back: IndexMap<u32, char, DefaultHashBuilder> = collection.clone().into();
    assert_eq!(back.len(), 4);
    let mut collection = collection;
    collection.as_index_map_mut().move_index(0, 3);
    assert_eq!(collection.last_key(), Some(&1));
    assert_eq!(
        format!("{:?}", letters().filter(|_, &key| key < 3)),
        "{1: 'a', 2: 'b'}"
    );
    let default: Collection<u32, u32> = Collection::default();
    assert!(default.is_empty());
    let with_capacity: Collection<u32, u32> = Collection::with_capacity(100);
    assert!(with_capacity.capacity() >= 100);
    let custom = Collection::<u32, u32, RandomState>::with_hasher(RandomState::new());
    assert!(custom.is_empty());
    let _ = custom.hasher();
    // Iterators can be cloned and printed.
    let collection = letters();
    let iter = collection.iter();
    assert_eq!(iter.clone().count(), 4);
    assert!(format!("{:?}", collection.keys()).starts_with("Keys"));
}
