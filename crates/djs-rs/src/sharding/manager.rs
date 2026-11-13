//! The manager process: starts the cluster processes, accepts their connections and waits
//! for them (D-296, D-297).

use core::{
    future::Future,
    net::{Ipv4Addr, SocketAddr},
    ops::Range,
    pin::Pin,
    task::{Context as TaskContext, Poll},
    time::Duration,
};
use std::{
    ffi::OsString,
    io,
    path::PathBuf,
    process::{ExitStatus, Stdio},
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
};

use djs_rest::Rest;
use futures_core::Stream;
use tokio::{
    net::{TcpListener, TcpStream},
    process::Command,
    sync::{mpsc, watch},
    task::JoinSet,
};

use super::{
    ClusterError,
    cluster::{CLUSTER_ENV, ClusterAssignment},
    ipc::{ToCluster, ToManager, Token, read_frame, write_frame},
};
use crate::Shards;

/// How long a new connection has to introduce itself.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);

/// The default time clusters get to stop after [`ClusterManager::shutdown`] before they are
/// killed.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Runs a bot's shards in several processes of the same executable, `shards_per_process`
/// shards each (D-296, D-297).
///
/// Cheap to clone (a reference count increment). Build it with [`ClusterManager::builder`]
/// in the process that was not started by a manager ([`process`](super::process) returned
/// `None`), then [`spawn`](Self::spawn) it.
///
/// discord.js counterpart: `ShardingManager` (and discord-hybrid-sharding's
/// `ClusterManager`).
///
/// # Examples
///
/// ```no_run
/// use djs_rs::{prelude::*, sharding::{self, ClusterManager}};
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let token = std::env::var("DISCORD_TOKEN")?;
/// match sharding::process()? {
///     // A process started by the manager: an ordinary client, given its shards.
///     Some(assignment) => {
///         let client = Client::builder(token).cluster(assignment).build();
///         client.on(Events::MessageCreate, |ctx, msg| async move {
///             if msg.content == "!ping" {
///                 msg.reply(&ctx, "Pong!").await?;
///             }
///             Ok(())
///         });
///         client.login().await?;
///     }
///     // The first process: it starts one process per shard and waits for them.
///     None => ClusterManager::builder(token).build().spawn().await?,
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct ClusterManager {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    rest: Rest,
    shards: Shards,
    shards_per_process: u32,
    executable: Option<PathBuf>,
    args: Option<Vec<OsString>>,
    envs: Vec<(OsString, OsString)>,
    shutdown_timeout: Duration,
    running: AtomicBool,
    shutdown: watch::Sender<bool>,
    subscribers: Mutex<Vec<mpsc::UnboundedSender<ClusterEvent>>>,
    clusters: Mutex<Vec<State>>,
}

/// What the manager knows of one cluster process.
#[derive(Debug)]
struct State {
    shards: Range<u32>,
    pid: Option<u32>,
    /// The frames to send, while the process is connected.
    sender: Option<mpsc::UnboundedSender<ToCluster>>,
}

impl ClusterManager {
    /// Starts building a manager for the bot with `token`, which it uses only to ask Discord
    /// for the recommended shard count ([`Shards::Auto`]). The processes read their own token.
    ///
    /// discord.js counterpart: `new ShardingManager(file, { token })`.
    pub fn builder(token: impl Into<String>) -> ClusterManagerBuilder {
        ClusterManagerBuilder {
            token: token.into(),
            rest: None,
            shards: Shards::Auto,
            shards_per_process: 1,
            executable: None,
            args: None,
            envs: Vec::new(),
            shutdown_timeout: SHUTDOWN_TIMEOUT,
        }
    }

