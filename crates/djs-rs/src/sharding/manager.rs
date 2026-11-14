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
    collections::HashMap,
    ffi::OsString,
    io,
    path::PathBuf,
    process::{ExitStatus, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use djs_gateway::{IdentifyQueue, LocalIdentifyQueue};
use djs_model::gateway::ShardId;
use djs_rest::Rest;
use futures_core::Stream;
use serde_json::Value;
use tokio::{
    net::{TcpListener, TcpStream},
    process::Command,
    sync::{mpsc, oneshot, watch},
    task::JoinSet,
};

use super::{
    ClusterCommand, ClusterError, ClusterReply,
    cluster::{CLUSTER_ENV, ClusterAssignment, Gateway},
    command::{Failure, Replies, decode_one, decode_replies, encode},
    ipc::{ToCluster, ToManager, Token, read_frame, write_frame},
    lock,
};
use crate::Shards;

/// How long a new connection has to introduce itself.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);

/// The default time clusters get to stop after [`ClusterManager::shutdown`] before they are
/// killed.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

/// The default time a cluster has to answer a command.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

/// The default first and largest delays before a process is restarted (D-306).
const RESTART_DELAY: (Duration, Duration) = (Duration::from_secs(1), Duration::from_secs(60));

/// How a cluster process ended: its ID, its exit status and how long it ran.
type Exit = (u32, io::Result<ExitStatus>, Duration);

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

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
    restart: Restart,
    restart_delay: (Duration, Duration),
    command_timeout: Duration,
    /// The ID of the next request sent to a cluster.
    next_request: AtomicU64,
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
    /// The delay before the next restart, unless the process ran long enough (D-306).
    delay: Duration,
    /// The requests sent to the connected process, waiting for its reply.
    pending: HashMap<u64, oneshot::Sender<Result<Value, Failure>>>,
}

