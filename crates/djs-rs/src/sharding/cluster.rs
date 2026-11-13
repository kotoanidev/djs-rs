//! The side of a cluster process: what the manager assigned it (D-296, D-300) and its
//! connection to the manager.

use core::{fmt, ops::Range, time::Duration};
use std::{net::SocketAddr, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::{net::TcpStream, task::JoinHandle};

use super::{
    ClusterError,
    ipc::{ToCluster, ToManager, read_frame, write_frame},
};
use crate::context::Shared;

/// The environment variable that carries a process's assignment (D-296).
pub(crate) const CLUSTER_ENV: &str = "DJS_RS_CLUSTER";

/// How long a process waits for the manager to accept it.
const HELLO_TIMEOUT: Duration = Duration::from_secs(30);

/// What the manager assigned to this process: its cluster ID and shards, and how to reach
/// the manager. Returned by [`process`](super::process) in a process the manager started,
/// and given to [`ClientBuilder::cluster`](crate::ClientBuilder::cluster).
///
/// discord.js counterpart: what `ShardClientUtil` reads from the environment (`SHARDS`,
/// `SHARD_COUNT`).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterAssignment {
    cluster: u32,
    shards: Range<u32>,
    total: u32,
    address: SocketAddr,
    token: String,
}

impl ClusterAssignment {
    pub(crate) fn new(
        cluster: u32,
        shards: Range<u32>,
        total: u32,
        address: SocketAddr,
        token: String,
    ) -> Self {
        Self {
            cluster,
            shards,
            total,
            address,
            token,
        }
    }

    /// This process's cluster ID (from 0).
    #[must_use]
    pub const fn cluster_id(&self) -> u32 {
        self.cluster
    }

    /// The shards this process runs.
    #[must_use]
    pub fn shards(&self) -> Range<u32> {
        self.shards.clone()
    }

    /// The bot's shard count, over every process.
    #[must_use]
    pub const fn total_shards(&self) -> u32 {
        self.total
    }

    /// Reads the assignment from the environment: `Ok(None)` when the variable is absent.
    pub(crate) fn from_env() -> Result<Option<Self>, ClusterError> {
        match std::env::var_os(CLUSTER_ENV) {
            Some(value) => value
                .into_string()
                .map_err(|_| ClusterError::Assignment)
                .and_then(|value| Self::parse(&value))
                .map(Some),
            None => Ok(None),
        }
    }

    /// Parses and checks the value of [`CLUSTER_ENV`].
    fn parse(value: &str) -> Result<Self, ClusterError> {
        let assignment: Self = serde_json::from_str(value).map_err(|_| ClusterError::Assignment)?;
        let valid = assignment.shards.start < assignment.shards.end
            && assignment.shards.end <= assignment.total
            && assignment.address.ip().is_loopback();
        if valid {
            Ok(assignment)
        } else {
            Err(ClusterError::Assignment)
        }
    }

    pub(crate) fn to_env(&self) -> String {
        serde_json::to_string(self).expect("an assignment always serializes")
    }

    /// Connects to the manager and introduces this process; then keeps the connection: a
    /// shutdown asked by the manager, or the manager going away, shuts the client down.
    pub(crate) async fn connect(&self, shared: Arc<Shared>) -> Result<Connection, ClusterError> {
        let stream = TcpStream::connect(self.address)
            .await
            .map_err(ClusterError::Io)?;
        stream.set_nodelay(true).map_err(ClusterError::Io)?;
        let (mut reader, mut writer) = stream.into_split();
        let hello = ToManager::Hello {
            cluster: self.cluster,
            token: self.token.clone(),
        };
        write_frame(&mut writer, &hello)
            .await
            .map_err(ClusterError::Io)?;
        match tokio::time::timeout(HELLO_TIMEOUT, read_frame(&mut reader)).await {
            Ok(Ok(Some(ToCluster::Welcome))) => {}
            Ok(Err(error)) => return Err(ClusterError::Io(error)),
            Ok(Ok(_)) | Err(_) => return Err(ClusterError::Refused),
        }
        let cluster = self.cluster;
        let task = tokio::spawn(async move {
            // The writer stays open while the process runs: the manager sees EOF when it ends.
            let _writer = writer;
            loop {
                match read_frame::<_, ToCluster>(&mut reader).await {
                    Ok(Some(ToCluster::Shutdown)) => {
                        tracing::info!(cluster, "the cluster manager asked to shut down");
                        break;
                    }
                    Ok(Some(ToCluster::Welcome)) => {}
                    Ok(None) | Err(_) => {
                        tracing::warn!(cluster, "lost the connection to the cluster manager");
                        break;
                    }
                }
            }
            shared.shutdown.send_replace(true);
        });
        Ok(Connection { task })
    }
}

impl fmt::Debug for ClusterAssignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClusterAssignment")
            .field("cluster", &self.cluster)
            .field("shards", &self.shards)
            .field("total", &self.total)
            .field("address", &self.address)
            .finish_non_exhaustive()
    }
}

/// A process's open connection to its manager; closed when dropped.
#[derive(Debug)]
pub(crate) struct Connection {
    task: JoinHandle<()>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::ClusterAssignment;
    use crate::sharding::ClusterError;

    #[test]
    fn assignments_round_trip_through_the_environment() {
        let assignment =
            ClusterAssignment::new(2, 4..6, 8, "127.0.0.1:4000".parse().unwrap(), "t".into());
        let value = assignment.to_env();
        assert_eq!(
            value,
            r#"{"cluster":2,"shards":{"start":4,"end":6},"total":8,"address":"127.0.0.1:4000","token":"t"}"#
        );
        assert_eq!(ClusterAssignment::parse(&value).unwrap(), assignment);
        assert_eq!(assignment.cluster_id(), 2);
        assert_eq!(assignment.shards(), 4..6);
        assert_eq!(assignment.total_shards(), 8);
        // The token stays out of logs.
        assert!(!format!("{assignment:?}").contains("token"));
    }

    #[test]
    fn invalid_assignments_are_rejected() {
        let with = |shards: &str, total: u32, address: &str| {
            ClusterAssignment::parse(&format!(
                r#"{{"cluster":0,"shards":{shards},"total":{total},"address":"{address}","token":"t"}}"#
            ))
        };
        assert!(with(r#"{"start":0,"end":1}"#, 1, "127.0.0.1:1").is_ok());
        assert!(with(r#"{"start":0,"end":1}"#, 1, "[::1]:1").is_ok());
        for invalid in [
            with(r#"{"start":1,"end":1}"#, 2, "127.0.0.1:1"),
            with(r#"{"start":0,"end":3}"#, 2, "127.0.0.1:1"),
            with(r#"{"start":0,"end":1}"#, 1, "10.0.0.1:1"),
            ClusterAssignment::parse("not json"),
        ] {
            assert!(
                matches!(invalid, Err(ClusterError::Assignment)),
                "{invalid:?}"
            );
        }
    }
}
