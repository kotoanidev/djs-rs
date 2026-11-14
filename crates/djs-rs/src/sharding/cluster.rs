//! The side of a cluster process: what the manager assigned it (D-296, D-300, D-305), its
//! connection to the manager and the IDENTIFY queue that goes through it (D-299).

use core::{fmt, future::Future, ops::Range, pin::Pin, time::Duration};
use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use djs_gateway::IdentifyQueue;
use djs_model::gateway::ShardId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use super::{
    ClusterError,
    command::{Commands, Failure, Replies},
    ipc::{TOO_LARGE, ToCluster, ToManager, read_frame, too_large, write_frame},
    lock,
};
use crate::context::Shared;

/// The environment variable that carries a process's assignment (D-296).
pub(crate) const CLUSTER_ENV: &str = "DJS_RS_CLUSTER";

/// How long a process waits for the manager to accept it.
const HELLO_TIMEOUT: Duration = Duration::from_secs(30);

/// What the manager assigned to this process: its cluster ID and shards, the gateway to
/// connect to, and how to reach the manager. Returned by [`process`](super::process) in a
/// process the manager started, and given to
/// [`ClientBuilder::cluster`](crate::ClientBuilder::cluster).
///
/// discord.js counterpart: what `ShardClientUtil` reads from the environment (`SHARDS`,
/// `SHARD_COUNT`).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterAssignment {
    cluster: u32,
    shards: Range<u32>,
    total: u32,
    gateway_url: String,
    max_concurrency: u32,
    address: SocketAddr,
    token: String,
}

