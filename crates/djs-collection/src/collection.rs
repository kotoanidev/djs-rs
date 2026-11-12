//! [`Collection`]: an insertion-ordered map with discord.js' helpers.

use core::{
    borrow::Borrow,
    cmp::Ordering,
    fmt,
    hash::{BuildHasher, Hash},
    ops::Index,
};

use indexmap::IndexMap;

use crate::{
    iter::{IntoIter, IntoKeys, IntoValues, Iter, IterMut, Keys, Values, ValuesMut},
    random,
};

/// The default hasher of a [`Collection`]: foldhash's fast hasher with a random seed
/// (D-216), as in `djs-cache`.
pub type DefaultHashBuilder = foldhash::fast::RandomState;

/// A map that remembers the order its keys were first inserted in, with the helpers of
/// discord.js' `Collection`.
///
/// Iteration, [`first`](Self::first), [`last`](Self::last), [`at`](Self::at) and
/// [`random`](Self::random) follow insertion order (D-209); inserting an existing key replaces
/// its value in place, and [`remove`](Self::remove) keeps the order of the rest (D-215).
/// Lookups are hash-based, with foldhash by default (D-216).
///
/// The helpers follow discord.js (D-210), with Rust's ownership (D-214):
///
/// - reading — [`find`](Self::find), [`first`](Self::first), [`at`](Self::at),
///   [`has_all`](Self::has_all), [`map`](Self::map), … — takes `&self`;
/// - changing in place — [`sweep`](Self::sweep), [`sort_by`](Self::sort_by),
///   [`reverse`](Self::reverse), [`ensure`](Self::ensure) — takes `&mut self`;
/// - building a new collection — [`filter`](Self::filter), [`partition`](Self::partition),
///   [`map_values`](Self::map_values), [`union`](Self::union), … — consumes the collection
///   and clones nothing (call `.clone()` first to keep the original).
///
/// discord.js' `each`, `some`, `every`, `reduce` and `tap` have no counterpart: use the
/// iterators (`.values().any(…)`, `.iter().fold(…)`, …). Callbacks receive the value first
/// and the key second, as in discord.js.
///
/// discord.js counterpart: `Collection` (`@discordjs/collection`).
///
/// # Examples
///
/// ```
/// use djs_collection::Collection;
///
/// let mut scores: Collection<&str, u32> = Collection::new();
/// scores.insert("ada", 31);
/// scores.insert("grace", 27);
/// scores.insert("linus", 40);
///
/// assert_eq!(scores.first(), Some(&31));
/// assert_eq!(scores.find_key(|&score, _| score > 30), Some(&"ada"));
///
/// scores.sort_by(|a, b, _, _| b.cmp(a));
/// assert_eq!(scores.keys().copied().collect::<Vec<_>>(), ["linus", "ada", "grace"]);
///
/// let (high, low) = scores.partition(|&score, _| score >= 30);
/// assert_eq!(high.len(), 2);
/// assert_eq!(low.get("grace"), Some(&27));
/// ```
#[derive(Clone)]
pub struct Collection<K, V, S = DefaultHashBuilder> {
    map: IndexMap<K, V, S>,
}

impl<K, V> Collection<K, V> {
    /// Creates an empty collection.
    ///
    /// discord.js counterpart: `new Collection()`.
    #[must_use]
    pub fn new() -> Self {
        Self::with_hasher(DefaultHashBuilder::default())
    }

    /// Creates an empty collection with room for `capacity` entries before it reallocates.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, DefaultHashBuilder::default())
    }
}

impl<K, V, S> Collection<K, V, S> {
    /// Creates an empty collection hashing keys with `hasher`.
    #[must_use]
    pub const fn with_hasher(hasher: S) -> Self {
        Self {
            map: IndexMap::with_hasher(hasher),
        }
    }

