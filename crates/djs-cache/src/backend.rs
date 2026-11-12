//! The storage the cache is built on: a [`Store`] per resource, created by a [`Backend`].

use core::{fmt, hash::Hash};
use std::sync::Arc;

use crate::{Limit, Resource};

/// The map holding the entries of one [`Resource`].
///
/// The cache keeps one store per enabled resource and does everything else itself: it
/// decides what to insert and remove for each gateway event, keeps the guild and channel
/// indexes, and enforces the [`Limit`]s. A store only has to be a concurrent map of
/// [`Arc`]ed values (D-151, D-155).
///
/// Every method takes `&self`: the cache is shared between shards, so a store synchronises
/// internally. Each call must be atomic on its own; nothing more is required (the cache never
/// relies on two calls happening together).
///
/// discord.js counterpart: the `Collection` a `makeCache` function returns for one manager.
///
/// # Examples
///
/// A store over a [`Mutex`](std::sync::Mutex)ed [`HashMap`](std::collections::HashMap), for
/// illustration; [`DashMapStore`](crate::DashMapStore) is the default:
///
/// ```
/// use std::{collections::HashMap, hash::Hash, sync::{Arc, Mutex}};
///
/// use djs_cache::Store;
///
/// #[derive(Debug, Default)]
/// struct MutexStore<K, V>(Mutex<HashMap<K, Arc<V>>>);
///
/// impl<K, V> Store<K, V> for MutexStore<K, V>
/// where
///     K: Eq + Hash + Copy + Send + Sync,
///     V: Send + Sync,
/// {
///     fn get(&self, key: &K) -> Option<Arc<V>> {
///         self.0.lock().unwrap().get(key).cloned()
///     }
///     fn insert(&self, key: K, value: Arc<V>) -> Option<Arc<V>> {
///         self.0.lock().unwrap().insert(key, value)
///     }
///     fn remove(&self, key: &K) -> Option<Arc<V>> {
///         self.0.lock().unwrap().remove(key)
///     }
///     fn len(&self) -> usize {
///         self.0.lock().unwrap().len()
///     }
///     fn retain(&self, keep: &mut dyn FnMut(&K, &V) -> bool) {
///         self.0.lock().unwrap().retain(|key, value| keep(key, value));
///     }
///     fn values(&self) -> Vec<Arc<V>> {
///         self.0.lock().unwrap().values().cloned().collect()
///     }
///     fn keys(&self) -> Vec<K> {
///         self.0.lock().unwrap().keys().copied().collect()
///     }
/// }
///
/// let store = MutexStore::default();
/// assert!(store.insert(1_u64, Arc::new("one")).is_none());
/// assert_eq!(store.insert(1, Arc::new("uno")).as_deref(), Some(&"one"));
/// assert_eq!(store.get(&1).as_deref(), Some(&"uno"));
/// assert_eq!(store.len(), 1);
/// ```
pub trait Store<K, V>: Send + Sync {
    /// Returns the value of `key`, if any.
    fn get(&self, key: &K) -> Option<Arc<V>>;

    /// Inserts or replaces the value of `key`, returning the previous one.
    fn insert(&self, key: K, value: Arc<V>) -> Option<Arc<V>>;

    /// Removes the value of `key`, returning it.
    fn remove(&self, key: &K) -> Option<Arc<V>>;

    /// Returns the number of entries.
    fn len(&self) -> usize;

    /// Returns whether the store has no entries.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns whether `key` has a value.
    fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// Removes every entry for which `keep` returns `false`.
    ///
    /// Entries inserted while this runs may or may not be visited.
    fn retain(&self, keep: &mut dyn FnMut(&K, &V) -> bool);

    /// Returns a snapshot of every value, in no particular order.
    ///
    /// Entries changed while this runs may appear in their old or new state, or not at all.
    fn values(&self) -> Vec<Arc<V>>;

    /// Returns a snapshot of every key, in no particular order, with the same consistency as
    /// [`values`](Self::values).
    fn keys(&self) -> Vec<K>;
}

/// Creates the [`Store`] of each [`Resource`].
///
/// A backend is chosen once, as the type parameter of [`Cache`](crate::Cache), so every store
/// call is a static call (D-155). The default is [`DashMapBackend`](crate::DashMapBackend).
///
/// discord.js counterpart: the `makeCache` client option, called once per manager.
///
/// # Examples
///
/// A backend creating the `MutexStore` of the [`Store`] example:
///
/// ```
/// # use std::{collections::HashMap, hash::Hash, sync::{Arc, Mutex}};
/// # use djs_cache::Store;
/// # #[derive(Debug, Default)]
/// # struct MutexStore<K, V>(Mutex<HashMap<K, Arc<V>>>);
/// # impl<K: Eq + Hash + Copy + Send + Sync, V: Send + Sync> Store<K, V> for MutexStore<K, V> {
/// #     fn get(&self, key: &K) -> Option<Arc<V>> { self.0.lock().unwrap().get(key).cloned() }
/// #     fn insert(&self, key: K, value: Arc<V>) -> Option<Arc<V>> { self.0.lock().unwrap().insert(key, value) }
/// #     fn remove(&self, key: &K) -> Option<Arc<V>> { self.0.lock().unwrap().remove(key) }
/// #     fn len(&self) -> usize { self.0.lock().unwrap().len() }
/// #     fn retain(&self, keep: &mut dyn FnMut(&K, &V) -> bool) { self.0.lock().unwrap().retain(|k, v| keep(k, v)); }
/// #     fn values(&self) -> Vec<Arc<V>> { self.0.lock().unwrap().values().cloned().collect() }
/// #     fn keys(&self) -> Vec<K> { self.0.lock().unwrap().keys().copied().collect() }
/// # }
/// use djs_cache::{Backend, Cache, CacheConfig, Limit, Resource};
///
/// #[derive(Debug)]
/// struct MutexBackend;
///
/// impl Backend for MutexBackend {
///     type Store<K, V>
///         = MutexStore<K, V>
///     where
///         K: Eq + Hash + Copy + Send + Sync + 'static,
///         V: Send + Sync + 'static;
///
///     fn store<K, V>(&self, _resource: Resource, _limit: Limit) -> MutexStore<K, V>
///     where
///         K: Eq + Hash + Copy + Send + Sync + 'static,
///         V: Send + Sync + 'static,
///     {
///         MutexStore(Mutex::new(HashMap::new()))
///     }
/// }
///
/// let cache = Cache::with_backend(CacheConfig::default(), MutexBackend);
/// # let _ = cache;
/// ```
pub trait Backend: fmt::Debug + Send + Sync + 'static {
    /// The store this backend creates for values of type `V` under keys of type `K`.
    type Store<K, V>: Store<K, V>
    where
        K: Eq + Hash + Copy + Send + Sync + 'static,
        V: Send + Sync + 'static;

    /// Creates the store of `resource`.
    ///
    /// Called once per enabled resource when the [`Cache`](crate::Cache) is created. `limit`
    /// is the resource's configured limit, for information: it counts in the resource's scope
    /// (per guild for members, D-159) and the cache enforces it, so a store does not evict
    /// anything itself.
    fn store<K, V>(&self, resource: Resource, limit: Limit) -> Self::Store<K, V>
    where
        K: Eq + Hash + Copy + Send + Sync + 'static,
        V: Send + Sync + 'static;
}