    /// Starts the cluster processes and runs until every one of them has exited (after
    /// [`shutdown`](Self::shutdown), or on its own).
    ///
    /// The shard count comes from [`ClusterManagerBuilder::shards`] (Discord's recommendation
    /// by default, through [Get Gateway Bot]); the shards are split into processes of
    /// [`shards_per_process`](ClusterManagerBuilder::shards_per_process) each, numbered from
    /// cluster 0. Each process is the same executable with the same arguments (unless
    /// configured otherwise), its standard output and error shared with the manager's and
    /// its standard input closed; it learns its shards from the environment
    /// ([`process`](super::process)) and connects back to the manager over `127.0.0.1`
    /// with a secret token (D-285). A process the manager did not start cannot connect.
    ///
    /// [Get Gateway Bot]: https://docs.discord.com/developers/events/gateway#get-gateway-bot
    ///
    /// discord.js counterpart: `ShardingManager#spawn` (which returns once the shards are
    /// started).
    ///
    /// # Errors
    ///
    /// [`ClusterError::Rest`] if the shard count cannot be fetched, [`ClusterError::Io`] if
    /// the manager cannot listen or start a process, [`ClusterError::AlreadyRunning`] if the
    /// manager is already running.
    pub fn spawn(&self) -> impl Future<Output = Result<(), ClusterError>> + Send + '_ {
        boxed(self.run())
    }

    /// Asks every cluster to shut down; [`spawn`](Self::spawn) returns once they have
    /// exited. Processes still running after the
    /// [`shutdown_timeout`](ClusterManagerBuilder::shutdown_timeout) are killed. Has no effect
    /// when the manager is not running.
    ///
    /// discord.js counterpart: none (discord.js kills the shards with `Shard#kill`).
    pub fn shutdown(&self) {
        self.inner.shutdown.send_replace(true);
    }

    /// Returns a stream of the manager's events: processes started, connected and exited.
    ///
    /// discord.js counterpart: `ShardingManager#on('shardCreate')` and the `Shard` events.
    #[must_use]
    pub fn events(&self) -> ClusterEvents {
        let (tx, rx) = mpsc::unbounded_channel();
        lock(&self.inner.subscribers).push(tx);
        ClusterEvents { rx }
    }

    /// The clusters of the current [`spawn`](Self::spawn) (empty before it).
    ///
    /// discord.js counterpart: `ShardingManager#shards`.
    #[must_use]
    pub fn clusters(&self) -> Vec<Cluster> {
        lock(&self.inner.clusters)
            .iter()
            .enumerate()
            .map(|(id, state)| Cluster {
                id: u32::try_from(id).unwrap_or(u32::MAX),
                shards: state.shards.clone(),
                pid: state.pid,
                connected: state.sender.is_some(),
            })
            .collect()
    }

    fn emit(&self, event: &ClusterEvent) {
        lock(&self.inner.subscribers).retain(|tx| tx.send(event.clone()).is_ok());
    }

    async fn run(&self) -> Result<(), ClusterError> {
        let inner = &*self.inner;
        if inner.running.swap(true, Ordering::AcqRel) {
            return Err(ClusterError::AlreadyRunning);
        }
        let result = self.supervise().await;
        inner.running.store(false, Ordering::Release);
        result
    }

    async fn supervise(&self) -> Result<(), ClusterError> {
        let inner = &*self.inner;
        inner.shutdown.send_replace(false);
        let shutdown = inner.shutdown.subscribe();

        let (range, total) = match &inner.shards {
            Shards::Count(count) => (0..*count, *count),
            Shards::Range { range, total } => (range.clone(), *total),
            Shards::Auto => {
                let total = inner.rest.get_gateway_bot().await?.shards.max(1);
                (0..total, total)
            }
        };
        let groups = split(range, inner.shards_per_process);
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(ClusterError::Io)?;
        let launch = Launch {
            executable: match &inner.executable {
                Some(path) => path.clone(),
                None => std::env::current_exe().map_err(ClusterError::Io)?,
            },
            args: inner
                .args
                .clone()
                .unwrap_or_else(|| std::env::args_os().skip(1).collect()),
            address: listener.local_addr().map_err(ClusterError::Io)?,
            token: Token::generate(),
            total,
        };
        *lock(&inner.clusters) = groups
            .iter()
            .map(|shards| State {
                shards: shards.clone(),
                pid: None,
                sender: None,
            })
            .collect();
        tracing::info!(
            clusters = groups.len(),
            total,
            "starting the cluster processes"
        );

        let accepting = tokio::spawn(accept(self.clone(), listener, launch.token.clone()));
        let (kill_tx, kill_rx) = watch::channel(false);
        let mut processes = JoinSet::new();
        let started = self.start(&launch, &groups, &kill_rx, &mut processes);
        let waited = self.wait(processes, shutdown, &kill_tx).await;
        accepting.abort();
        started.and(waited)
    }

    /// Starts one process per group of shards. On an error, the processes already started
    /// are asked to shut down.
    fn start(
        &self,
        launch: &Launch,
        groups: &[Range<u32>],
        kill: &watch::Receiver<bool>,
        processes: &mut JoinSet<(u32, io::Result<ExitStatus>)>,
    ) -> Result<(), ClusterError> {
        let inner = &*self.inner;
        for (id, shards) in (0_u32..).zip(groups) {
            let assignment = ClusterAssignment::new(
                id,
                shards.clone(),
                launch.total,
                launch.address,
                launch.token.as_str().into(),
            );
            let mut command = Command::new(&launch.executable);
            command
                .args(&launch.args)
                .envs(inner.envs.iter().map(|(key, value)| (key, value)))
                .env(CLUSTER_ENV, assignment.to_env())
                .stdin(Stdio::null())
                .kill_on_drop(true);
            let mut child = match command.spawn() {
                Ok(child) => child,
                Err(error) => {
                    tracing::error!(cluster = id, %error, "could not start a cluster process");
                    inner.shutdown.send_replace(true);
                    return Err(ClusterError::Io(error));
                }
            };
            let pid = child.id();
            if let Some(state) = lock(&inner.clusters).get_mut(id as usize) {
                state.pid = pid;
            }
            self.emit(&ClusterEvent::Spawned { cluster: id, pid });
            let mut kill = kill.clone();
            processes.spawn(async move {
                let status = tokio::select! {
                    status = child.wait() => status,
                    () = async { let _ = kill.wait_for(|kill| *kill).await; } => {
                        let _ = child.start_kill();
                        child.wait().await
                    }
                };
                (id, status)
            });
        }
        Ok(())
    }

    /// Waits for every process to exit: on a shutdown, asks the connected ones to stop, then
    /// kills those still running after the grace period.
    async fn wait(
        &self,
        mut processes: JoinSet<(u32, io::Result<ExitStatus>)>,
        mut shutdown: watch::Receiver<bool>,
        kill: &watch::Sender<bool>,
    ) -> Result<(), ClusterError> {
        let inner = &*self.inner;
        let mut deadline: Option<tokio::time::Instant> = None;
        loop {
            let grace = async {
                match deadline {
                    Some(deadline) => tokio::time::sleep_until(deadline).await,
                    None => core::future::pending().await,
                }
            };
            tokio::select! {
                changed = shutdown.changed(), if deadline.is_none() => {
                    if changed.is_ok() && !*shutdown.borrow() {
                        continue;
                    }
                    tracing::info!("asking the clusters to shut down");
                    for state in lock(&inner.clusters).iter() {
                        if let Some(sender) = &state.sender {
                            let _ = sender.send(ToCluster::Shutdown);
                        }
                    }
                    deadline = Some(tokio::time::Instant::now() + inner.shutdown_timeout);
                }
                () = grace => {
                    tracing::warn!("killing the clusters that did not shut down in time");
                    kill.send_replace(true);
                    deadline = None;
                }
                finished = processes.join_next() => {
                    let Some(finished) = finished else { return Ok(()) };
                    let (id, status) = finished.map_err(|error| {
                        ClusterError::Io(io::Error::other(error))
                    })?;
                    self.exited(id, status);
                }
            }
        }
    }

    /// Records that cluster `id`'s process has exited.
    fn exited(&self, id: u32, status: io::Result<ExitStatus>) {
        if let Some(state) = lock(&self.inner.clusters).get_mut(id as usize) {
            state.sender = None;
            state.pid = None;
        }
        match status {
            Ok(status) => {
                if status.success() {
                    tracing::info!(cluster = id, "cluster process exited");
                } else {
                    tracing::warn!(cluster = id, %status, "cluster process failed");
                }
                self.emit(&ClusterEvent::Exited {
                    cluster: id,
                    status,
                });
            }
            Err(error) => {
                tracing::error!(cluster = id, %error, "lost a cluster process");
            }
        }
    }
}

