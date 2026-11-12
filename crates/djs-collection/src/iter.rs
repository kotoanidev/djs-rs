//! The iterators of a [`Collection`](crate::Collection), in insertion order (D-217).

use core::{fmt, iter::FusedIterator};

/// Defines an iterator wrapping one of `indexmap`'s, so that its type stays private.
macro_rules! iterator {
    (
        $(#[$meta:meta])*
        $name:ident<$($lifetime:lifetime,)? K, V> => $inner:ident, $item:ty $(,)?
    ) => {
        $(#[$meta])*
        pub struct $name<$($lifetime,)? K, V>(pub(crate) indexmap::map::$inner<$($lifetime,)? K, V>);

        impl<$($lifetime,)? K, V> Iterator for $name<$($lifetime,)? K, V> {
            type Item = $item;

            fn next(&mut self) -> Option<Self::Item> {
                self.0.next()
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.0.size_hint()
            }

            fn count(self) -> usize {
                self.0.count()
            }

            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.0.nth(n)
            }

            fn last(self) -> Option<Self::Item> {
                self.0.last()
            }
        }

        impl<$($lifetime,)? K, V> DoubleEndedIterator for $name<$($lifetime,)? K, V> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.0.next_back()
            }

            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.0.nth_back(n)
            }
        }

        impl<$($lifetime,)? K, V> ExactSizeIterator for $name<$($lifetime,)? K, V> {
            fn len(&self) -> usize {
                self.0.len()
            }
        }

        impl<$($lifetime,)? K, V> FusedIterator for $name<$($lifetime,)? K, V> {}

        impl<$($lifetime,)? K: fmt::Debug, V: fmt::Debug> fmt::Debug for $name<$($lifetime,)? K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }
    };
}

/// Implements `Clone` for an iterator over borrowed entries (no bounds on `K` and `V`).
macro_rules! clone {
    ($($name:ident),*) => {
        $(
            impl<K, V> Clone for $name<'_, K, V> {
                fn clone(&self) -> Self {
                    Self(self.0.clone())
                }
            }
        )*
    };
}

iterator! {
    /// The entries of a collection, borrowed, as `(&key, &value)`. Returned by
    /// [`Collection::iter`](crate::Collection::iter).
    Iter<'a, K, V> => Iter, (&'a K, &'a V),
}

iterator! {
    /// The entries of a collection with mutable values, as `(&key, &mut value)`. Returned by
    /// [`Collection::iter_mut`](crate::Collection::iter_mut).
    IterMut<'a, K, V> => IterMut, (&'a K, &'a mut V),
}

iterator! {
    /// The entries of a collection, moved out, as `(key, value)`. Returned by `into_iter` of a
    /// [`Collection`](crate::Collection).
    IntoIter<K, V> => IntoIter, (K, V),
}

iterator! {
    /// The keys of a collection. Returned by [`Collection::keys`](crate::Collection::keys).
    Keys<'a, K, V> => Keys, &'a K,
}

iterator! {
    /// The values of a collection. Returned by [`Collection::values`](crate::Collection::values).
    Values<'a, K, V> => Values, &'a V,
}

iterator! {
    /// The values of a collection, mutable. Returned by
    /// [`Collection::values_mut`](crate::Collection::values_mut).
    ValuesMut<'a, K, V> => ValuesMut, &'a mut V,
}

iterator! {
    /// The keys of a collection, moved out. Returned by
    /// [`Collection::into_keys`](crate::Collection::into_keys).
    IntoKeys<K, V> => IntoKeys, K,
}

iterator! {
    /// The values of a collection, moved out. Returned by
    /// [`Collection::into_values`](crate::Collection::into_values).
    IntoValues<K, V> => IntoValues, V,
}

clone!(Iter, Keys, Values);