impl ClusterManager {
    /// Starts building a manager for the bot with `token`, which it uses only for one
    /// [Get Gateway Bot] request when it starts (D-305). The processes read their own token.
    ///
    /// [Get Gateway Bot]: https://docs.discord.com/developers/events/gateway#get-gateway-bot
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
            restart: Restart::OnFailure,
            restart_delay: RESTART_DELAY,
            command_timeout: COMMAND_TIMEOUT,
        }
    }

    /// Starts the cluster processes and runs until every one of them has stopped (after
    /// [`shutdown`](Self::shutdown), or on its own without being restarted).
    ///
    /// The manager first asks Discord for the gateway's details once, for every process
    /// ([Get Gateway Bot], D-305): the gateway URL, the recommended shard count (used with
    /// the default [`Shards::Auto`], see [`ClusterManagerBuilder::shards`]), the session
    /// starts left (enough for every shard, or nothing is started) and `max_concurrency`.
    /// The shards are split into processes of
    /// [`shards_per_process`](ClusterManagerBuilder::shards_per_process) each, numbered from
    /// cluster 0. Each process is the same executable with the same arguments (unless
    /// configured otherwise), its standard output and error shared with the manager's and
    /// its standard input closed; it learns its shards and the gateway from the environment
    /// ([`process`](super::process)) and connects back to the manager over `127.0.0.1`
    /// with a secret token (D-285). A process the manager did not start cannot connect.
    ///
    /// The manager is the IDENTIFY queue of every process (D-299): a shard asks it for a
    /// turn before identifying, and it grants one per rate-limit bucket
    /// (`shard % max_concurrency`) every 5 seconds over all the processes, so the shards
    /// start as fast as Discord allows and none is refused for identifying too fast.
    ///
    /// A process that fails is started again with the same shards, after a delay that grows
    /// while it keeps failing ([`ClusterManagerBuilder::restart`] and
    /// [`restart_delay`](ClusterManagerBuilder::restart_delay), D-306). `spawn` returns once
    /// every process has exited and none is waiting for a restart.
    ///
    /// [Get Gateway Bot]: https://docs.discord.com/developers/events/gateway#get-gateway-bot
    ///
    /// discord.js counterpart: `ShardingManager#spawn` (which returns once the shards are
    /// started).
    ///
    /// # Errors
    ///
    /// [`ClusterError::Rest`] if Get Gateway Bot fails, [`ClusterError::SessionStartLimit`]
    /// if too few session starts are left for the shards, [`ClusterError::Io`] if the manager
    /// cannot listen or start the first processes (a restart that cannot start is tried
    /// again after the next delay), [`ClusterError::AlreadyRunning`] if the manager is
    /// already running.
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

    /// Sends the command `C` to every cluster and returns each one's result, by cluster ID
    /// (D-307–D-309). A cluster that is not connected (down, restarting, or the manager is
    /// not running), does not handle `C`, fails or does not answer within the
    /// [`command_timeout`](ClusterManagerBuilder::command_timeout) has an error in its
    /// [`ClusterReply`]; the others still have their results.
    ///
    /// discord.js counterpart: `ShardingManager#broadcastEval`.
    ///
    /// # Errors
    ///
    /// [`ClusterError::Encoding`] if `input` cannot be converted to JSON.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use djs_rs::sharding::{ClusterCommand, ClusterManager};
    ///
    /// struct GuildCount;
    ///
    /// impl ClusterCommand for GuildCount {
    ///     const NAME: &'static str = "guild_count";
    ///     type Input = ();
    ///     type Output = usize;
    /// }
    ///
    /// # async fn run(manager: ClusterManager) -> Result<(), djs_rs::sharding::ClusterError> {
    /// let total: usize = manager
    ///     .broadcast::<GuildCount>(())
    ///     .await?
    ///     .into_iter()
    ///     .filter_map(|reply| reply.result.ok())
    ///     .sum();
    /// println!("{total} guilds");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn broadcast<C: ClusterCommand>(
        &self,
        input: C::Input,
    ) -> Result<Vec<ClusterReply<C::Output>>, ClusterError> {
        let input = encode(&input)?;
        Ok(decode_replies(self.ask(None, C::NAME, input).await))
    }

    /// Asks every cluster for its [`ClusterStats`](super::ClusterStats) (D-310): the built-in
    /// [`Stats`](super::Stats) command, broadcast.
    ///
    /// discord.js counterpart: `ShardingManager#fetchClientValues`.
    ///
    /// # Errors
    ///
    /// None for now: a cluster that does not answer has an error in its [`ClusterReply`].
    pub async fn stats(&self) -> Result<Vec<ClusterReply<super::ClusterStats>>, ClusterError> {
        self.broadcast::<super::Stats>(()).await
    }

    /// Sends the command `C` to cluster `cluster` and returns its result.
    ///
    /// discord.js counterpart: `broadcastEval` with the `shard` option.
    ///
    /// # Errors
    ///
    /// [`ClusterError::NotConnected`] if the cluster is not connected (or does not exist),
    /// [`ClusterError::UnknownCommand`] if it does not handle `C`,
    /// [`ClusterError::CommandFailed`] if its handler fails, [`ClusterError::Timeout`] if it
    /// does not answer in time, [`ClusterError::Encoding`] if the input or output cannot be
    /// converted.
    pub async fn send<C: ClusterCommand>(
        &self,
        cluster: u32,
        input: C::Input,
    ) -> Result<C::Output, ClusterError> {
        let input = encode(&input)?;
        decode_one(self.ask(Some(cluster), C::NAME, input).await)
    }

    /// Sends `command` to every cluster (`to: None`) or one, and waits for each one's result
    /// until the command timeout. Boxed here so that its code is compiled in djs-rs (D-287).
    pub(crate) fn ask<'a>(
        &'a self,
        to: Option<u32>,
        command: &'a str,
        input: Value,
    ) -> BoxFuture<'a, Replies> {
        Box::pin(async move {
            let inner = &*self.inner;
            let mut waiting = Vec::new();
            {
                let mut clusters = lock(&inner.clusters);
                let targets = match to {
                    Some(cluster) => cluster..cluster.saturating_add(1),
                    None => 0..u32::try_from(clusters.len()).unwrap_or(u32::MAX),
                };
                for cluster in targets {
                    let state = clusters.get_mut(cluster as usize);
                    let Some((state, sender)) =
                        state.and_then(|state| state.sender.clone().map(|sender| (state, sender)))
                    else {
                        waiting.push((cluster, 0, None));
                        continue;
                    };
                    let id = inner.next_request.fetch_add(1, Ordering::Relaxed);
                    let (tx, rx) = oneshot::channel();
                    state.pending.insert(id, tx);
                    let request = ToCluster::Request {
                        id,
                        command: command.to_owned(),
                        input: input.clone(),
                    };
                    let sent = sender.send(request).is_ok();
                    waiting.push((cluster, id, sent.then_some(rx)));
                }
            }
            let deadline = tokio::time::Instant::now() + inner.command_timeout;
            let mut replies = Vec::with_capacity(waiting.len());
            for (cluster, id, reply) in waiting {
                let result = match reply {
                    None => Err(Failure::NotConnected),
                    Some(reply) => match tokio::time::timeout_at(deadline, reply).await {
                        Ok(Ok(result)) => result,
                        Ok(Err(_)) => Err(Failure::NotConnected),
                        Err(_) => {
                            if let Some(state) = lock(&inner.clusters).get_mut(cluster as usize) {
                                state.pending.remove(&id);
                            }
                            Err(Failure::Timeout)
                        }
                    },
                };
                replies.push((cluster, result));
            }
            replies
        })
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

        // One request for every process (D-305).
        let bot = inner.rest.get_gateway_bot().await?;
        let (range, total) = match &inner.shards {
            Shards::Count(count) => (0..*count, *count),
            Shards::Range { range, total } => (range.clone(), *total),
            Shards::Auto => {
                let total = bot.shards.max(1);
                (0..total, total)
            }
        };
        let limit = &bot.session_start_limit;
        let required = range.end - range.start;
        if limit.remaining < required {
            return Err(ClusterError::SessionStartLimit {
                required,
                remaining: limit.remaining,
                reset_after: Duration::from_millis(limit.reset_after),
            });
        }
        let gateway = Gateway {
            url: bot.url.into(),
            total,
            max_concurrency: limit.max_concurrency.max(1),
        };
        let groups = split(range, inner.shards_per_process);
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(ClusterError::Io)?;
        let session = Arc::new(Session {
            token: Token::generate(),
            identify: LocalIdentifyQueue::new(gateway.max_concurrency),
            gateway,
        });
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
        };
        *lock(&inner.clusters) = groups
            .iter()
            .map(|shards| State {
                shards: shards.clone(),
                pid: None,
                sender: None,
                delay: inner.restart_delay.0,
                pending: HashMap::new(),
            })
            .collect();
        tracing::info!(
            clusters = groups.len(),
            total,
            max_concurrency = session.gateway.max_concurrency,
            "starting the cluster processes"
        );

        let accepting = tokio::spawn(accept(self.clone(), listener, Arc::clone(&session)));
        let mut run = Run {
            manager: self,
            launch,
            session,
            kill: watch::channel(false).0,
            processes: JoinSet::new(),
            restarts: JoinSet::new(),
        };
        let mut started = Ok(());
        for id in (0_u32..).take(groups.len()) {
            if let Err(error) = run.start(id) {
                // Stop what was started and report the error.
                tracing::error!(cluster = id, %error, "could not start a cluster process");
                inner.shutdown.send_replace(true);
                started = Err(ClusterError::Io(error));
                break;
            }
        }
        let waited = run.wait(shutdown).await;
        accepting.abort();
        started.and(waited)
    }
}