/// What one [`ClusterManager::spawn`] starts its processes with.
struct Launch {
    executable: PathBuf,
    args: Vec<OsString>,
    address: SocketAddr,
    token: Token,
    total: u32,
}

/// Boxes the manager's loop here, so that its code is compiled in djs-rs (D-287).
fn boxed<'a, T>(future: impl Future<Output = T> + Send + 'a) -> BoxFuture<'a, T> {
    Box::pin(future)
}

/// Splits `range` into groups of `size` shards (the last may be smaller).
fn split(range: Range<u32>, size: u32) -> Vec<Range<u32>> {
    let size = size.max(1);
    let mut groups = Vec::new();
    let mut start = range.start;
    while start < range.end {
        let end = start.saturating_add(size).min(range.end);
        groups.push(start..end);
        start = end;
    }
    groups
}

/// Accepts the processes' connections until aborted.
async fn accept(manager: ClusterManager, listener: TcpListener, token: Token) {
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            // Out of file descriptors, say: try again a little later.
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        };
        tokio::spawn(handshake(manager.clone(), stream, token.clone()));
    }
}

/// Checks a connection's [`ToManager::Hello`], then serves it.
async fn handshake(manager: ClusterManager, stream: TcpStream, token: Token) {
    let _ = stream.set_nodelay(true);
    let (mut reader, mut writer) = stream.into_split();
    let hello = tokio::time::timeout(HELLO_TIMEOUT, read_frame::<_, ToManager>(&mut reader)).await;
    let Ok(Ok(Some(ToManager::Hello {
        cluster,
        token: presented,
    }))) = hello
    else {
        tracing::warn!("refused a connection that did not introduce itself");
        return;
    };
    if !token.matches(&presented) {
        tracing::warn!(cluster, "refused a connection with a wrong token");
        return;
    }
    let (tx, mut rx) = mpsc::unbounded_channel();
    // Queued before the cluster is registered: a shutdown sent to it cannot come first.
    let _ = tx.send(ToCluster::Welcome);
    {
        let mut clusters = lock(&manager.inner.clusters);
        let Some(state) = clusters.get_mut(cluster as usize) else {
            tracing::warn!(cluster, "refused a connection for an unknown cluster");
            return;
        };
        if state.sender.is_some() {
            tracing::warn!(cluster, "refused a second connection for a cluster");
            return;
        }
        state.sender = Some(tx.clone());
    }
    manager.emit(&ClusterEvent::Connected { cluster });
    // Connected after a shutdown was asked: it stops at once.
    if *manager.inner.shutdown.borrow() {
        let _ = tx.send(ToCluster::Shutdown);
    }
    let writing = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if write_frame(&mut writer, &message).await.is_err() {
                break;
            }
        }
    });
    // The process sends nothing else yet; the connection ends with the process.
    while let Ok(Some(_)) = read_frame::<_, ToManager>(&mut reader).await {}
    writing.abort();
    let mut clusters = lock(&manager.inner.clusters);
    if let Some(state) = clusters.get_mut(cluster as usize)
        && state
            .sender
            .as_ref()
            .is_some_and(|sender| sender.same_channel(&tx))
    {
        state.sender = None;
    }
}

