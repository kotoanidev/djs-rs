//! Process metrics read from `/proc` (Linux) and the timing loop — implemented the same way in
//! `node/common.mjs`, so both sides are measured identically.

use std::time::{Duration, Instant};

/// Clock ticks per second of the `utime`/`stime` fields of `/proc/<pid>/stat` (`USER_HZ`,
/// which is 100 on every mainstream Linux configuration).
const TICKS_PER_SECOND: f64 = 100.0;

/// CPU time (user + system, all threads) the process has used so far, in milliseconds.
///
/// # Panics
///
/// Panics if `/proc/self/stat` cannot be read or parsed (non-Linux systems).
#[must_use]
pub fn cpu_ms() -> f64 {
    let stat = std::fs::read_to_string("/proc/self/stat").expect("/proc/self/stat is readable");
    // The command name (field 2) is in parentheses and may contain spaces; field 3 follows.
    let rest = &stat[stat.rfind(')').expect("stat has a command name") + 2..];
    let mut fields = rest.split(' ');
    // `rest` starts at field 3, so utime (14) and stime (15) are its 12th and 13th entries.
    let user: u64 = fields.nth(11).and_then(|v| v.parse().ok()).expect("utime");
    let system: u64 = fields.next().and_then(|v| v.parse().ok()).expect("stime");
    #[expect(clippy::cast_precision_loss, reason = "tick counts are far below 2^52")]
    let ticks = (user + system) as f64;
    ticks * 1000.0 / TICKS_PER_SECOND
}

/// A `kB` value of `/proc/self/status`, such as `VmRSS` (resident memory now) or `VmHWM`
/// (peak resident memory).
///
/// # Panics
///
/// Panics if the file or the key is missing (non-Linux systems).
#[must_use]
pub fn status_kib(key: &str) -> u64 {
    let status =
        std::fs::read_to_string("/proc/self/status").expect("/proc/self/status is readable");
    status
        .lines()
        .find_map(|line| {
            let value = line.strip_prefix(key)?.strip_prefix(':')?;
            value.trim().trim_end_matches("kB").trim().parse().ok()
        })
        .unwrap_or_else(|| panic!("{key} is in /proc/self/status"))
}

/// How long the timing loop warms up before measuring.
const WARM_UP: Duration = Duration::from_secs(1);
/// The target length of one measured round.
const ROUND: Duration = Duration::from_millis(200);
/// The number of measured rounds; the median is reported.
const ROUNDS: usize = 15;

/// The result of [`measure`]: nanoseconds per call.
#[derive(Clone, Copy, Debug)]
pub struct Timing {
    /// The median over the rounds.
    pub median_ns: f64,
    /// The fastest round.
    pub min_ns: f64,
}

/// Times `f`: warms up for a second, picks an iteration count that takes about 200 ms, then
/// runs 15 rounds of it and returns the median and minimum time per call.
pub fn measure(mut f: impl FnMut()) -> Timing {
    let started = Instant::now();
    let mut calls = 0_u64;
    while started.elapsed() < WARM_UP {
        f();
        calls += 1;
    }
    let per_call = WARM_UP.as_secs_f64() / f64::from(u32::try_from(calls).unwrap_or(u32::MAX));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a positive iteration count well below u64::MAX"
    )]
    let iterations = ((ROUND.as_secs_f64() / per_call) as u64).max(1);
    let mut rounds: Vec<f64> = (0..ROUNDS)
        .map(|_| {
            let started = Instant::now();
            for _ in 0..iterations {
                f();
            }
            #[expect(clippy::cast_precision_loss, reason = "iteration counts are small")]
            let per_call = started.elapsed().as_secs_f64() * 1e9 / iterations as f64;
            per_call
        })
        .collect();
    rounds.sort_by(f64::total_cmp);
    Timing {
        median_ns: rounds[ROUNDS / 2],
        min_ns: rounds[0],
    }
}

/// The `p`-th percentile (0–100) of sorted values (nearest rank).
#[must_use]
pub fn percentile(sorted: &[u32], p: f64) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    #[expect(clippy::cast_precision_loss, reason = "sample counts are small")]
    let rank = (p / 100.0 * sorted.len() as f64).ceil();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "rank is between 0 and len"
    )]
    let index = (rank as usize).clamp(1, sorted.len()) - 1;
    sorted[index]
}
