//! Random picks without a dependency (D-210): each pick, or each `random_n` call, takes a
//! seed from std's `RandomState` (seeded randomly per thread, different for every instance)
//! and draws from a wyrand generator.
//!
//! Good for games and giveaways; not for anything that must resist prediction.

use std::{
    collections::HashSet,
    hash::{BuildHasher, Hasher, RandomState},
};

/// A small, fast generator (wyrand) for the picks of one call.
pub(crate) struct Rng(u64);

impl Rng {
    /// A generator with a fresh seed.
    pub(crate) fn new() -> Self {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(0x5EED);
        Self(hasher.finish())
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0xA076_1D64_78BD_642F);
        let product = u128::from(self.0) * u128::from(self.0 ^ 0xE703_7ED1_A0B4_28DB);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "keeping the low 64 bits is the point"
        )]
        let (low, high) = (product as u64, (product >> 64) as u64);
        low ^ high
    }

    /// A random number below `bound` (which must not be 0).
    pub(crate) fn below(&mut self, bound: usize) -> usize {
        debug_assert!(bound > 0, "no number is below 0");
        // Multiply-shift: maps 64 random bits onto 0..bound with a bias below 2^-32 for any
        // collection that fits in memory.
        let scaled = (u128::from(self.next_u64()) * bound as u128) >> 64;
        usize::try_from(scaled).unwrap_or(0)
    }
}

/// `count` distinct random indexes below `len` (`count` ≤ `len`), in random order.
pub(crate) fn distinct(len: usize, count: usize) -> Vec<usize> {
    debug_assert!(count <= len);
    let mut rng = Rng::new();
    if count * 4 >= len {
        // Many of them: a partial Fisher–Yates shuffle of every index.
        let mut indexes: Vec<usize> = (0..len).collect();
        for at in 0..count {
            let other = at + rng.below(len - at);
            indexes.swap(at, other);
        }
        indexes.truncate(count);
        return indexes;
    }
    // A few of a large collection: Floyd's sampling, then a shuffle for a random order.
    let mut chosen =
        HashSet::with_capacity_and_hasher(count, foldhash::fast::RandomState::default());
    let mut picks = Vec::with_capacity(count);
    for upper in len - count..len {
        let pick = rng.below(upper + 1);
        let pick = if chosen.insert(pick) { pick } else { upper };
        if pick == upper {
            chosen.insert(upper);
        }
        picks.push(pick);
    }
    for at in (1..picks.len()).rev() {
        picks.swap(at, rng.below(at + 1));
    }
    picks
}