/// Configures a [`ClusterManager`]. Created by [`ClusterManager::builder`].
#[derive(Debug)]
#[must_use = "a builder does nothing until `build` is called"]
pub struct ClusterManagerBuilder {
    token: String,
    rest: Option<Rest>,
    shards: Shards,
    shards_per_process: u32,
    executable: Option<PathBuf>,
    args: Option<Vec<OsString>>,
    envs: Vec<(OsString, OsString)>,
    shutdown_timeout: Duration,
}

impl ClusterManagerBuilder {
    /// Uses a preconfigured REST client to ask Discord for the shard count.
    pub fn rest(mut self, rest: Rest) -> Self {
        self.rest = Some(rest);
        self
    }

    /// Sets which shards the processes run (default [`Shards::Auto`]: as many as Discord
    /// recommends); a [`Shards::Range`] runs part of the bot here (the rest elsewhere).
    ///
    /// discord.js counterpart: the `totalShards` and `shardList` options.
    ///
    /// # Panics
    ///
    /// Panics on a shard count of `0`, or a range that is empty or reaches past `total`.
    #[track_caller]
    pub fn shards(mut self, shards: Shards) -> Self {
        match &shards {
            Shards::Auto => {}
            Shards::Count(count) => assert!(*count > 0, "the shard count must not be zero"),
            Shards::Range { range, total } => assert!(
                range.start < range.end && range.end <= *total,
                "shard range {range:?} must be non-empty and within the total of {total}"
            ),
        }
        self.shards = shards;
        self
    }