/// One [`ClusterManager::spawn`] once its processes can start: their processes and
/// restarts.
struct Run<'a> {
    manager: &'a ClusterManager,
    launch: Launch,
    session: Arc<Session>,
    /// Set once the processes still running after the grace period must be killed.
    kill: watch::Sender<bool>,
    processes: JoinSet<Exit>,
    /// The clusters waiting for their restart delay.
    restarts: JoinSet<u32>,
}

impl Run<'_> {
    /// Starts cluster `id`'s process.
    fn start(&mut self, id: u32) -> io::Result<()> {
        let inner = &*self.manager.inner;
        let Some(shards) = lock(&inner.clusters)
            .get(id as usize)
            .map(|state| state.shards.clone())
        else {
            return Ok(());
        };
        let assignment = ClusterAssignment::new(
            id,
            shards,
            &self.session.gateway,
            self.launch.address,
            self.session.token.as_str().into(),
        );
        let mut command = Command::new(&self.launch.executable);
        command
            .args(&self.launch.args)
            .envs(inner.envs.iter().map(|(key, value)| (key, value)))
            .env(CLUSTER_ENV, assignment.to_env())
            .stdin(Stdio::null())
            .kill_on_drop(true);
        let mut child = command.spawn()?;
        let pid = child.id();
        if let Some(state) = lock(&inner.clusters).get_mut(id as usize) {
            state.pid = pid;
        }
        self.manager
            .emit(&ClusterEvent::Spawned { cluster: id, pid });
        let mut kill = self.kill.subscribe();
        let started = tokio::time::Instant::now();
        self.processes.spawn(async move {
            let status = tokio::select! {
                status = child.wait() => status,
                () = async { let _ = kill.wait_for(|kill| *kill).await; } => {
                    let _ = child.start_kill();
                    child.wait().await
                }
            };
            (id, status, started.elapsed())
        });
        Ok(())
    }

    /// Waits for every process to exit, restarting those the policy says (D-306). On a
    /// shutdown, cancels the restarts, asks the connected processes to stop, then kills
    /// those still running after the grace period.
    async fn wait(&mut self, mut shutdown: watch::Receiver<bool>) -> Result<(), ClusterError> {
        let inner = &*self.manager.inner;
        let mut deadline: Option<tokio::time::Instant> = None;
        loop {
            if self.processes.is_empty() && self.restarts.is_empty() {
                return Ok(());
            }
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
                    // Dropping the set cancels the restarts still waiting.
                    self.restarts = JoinSet::new();
                    for state in lock(&inner.clusters).iter() {
                        if let Some(sender) = &state.sender {
                            let _ = sender.send(ToCluster::Shutdown);
                        }
                    }
                    deadline = Some(tokio::time::Instant::now() + inner.shutdown_timeout);
                }
                () = grace => {
                    tracing::warn!("killing the clusters that did not shut down in time");
                    self.kill.send_replace(true);
                    deadline = None;
                }
                Some(finished) = self.processes.join_next() => {
                    let (id, status, ran) = finished.map_err(|error| {
                        ClusterError::Io(io::Error::other(error))
                    })?;
                    let failed = !matches!(&status, Ok(status) if status.success());
                    self.exited(id, status);
                    let restart = match inner.restart {
                        Restart::OnFailure => failed,
                        Restart::Always => true,
                        Restart::Never => false,
                    };
                    if restart && !*shutdown.borrow() {
                        self.restart_later(id, ran);
                    }
                }
                Some(restart) = self.restarts.join_next() => {
                    let Ok(id) = restart else { continue };
                    if let Err(error) = self.start(id) {
                        tracing::error!(cluster = id, %error, "could not restart a cluster process");
                        self.restart_later(id, Duration::ZERO);
                    }
                }
            }
        }
    }

    /// Records that cluster `id`'s process has exited.
    fn exited(&self, id: u32, status: io::Result<ExitStatus>) {
        if let Some(state) = lock(&self.manager.inner.clusters).get_mut(id as usize) {
            state.sender = None;
            state.pid = None;
            // Its requests still waiting fail as not connected.
            state.pending.clear();
        }
        match status {
            Ok(status) => {
                if status.success() {
                    tracing::info!(cluster = id, "cluster process exited");
                } else {
                    tracing::warn!(cluster = id, %status, "cluster process failed");
                }
                self.manager.emit(&ClusterEvent::Exited {
                    cluster: id,
                    status,
                });
            }
            Err(error) => {
                tracing::error!(cluster = id, %error, "lost a cluster process");
            }
        }
    }

    /// Restarts cluster `id` after its delay, which grows while it keeps exiting before
    /// running for the largest delay (D-306).
    fn restart_later(&mut self, id: u32, ran: Duration) {
        let (first, max) = self.manager.inner.restart_delay;
        let delay = {
            let mut clusters = lock(&self.manager.inner.clusters);
            let Some(state) = clusters.get_mut(id as usize) else {
                return;
            };
            let (delay, next) = restart_delay(state.delay, ran, first, max);
            state.delay = next;
            delay
        };
        tracing::info!(cluster = id, ?delay, "restarting a cluster process");
        self.manager
            .emit(&ClusterEvent::Restarting { cluster: id, delay });
        self.restarts.spawn(async move {
            tokio::time::sleep(delay).await;
            id
        });
    }
}

