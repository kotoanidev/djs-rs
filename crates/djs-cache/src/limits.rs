//! Enforcing [`Limit`]s on top of a [`Store`] (D-157, D-159).
//!
//! A resource counted over the whole cache is a [`Global`]; one counted per guild or channel
//! is a [`Scoped`], which stores its entries under `(parent, key)` and indexes them by parent
//! (the cache needs the index anyway, for example to drop a deleted guild's members). Only
//! limited resources pay for the insertion order. An [`Index`] lists the children of a parent
//! for a [`Global`] resource that belongs to one (channels of a guild).

use core::{hash::Hash, marker::PhantomData};
use std::{
    collections::{HashMap, HashSet, VecDeque, hash_map::Entry},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use dashmap::DashMap;

use crate::{Limit, Store, dashmap_store::Hasher};

/// A resource kept over the whole cache, in backend `B`'s store.
pub(crate) type GlobalOf<K, V, B> = Global<K, V, <B as crate::Backend>::Store<K, V>>;
/// A resource kept per guild, in backend `B`'s store keyed by `(guild, id)`.
pub(crate) type ScopedOf<K, V, B> = Scoped<
    djs_model::id::GuildId,
    K,
    V,
    <B as crate::Backend>::Store<(djs_model::id::GuildId, K), V>,
>;

/// A resource kept per channel (messages), in backend `B`'s store keyed by `(channel, id)`.
pub(crate) type ChannelScopedOf<K, V, B> = Scoped<
    djs_model::id::ChannelId,
    K,
    V,
    <B as crate::Backend>::Store<(djs_model::id::ChannelId, K), V>,
>;

/// The result of an insertion.
#[derive(Debug)]
pub(crate) struct Inserted<K, V> {
    /// The value the key had before, if any.
    pub(crate) previous: Option<Arc<V>>,
    /// The entry pushed out to stay within the limit, if any.
    pub(crate) evicted: Option<(K, Arc<V>)>,
}

impl<K, V> Inserted<K, V> {
    const fn replaced(previous: Option<Arc<V>>) -> Self {
        Self {
            previous,
            evicted: None,
        }
    }
}

/// Keys in insertion order, with removal in amortised O(1).
///
/// A removed key stays in `queue` until it reaches the front or the queue is compacted; the
/// sequence number tells a stale queue entry from a key inserted again later.
#[derive(Debug)]
struct Order<K> {
    live: HashMap<K, u64, Hasher>,
    queue: VecDeque<(K, u64)>,
    next: u64,
}

impl<K: Eq + Hash + Copy> Order<K> {
    fn new() -> Self {
        Self {
            live: HashMap::with_hasher(Hasher::default()),
            queue: VecDeque::new(),
            next: 0,
        }
    }

    /// Adds `key` as the newest entry. A key already present keeps its place (D-157).
    /// Returns whether the key is new.
    fn insert(&mut self, key: K) -> bool {
        match self.live.entry(key) {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                let sequence = self.next;
                self.next += 1;
                entry.insert(sequence);
                self.queue.push_back((key, sequence));
                true
            }
        }
    }

    fn remove(&mut self, key: &K) -> bool {
        let removed = self.live.remove(key).is_some();
        // Stale entries may take at most as much room as live ones (plus some slack), so the
        // queue stays O(live) and compacting costs O(1) per removal on average.
        if removed && self.queue.len() > 2 * self.live.len() + 32 {
            let live = &self.live;
            self.queue
                .retain(|(key, sequence)| live.get(key) == Some(sequence));
        }
        removed
    }

    /// Removes and returns the oldest key.
    fn pop_oldest(&mut self) -> Option<K> {
        while let Some((key, sequence)) = self.queue.pop_front() {
            if self.live.get(&key) == Some(&sequence) {
                self.live.remove(&key);
                return Some(key);
            }
        }
        None
    }

    fn len(&self) -> usize {
        self.live.len()
    }

    fn keys(&self) -> impl Iterator<Item = K> + '_ {
        self.live.keys().copied()
    }
}

/// The keys of one guild or channel: a plain set, or an [`Order`] when the resource is
/// limited.
#[derive(Debug)]
enum Children<K> {
    Unordered(HashSet<K, Hasher>),
    Ordered(Order<K>),
}

impl<K: Eq + Hash + Copy> Children<K> {
    fn new(ordered: bool) -> Self {
        if ordered {
            Self::Ordered(Order::new())
        } else {
            Self::Unordered(HashSet::with_hasher(Hasher::default()))
        }
    }

