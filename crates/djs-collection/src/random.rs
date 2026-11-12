//! Random picks without a dependency (D-210): randomness comes from std's `RandomState`,
//! whose keys are seeded randomly per thread and change with every instance.
//!
//! Good for games and giveaways; not for anything that must resist prediction.

use std::{
    collections::HashSet,
    hash::{BuildHasher, Hasher, RandomState},
};

/// A random number below `bound` (which must not be 0).
pub(crate) fn below(bound: usize) -> usize {
    debug_assert!(bound > 0, "no number is below 0");
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_usize(bound);
    let bits = hasher.finish();
    // Multiply-shift: maps 64 random bits onto 0..bound with a bias below 2^-32 for any
    // collection that fits in memory.
    let scaled = (u128::from(bits) * bound as u128) >> 64;
    usize::try_from(scaled).unwrap_or(0)
}

/// `count` distinct random indexes below `len` (`count` ≤ `len`), in random order.
pub(crate) fn distinct(len: usize, count: usize) -> Vec<usize> {
    debug_assert!(count <= len);
    if count * 4 >= len {
        // Many of them: a partial Fisher–Yates shuffle of every index.
        let mut indexes: Vec<usize> = (0..len).collect();
        for at in 0..count {
            let other = at + below(len - at);
            indexes.swap(at, other);
        }
        indexes.truncate(count);
        return indexes;
    }
    // A few of a large collection: Floyd's sampling, then a shuffle for a random order.
    let mut chosen = HashSet::with_capacity(count);
    let mut picks = Vec::with_capacity(count);
    for upper in len - count..len {
        let pick = below(upper + 1);
        let pick = if chosen.insert(pick) { pick } else { upper };
        if pick == upper {
            chosen.insert(upper);
        }
        picks.push(pick);
    }
    for at in (1..picks.len()).rev() {
        picks.swap(at, below(at + 1));
    }
    picks
}