    /// Sets how many shards each process runs (default 1, as discord.js, D-297): with more,
    /// a large bot runs fewer processes, each with one cache, runtime and REST client for its
    /// shards. The last process may get fewer.
    ///
    /// discord.js counterpart: discord-hybrid-sharding's `shardsPerClusters`.
    ///
    /// # Panics
    ///
    /// Panics on `0`.
    #[track_caller]
    pub fn shards_per_process(mut self, shards: u32) -> Self {
        assert!(shards > 0, "a process must run at least one shard");
        self.shards_per_process = shards;
        self
    }

    /// Starts this executable instead of the manager's own (D-296).
    ///
    /// discord.js counterpart: the `file` argument of `ShardingManager`.
    pub fn executable(mut self, path: impl Into<PathBuf>) -> Self {
        self.executable = Some(path.into());
        self
    }

    /// Gives the processes these arguments instead of the manager's own.
    ///
    /// discord.js counterpart: the `shardArgs` option.
    pub fn args<A: Into<OsString>>(mut self, args: impl IntoIterator<Item = A>) -> Self {
        self.args = Some(args.into_iter().map(Into::into).collect());
        self
    }

    /// Adds an environment variable for the processes (they inherit the manager's
    /// environment too).
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.envs.push((key.into(), value.into()));
        self
    }

    /// Sets how long the clusters get to stop after [`ClusterManager::shutdown`] before they
    /// are killed (default 30 seconds).
    pub const fn shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// Builds the manager.
    #[must_use]
    pub fn build(self) -> ClusterManager {
        ClusterManager {
            inner: Arc::new(Inner {
                rest: self.rest.unwrap_or_else(|| Rest::new(self.token)),
                shards: self.shards,
                shards_per_process: self.shards_per_process,
                executable: self.executable,
                args: self.args,
                envs: self.envs,
                shutdown_timeout: self.shutdown_timeout,
                running: AtomicBool::new(false),
                shutdown: watch::channel(false).0,
                subscribers: Mutex::new(Vec::new()),
                clusters: Mutex::new(Vec::new()),
            }),
        }
    }
}

/// One cluster process as the manager last saw it: its ID, shards and process. Returned by
/// [`ClusterManager::clusters`].
///
/// discord.js counterpart: `Shard` (one process per shard there).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Cluster {
    /// The cluster's ID, from 0.
    pub id: u32,
    /// The shards it runs.
    pub shards: Range<u32>,
    /// Its process ID while it runs.
    pub pid: Option<u32>,
    /// Whether it is connected to the manager.
    pub connected: bool,
}

/// An event of a [`ClusterManager`]: see [`ClusterManager::events`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClusterEvent {
    /// A cluster's process was started.
    ///
    /// discord.js counterpart: `ShardingManager#shardCreate` / `Shard#spawn`.
    Spawned {
        /// The cluster's ID.
        cluster: u32,
        /// Its process ID, if the system gave one.
        pid: Option<u32>,
    },
    /// A cluster's process connected to the manager.
    Connected {
        /// The cluster's ID.
        cluster: u32,
    },
    /// A cluster's process exited.
    ///
    /// discord.js counterpart: `Shard#death`.
    Exited {
        /// The cluster's ID.
        cluster: u32,
        /// How it exited.
        status: ExitStatus,
    },
}

/// The stream of a [`ClusterManager`]'s events. Created by [`ClusterManager::events`].
///
/// Implements [`Stream`]; [`next`](Self::next) is provided so no extension trait is needed.
#[derive(Debug)]
pub struct ClusterEvents {
    rx: mpsc::UnboundedReceiver<ClusterEvent>,
}

impl ClusterEvents {
    /// Waits for the next event. Returns `None` once the manager has been dropped.
    pub async fn next(&mut self) -> Option<ClusterEvent> {
        self.rx.recv().await
    }
}

impl Stream for ClusterEvents {
    type Item = ClusterEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<ClusterEvent>> {
        self.rx.poll_recv(cx)
    }
}

#[cfg(test)]
mod tests {
    use core::{ops::Range, time::Duration};
    use std::{net::SocketAddr, sync::Arc};

    use tokio::net::TcpListener;