    /// Creates an empty collection with room for `capacity` entries, hashing with `hasher`.
    #[must_use]
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        Self {
            map: IndexMap::with_capacity_and_hasher(capacity, hasher),
        }
    }

    /// The number of entries.
    ///
    /// discord.js counterpart: `size`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether there are no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// How many entries fit before the collection reallocates.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    /// Removes every entry, keeping the memory for reuse.
    ///
    /// discord.js counterpart: `clear`.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// The hasher keys are hashed with.
    #[must_use]
    pub fn hasher(&self) -> &S {
        self.map.hasher()
    }

    /// The entries in order, as `(&key, &value)`.
    ///
    /// discord.js counterpart: `entries`.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter(self.map.iter())
    }

    /// The entries in order, with mutable values.
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut(self.map.iter_mut())
    }

    /// The keys in order.
    ///
    /// discord.js counterpart: `keys`.
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys(self.map.keys())
    }

    /// The values in order.
    ///
    /// discord.js counterpart: `values`.
    pub fn values(&self) -> Values<'_, K, V> {
        Values(self.map.values())
    }

    /// The values in order, mutable.
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut(self.map.values_mut())
    }

    /// The keys in order, moved out of the collection.
    pub fn into_keys(self) -> IntoKeys<K, V> {
        IntoKeys(self.map.into_keys())
    }

    /// The values in order, moved out of the collection.
    pub fn into_values(self) -> IntoValues<K, V> {
        IntoValues(self.map.into_values())
    }

    /// The inner `IndexMap`, for operations this type does not offer (D-217).
    #[must_use]
    pub const fn as_index_map(&self) -> &IndexMap<K, V, S> {
        &self.map
    }

    /// The inner `IndexMap`, mutable (D-217).
    pub const fn as_index_map_mut(&mut self) -> &mut IndexMap<K, V, S> {
        &mut self.map
    }

    /// Turns the collection into its inner `IndexMap` (D-217).
    #[must_use]
    pub fn into_index_map(self) -> IndexMap<K, V, S> {
        self.map
    }

    /// The first value inserted, or `None` if the collection is empty. The first `n` are
    /// `.values().take(n)`.
    ///
    /// discord.js counterpart: `first()`.
    #[must_use]
    pub fn first(&self) -> Option<&V> {
        self.map.first().map(|(_, value)| value)
    }

    /// The first key inserted.
    ///
    /// discord.js counterpart: `firstKey()`.
    #[must_use]
    pub fn first_key(&self) -> Option<&K> {
        self.map.first().map(|(key, _)| key)
    }

    /// The last value inserted. The last `n` are `.values().rev().take(n)`.
    ///
    /// discord.js counterpart: `last()`.
    #[must_use]
    pub fn last(&self) -> Option<&V> {
        self.map.last().map(|(_, value)| value)
    }

    /// The last key inserted.
    ///
    /// discord.js counterpart: `lastKey()`.
    #[must_use]
    pub fn last_key(&self) -> Option<&K> {
        self.map.last().map(|(key, _)| key)
    }

    /// The position `index` counts to: from the start, or from the end when negative.
    fn position(&self, index: isize) -> Option<usize> {
        if index >= 0 {
            usize::try_from(index).ok()
        } else {
            self.len().checked_sub(index.unsigned_abs())
        }
    }

    /// The value at position `index` in insertion order; a negative index counts from the
    /// end (`-1` is the last).
    ///
    /// discord.js counterpart: `at`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_collection::Collection;
    ///
    /// let letters: Collection<u8, char> = [(1, 'a'), (2, 'b'), (3, 'c')].into();
    /// assert_eq!(letters.at(0), Some(&'a'));
    /// assert_eq!(letters.at(-1), Some(&'c'));
    /// assert_eq!(letters.at(3), None);
    /// assert_eq!(letters.key_at(-3), Some(&1));
    /// ```
    #[must_use]
    pub fn at(&self, index: isize) -> Option<&V> {
        let position = self.position(index)?;
        self.map.get_index(position).map(|(_, value)| value)
    }

    /// The key at position `index`; a negative index counts from the end.
    ///
    /// discord.js counterpart: `keyAt`.
    #[must_use]
    pub fn key_at(&self, index: isize) -> Option<&K> {
        let position = self.position(index)?;
        self.map.get_index(position).map(|(key, _)| key)
    }

    /// A random value, or `None` if the collection is empty.
    ///
    /// The randomness comes from std's `RandomState` (D-210): fine for games and giveaways,
    /// not for anything that must resist prediction.
    ///
    /// discord.js counterpart: `random()`.
    #[must_use]
    pub fn random(&self) -> Option<&V> {
        self.random_entry().map(|(_, value)| value)
    }

    /// A random key.
    ///
    /// discord.js counterpart: `randomKey()`.
    #[must_use]
    pub fn random_key(&self) -> Option<&K> {
        self.random_entry().map(|(key, _)| key)
    }

    fn random_entry(&self) -> Option<(&K, &V)> {
        if self.is_empty() {
            return None;
        }
        self.map.get_index(random::Rng::new().below(self.len()))
    }

    /// Up to `count` different random values, in random order — all of them, shuffled, if
    /// `count` is at least the length.
    ///
    /// discord.js counterpart: `random(amount)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_collection::Collection;
    ///
    /// let entrants: Collection<u64, &str> = [(1, "ada"), (2, "grace"), (3, "linus")].into();
    /// let winners = entrants.random_n(2);
    /// assert_eq!(winners.len(), 2);
    /// assert_ne!(winners[0], winners[1]);
    /// ```
    #[must_use]
    pub fn random_n(&self, count: usize) -> Vec<&V> {
        let count = count.min(self.len());
        random::distinct(self.len(), count)
            .into_iter()
            .filter_map(|index| self.map.get_index(index).map(|(_, value)| value))
            .collect()
    }

    /// The first value for which `predicate(value, key)` is true.
    ///
    /// discord.js counterpart: `find`.
    pub fn find(&self, mut predicate: impl FnMut(&V, &K) -> bool) -> Option<&V> {
        self.iter()
            .find(|&(key, value)| predicate(value, key))
            .map(|(_, value)| value)
    }

    /// The key of the first value for which `predicate(value, key)` is true.
    ///
    /// discord.js counterpart: `findKey`.
    pub fn find_key(&self, mut predicate: impl FnMut(&V, &K) -> bool) -> Option<&K> {
        self.iter()
            .find(|&(key, value)| predicate(value, key))
            .map(|(key, _)| key)
    }

    /// The last value for which `predicate(value, key)` is true.
    ///
    /// discord.js counterpart: `findLast`.
    pub fn find_last(&self, mut predicate: impl FnMut(&V, &K) -> bool) -> Option<&V> {
        self.iter()
            .rev()
            .find(|&(key, value)| predicate(value, key))
            .map(|(_, value)| value)
    }

    /// The key of the last value for which `predicate(value, key)` is true.
    ///
    /// discord.js counterpart: `findLastKey`.
    pub fn find_last_key(&self, mut predicate: impl FnMut(&V, &K) -> bool) -> Option<&K> {
        self.iter()
            .rev()
            .find(|&(key, value)| predicate(value, key))
            .map(|(key, _)| key)
    }

    /// `f(value, key)` of every entry, in order.
    ///
    /// discord.js counterpart: `map`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_collection::Collection;
    ///
    /// let users: Collection<u64, &str> = [(1, "ada"), (2, "grace")].into();
    /// assert_eq!(users.map(|name, id| format!("{id}:{name}")), ["1:ada", "2:grace"]);
    /// ```
    pub fn map<T>(&self, mut f: impl FnMut(&V, &K) -> T) -> Vec<T> {
        self.iter().map(|(key, value)| f(value, key)).collect()
    }

    /// Reverses the order of the entries.
    ///
    /// discord.js counterpart: `reverse`.
    pub fn reverse(&mut self) {
        self.map.reverse();
    }

    /// The collection with its entries in reverse order.
    ///
    /// discord.js counterpart: `toReversed`.
    #[must_use]
    pub fn to_reversed(mut self) -> Self {
        self.reverse();
        self
    }

    /// Sorts the entries with `compare(value_a, value_b, key_a, key_b)`, keeping the order of
    /// equal ones.
    ///
    /// discord.js counterpart: `sort(compareFunction)`.
    pub fn sort_by(&mut self, mut compare: impl FnMut(&V, &V, &K, &K) -> Ordering) {
        self.map
            .sort_by(|key_a, value_a, key_b, value_b| compare(value_a, value_b, key_a, key_b));
    }

    /// The collection sorted with `compare(value_a, value_b, key_a, key_b)`.
    ///
    /// discord.js counterpart: `toSorted(compareFunction)`.
    #[must_use]
    pub fn to_sorted_by(mut self, compare: impl FnMut(&V, &V, &K, &K) -> Ordering) -> Self {
        self.sort_by(compare);
        self
    }

    /// Sorts the entries by value, keeping the order of equal ones.
    ///
    /// discord.js counterpart: `sort()` (which compares values as strings).
    pub fn sort(&mut self)
    where
        V: Ord,
    {
        self.map.sort_by(|_, a, _, b| a.cmp(b));
    }

    /// The collection sorted by value.
    ///
    /// discord.js counterpart: `toSorted()`.
    #[must_use]
    pub fn to_sorted(mut self) -> Self
    where
        V: Ord,
    {
        self.sort();
        self
    }
}

