//! The built-in values of a cluster (D-286, D-310): [`ClusterStats`] and the [`Stats`]
//! command behind `stats()`.

use core::{
    sync::atomic::{AtomicU8, AtomicU64, Ordering},
    time::Duration,
};
use std::sync::Arc;

use djs_gateway::ShardState;
use serde::{Deserialize, Serialize};

use super::ClusterCommand;

/// The built-in command that returns a cluster's [`ClusterStats`]; every cluster process
/// handles it. [`ClusterManager::stats`](super::ClusterManager::stats) and
/// [`ClusterClient::stats`](super::ClusterClient::stats) broadcast it; `send::<Stats>(id, ())`
/// asks one cluster.
///
/// discord.js counterpart: `fetchClientValues`.
#[derive(Clone, Copy, Debug)]
pub struct Stats;

impl ClusterCommand for Stats {
    const NAME: &'static str = "djs-rs:stats";
    type Input = ();
    type Output = ClusterStats;
}

/// What a cluster reports about itself (D-310). Returned by `stats()`.
///
/// discord.js counterpart: the values `fetchClientValues` reads (`guilds.cache.size`,
/// `ws.ping`, `uptime`, …), and discord-hybrid-sharding's cluster info.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterStats {
    /// The cluster's ID.
    pub cluster: u32,
    /// Its shards, by shard ID.
    pub shards: Vec<ShardStats>,
    /// The number of guilds in its cache; `None` without a cache.
    pub guilds: Option<u64>,
    /// The number of users in its cache; `None` without a cache.
    pub users: Option<u64>,
    /// How long its client has been logged in.
    pub uptime: Duration,
    /// The memory its process uses (resident set size, in bytes); `None` where the system
    /// does not tell (only Linux does for now).
    pub memory: Option<u64>,
}

/// One shard of a [`ClusterStats`].
///
/// discord.js counterpart: `WebSocketShard#status` and `#ping`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ShardStats {
    /// The shard's ID.
    pub id: u32,
    /// Its connection state.
    pub status: ShardStatus,
    /// The round trip of its last acknowledged heartbeat; `None` before the first.
    pub latency: Option<Duration>,
}

/// The connection state of a shard in a [`ShardStats`]: its
/// [`ShardState`](djs_gateway::ShardState), which crosses the processes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ShardStatus {
    /// Not connected (it reconnects by itself).
    Disconnected,
    /// Connected and identifying.
    Identifying,
    /// Connected and resuming.
    Resuming,
    /// The session is established and events are flowing.
    Active,
    /// Closed, by the client or by Discord with a code that forbids reconnecting.
    Closed,
}

impl From<ShardState> for ShardStatus {
    fn from(state: ShardState) -> Self {
        match state {
            ShardState::Identifying => Self::Identifying,
            ShardState::Resuming => Self::Resuming,
            ShardState::Active => Self::Active,
            ShardState::FatallyClosed(_) | ShardState::Closed => Self::Closed,
            // `Disconnected`, and any state added later until it is mapped here.
            _ => Self::Disconnected,
        }
    }
}

/// A shard's state and latency as its task last saw them, for [`Stats`]: written by the
/// shard's task when they change, read when a cluster is asked.
#[derive(Debug)]
pub(crate) struct ShardSlot {
    number: u32,
    status: AtomicU8,
    /// In nanoseconds; `u64::MAX` before the first heartbeat acknowledgement.
    latency: AtomicU64,
}

impl ShardSlot {
    pub(crate) const fn new(number: u32) -> Self {
        Self {
            number,
            status: AtomicU8::new(ShardStatus::Disconnected as u8),
            latency: AtomicU64::new(u64::MAX),
        }
    }

    pub(crate) const fn number(&self) -> u32 {
        self.number
    }

    pub(crate) fn set_state(&self, state: ShardState) {
        self.status
            .store(ShardStatus::from(state) as u8, Ordering::Relaxed);
    }

    pub(crate) fn set_latency(&self, latency: Option<Duration>) {
        let nanos = latency.map_or(u64::MAX, |latency| {
            u64::try_from(latency.as_nanos()).unwrap_or(u64::MAX - 1)
        });
        self.latency.store(nanos, Ordering::Relaxed);
    }

    pub(crate) fn stats(&self) -> ShardStats {
        let status = match self.status.load(Ordering::Relaxed) {
            1 => ShardStatus::Identifying,
            2 => ShardStatus::Resuming,
            3 => ShardStatus::Active,
            4 => ShardStatus::Closed,
            _ => ShardStatus::Disconnected,
        };
        let latency = match self.latency.load(Ordering::Relaxed) {
            u64::MAX => None,
            nanos => Some(Duration::from_nanos(nanos)),
        };
        ShardStats {
            id: self.number,
            status,
            latency,
        }
    }
}

/// What a shard's task reports to its [`ShardSlot`]: nothing outside a cluster process.
/// The state is written only when it changes.
#[derive(Debug)]
pub(crate) struct Reporter {
    slot: Option<Arc<ShardSlot>>,
    reported: ShardState,
}

impl Reporter {
    pub(crate) const fn new(slot: Option<Arc<ShardSlot>>) -> Self {
        Self {
            slot,
            reported: ShardState::Disconnected,
        }
    }

    pub(crate) fn latency(&self, latency: Option<Duration>) {
        if let Some(slot) = &self.slot {
            slot.set_latency(latency);
        }
    }

    pub(crate) fn state(&mut self, state: ShardState) {
        if state != self.reported
            && let Some(slot) = &self.slot
        {
            slot.set_state(state);
            self.reported = state;
        }
    }
}

/// The resident set size of this process, in bytes.
pub(crate) fn resident_memory() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        // A small file the kernel writes on read: no disk access.
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
        let kib: u64 = line
            .trim_start_matches("VmRSS:")
            .trim()
            .trim_end_matches("kB")
            .trim()
            .parse()
            .ok()?;
        kib.checked_mul(1024)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use djs_gateway::ShardState;
    use djs_model::gateway::CloseCode;

    use super::{ShardSlot, ShardStatus, resident_memory};

    #[test]
    fn slots_report_the_last_state_and_latency() {
        let slot = ShardSlot::new(3);
        let stats = slot.stats();
        assert_eq!(
            (stats.id, stats.status, stats.latency),
            (3, ShardStatus::Disconnected, None)
        );
        for (state, status) in [
            (ShardState::Identifying, ShardStatus::Identifying),
            (ShardState::Resuming, ShardStatus::Resuming),
            (ShardState::Active, ShardStatus::Active),
            (ShardState::Closed, ShardStatus::Closed),
            (
                ShardState::FatallyClosed(CloseCode::AuthenticationFailed),
                ShardStatus::Closed,
            ),
            (ShardState::Disconnected, ShardStatus::Disconnected),
        ] {
            slot.set_state(state);
            assert_eq!(slot.stats().status, status);
        }
        slot.set_latency(Some(Duration::from_millis(42)));
        assert_eq!(slot.stats().latency, Some(Duration::from_millis(42)));
        slot.set_latency(None);
        assert_eq!(slot.stats().latency, None);
    }

    #[test]
    fn statuses_have_a_stable_wire_format() {
        assert_eq!(
            serde_json::to_string(&ShardStatus::Active).unwrap(),
            r#""active""#
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_resident_memory_is_read_on_linux() {
        let memory = resident_memory().unwrap();
        // More than a megabyte and less than the 6 GiB the guard allows.
        assert!((1 << 20..6 << 30).contains(&memory), "{memory}");
    }
}