    fn insert(&mut self, key: K) {
        match self {
            Self::Unordered(keys) => {
                keys.insert(key);
            }
            Self::Ordered(order) => {
                order.insert(key);
            }
        }
    }

    fn remove(&mut self, key: &K) {
        match self {
            Self::Unordered(keys) => {
                keys.remove(key);
            }
            Self::Ordered(order) => {
                order.remove(key);
            }
        }
    }

    fn pop_oldest(&mut self) -> Option<K> {
        match self {
            Self::Unordered(_) => None,
            Self::Ordered(order) => order.pop_oldest(),
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Unordered(keys) => keys.len(),
            Self::Ordered(order) => order.len(),
        }
    }

    fn keys(&self) -> Vec<K> {
        match self {
            Self::Unordered(keys) => keys.iter().copied().collect(),
            Self::Ordered(order) => order.keys().collect(),
        }
    }
}

/// A resource whose limit counts over the whole cache (guilds, channels, users).
#[derive(Debug)]
pub(crate) struct Global<K, V, S> {
    store: S,
    /// The maximum and the insertion order, for a limited resource. Inserts and removals
    /// hold the lock around the store call, so the order always matches the store.
    limited: Option<(usize, Mutex<Order<K>>)>,
    value: PhantomData<fn() -> V>,
}

impl<K, V, S> Global<K, V, S>
where
    K: Eq + Hash + Copy,
    S: Store<K, V>,
{
    /// Wraps `store`, created for an enabled `limit`.
    pub(crate) fn new(store: S, limit: Limit) -> Self {
        Self {
            store,
            limited: limit.max().map(|max| (max, Mutex::new(Order::new()))),
            value: PhantomData,
        }
    }

    pub(crate) fn get(&self, key: &K) -> Option<Arc<V>> {
        self.store.get(key)
    }

    pub(crate) fn insert(&self, key: K, value: Arc<V>) -> Inserted<K, V> {
        let Some((max, order)) = &self.limited else {
            return Inserted::replaced(self.store.insert(key, value));
        };
        let mut order = lock(order);
        let previous = self.store.insert(key, value);
        let mut evicted = None;
        if order.insert(key) && order.len() > *max {
            evicted = order
                .pop_oldest()
                .and_then(|oldest| self.store.remove(&oldest).map(|value| (oldest, value)));
        }
        Inserted { previous, evicted }
    }

    pub(crate) fn remove(&self, key: &K) -> Option<Arc<V>> {
        match &self.limited {
            None => self.store.remove(key),
            Some((_, order)) => {
                let mut order = lock(order);
                order.remove(key);
                self.store.remove(key)
            }
        }
    }

    pub(crate) fn contains(&self, key: &K) -> bool {
        self.store.contains_key(key)
    }

    /// Removes every entry for which `sweep` returns `true`, returning how many it removed.
    pub(crate) fn sweep(&self, sweep: &mut dyn FnMut(&K, &V) -> bool) -> usize {
        let mut removed = Vec::new();
        // A limited resource holds its order across the sweep, as inserts do.
        let mut order = self.limited.as_ref().map(|(_, order)| lock(order));
        self.store.retain(&mut |key, value| {
            let sweep = sweep(key, value);
            if sweep {
                removed.push(*key);
            }
            !sweep
        });
        if let Some(order) = order.as_mut() {
            for key in &removed {
                order.remove(key);
            }
        }
        removed.len()
    }

    pub(crate) fn keys(&self) -> Vec<K> {
        self.store.keys()
    }

    pub(crate) fn values(&self) -> Vec<Arc<V>> {
        self.store.values()
    }

    pub(crate) fn len(&self) -> usize {
        self.store.len()
    }
}

/// A resource whose limit counts per guild or per channel (`P`, the parent's ID). Entries are
/// stored under `(parent, key)`, so a lookup is exact without consulting the index.
pub(crate) struct Scoped<P, K, V, S> {
    store: S,
    max: Option<usize>,
    /// Each parent's keys. Every change to a parent's entries holds that parent's index entry
    /// (a `DashMap` shard lock) around the store call, so the index matches the store. The
    /// store is never called with a store guard held, so the lock order is always index →
    /// store.
    index: DashMap<P, Children<K>, Hasher>,
    value: PhantomData<fn() -> V>,
}