/// The delay before a restart, and the one after it: `current` doubles up to `max`, and a
/// process that ran for at least `max` starts again from `first`.
fn restart_delay(
    current: Duration,
    ran: Duration,
    first: Duration,
    max: Duration,
) -> (Duration, Duration) {
    let delay = if ran >= max { first } else { current };
    (delay, delay.saturating_mul(2).min(max))
}

/// What one [`ClusterManager::spawn`] starts its processes with.
struct Launch {
    executable: PathBuf,
    args: Vec<OsString>,
    address: SocketAddr,
}

/// What the connections of one [`ClusterManager::spawn`] share.
#[derive(Debug)]
struct Session {
    /// The secret the processes present.
    token: Token,
    gateway: Gateway,
    /// The IDENTIFY turns of every process, by rate-limit bucket (D-299).
    identify: LocalIdentifyQueue,
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
async fn accept(manager: ClusterManager, listener: TcpListener, session: Arc<Session>) {
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            // Out of file descriptors, say: try again a little later.
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        };
        tokio::spawn(handshake(manager.clone(), stream, Arc::clone(&session)));
    }
}

/// Checks a connection's [`ToManager::Hello`], then serves it.
async fn handshake(manager: ClusterManager, stream: TcpStream, session: Arc<Session>) {
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
    if !session.token.matches(&presented) {
        tracing::warn!(cluster, "refused a connection with a wrong token");
        return;
    }
    let (tx, mut rx) = mpsc::unbounded_channel();
    // Queued before the cluster is registered: a shutdown sent to it cannot come first.
    let _ = tx.send(ToCluster::Welcome);
    let shards = {
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
        state.shards.clone()
    };
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
    serve(&manager, cluster, &shards, reader, &tx, &session).await;
    writing.abort();
    let mut clusters = lock(&manager.inner.clusters);
    if let Some(state) = clusters.get_mut(cluster as usize)
        && state
            .sender
            .as_ref()
            .is_some_and(|sender| sender.same_channel(&tx))
    {
        state.sender = None;
        state.pending.clear();
    }
}