impl ClusterAssignment {
    pub(crate) fn new(
        cluster: u32,
        shards: Range<u32>,
        gateway: &Gateway,
        address: SocketAddr,
        token: String,
    ) -> Self {
        Self {
            cluster,
            shards,
            total: gateway.total,
            gateway_url: gateway.url.clone(),
            max_concurrency: gateway.max_concurrency,
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

    /// The gateway URL the manager got from Discord (D-305).
    pub(crate) fn gateway_url(&self) -> &str {
        &self.gateway_url
    }

    /// The bot's `max_concurrency`, which the manager got from Discord (D-305).
    pub(crate) const fn max_concurrency(&self) -> u32 {
        self.max_concurrency
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
            && assignment.max_concurrency > 0
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
    /// shutdown asked by the manager, or the manager going away, shuts the client down. The
    /// connection carries the shards' IDENTIFY requests ([`Connection::identify_queue`]),
    /// runs the commands the manager sends with `commands`, and carries this process's own
    /// requests ([`Link`], in `shared`).
    pub(crate) async fn connect(
        &self,
        shared: Arc<Shared>,
        commands: Arc<dyn Commands>,
    ) -> Result<Connection, ClusterError> {
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

        let (requests, mut outgoing) = mpsc::unbounded_channel();
        let queue = Arc::new(ManagerQueue {
            requests: requests.clone(),
            waiting: Mutex::default(),
        });
        let link = Arc::new(Link {
            requests,
            next: AtomicU64::new(0),
            pending: Mutex::default(),
        });
        *lock(&shared.link) = Some(Arc::clone(&link));
        // Open until the connection is dropped: the manager then sees its end.
        let writing = tokio::spawn({
            let link = Arc::clone(&link);
            async move {
                while let Some(message) = outgoing.recv().await {
                    match write_frame(&mut writer, &message).await {
                        Ok(()) => {}
                        // Nothing was written: answer for the message instead, and go on.
                        Err(error) if too_large(&error) => {
                            let Some(instead) = link.too_large(&message) else {
                                continue;
                            };
                            if write_frame(&mut writer, &instead).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        });
        let cluster = self.cluster;
        let granted = Arc::clone(&queue);
        let (connected, reader_link) = (Arc::clone(&shared), Arc::clone(&link));
        let reading = tokio::spawn(async move {
            let (shared, link) = (connected, reader_link);
            loop {
                match read_frame::<_, ToCluster>(&mut reader).await {
                    Ok(Some(ToCluster::Identify { shard })) => granted.grant(shard),
                    Ok(Some(ToCluster::Request { id, command, input })) => {
                        link.run(id, commands.call(&command, input));
                    }
                    Ok(Some(ToCluster::Replies { id, replies })) => link.replied(id, replies),
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
            disconnect(&shared, &link);
            shared.shutdown.send_replace(true);
        });
        Ok(Connection {
            queue,
            shared,
            link,
            reading,
            writing,
        })
    }
}

/// Forgets `link`: its requests still waiting fail with [`ClusterError::NotConnected`].
fn disconnect(shared: &Shared, link: &Arc<Link>) {
    let mut current = lock(&shared.link);
    if current
        .as_ref()
        .is_some_and(|current| Arc::ptr_eq(current, link))
    {
        *current = None;
    }
    drop(current);
    lock(&link.pending).clear();
}

impl fmt::Debug for ClusterAssignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClusterAssignment")
            .field("cluster", &self.cluster)
            .field("shards", &self.shards)
            .field("total", &self.total)
            .field("gateway_url", &self.gateway_url)
            .field("max_concurrency", &self.max_concurrency)
            .field("address", &self.address)
            .finish_non_exhaustive()
    }
}

/// What the manager learned from Get Gateway Bot and passes to every process (D-305).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Gateway {
    pub(crate) url: String,
    pub(crate) total: u32,
    pub(crate) max_concurrency: u32,
}

/// A process's open connection to its manager; closed when dropped.
#[derive(Debug)]
pub(crate) struct Connection {
    queue: Arc<ManagerQueue>,
    shared: Arc<Shared>,
    link: Arc<Link>,
    reading: JoinHandle<()>,
    writing: JoinHandle<()>,
}

impl Connection {
    /// The IDENTIFY queue of this process's shards: every turn comes from the manager.
    pub(crate) fn identify_queue(&self) -> Arc<dyn IdentifyQueue> {
        Arc::clone(&self.queue) as Arc<dyn IdentifyQueue>
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.reading.abort();
        self.writing.abort();
        // The requests still waiting fail instead of waiting forever.
        disconnect(&self.shared, &self.link);
    }
}

/// A cluster process's requests to the other clusters, through its manager (D-307).
#[derive(Debug)]
pub(crate) struct Link {
    requests: mpsc::UnboundedSender<ToManager>,
    next: AtomicU64,
    /// The requests waiting for their replies, by ID.
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<Replies, ClusterError>>>>,
}

impl Link {
    /// Sends `command` to every cluster (`to: None`) or one and waits for the results. The
    /// manager answers each cluster within its command timeout. Boxed here so that its code
    /// is compiled in djs-rs (D-287).
    pub(crate) fn ask<'a>(
        &'a self,
        to: Option<u32>,
        command: &'a str,
        input: Value,
    ) -> super::command::BoxFuture<'a, Result<Replies, ClusterError>> {
        Box::pin(self.request(to, command, input))
    }

    async fn request(
        &self,
        to: Option<u32>,
        command: &str,
        input: Value,
    ) -> Result<Replies, ClusterError> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        lock(&self.pending).insert(id, tx);
        let request = ToManager::Request {
            id,
            to,
            command: command.to_owned(),
            input,
        };
        if self.requests.send(request).is_err() {
            lock(&self.pending).remove(&id);
            return Err(ClusterError::NotConnected);
        }
        // Dropped unanswered when the connection ends.
        rx.await.map_err(|_| ClusterError::NotConnected)?
    }

    fn replied(&self, id: u64, replies: Replies) {
        if let Some(waiting) = lock(&self.pending).remove(&id) {
            let _ = waiting.send(Ok(replies));
        }
    }

    /// Answers for a message that did not fit in a frame: its own request fails here, and a
    /// handler's result is replaced by an error (sent instead).
    fn too_large(&self, message: &ToManager) -> Option<ToManager> {
        match *message {
            ToManager::Request { id, .. } => {
                tracing::warn!("a command is too large to send to the cluster manager");
                if let Some(waiting) = lock(&self.pending).remove(&id) {
                    let _ = waiting.send(Err(ClusterError::Encoding(TOO_LARGE.into())));
                }
                None
            }
            ToManager::Reply { id, .. } => {
                tracing::warn!("the result of a command is too large to send");
                Some(ToManager::Reply {
                    id,
                    result: Err(Failure::Encoding {
                        message: TOO_LARGE.into(),
                    }),
                })
            }
            _ => None,
        }
    }

    /// Runs a handler for the manager's request `id` and sends its result back. A handler
    /// that panics is reported as failed.
    fn run(&self, id: u64, handler: super::command::BoxFuture<'static, Result<Value, Failure>>) {
        let requests = self.requests.clone();
        tokio::spawn(async move {
            let result = tokio::spawn(handler).await.unwrap_or_else(|_| {
                Err(Failure::Failed {
                    message: "the command handler panicked".into(),
                })
            });
            let _ = requests.send(ToManager::Reply { id, result });
        });
    }
}

/// The [`IdentifyQueue`] of a cluster process (D-299): each turn is asked from the manager,
/// which grants them by rate-limit bucket over every process.
#[derive(Debug)]
pub(crate) struct ManagerQueue {
    requests: mpsc::UnboundedSender<ToManager>,
    /// Per shard, the shards waiting for a turn, oldest first.
    waiting: Mutex<HashMap<u32, VecDeque<oneshot::Sender<()>>>>,
}

impl ManagerQueue {
    /// Gives a turn the manager granted to the oldest waiter of `shard` still waiting. A
    /// waiter that gave up (its connection closed) is skipped; a turn nobody waits for any
    /// more is dropped, which only delays its bucket.
    fn grant(&self, shard: u32) {
        let mut waiting = lock(&self.waiting);
        let Some(waiters) = waiting.get_mut(&shard) else {
            return;
        };
        while let Some(waiter) = waiters.pop_front() {
            if waiter.send(()).is_ok() {
                break;
            }
        }
        if waiters.is_empty() {
            waiting.remove(&shard);
        }
    }
}

impl IdentifyQueue for ManagerQueue {
    fn wait_turn(&self, shard: ShardId) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let (tx, rx) = oneshot::channel();
        lock(&self.waiting)
            .entry(shard.number())
            .or_default()
            .push_back(tx);
        let _ = self.requests.send(ToManager::Identify {
            shard: shard.number(),
        });
        Box::pin(async move {
            if rx.await.is_err() {
                // The manager is gone and the client shuts down: never identify unthrottled.
                core::future::pending::<()>().await;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;
    use std::sync::Mutex;

    use djs_gateway::IdentifyQueue;
    use djs_model::gateway::ShardId;
    use tokio::{sync::mpsc, time::timeout};

    use super::{ClusterAssignment, Gateway, ManagerQueue};
    use crate::sharding::{ClusterError, ipc::ToManager, lock};

    #[tokio::test]
    async fn turns_go_to_the_shards_still_waiting() {
        let (requests, mut sent) = mpsc::unbounded_channel();
        let queue = ManagerQueue {
            requests,
            waiting: Mutex::default(),
        };
        let shard = ShardId::new(3, 4);
        // A shard whose connection closed while it waited.
        drop(queue.wait_turn(shard));
        let mut waiting = queue.wait_turn(shard);
        for _ in 0..2 {
            assert_eq!(sent.recv().await, Some(ToManager::Identify { shard: 3 }));
        }
        // No turn without the manager's grant…
        assert!(
            timeout(Duration::from_millis(20), &mut waiting)
                .await
                .is_err()
        );
        // …and the first one skips the shard that gave up.
        queue.grant(3);
        timeout(Duration::from_secs(5), waiting).await.unwrap();
        // A turn nobody waits for is dropped.
        queue.grant(3);
        queue.grant(9);
        assert!(lock(&queue.waiting).is_empty());
    }

    #[test]
    fn assignments_round_trip_through_the_environment() {
        let gateway = Gateway {
            url: "wss://gateway.example".into(),
            total: 8,
            max_concurrency: 16,
        };
        let assignment = ClusterAssignment::new(
            2,
            4..6,
            &gateway,
            "127.0.0.1:4000".parse().unwrap(),
            "t".into(),
        );
        let value = assignment.to_env();
        assert_eq!(
            value,
            r#"{"cluster":2,"shards":{"start":4,"end":6},"total":8,"gateway_url":"wss://gateway.example","max_concurrency":16,"address":"127.0.0.1:4000","token":"t"}"#
        );
        assert_eq!(assignment.gateway_url(), "wss://gateway.example");
        assert_eq!(assignment.max_concurrency(), 16);
        assert_eq!(ClusterAssignment::parse(&value).unwrap(), assignment);
        assert_eq!(assignment.cluster_id(), 2);
        assert_eq!(assignment.shards(), 4..6);
        assert_eq!(assignment.total_shards(), 8);
        // The token stays out of logs.
        assert!(!format!("{assignment:?}").contains("token"));
    }

    #[test]
    fn invalid_assignments_are_rejected() {
        let with = |shards: &str, total: u32, max_concurrency: u32, address: &str| {
            ClusterAssignment::parse(&format!(
                r#"{{"cluster":0,"shards":{shards},"total":{total},"gateway_url":"wss://g","max_concurrency":{max_concurrency},"address":"{address}","token":"t"}}"#
            ))
        };
        assert!(with(r#"{"start":0,"end":1}"#, 1, 1, "127.0.0.1:1").is_ok());
        assert!(with(r#"{"start":0,"end":1}"#, 1, 1, "[::1]:1").is_ok());
        for invalid in [
            with(r#"{"start":1,"end":1}"#, 2, 1, "127.0.0.1:1"),
            with(r#"{"start":0,"end":3}"#, 2, 1, "127.0.0.1:1"),
            with(r#"{"start":0,"end":1}"#, 1, 0, "127.0.0.1:1"),
            with(r#"{"start":0,"end":1}"#, 1, 1, "10.0.0.1:1"),
            ClusterAssignment::parse("not json"),
        ] {
            assert!(
                matches!(invalid, Err(ClusterError::Assignment)),
                "{invalid:?}"
            );
        }
    }
}