impl<P, K, V, S> Scoped<P, K, V, S>
where
    P: Eq + Hash + Copy,
    K: Eq + Hash + Copy,
    S: Store<(P, K), V>,
{
    /// Wraps `store`, created for an enabled `limit`.
    pub(crate) fn new(store: S, limit: Limit) -> Self {
        Self {
            store,
            max: limit.max(),
            index: DashMap::with_hasher(Hasher::default()),
            value: PhantomData,
        }
    }

    pub(crate) fn get(&self, parent: P, key: K) -> Option<Arc<V>> {
        self.store.get(&(parent, key))
    }

    pub(crate) fn insert(&self, parent: P, key: K, value: Arc<V>) -> Inserted<K, V> {
        let mut children = self
            .index
            .entry(parent)
            .or_insert_with(|| Children::new(self.max.is_some()));
        let previous = self.store.insert((parent, key), value);
        children.insert(key);
        let mut evicted = None;
        if let Some(max) = self.max
            && children.len() > max
        {
            evicted = children.pop_oldest().and_then(|oldest| {
                self.store
                    .remove(&(parent, oldest))
                    .map(|value| (oldest, value))
            });
        }
        Inserted { previous, evicted }
    }

    pub(crate) fn remove(&self, parent: P, key: K) -> Option<Arc<V>> {
        let Some(mut children) = self.index.get_mut(&parent) else {
            return self.store.remove(&(parent, key));
        };
        children.remove(&key);
        let removed = self.store.remove(&(parent, key));
        let now_empty = children.len() == 0;
        // The guard must go before `remove_if`, which locks the same shard.
        drop(children);
        if now_empty {
            self.index
                .remove_if(&parent, |_, children| children.len() == 0);
        }
        removed
    }

    /// Removes every entry of `parent` (for example a deleted guild's members), returning
    /// them.
    pub(crate) fn remove_parent(&self, parent: P) -> Vec<Arc<V>> {
        let Some((_, children)) = self.index.remove(&parent) else {
            return Vec::new();
        };
        children
            .keys()
            .into_iter()
            .filter_map(|key| self.store.remove(&(parent, key)))
            .collect()
    }

    /// Returns the keys of `parent`'s entries, in no particular order.
    pub(crate) fn keys_of(&self, parent: P) -> Vec<K> {
        self.index
            .get(&parent)
            .map(|children| children.keys())
            .unwrap_or_default()
    }

    /// Returns the values of `parent`'s entries, in no particular order.
    pub(crate) fn values_of(&self, parent: P) -> Vec<Arc<V>> {
        self.keys_of(parent)
            .into_iter()
            .filter_map(|key| self.store.get(&(parent, key)))
            .collect()
    }

    /// Removes every entry for which `sweep(parent, value)` returns `true`, returning how
    /// many it removed. Each parent is swept holding its index entry, as inserts do.
    pub(crate) fn sweep(&self, sweep: &mut dyn FnMut(P, &V) -> bool) -> usize {
        let parents: Vec<P> = self.index.iter().map(|entry| *entry.key()).collect();
        let mut removed = 0;
        for parent in parents {
            let Some(mut children) = self.index.get_mut(&parent) else {
                continue;
            };
            for key in children.keys() {
                if let Some(value) = self.store.get(&(parent, key))
                    && sweep(parent, &value)
                {
                    self.store.remove(&(parent, key));
                    children.remove(&key);
                    removed += 1;
                }
            }
            let now_empty = children.len() == 0;
            drop(children);
            if now_empty {
                self.index
                    .remove_if(&parent, |_, children| children.len() == 0);
            }
        }
        removed
    }

    /// Returns the number of `parent`'s entries.
    pub(crate) fn len_of(&self, parent: P) -> usize {
        self.index.get(&parent).map_or(0, |children| children.len())
    }

    /// Returns the number of entries over all parents.
    pub(crate) fn len(&self) -> usize {
        self.store.len()
    }
}

/// The keys of each parent, for a [`Global`] resource whose entries belong to a parent (the
/// channels of a guild). Only a listing: the limit counts over the whole cache.
pub(crate) struct Index<P, K> {
    map: DashMap<P, HashSet<K, Hasher>, Hasher>,
}

impl<P: Eq + Hash + Copy, K: Eq + Hash + Copy> Index<P, K> {
    pub(crate) fn new() -> Self {
        Self {
            map: DashMap::with_hasher(Hasher::default()),
        }
    }

    pub(crate) fn insert(&self, parent: P, key: K) {
        self.map
            .entry(parent)
            .or_insert_with(|| HashSet::with_hasher(Hasher::default()))
            .insert(key);
    }

    pub(crate) fn remove(&self, parent: P, key: K) {
        let now_empty = self.map.get_mut(&parent).is_some_and(|mut keys| {
            keys.remove(&key);
            keys.is_empty()
        });
        if now_empty {
            self.map.remove_if(&parent, |_, keys| keys.is_empty());
        }
    }