/// Answers a connected process's requests until its connection ends: IDENTIFY turns, its
/// commands for the clusters, and its replies to the manager's. Its turns still waiting in
/// the IDENTIFY queue are then dropped, so they do not hold up the others.
async fn serve(
    manager: &ClusterManager,
    cluster: u32,
    shards: &Range<u32>,
    mut reader: tokio::net::tcp::OwnedReadHalf,
    tx: &mpsc::UnboundedSender<ToCluster>,
    session: &Session,
) {
    // Frames are read in their own task: reading one is not cancel-safe, and the turns are
    // awaited next to it.
    let (frames_tx, mut frames) = mpsc::unbounded_channel();
    let reading = tokio::spawn(async move {
        while let Ok(Some(frame)) = read_frame::<_, ToManager>(&mut reader).await {
            if frames_tx.send(frame).is_err() {
                break;
            }
        }
    });
    let mut tasks = JoinSet::new();
    loop {
        tokio::select! {
            frame = frames.recv() => match frame {
                Some(ToManager::Identify { shard }) if shards.contains(&shard) => {
                    let turn = session
                        .identify
                        .wait_turn(ShardId::new(shard, session.gateway.total));
                    let tx = tx.clone();
                    tasks.spawn(async move {
                        turn.await;
                        let _ = tx.send(ToCluster::Identify { shard });
                    });
                }
                Some(ToManager::Request { id, to, command, input }) => {
                    let (manager, tx) = (manager.clone(), tx.clone());
                    tasks.spawn(async move {
                        let replies = manager.ask(to, &command, input).await;
                        let _ = tx.send(ToCluster::Replies { id, replies });
                    });
                }
                Some(ToManager::Reply { id, result }) => {
                    let waiting = lock(&manager.inner.clusters)
                        .get_mut(cluster as usize)
                        .and_then(|state| state.pending.remove(&id));
                    if let Some(waiting) = waiting {
                        let _ = waiting.send(result);
                    }
                }
                Some(ToManager::Identify { shard }) => {
                    tracing::warn!(cluster, shard, "ignored a turn asked for another cluster's shard");
                }
                Some(ToManager::Hello { .. }) => {}
                None => break,
            },
            Some(_) = tasks.join_next(), if !tasks.is_empty() => {}
        }
    }
    reading.abort();
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
    restart: Restart,
    restart_delay: (Duration, Duration),
    command_timeout: Duration,
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

    /// Sets which exits of a process restart it (default [`Restart::OnFailure`], D-306).
    /// Processes stopped by [`ClusterManager::shutdown`] are never restarted.
    ///
    /// discord.js counterpart: the `respawn` option (which restarts every exit).
    pub const fn restart(mut self, restart: Restart) -> Self {
        self.restart = restart;
        self
    }

    /// Sets the delays before a process is restarted (default 1 second, up to 60): the first
    /// restart waits `first`, each next one twice as long up to `max`, and a process that ran
    /// for at least `max` before exiting starts again from `first` (D-306). A failing bot so
    /// does not restart in a loop.
    ///
    /// discord.js counterpart: the `respawnDelay` of `Shard#respawn` (a fixed delay).
    ///
    /// # Panics
    ///
    /// Panics if `first` is zero or longer than `max`.
    #[track_caller]
    pub const fn restart_delay(mut self, first: Duration, max: Duration) -> Self {
        assert!(!first.is_zero(), "the first restart delay must not be zero");
        assert!(
            first.as_nanos() <= max.as_nanos(),
            "the first restart delay must not be longer than the largest"
        );
        self.restart_delay = (first, max);
        self
    }

    /// Sets how long a cluster has to answer a command
    /// ([`ClusterManager::broadcast`], [`ClusterClient::broadcast`](super::ClusterClient::broadcast), …)
    /// before its result is [`ClusterError::Timeout`] (default 30 seconds).
    ///
    /// discord.js counterpart: the `timeout` option of `broadcastEval`.
    pub const fn command_timeout(mut self, timeout: Duration) -> Self {
        self.command_timeout = timeout;
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
                restart: self.restart,
                restart_delay: self.restart_delay,
                command_timeout: self.command_timeout,
                next_request: AtomicU64::new(0),
                running: AtomicBool::new(false),
                shutdown: watch::channel(false).0,
                subscribers: Mutex::new(Vec::new()),
                clusters: Mutex::new(Vec::new()),
            }),
        }
    }
}