    use super::{ClusterEvent, ClusterManager, State, Token, accept, lock, split};
    use crate::{
        Shards,
        context::Shared,
        sharding::{
            ClusterAssignment, ClusterError,
            ipc::{ToCluster, ToManager, read_frame, write_frame},
        },
    };

    const STEP: Duration = Duration::from_secs(5);

    #[test]
    fn shards_are_split_into_processes() {
        assert_eq!(split(0..4, 1), vec![0..1, 1..2, 2..3, 3..4]);
        assert_eq!(split(0..10, 4), vec![0..4, 4..8, 8..10]);
        assert_eq!(split(8..16, 8), vec![8..16]);
        assert_eq!(split(0..3, 10), vec![0..3]);
    }

    /// A manager with two clusters that accepts connections, without starting processes.
    async fn listening() -> (ClusterManager, SocketAddr, Token) {
        let manager = ClusterManager::builder("tok")
            .shards(Shards::Count(2))
            .build();
        *lock(&manager.inner.clusters) = [0..1, 1..2]
            .into_iter()
            .map(|shards: Range<u32>| State {
                shards,
                pid: None,
                sender: None,
            })
            .collect();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let token = Token::generate();
        tokio::spawn(accept(manager.clone(), listener, token.clone()));
        (manager, address, token)
    }

    fn assignment(cluster: u32, address: SocketAddr, token: &str) -> ClusterAssignment {
        ClusterAssignment::new(cluster, cluster..cluster + 1, 2, address, token.into())
    }

    #[tokio::test]
    async fn only_the_processes_the_manager_started_connect() {
        let (manager, address, token) = listening().await;
        let mut events = manager.events();
        let shared = Arc::new(Shared::default());

        // A wrong token, or a cluster the manager does not have, is refused.
        let refused = assignment(0, address, "not-the-token")
            .connect(Arc::clone(&shared))
            .await;
        assert!(matches!(refused, Err(ClusterError::Refused)), "{refused:?}");
        let unknown = assignment(7, address, token.as_str())
            .connect(Arc::clone(&shared))
            .await;
        assert!(matches!(unknown, Err(ClusterError::Refused)), "{unknown:?}");
        assert!(manager.clusters().iter().all(|cluster| !cluster.connected));

        // The right token is welcomed, once per cluster.
        let connection = assignment(0, address, token.as_str())
            .connect(Arc::clone(&shared))
            .await
            .unwrap();
        assert_eq!(
            events.next().await,
            Some(ClusterEvent::Connected { cluster: 0 })
        );
        let clusters = manager.clusters();
        assert!(clusters[0].connected && !clusters[1].connected);
        let second = assignment(0, address, token.as_str())
            .connect(Arc::new(Shared::default()))
            .await;
        assert!(matches!(second, Err(ClusterError::Refused)), "{second:?}");
        assert!(!*shared.shutdown.borrow());

        // A shutdown from the manager shuts the process's client down…
        let sender = lock(&manager.inner.clusters)[0].sender.clone().unwrap();
        sender.send(ToCluster::Shutdown).unwrap();
        let mut shutdown = shared.shutdown.subscribe();
        tokio::time::timeout(STEP, shutdown.wait_for(|stop| *stop))
            .await
            .unwrap()
            .unwrap();
        // …and once the process has gone, the cluster may connect again.
        drop(connection);
        tokio::time::timeout(STEP, async {
            while manager.clusters()[0].connected {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let again = assignment(0, address, token.as_str())
            .connect(Arc::new(Shared::default()))
            .await;
        assert!(again.is_ok(), "{again:?}");
    }

    #[tokio::test]
    async fn losing_the_manager_shuts_the_client_down() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let manager = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let hello = read_frame::<_, ToManager>(&mut stream).await.unwrap();
            assert_eq!(
                hello,
                Some(ToManager::Hello {
                    cluster: 1,
                    token: "t".into()
                })
            );
            write_frame(&mut stream, &ToCluster::Welcome).await.unwrap();
            // Then the manager's process dies: its end of the connection closes.
        });
        let shared = Arc::new(Shared::default());
        let _connection = assignment(1, address, "t")
            .connect(Arc::clone(&shared))
            .await
            .unwrap();
        manager.await.unwrap();
        let mut shutdown = shared.shutdown.subscribe();
        tokio::time::timeout(STEP, shutdown.wait_for(|stop| *stop))
            .await
            .unwrap()
            .unwrap();
    }
}