    /// Removes `parent`, returning its keys.
    pub(crate) fn remove_parent(&self, parent: P) -> Vec<K> {
        self.map
            .remove(&parent)
            .map(|(_, keys)| keys.into_iter().collect())
            .unwrap_or_default()
    }

    pub(crate) fn keys_of(&self, parent: P) -> Vec<K> {
        self.map
            .get(&parent)
            .map(|keys| keys.iter().copied().collect())
            .unwrap_or_default()
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use super::{Global, Index, Order, Scoped};
    use crate::{DashMapStore, Limit, Store};

    fn sorted(mut keys: Vec<u64>) -> Vec<u64> {
        keys.sort_unstable();
        keys
    }

    #[test]
    fn order_pops_in_insertion_order() {
        let mut order = Order::new();
        for key in [3, 1, 2] {
            assert!(order.insert(key));
        }
        // Inserting an existing key does not move it.
        assert!(!order.insert(3));
        assert_eq!(order.pop_oldest(), Some(3));
        assert_eq!(order.pop_oldest(), Some(1));
        assert_eq!(order.pop_oldest(), Some(2));
        assert_eq!(order.pop_oldest(), None);
    }

    #[test]
    fn order_skips_removed_keys_and_reinsertions() {
        let mut order = Order::new();
        order.insert(1);
        order.insert(2);
        order.remove(&1);
        // Inserted again: now the newest, not the oldest.
        order.insert(1);
        assert_eq!(order.len(), 2);
        assert_eq!(order.pop_oldest(), Some(2));
        assert_eq!(order.pop_oldest(), Some(1));
        assert_eq!(order.pop_oldest(), None);
    }

    #[test]
    fn order_compacts_stale_entries() {
        let mut order = Order::new();
        for key in 0..10_000_u64 {
            order.insert(key);
        }
        for key in 0..9_990 {
            order.remove(&key);
        }
        assert_eq!(order.len(), 10);
        assert!(order.queue.len() <= 2 * 10 + 32, "{}", order.queue.len());
        assert_eq!(order.pop_oldest(), Some(9_990));
    }

    #[test]
    fn global_unlimited_keeps_everything() {
        let global = Global::new(DashMapStore::new(), Limit::Unlimited);
        for key in 0..1_000_u64 {
            assert!(global.insert(key, Arc::new(key)).evicted.is_none());
        }
        assert_eq!(global.len(), 1_000);
    }

    #[test]
    fn global_limited_evicts_the_oldest_insertion() {
        let global = Global::new(DashMapStore::new(), Limit::Max(3));
        for key in 1..=3_u64 {
            global.insert(key, Arc::new(key));
        }
        // An update keeps its place and evicts nothing.
        let updated = global.insert(1, Arc::new(10));
        assert_eq!(updated.previous.as_deref(), Some(&1));
        assert!(updated.evicted.is_none());

        let inserted = global.insert(4, Arc::new(4));
        assert!(inserted.previous.is_none());
        let (key, value) = inserted.evicted.unwrap();
        assert_eq!((key, *value), (1, 10));
        assert_eq!(global.len(), 3);
        assert!(global.get(&1).is_none());

        // A removed key frees its room.
        assert_eq!(global.remove(&3).as_deref(), Some(&3));
        assert!(global.insert(5, Arc::new(5)).evicted.is_none());
        assert_eq!(
            global.insert(6, Arc::new(6)).evicted.map(|(key, _)| key),
            Some(2)
        );
    }

    #[test]
    fn global_limit_holds_under_concurrent_inserts() {
        let global = Arc::new(Global::new(DashMapStore::new(), Limit::Max(1_000)));
        let threads: Vec<_> = (0..8_u64)
            .map(|thread| {
                let global = Arc::clone(&global);
                thread::spawn(move || {
                    for key in 0..20_000 {
                        global.insert(thread * 20_000 + key, Arc::new(key));
                        if key % 3 == 0 {
                            global.remove(&(thread * 20_000 + key / 2));
                        }
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(global.len() <= 1_000, "{}", global.len());
        let order = super::lock(&global.limited.as_ref().unwrap().1);
        assert_eq!(order.len(), global.len());
    }

    #[test]
    fn scoped_limits_count_per_parent() {
        let scoped = Scoped::new(DashMapStore::new(), Limit::Max(2));
        // Parent 1 gets three entries, parent 2 one.
        for (parent, key) in [(1, 10), (1, 11), (2, 20), (1, 12)] {
            scoped.insert(parent, key, Arc::new(key));
        }
        assert_eq!(sorted(scoped.keys_of(1)), [11, 12]);
        assert_eq!(scoped.keys_of(2), [20]);
        assert_eq!(scoped.len(), 3);
        assert!(scoped.get(1, 10).is_none());
        assert_eq!(scoped.get(1, 11).as_deref(), Some(&11));
        // A key is found under its own parent only.
        assert!(scoped.get(2, 11).is_none());
    }

    #[test]
    fn scoped_eviction_returns_the_oldest_of_the_parent() {
        let scoped = Scoped::new(DashMapStore::new(), Limit::Max(1));
        scoped.insert(1_u64, 10_u64, Arc::new("a"));
        let inserted = scoped.insert(1, 11, Arc::new("b"));
        let (key, value) = inserted.evicted.unwrap();
        assert_eq!((key, *value), (10, "a"));
    }

    #[test]
    fn scoped_remove_and_remove_parent() {
        let scoped = Scoped::new(DashMapStore::new(), Limit::Unlimited);
        for key in 0..10_u64 {
            scoped.insert(key % 2, key, Arc::new(key));
        }
        assert_eq!(scoped.remove(0, 4).as_deref(), Some(&4));
        assert_eq!(scoped.len_of(0), 4);
        assert_eq!(
            sorted(scoped.values_of(0).iter().map(|value| **value).collect()),
            [0, 2, 6, 8]
        );

        let removed: Vec<u64> = scoped
            .remove_parent(1)
            .into_iter()
            .map(|value| *value)
            .collect();
        assert_eq!(sorted(removed), [1, 3, 5, 7, 9]);
        assert_eq!(scoped.len(), 4);
        assert_eq!(scoped.len_of(1), 0);
        assert_eq!(scoped.remove_parent(1).len(), 0);
    }

    #[test]
    fn an_emptied_parent_leaves_the_index() {
        let scoped = Scoped::new(DashMapStore::new(), Limit::Max(5));
        scoped.insert(7_u64, 1_u64, Arc::new(()));
        scoped.remove(7, 1);
        assert!(scoped.index.is_empty());
        // Removing a key of an unindexed parent still removes it from the store.
        let store_only = Scoped::new(DashMapStore::new(), Limit::Unlimited);
        store_only.store.insert((9_u64, 1_u64), Arc::new(()));
        assert!(store_only.remove(9, 1).is_some());
    }

    #[test]
    fn scoped_limit_holds_under_concurrent_inserts() {
        let scoped = Arc::new(Scoped::new(DashMapStore::new(), Limit::Max(100)));
        let threads: Vec<_> = (0..8_u64)
            .map(|thread| {
                let scoped = Arc::clone(&scoped);
                thread::spawn(move || {
                    for key in 0..10_000 {
                        // Every thread writes into the same 4 parents.
                        scoped.insert(key % 4, thread * 10_000 + key, Arc::new(key));
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        for parent in 0..4 {
            assert_eq!(scoped.len_of(parent), 100);
        }
        assert_eq!(scoped.len(), 400);
    }

    #[test]
    fn sweeps_keep_the_order_and_index_consistent() {
        let global = Global::new(DashMapStore::new(), Limit::Max(10));
        for key in 0..10_u64 {
            global.insert(key, Arc::new(key));
        }
        assert_eq!(global.sweep(&mut |key, _| key % 2 == 0), 5);
        assert_eq!(global.len(), 5);
        // The swept keys left the order: five more fit before anything is evicted.
        for key in 10..15 {
            assert!(global.insert(key, Arc::new(key)).evicted.is_none());
        }
        assert_eq!(
            global.insert(15, Arc::new(15)).evicted.map(|(key, _)| key),
            Some(1)
        );

        let scoped = Scoped::new(DashMapStore::new(), Limit::Unlimited);
        for key in 0..10_u64 {
            scoped.insert(key % 2, key, Arc::new(key));
        }
        assert_eq!(
            scoped.sweep(&mut |parent, value| parent == 1 || *value == 0),
            6
        );
        assert_eq!(sorted(scoped.keys_of(0)), [2, 4, 6, 8]);
        assert_eq!(scoped.len_of(1), 0);
        assert!(scoped.index.get(&1).is_none());
    }

    #[test]
    fn index_lists_and_forgets_children() {
        let index = Index::new();
        index.insert(1_u64, 10_u64);
        index.insert(1, 11);
        index.insert(2, 20);
        assert_eq!(sorted(index.keys_of(1)), [10, 11]);
        index.remove(2, 20);
        assert!(index.map.get(&2).is_none());
        assert_eq!(sorted(index.remove_parent(1)), [10, 11]);
        assert_eq!(index.keys_of(1).len(), 0);
    }
}