impl<K, V, S> Collection<K, V, S>
where
    K: Hash + Eq,
    S: BuildHasher,
{
    /// The value of `key`.
    ///
    /// discord.js counterpart: `get`.
    #[must_use]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.get(key)
    }

    /// The value of `key`, mutable.
    #[must_use]
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.get_mut(key)
    }

    /// The stored key equal to `key`, with its value.
    #[must_use]
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.get_key_value(key)
    }

    /// Whether `key` has a value.
    ///
    /// discord.js counterpart: `has`.
    #[must_use]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.contains_key(key)
    }

    /// The position of `key` in insertion order.
    #[must_use]
    pub fn index_of<Q>(&self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.get_index_of(key)
    }

    /// Sets the value of `key`, returning the previous one. A new key goes last; an existing
    /// key keeps its position.
    ///
    /// discord.js counterpart: `set`.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.map.insert(key, value)
    }

    /// Removes `key`, returning its value; the entries after it move up, so the order is
    /// kept (O(n), D-215). [`swap_remove`](Self::swap_remove) is O(1).
    ///
    /// To remove many entries, use one [`sweep`](Self::sweep) or [`retain`](Self::retain)
    /// pass, which is linear however many it removes: removing 1 000 entries near the front
    /// of a million one by one moves the rest 1 000 times (seconds instead of milliseconds).
    ///
    /// discord.js counterpart: `delete`.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.shift_remove(key)
    }

    /// Removes `key`, returning the stored key and its value, keeping the order.
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.shift_remove_entry(key)
    }

    /// Removes `key` in O(1) by moving the last entry into its place, so the order changes.
    pub fn swap_remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.swap_remove(key)
    }

    /// The value of `key`, inserting `default(&key)` first if there is none.
    ///
    /// discord.js counterpart: `ensure`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_collection::Collection;
    ///
    /// let mut warnings: Collection<u64, u32> = Collection::new();
    /// *warnings.ensure(1, |_| 0) += 1;
    /// *warnings.ensure(1, |_| 0) += 1;
    /// assert_eq!(warnings.get(&1), Some(&2));
    /// ```
    pub fn ensure(&mut self, key: K, default: impl FnOnce(&K) -> V) -> &mut V {
        self.map.entry(key).or_insert_with_key(default)
    }

    /// Whether every key of `keys` has a value.
    ///
    /// discord.js counterpart: `hasAll`.
    pub fn has_all<'q, Q>(&self, keys: impl IntoIterator<Item = &'q Q>) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized + 'q,
    {
        keys.into_iter().all(|key| self.contains_key(key))
    }

    /// Whether any key of `keys` has a value.
    ///
    /// discord.js counterpart: `hasAny`.
    pub fn has_any<'q, Q>(&self, keys: impl IntoIterator<Item = &'q Q>) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized + 'q,
    {
        keys.into_iter().any(|key| self.contains_key(key))
    }

    /// Keeps only the entries for which `keep(&key, &mut value)` is true, in one pass that
    /// keeps the order.
    pub fn retain(&mut self, mut keep: impl FnMut(&K, &mut V) -> bool) {
        self.map.retain(|key, value| keep(key, value));
    }

    /// Removes the entries for which `remove(value, key)` is true, in one pass that keeps the
    /// order, and returns how many were removed.
    ///
    /// discord.js counterpart: `sweep`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_collection::Collection;
    ///
    /// let mut ages: Collection<&str, u32> = [("a", 3), ("b", 40), ("c", 7)].into();
    /// assert_eq!(ages.sweep(|&age, _| age < 10), 2);
    /// assert_eq!(ages.len(), 1);
    /// ```
    pub fn sweep(&mut self, mut remove: impl FnMut(&V, &K) -> bool) -> usize {
        let before = self.len();
        self.map.retain(|key, value| !remove(value, key));
        before - self.len()
    }

    /// The entries for which `keep(value, key)` is true, in order (D-214: the collection is
    /// consumed).
    ///
    /// discord.js counterpart: `filter`.
    #[must_use]
    pub fn filter(mut self, keep: impl FnMut(&V, &K) -> bool) -> Self {
        self.sweep(not(keep));
        self
    }

    /// Splits the entries in two, in order: those for which `predicate(value, key)` is true,
    /// and the others.
    ///
    /// discord.js counterpart: `partition`.
    #[must_use]
    pub fn partition(self, mut predicate: impl FnMut(&V, &K) -> bool) -> (Self, Self)
    where
        S: Clone,
    {
        // The passing entries move to a new map; the others stay where they are, unhashed.
        let mut failed = self.map;
        let mut passed = IndexMap::with_hasher(failed.hasher().clone());
        passed.extend(failed.extract_if(.., |key, value| predicate(value, key)));
        (Self { map: passed }, Self { map: failed })
    }

    /// A collection with the same keys and the values `f(value, &key)`, in order.
    ///
    /// discord.js counterpart: `mapValues`.
    #[must_use]
    pub fn map_values<T>(self, mut f: impl FnMut(V, &K) -> T) -> Collection<K, T, S>
    where
        S: Clone,
    {
        let mut mapped =
            Collection::with_capacity_and_hasher(self.len(), self.map.hasher().clone());
        for (key, value) in self.map {
            let value = f(value, &key);
            mapped.map.insert(key, value);
        }
        mapped
    }

    /// The collections `f(value, key)` of every entry, joined in order; a later value of a
    /// key replaces an earlier one.
    ///
    /// discord.js counterpart: `flatMap`.
    #[must_use]
    pub fn flat_map<K2, V2, S2>(
        self,
        mut f: impl FnMut(V, K) -> Collection<K2, V2, S2>,
    ) -> Collection<K2, V2, S2>
    where
        K2: Hash + Eq,
        S2: BuildHasher + Default,
    {
        let mut joined = Collection::with_hasher(S2::default());
        for (key, value) in self.map {
            joined.map.extend(f(value, key).map);
        }
        joined
    }

    /// The collection with the entries of `others` added in order; an existing key gets the
    /// later value and keeps its position.
    ///
    /// discord.js counterpart: `concat`.
    #[must_use]
    pub fn concat<S2>(mut self, others: impl IntoIterator<Item = Collection<K, V, S2>>) -> Self {
        for other in others {
            self.map.extend(other.map);
        }
        self
    }

    /// The entries of the collection, then those of `other` whose keys it does not have.
    ///
    /// discord.js counterpart: `union`.
    #[must_use]
    pub fn union<S2>(mut self, other: Collection<K, V, S2>) -> Self {
        for (key, value) in other.map {
            self.map.entry(key).or_insert(value);
        }
        self
    }

    /// The entries whose keys `other` also has, in this collection's order.
    ///
    /// discord.js counterpart: `intersection`.
    #[must_use]
    pub fn intersection<V2, S2: BuildHasher>(mut self, other: &Collection<K, V2, S2>) -> Self {
        self.map.retain(|key, _| other.contains_key(key));
        self
    }

    /// The entries whose keys `other` does not have.
    ///
    /// discord.js counterpart: `difference`.
    #[must_use]
    pub fn difference<V2, S2: BuildHasher>(mut self, other: &Collection<K, V2, S2>) -> Self {
        self.map.retain(|key, _| !other.contains_key(key));
        self
    }

    /// The entries whose keys only one of the two collections has: this one's, then
    /// `other`'s.
    ///
    /// discord.js counterpart: `symmetricDifference`.
    #[must_use]
    pub fn symmetric_difference<S2: BuildHasher>(mut self, other: Collection<K, V, S2>) -> Self {
        let mut other = other.map;
        let mut shared = vec![false; other.len()];
        self.map.retain(|key, _| match other.get_index_of(key) {
            Some(index) => {
                shared[index] = true;
                false
            }
            None => true,
        });
        let mut index = 0;
        other.retain(|_, _| {
            let keep = !shared[index];
            index += 1;
            keep
        });
        self.map.extend(other);
        self
    }

    /// Combines this collection with `other` key by key, in the order of this collection's
    /// keys and then `other`'s new ones: `in_self(value, key)` for a key only here,
    /// `in_other(value, key)` for a key only in `other`, `in_both(value, other_value, key)`
    /// for both. A key is kept when its function returns `Some`.
    ///
    /// discord.js counterpart: `merge` (`Some(value)` is `{ keep: true, value }`).
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_collection::Collection;
    ///
    /// let before: Collection<&str, u32> = [("ada", 1), ("grace", 2)].into();
    /// let after: Collection<&str, u32> = [("grace", 5), ("linus", 3)].into();
    /// let changes = before.merge(
    ///     after,
    ///     |_, _| Some("left"),
    ///     |_, _| Some("joined"),
    ///     |old, new, _| (old != new).then_some("changed"),
    /// );
    /// assert_eq!(changes.iter().collect::<Vec<_>>(), [(&"ada", &"left"), (&"grace", &"changed"), (&"linus", &"joined")]);
    /// ```
    #[must_use]
    pub fn merge<O, R, S2: BuildHasher>(
        self,
        other: Collection<K, O, S2>,
        mut in_self: impl FnMut(V, &K) -> Option<R>,
        mut in_other: impl FnMut(O, &K) -> Option<R>,
        mut in_both: impl FnMut(V, O, &K) -> Option<R>,
    ) -> Collection<K, R, S>
    where
        S: Clone,
    {
        let matches: Vec<Option<usize>> = self
            .map
            .keys()
            .map(|key| other.map.get_index_of(key))
            .collect();
        let mut others: Vec<Option<(K, O)>> = other.map.into_iter().map(Some).collect();
        let mut merged = Collection::with_capacity_and_hasher(
            self.map.len().max(others.len()),
            self.map.hasher().clone(),
        );
        for ((key, value), matched) in self.map.into_iter().zip(matches) {
            let result = match matched.and_then(|index| others[index].take()) {
                Some((_, other_value)) => in_both(value, other_value, &key),
                None => in_self(value, &key),
            };
            if let Some(result) = result {
                merged.map.insert(key, result);
            }
        }
        for (key, value) in others.into_iter().flatten() {
            if let Some(result) = in_other(value, &key) {
                merged.map.insert(key, result);
            }
        }
        merged
    }

    /// A collection of `entries` in order, where a repeated key gets
    /// `combine(earlier_value, later_value, key)`.
    ///
    /// discord.js counterpart: `Collection.combineEntries`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_collection::Collection;
    ///
    /// let votes = [("yes", 1), ("no", 1), ("yes", 1)];
    /// let totals: Collection<&str, u32> = Collection::combine_entries(votes, |a, b, _| a + b);
    /// assert_eq!(totals.get("yes"), Some(&2));
    /// ```
    #[must_use]
    pub fn combine_entries(
        entries: impl IntoIterator<Item = (K, V)>,
        mut combine: impl FnMut(V, V, &K) -> V,
    ) -> Self
    where
        S: Default,
    {
        let mut combined: IndexMap<K, Option<V>, S> = IndexMap::default();
        for (key, value) in entries {
            match combined.entry(key) {
                indexmap::map::Entry::Occupied(mut entry) => {
                    let earlier = entry.get_mut().take();
                    let value = match earlier {
                        Some(earlier) => combine(earlier, value, entry.key()),
                        None => value,
                    };
                    *entry.get_mut() = Some(value);
                }
                indexmap::map::Entry::Vacant(entry) => {
                    entry.insert(Some(value));
                }
            }
        }
        let mut collection = Self::with_capacity_and_hasher(combined.len(), S::default());
        collection.map.extend(
            combined
                .into_iter()
                .filter_map(|(key, value)| Some((key, value?))),
        );
        collection
    }
}

