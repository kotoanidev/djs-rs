//! The default backend: a [`DashMap`] per resource.

use core::{fmt, hash::Hash};
use std::sync::Arc;

use dashmap::DashMap;

use crate::{Backend, Limit, Resource, Store};

/// The hasher of every map in this crate (D-158): a few nanoseconds per snowflake, with
/// well-mixed high bits (`DashMap` picks a shard from them) and minimal resistance to
/// hash flooding.
pub(crate) type Hasher = foldhash::fast::RandomState;

/// The default [`Backend`]: every resource is a [`DashMapStore`].
///
/// # Examples
///
/// ```
/// use djs_cache::{Cache, CacheConfig, DashMapBackend};
///
/// // The same as `Cache::with_config(CacheConfig::default())`.
/// let cache = Cache::with_backend(CacheConfig::default(), DashMapBackend::new());
/// # let _ = cache;
/// ```
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct DashMapBackend;

impl DashMapBackend {
    /// Creates the backend.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Backend for DashMapBackend {
    type Store<K, V>
        = DashMapStore<K, V>
    where
        K: Eq + Hash + Copy + Send + Sync + 'static,
        V: Send + Sync + 'static;

    fn store<K, V>(&self, _resource: Resource, _limit: Limit) -> DashMapStore<K, V>
    where
        K: Eq + Hash + Copy + Send + Sync + 'static,
        V: Send + Sync + 'static,
    {
        DashMapStore::new()
    }
}

/// A [`Store`] backed by a sharded concurrent map ([`DashMap`]), with keys hashed by
/// `foldhash`.
///
/// Reads and writes lock one shard of the map for the duration of the call only: values are
/// handed out as [`Arc`] clones, so no lock is held by the caller (D-151).
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// use djs_cache::{DashMapStore, Store};
///
/// let store = DashMapStore::new();
/// store.insert(1_u64, Arc::new("one"));
/// assert_eq!(store.get(&1).as_deref(), Some(&"one"));
/// assert_eq!(store.remove(&1).as_deref(), Some(&"one"));
/// assert!(store.is_empty());
/// ```
pub struct DashMapStore<K, V> {
    map: DashMap<K, Arc<V>, Hasher>,
}

impl<K: Eq + Hash, V> DashMapStore<K, V> {
    /// Creates an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: DashMap::with_hasher(Hasher::default()),
        }
    }
}

impl<K: Eq + Hash, V> Default for DashMapStore<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Eq + Hash, V> fmt::Debug for DashMapStore<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DashMapStore")
            .field("len", &self.map.len())
            .finish_non_exhaustive()
    }
}

impl<K, V> Store<K, V> for DashMapStore<K, V>
where
    K: Eq + Hash + Copy + Send + Sync,
    V: Send + Sync,
{
    fn get(&self, key: &K) -> Option<Arc<V>> {
        self.map.get(key).map(|entry| Arc::clone(entry.value()))
    }

    fn insert(&self, key: K, value: Arc<V>) -> Option<Arc<V>> {
        self.map.insert(key, value)
    }

    fn remove(&self, key: &K) -> Option<Arc<V>> {
        self.map.remove(key).map(|(_, value)| value)
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    fn retain(&self, keep: &mut dyn FnMut(&K, &V) -> bool) {
        self.map.retain(|key, value| keep(key, value));
    }

    fn values(&self) -> Vec<Arc<V>> {
        self.map
            .iter()
            .map(|entry| Arc::clone(entry.value()))
            .collect()
    }

    fn keys(&self) -> Vec<K> {
        self.map.iter().map(|entry| *entry.key()).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use super::{DashMapBackend, DashMapStore};
    use crate::{Backend, Limit, Resource, Store};

    #[test]
    fn insert_get_remove() {
        let store = DashMapBackend::new().store::<u64, String>(Resource::Guild, Limit::Unlimited);
        assert!(store.is_empty());
        assert!(store.insert(1, Arc::new("a".to_owned())).is_none());
        assert_eq!(
            store
                .insert(1, Arc::new("b".to_owned()))
                .as_deref()
                .map(String::as_str),
            Some("a")
        );
        assert!(store.contains_key(&1));
        assert_eq!(store.get(&1).as_deref().map(String::as_str), Some("b"));
        assert_eq!(store.len(), 1);
        assert_eq!(store.remove(&1).as_deref().map(String::as_str), Some("b"));
        assert!(store.remove(&1).is_none());
        assert!(!store.contains_key(&1));
    }

    #[test]
    fn retain_and_values() {
        let store = DashMapStore::new();
        for key in 0..100_u64 {
            store.insert(key, Arc::new(key * 10));
        }
        store.retain(&mut |key, _| key % 2 == 0);
        assert_eq!(store.len(), 50);
        let mut keys = store.keys();
        keys.sort_unstable();
        assert_eq!(keys, (0..100).step_by(2).collect::<Vec<_>>());
        let mut values: Vec<u64> = store.values().iter().map(|value| **value).collect();
        values.sort_unstable();
        assert_eq!(
            values,
            (0..100).step_by(2).map(|key| key * 10).collect::<Vec<_>>()
        );
    }

    #[test]
    fn concurrent_writers_do_not_lose_entries() {
        let store = Arc::new(DashMapStore::new());
        let threads: Vec<_> = (0..8_u64)
            .map(|thread| {
                let store = Arc::clone(&store);
                thread::spawn(move || {
                    for key in 0..10_000 {
                        store.insert(thread * 10_000 + key, Arc::new(key));
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(store.len(), 80_000);
    }

    #[test]
    fn debug_shows_the_length() {
        let store = DashMapStore::new();
        store.insert(1_u64, Arc::new(()));
        assert_eq!(format!("{store:?}"), "DashMapStore { len: 1, .. }");
    }
}