/// Which exits of a cluster process make the manager restart it (D-306): see
/// [`ClusterManagerBuilder::restart`]. A process stopped by [`ClusterManager::shutdown`] is
/// never restarted.
///
/// discord.js counterpart: the `respawn` option.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Restart {
    /// Restart a process that failed: a non-zero exit code, or killed by a signal (a panic,
    /// an error from `login`, out of memory). A clean exit (code 0, for example after
    /// [`Context::shutdown`](crate::Context::shutdown) in the process) ends that cluster.
    #[default]
    OnFailure,
    /// Restart every exit, clean ones included (as discord.js's `respawn: true`).
    Always,
    /// Never restart: an exited cluster stays stopped.
    Never,
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
    /// A cluster's process will be started again after `delay` ([`Restart`], D-306); a
    /// [`Spawned`](Self::Spawned) follows, unless the manager shuts down first.
    Restarting {
        /// The cluster's ID.
        cluster: u32,
        /// How long the manager waits before starting it.
        delay: Duration,
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

    use djs_gateway::LocalIdentifyQueue;
    use djs_model::gateway::ShardId;

    use std::collections::HashMap;

    use serde_json::{Value, json};

    use super::{
        ClusterEvent, ClusterManager, Session, State, Token, accept, lock, restart_delay, split,
    };
    use crate::{
        Shards,
        context::Shared,
        sharding::{
            ClusterAssignment, ClusterError,
            cluster::Gateway,
            command::{BoxFuture, Commands, Failure},
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

    /// The commands of the test clusters: `echo` returns its input, `hang` never answers.
    struct Echo;

    impl Commands for Echo {
        fn call(&self, name: &str, input: Value) -> BoxFuture<'static, Result<Value, Failure>> {
            match name {
                "echo" => Box::pin(async move { Ok(input) }),
                "hang" => Box::pin(core::future::pending()),
                _ => Box::pin(async { Err(Failure::UnknownCommand) }),
            }
        }
    }

    fn commands() -> Arc<dyn Commands> {
        Arc::new(Echo)
    }

    fn gateway() -> Gateway {
        Gateway {
            url: "ws://127.0.0.1:9".into(),
            total: 2,
            max_concurrency: 1,
        }
    }

    #[test]
    fn restart_delays_grow_until_a_process_runs_long_enough() {
        let (first, max) = (Duration::from_secs(1), Duration::from_secs(8));
        let quick = Duration::from_millis(10);
        let mut current = first;
        let mut delays = Vec::new();
        for _ in 0..6 {
            let (delay, next) = restart_delay(current, quick, first, max);
            delays.push(delay.as_secs());
            current = next;
        }
        assert_eq!(delays, [1, 2, 4, 8, 8, 8]);
        // A process that ran for the largest delay starts again from the first.
        assert_eq!(restart_delay(current, max, first, max), (first, first * 2));
    }

    /// A manager with two clusters that accepts connections, without starting processes.
    async fn listening() -> (ClusterManager, SocketAddr, Token) {
        let manager = ClusterManager::builder("tok")
            .shards(Shards::Count(2))
            .command_timeout(Duration::from_millis(200))
            .build();
        *lock(&manager.inner.clusters) = [0..1, 1..2]
            .into_iter()
            .map(|shards: Range<u32>| State {
                shards,
                pid: None,
                sender: None,
                delay: Duration::from_secs(1),
                pending: HashMap::new(),
            })
            .collect();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let token = Token::generate();
        let session = Arc::new(Session {
            token: token.clone(),
            gateway: gateway(),
            identify: LocalIdentifyQueue::new(1),
        });
        tokio::spawn(accept(manager.clone(), listener, session));
        (manager, address, token)
    }

    fn assignment(cluster: u32, address: SocketAddr, token: &str) -> ClusterAssignment {
        ClusterAssignment::new(
            cluster,
            cluster..cluster + 1,
            &gateway(),
            address,
            token.into(),
        )
    }

    #[tokio::test]
    async fn only_the_processes_the_manager_started_connect() {
        let (manager, address, token) = listening().await;
        let mut events = manager.events();
        let shared = Arc::new(Shared::default());

        // A wrong token, or a cluster the manager does not have, is refused.
        let refused = assignment(0, address, "not-the-token")
            .connect(Arc::clone(&shared), commands())
            .await;
        assert!(matches!(refused, Err(ClusterError::Refused)), "{refused:?}");
        let unknown = assignment(7, address, token.as_str())
            .connect(Arc::clone(&shared), commands())
            .await;
        assert!(matches!(unknown, Err(ClusterError::Refused)), "{unknown:?}");
        assert!(manager.clusters().iter().all(|cluster| !cluster.connected));

        // The right token is welcomed, once per cluster.
        let connection = assignment(0, address, token.as_str())
            .connect(Arc::clone(&shared), commands())
            .await
            .unwrap();
        assert_eq!(
            events.next().await,
            Some(ClusterEvent::Connected { cluster: 0 })
        );
        let clusters = manager.clusters();
        assert!(clusters[0].connected && !clusters[1].connected);
        let second = assignment(0, address, token.as_str())
            .connect(Arc::new(Shared::default()), commands())
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
            .connect(Arc::new(Shared::default()), commands())
            .await;
        assert!(again.is_ok(), "{again:?}");
    }

    #[tokio::test]
    async fn turns_are_granted_for_the_clusters_own_shards() {
        let (_manager, address, token) = listening().await;
        let connection = assignment(1, address, token.as_str())
            .connect(Arc::new(Shared::default()), commands())
            .await
            .unwrap();
        let queue = connection.identify_queue();
        // Shard 0 is cluster 0's: the manager does not answer.
        let other = queue.wait_turn(ShardId::new(0, 2));
        assert!(
            tokio::time::timeout(Duration::from_millis(100), other)
                .await
                .is_err()
        );
        // Its own shard gets a turn at once: nothing has identified yet.
        tokio::time::timeout(STEP, queue.wait_turn(ShardId::new(1, 2)))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn commands_reach_every_connected_cluster() {
        let (manager, address, token) = listening().await;
        let shared = Arc::new(Shared::default());
        let _connection = assignment(0, address, token.as_str())
            .connect(Arc::clone(&shared), commands())
            .await
            .unwrap();
        // Cluster 1 is not connected; the others' results stay usable.
        assert_eq!(
            manager.ask(None, "echo", json!({"n": 1})).await,
            [(0, Ok(json!({"n": 1}))), (1, Err(Failure::NotConnected))]
        );
        assert_eq!(
            manager.ask(Some(0), "nope", Value::Null).await,
            [(0, Err(Failure::UnknownCommand))]
        );
        assert_eq!(
            manager.ask(Some(9), "echo", Value::Null).await,
            [(9, Err(Failure::NotConnected))]
        );
        // No answer within the command timeout; the request is forgotten.
        assert_eq!(
            manager.ask(Some(0), "hang", Value::Null).await,
            [(0, Err(Failure::Timeout))]
        );
        assert!(lock(&manager.inner.clusters)[0].pending.is_empty());

        // A cluster's own request goes through the manager to every cluster, itself too.
        let link = lock(&shared.link).clone().unwrap();
        assert_eq!(
            link.ask(None, "echo", json!(2)).await.unwrap(),
            [(0, Ok(json!(2))), (1, Err(Failure::NotConnected))]
        );
        assert_eq!(
            link.ask(Some(0), "echo", json!("one")).await.unwrap(),
            [(0, Ok(json!("one")))]
        );
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
            .connect(Arc::clone(&shared), commands())
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