/// Negates a predicate.
fn not<V, K>(mut predicate: impl FnMut(&V, &K) -> bool) -> impl FnMut(&V, &K) -> bool {
    move |value, key| !predicate(value, key)
}

impl<K, V, S: Default> Default for Collection<K, V, S> {
    fn default() -> Self {
        Self::with_hasher(S::default())
    }
}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for Collection<K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// Two collections are equal when they have the same keys with equal values, in any order
/// (as discord.js' `equals`).
impl<K, V1, S1, V2, S2> PartialEq<Collection<K, V2, S2>> for Collection<K, V1, S1>
where
    K: Hash + Eq,
    V1: PartialEq<V2>,
    S1: BuildHasher,
    S2: BuildHasher,
{
    fn eq(&self, other: &Collection<K, V2, S2>) -> bool {
        self.map == other.map
    }
}

impl<K: Hash + Eq, V: Eq, S: BuildHasher> Eq for Collection<K, V, S> {}

/// Indexing by key.
///
/// # Panics
///
/// If the key has no value; use [`Collection::get`] when it may not.
impl<K, V, Q, S> Index<&Q> for Collection<K, V, S>
where
    K: Hash + Eq + Borrow<Q>,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        self.get(key)
            .expect("the collection has no entry for this key")
    }
}

impl<K: Hash + Eq, V, S: BuildHasher + Default> FromIterator<(K, V)> for Collection<K, V, S> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(entries: I) -> Self {
        Self {
            map: IndexMap::from_iter(entries),
        }
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> Extend<(K, V)> for Collection<K, V, S> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, entries: I) {
        self.map.extend(entries);
    }
}

impl<K: Hash + Eq, V, const N: usize> From<[(K, V); N]> for Collection<K, V> {
    fn from(entries: [(K, V); N]) -> Self {
        entries.into_iter().collect()
    }
}

impl<K, V, S> From<IndexMap<K, V, S>> for Collection<K, V, S> {
    fn from(map: IndexMap<K, V, S>) -> Self {
        Self { map }
    }
}

impl<K, V, S> From<Collection<K, V, S>> for IndexMap<K, V, S> {
    fn from(collection: Collection<K, V, S>) -> Self {
        collection.map
    }
}

impl<K, V, S> IntoIterator for Collection<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.map.into_iter())
    }
}

impl<'a, K, V, S> IntoIterator for &'a Collection<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a mut Collection<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
