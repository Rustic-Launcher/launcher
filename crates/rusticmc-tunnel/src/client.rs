//! The tunnel client: keeps one authenticated tunnel to the relay open (reconnecting with backoff)
//! and forwards each player stream to the local server.
use crate::{
    mux, proxy,
    splice::{Flow, splice},
    tls::{self, TlsError, TlsRoots},
    wire::{
        ACK_OK, ACK_UNREACHABLE, ClientMsg, ErrorCode, Intent, Limits, NoticeCode, RelayMsg,
        Routes, StreamHeader, VERSION, read_frame, write_frame,
    },
};
use futures::future::BoxFuture;
use rustls::pki_types::ServerName;
use std::{
    fmt,
    future::Future,
    hash::BuildHasher,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
    sync::{Notify, broadcast, mpsc, watch},
    task::JoinHandle,
    time::timeout,
};
use tokio_rustls::TlsConnector;
use tokio_util::{compat::TokioAsyncReadCompatExt, sync::CancellationToken};

/// Where tunnel tokens come from (Rustic Cloud's tunnel-token endpoint, through the desktop app).
/// Called before every connection and before the current token expires.
pub trait TokenSource: Send + Sync + 'static {
    fn token(&self) -> BoxFuture<'_, Result<String, TokenSourceError>>;
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("{0}")]
pub struct TokenSourceError(pub String);

/// Always the same token (tests, short sessions).
#[derive(Clone)]
pub struct StaticToken(pub String);

impl TokenSource for StaticToken {
    fn token(&self) -> BoxFuture<'_, Result<String, TokenSourceError>> {
        let t = self.0.clone();
        Box::pin(async move { Ok(t) })
    }
}

/// A closure returning a future of a token.
pub struct FnTokenSource<F>(pub F);

impl<F, Fut> TokenSource for FnTokenSource<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<String, TokenSourceError>> + Send + 'static,
{
    fn token(&self) -> BoxFuture<'_, Result<String, TokenSourceError>> {
        Box::pin((self.0)())
    }
}

/// Reconnection delays: `initial`, doubling up to `max`, ±20 % jitter. A session that stayed up
/// `stable_after` starts again from `initial`.
#[derive(Clone, Copy, Debug)]
pub struct Backoff {
    pub initial: Duration,
    pub max: Duration,
    pub stable_after: Duration,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            initial: Duration::from_secs(1),
            max: Duration::from_secs(60),
            stable_after: Duration::from_secs(30),
        }
    }
}

impl Backoff {
    /// The delay before attempt `attempt + 1` (1-based `attempt`).
    pub fn delay(&self, attempt: u32) -> Duration {
        let shift = attempt.saturating_sub(1).min(16);
        let base = self.initial.saturating_mul(1u32 << shift).min(self.max);
        let seed = std::collections::hash_map::RandomState::new().hash_one(attempt);
        let factor = 0.8 + (seed % 1000) as f64 / 2500.0;
        base.mul_f64(factor).min(self.max)
    }
}

/// Everything a tunnel needs.
#[derive(Clone)]
pub struct TunnelConfig {
    /// The relay's tunnel endpoint, `host:port` (e.g. `tunnel.relay.example:7443`).
    pub relay_addr: String,
    /// The name checked against the relay's certificate; the host of `relay_addr` by default.
    pub tls_server_name: Option<String>,
    pub roots: TlsRoots,
    /// The local server (`127.0.0.1:<port>`).
    pub local_addr: SocketAddr,
    pub token_source: Arc<dyn TokenSource>,
    /// Put a PROXY protocol v2 header in front of each connection to the local server. Only for
    /// servers that read it (Horizon does not).
    pub proxy_protocol: bool,
    pub backoff: Backoff,
    /// Ping period on the control stream.
    pub keepalive: Duration,
    /// Reconnect when the relay has not answered a ping for this long.
    pub keepalive_timeout: Duration,
    /// TCP connect, TLS and `Hello` each get this long.
    pub connect_timeout: Duration,
    /// How often [`TunnelEvent::Stats`] is sent while connected.
    pub stats_every: Duration,
    /// `rusticmc-tunnel/<version>` by default.
    pub agent: String,
}

impl TunnelConfig {
    pub fn new(
        relay_addr: impl Into<String>,
        local_addr: SocketAddr,
        token_source: Arc<dyn TokenSource>,
    ) -> Self {
        Self {
            relay_addr: relay_addr.into(),
            tls_server_name: None,
            roots: TlsRoots::Native,
            local_addr,
            token_source,
            proxy_protocol: false,
            backoff: Backoff::default(),
            keepalive: Duration::from_secs(15),
            keepalive_timeout: Duration::from_secs(45),
            connect_timeout: Duration::from_secs(10),
            stats_every: Duration::from_secs(5),
            agent: concat!("rusticmc-tunnel/", env!("CARGO_PKG_VERSION")).to_owned(),
        }
    }
}

impl fmt::Debug for TunnelConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TunnelConfig")
            .field("relay_addr", &self.relay_addr)
            .field("tls_server_name", &self.tls_server_name)
            .field("local_addr", &self.local_addr)
            .field("proxy_protocol", &self.proxy_protocol)
            .finish_non_exhaustive()
    }
}

/// Traffic counters for the UI. `bytes_in` goes to the local server, `bytes_out` to players.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TunnelStats {
    pub players: u32,
    pub bytes_in: u64,
    pub bytes_out: u64,
    /// Bytes a second over the last stats period.
    pub rate_in: u64,
    pub rate_out: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TunnelState {
    #[default]
    Connecting,
    Connected,
    Reconnecting,
    Stopped,
}

/// The latest state, for a UI that polls or watches.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TunnelStatus {
    pub state: TunnelState,
    pub server_id: Option<String>,
    /// The hostnames players can use now (temporary ones with their expiry).
    pub routes: Routes,
    pub limits: Option<Limits>,
    /// When the current token expires (Unix seconds); refreshed before then.
    pub token_expires_at: Option<u64>,
    pub stats: TunnelStats,
    pub last_error: Option<String>,
}

/// What happens to a tunnel, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TunnelEvent {
    Connecting {
        attempt: u32,
    },
    Connected {
        server_id: String,
        routes: Routes,
        limits: Limits,
    },
    /// A token refresh changed the routed hostnames or a temporary one's expiry.
    RoutesChanged(Routes),
    /// The relay accepted a fresh token.
    TokenRefreshed {
        expires_at: u64,
    },
    Disconnected {
        reason: String,
        retry_in: Duration,
    },
    /// The relay refused the token; a new one is fetched before the next attempt.
    AuthFailed {
        code: ErrorCode,
        message: String,
    },
    Notice {
        code: NoticeCode,
        message: String,
    },
    /// The relay is restarting; the tunnel reconnects once it closes.
    Draining {
        deadline: Duration,
    },
    PlayerJoined {
        id: u64,
        intent: Intent,
    },
    PlayerLeft {
        id: u64,
        bytes_in: u64,
        bytes_out: u64,
        duration: Duration,
    },
    /// A player reached the relay but the local server did not answer.
    LocalServerUnreachable {
        error: String,
    },
    Stats(TunnelStats),
    Stopped,
}

#[derive(Debug, thiserror::Error)]
pub enum TunnelError {
    #[error("Tunnel::connect needs a Tokio runtime")]
    NoRuntime,
    #[error(transparent)]
    Tls(#[from] TlsError),
    #[error("invalid tunnel configuration: {0}")]
    Config(String),
}

struct Shared {
    events: broadcast::Sender<TunnelEvent>,
    status: watch::Sender<TunnelStatus>,
    players: AtomicU32,
    bytes_in: AtomicU64,
    bytes_out: AtomicU64,
    next_stream: AtomicU64,
    refresh_now: Notify,
}

impl Shared {
    fn emit(&self, e: TunnelEvent) {
        let _ = self.events.send(e);
    }
    fn stats(&self) -> TunnelStats {
        TunnelStats {
            players: self.players.load(Ordering::Relaxed),
            bytes_in: self.bytes_in.load(Ordering::Relaxed),
            bytes_out: self.bytes_out.load(Ordering::Relaxed),
            rate_in: 0,
            rate_out: 0,
        }
    }
}

/// Entry point: `Tunnel::connect(config)`.
pub struct Tunnel;

impl Tunnel {
    /// Start the tunnel on the current Tokio runtime. It connects, authenticates and reconnects on
    /// its own until [`TunnelHandle::shutdown`] (or the handle is dropped).
    pub fn connect(config: TunnelConfig) -> Result<TunnelHandle, TunnelError> {
        let rt = tokio::runtime::Handle::try_current().map_err(|_| TunnelError::NoRuntime)?;
        if config.relay_addr.trim().is_empty() {
            return Err(TunnelError::Config("relay_addr is empty".into()));
        }
        let server_name = config
            .tls_server_name
            .clone()
            .unwrap_or_else(|| host_of(&config.relay_addr).to_owned());
        let server_name = ServerName::try_from(server_name)
            .map_err(|e| TunnelError::Config(format!("TLS server name: {e}")))?;
        let tls = tls::client_config(&config.roots)?;
        let (events, first_events) = broadcast::channel(256);
        let (status_tx, status_rx) = watch::channel(TunnelStatus::default());
        let shared = Arc::new(Shared {
            events,
            status: status_tx,
            players: AtomicU32::new(0),
            bytes_in: AtomicU64::new(0),
            bytes_out: AtomicU64::new(0),
            next_stream: AtomicU64::new(1),
            refresh_now: Notify::new(),
        });
        let cancel = CancellationToken::new();
        let connector = Connector {
            tls: TlsConnector::from(tls),
            server_name,
        };
        let task = rt.spawn(run(config, connector, shared.clone(), cancel.clone()));
        Ok(TunnelHandle {
            shared,
            status: status_rx,
            cancel,
            task: Some(task),
            first_events: Some(first_events),
        })
    }
}

/// A running tunnel. Dropping it stops the tunnel.
pub struct TunnelHandle {
    shared: Arc<Shared>,
    status: watch::Receiver<TunnelStatus>,
    cancel: CancellationToken,
    task: Option<JoinHandle<()>>,
    first_events: Option<broadcast::Receiver<TunnelEvent>>,
}

impl TunnelHandle {
    /// The receiver created before the tunnel started: it misses no event. `None` the second time.
    pub fn take_events(&mut self) -> Option<broadcast::Receiver<TunnelEvent>> {
        self.first_events.take()
    }

    /// A receiver of the events from now on.
    pub fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.shared.events.subscribe()
    }

    /// The current state.
    pub fn status(&self) -> TunnelStatus {
        self.status.borrow().clone()
    }

    /// A watch on the state (changes on connect, disconnect, player counts, stats).
    pub fn watch_status(&self) -> watch::Receiver<TunnelStatus> {
        self.status.clone()
    }

    /// Fetch a new token and send it now (e.g. right after Rustic Cloud verified a custom domain,
    /// so the new hostname routes without waiting for the scheduled refresh).
    pub fn refresh_now(&self) {
        self.shared.refresh_now.notify_one();
    }

    /// Close the tunnel (players are disconnected) and wait for it to stop.
    pub async fn shutdown(mut self) {
        self.cancel.cancel();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for TunnelHandle {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

struct Connector {
    tls: TlsConnector,
    server_name: ServerName<'static>,
}

fn host_of(addr: &str) -> &str {
    if let Some(rest) = addr.strip_prefix('[') {
        return rest.split(']').next().unwrap_or(rest);
    }
    addr.rsplit_once(':').map_or(addr, |(h, _)| h)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

enum End {
    Cancelled,
    Refused {
        code: ErrorCode,
        message: String,
        retry_after: Option<Duration>,
    },
    Goodbye {
        code: ErrorCode,
        message: String,
    },
    Failed(String),
}

async fn run(
    config: TunnelConfig,
    connector: Connector,
    shared: Arc<Shared>,
    cancel: CancellationToken,
) {
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        shared.emit(TunnelEvent::Connecting { attempt });
        let started = Instant::now();
        let end = session(&config, &connector, &shared, &cancel).await;
        if started.elapsed() >= config.backoff.stable_after {
            attempt = 1;
        }
        let mut delay = config.backoff.delay(attempt);
        let mut stop = false;
        let reason = match end {
            End::Cancelled => break,
            End::Refused {
                code,
                message,
                retry_after,
            } => {
                if code.is_auth() {
                    shared.emit(TunnelEvent::AuthFailed {
                        code,
                        message: message.clone(),
                    });
                }
                if let Some(r) = retry_after {
                    delay = delay.max(r);
                }
                format!("refused ({code:?}): {message}")
            }
            End::Goodbye { code, message } => {
                if code == ErrorCode::Draining {
                    delay = config.backoff.initial;
                }
                // Another tunnel for this server took over (the same server started elsewhere):
                // reconnecting would take it back and the two would alternate.
                stop = code == ErrorCode::Replaced;
                if code.is_auth() {
                    shared.emit(TunnelEvent::AuthFailed {
                        code,
                        message: message.clone(),
                    });
                }
                format!("closed by the relay ({code:?}): {message}")
            }
            End::Failed(e) => e,
        };
        tracing::info!(%reason, retry_in = ?delay, "tunnel disconnected");
        shared.status.send_modify(|s| {
            s.state = TunnelState::Reconnecting;
            s.last_error = Some(reason.clone());
            s.stats.players = 0;
        });
        if stop {
            shared.emit(TunnelEvent::Disconnected {
                reason,
                retry_in: Duration::ZERO,
            });
            break;
        }
        shared.emit(TunnelEvent::Disconnected {
            reason,
            retry_in: delay,
        });
        tokio::select! {
            () = tokio::time::sleep(delay) => {}
            () = cancel.cancelled() => break,
        }
    }
    shared.status.send_modify(|s| {
        s.state = TunnelState::Stopped;
        s.stats.players = 0;
    });
    shared.emit(TunnelEvent::Stopped);
}

/// Aborts a task when dropped.
struct AbortOnDrop(JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn session(
    config: &TunnelConfig,
    connector: &Connector,
    shared: &Arc<Shared>,
    cancel: &CancellationToken,
) -> End {
    tokio::select! {
        end = session_inner(config, connector, shared, cancel) => end,
        () = cancel.cancelled() => End::Cancelled,
    }
}

async fn session_inner(
    config: &TunnelConfig,
    connector: &Connector,
    shared: &Arc<Shared>,
    cancel: &CancellationToken,
) -> End {
    let t = config.connect_timeout;
    let token = match timeout(t, config.token_source.token()).await {
        Ok(Ok(token)) => token,
        Ok(Err(e)) => return End::Failed(format!("no tunnel token: {e}")),
        Err(_) => return End::Failed("no tunnel token: timed out".into()),
    };
    let tcp = match timeout(t, TcpStream::connect(&config.relay_addr)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return End::Failed(format!("can't reach the relay: {e}")),
        Err(_) => return End::Failed("can't reach the relay: timed out".into()),
    };
    let _ = tcp.set_nodelay(true);
    let mut tls = match timeout(t, connector.tls.connect(connector.server_name.clone(), tcp)).await
    {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return End::Failed(format!("TLS with the relay failed: {e}")),
        Err(_) => return End::Failed("TLS with the relay timed out".into()),
    };
    let hello = ClientMsg::Hello {
        v: VERSION,
        token,
        agent: config.agent.clone(),
    };
    if let Err(e) = write_frame(&mut tls, &hello).await {
        return End::Failed(format!("relay connection lost: {e}"));
    }
    let welcome = match timeout(t, read_frame::<_, RelayMsg>(&mut tls)).await {
        Ok(Ok(RelayMsg::Welcome(w))) => w,
        Ok(Ok(RelayMsg::Refused {
            code,
            message,
            retry_after_s,
        })) => {
            return End::Refused {
                code,
                message,
                retry_after: retry_after_s.map(|s| Duration::from_secs(u64::from(s))),
            };
        }
        Ok(Ok(other)) => return End::Failed(format!("unexpected relay answer: {other:?}")),
        Ok(Err(e)) => return End::Failed(format!("relay connection lost: {e}")),
        Err(_) => return End::Failed("the relay did not answer".into()),
    };

    let (ctl, mut inbound, _driver) =
        mux::spawn(tls.compat(), yamux::Mode::Client, mux::config(256), 64);
    let control = match timeout(t, ctl.open()).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return End::Failed(format!("control stream: {e}")),
        Err(_) => return End::Failed("control stream: timed out".into()),
    };
    let (mut cr, mut cw) = tokio::io::split(control);
    // yamux announces a stream with its first frame: write at once so the relay sees the control
    // stream (and answers `Ready`).
    if let Err(e) = write_frame(&mut cw, &ClientMsg::Ping { seq: 0 }).await {
        return End::Failed(format!("control stream: {e}"));
    }
    let (msg_tx, mut msg_rx) = mpsc::channel::<RelayMsg>(16);
    let _reader = AbortOnDrop(tokio::spawn(async move {
        while let Ok(m) = read_frame::<_, RelayMsg>(&mut cr).await {
            if msg_tx.send(m).await.is_err() {
                break;
            }
        }
    }));

    let mut routes = match timeout(t, msg_rx.recv()).await {
        Ok(Some(RelayMsg::Ready(routes))) => routes,
        Ok(Some(RelayMsg::Pong { .. })) => match timeout(t, msg_rx.recv()).await {
            Ok(Some(RelayMsg::Ready(routes))) => routes,
            _ => return End::Failed("the relay did not confirm the tunnel".into()),
        },
        Ok(Some(RelayMsg::Goodbye { code, message })) => return End::Goodbye { code, message },
        Ok(Some(other)) => return End::Failed(format!("unexpected relay message: {other:?}")),
        Ok(None) => return End::Failed("the relay closed the control stream".into()),
        Err(_) => return End::Failed("the relay did not confirm the tunnel".into()),
    };
    tracing::info!(server = %welcome.server_id, hostnames = ?routes.hostnames, "tunnel connected");
    shared.status.send_modify(|s| {
        s.state = TunnelState::Connected;
        s.server_id = Some(welcome.server_id.clone());
        s.routes = routes.clone();
        s.limits = Some(welcome.limits);
        s.token_expires_at = Some(welcome.expires_at);
        s.last_error = None;
    });
    shared.emit(TunnelEvent::Connected {
        server_id: welcome.server_id.clone(),
        routes: routes.clone(),
        limits: welcome.limits,
    });

    let ping_every =
        Duration::from_secs(u64::from(welcome.ping_every_s.max(1))).min(config.keepalive);
    let mut ping = tokio::time::interval(ping_every);
    let mut stats = tokio::time::interval(config.stats_every);
    stats.tick().await;
    let mut last_pong = Instant::now();
    let mut seq = 0u64;
    let refresh = tokio::time::sleep(refresh_delay(welcome.expires_at));
    tokio::pin!(refresh);
    let mut last_stats = (shared.stats(), Instant::now());

    loop {
        // Biased: what the relay said (a `Goodbye`, a `Drain`) is handled before the timers.
        tokio::select! {
            biased;
            () = cancel.cancelled() => {
                ctl.close().await;
                return End::Cancelled;
            }
            m = msg_rx.recv() => match m {
                None => return End::Failed("the relay closed the control stream".into()),
                Some(RelayMsg::Pong { .. }) => last_pong = Instant::now(),
                Some(RelayMsg::Refreshed { expires_at, routes: new }) => {
                    refresh.as_mut().reset(tokio::time::Instant::now() + refresh_delay(expires_at));
                    shared.status.send_modify(|s| s.token_expires_at = Some(expires_at));
                    shared.emit(TunnelEvent::TokenRefreshed { expires_at });
                    if new != routes {
                        routes = new.clone();
                        shared.status.send_modify(|s| s.routes = new.clone());
                        shared.emit(TunnelEvent::RoutesChanged(new));
                    }
                }
                Some(other) => {
                    if let Some(end) = relay_said(shared, other) {
                        return end;
                    }
                }
            },
            () = shared.refresh_now.notified() => {
                refresh.as_mut().reset(tokio::time::Instant::now());
            }
            _ = ping.tick() => {
                if last_pong.elapsed() > config.keepalive_timeout {
                    return End::Failed("the relay stopped answering".into());
                }
                seq += 1;
                if let Err(e) = write_frame(&mut cw, &ClientMsg::Ping { seq }).await {
                    return last_words(&mut msg_rx, shared, format!("control stream lost: {e}")).await;
                }
            }
            () = &mut refresh => {
                // Retried every minute until the relay confirms with `Refreshed`.
                refresh.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(60));
                match timeout(t, config.token_source.token()).await {
                    Ok(Ok(token)) => {
                        if let Err(e) = write_frame(&mut cw, &ClientMsg::Refresh { token }).await {
                            return last_words(&mut msg_rx, shared, format!("control stream lost: {e}")).await;
                        }
                    }
                    Ok(Err(e)) => tracing::warn!(error = %e, "could not refresh the tunnel token"),
                    Err(_) => tracing::warn!("could not refresh the tunnel token: timed out"),
                }
            }
            _ = stats.tick() => {
                let now = shared.stats();
                let secs = last_stats.1.elapsed().as_secs_f64().max(0.001);
                let s = TunnelStats {
                    rate_in: ((now.bytes_in - last_stats.0.bytes_in) as f64 / secs) as u64,
                    rate_out: ((now.bytes_out - last_stats.0.bytes_out) as f64 / secs) as u64,
                    ..now
                };
                last_stats = (now, Instant::now());
                shared.status.send_modify(|st| st.stats = s);
                shared.emit(TunnelEvent::Stats(s));
            }
            s = inbound.recv() => match s {
                Some(stream) => {
                    tokio::spawn(forward(stream, config.local_addr, config.proxy_protocol, shared.clone()));
                }
                None => {
                    return last_words(&mut msg_rx, shared, "the relay closed the tunnel".into()).await;
                }
            },
        }
    }
}

/// Handle a control message other than `Pong`/`Refreshed`; `Some` when it ends the session.
fn relay_said(shared: &Shared, msg: RelayMsg) -> Option<End> {
    match msg {
        RelayMsg::Notice { code, message } => shared.emit(TunnelEvent::Notice { code, message }),
        RelayMsg::Drain { deadline_s } => shared.emit(TunnelEvent::Draining {
            deadline: Duration::from_secs(u64::from(deadline_s)),
        }),
        RelayMsg::Goodbye { code, message } => return Some(End::Goodbye { code, message }),
        other => tracing::debug!(?other, "ignoring a relay message"),
    }
    None
}

/// The tunnel broke: the relay may have said why just before. Read what it sent (briefly).
async fn last_words(msgs: &mut mpsc::Receiver<RelayMsg>, shared: &Shared, error: String) -> End {
    let said = timeout(Duration::from_millis(500), async {
        while let Some(m) = msgs.recv().await {
            if let Some(end) = relay_said(shared, m) {
                return Some(end);
            }
        }
        None
    })
    .await;
    said.ok().flatten().unwrap_or(End::Failed(error))
}

/// When to send the next refresh: two thirds of the way to `expires_at`, at least 5 s from now.
fn refresh_delay(expires_at: u64) -> Duration {
    let left = expires_at.saturating_sub(unix_now());
    Duration::from_secs((left * 2 / 3).max(5))
}

/// One player stream: header, local connection, ack, then bytes both ways.
async fn forward(
    mut stream: mux::MuxStream,
    local_addr: SocketAddr,
    proxy_protocol: bool,
    shared: Arc<Shared>,
) {
    let header: StreamHeader = match timeout(Duration::from_secs(10), read_frame(&mut stream)).await
    {
        Ok(Ok(h)) => h,
        _ => return,
    };
    let local = match timeout(Duration::from_secs(5), TcpStream::connect(local_addr)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return unreachable(stream, &shared, e.to_string()).await,
        Err(_) => return unreachable(stream, &shared, "timed out".into()).await,
    };
    let _ = local.set_nodelay(true);
    let mut local = local;
    if stream.write_all(&[ACK_OK]).await.is_err() || stream.flush().await.is_err() {
        return;
    }
    if proxy_protocol
        && local
            .write_all(&proxy::header(header.peer, header.relay))
            .await
            .is_err()
    {
        return;
    }
    let id = shared.next_stream.fetch_add(1, Ordering::Relaxed);
    let player = header.intent.is_player();
    if player {
        let n = shared.players.fetch_add(1, Ordering::Relaxed) + 1;
        shared.status.send_modify(|s| s.stats.players = n);
        shared.emit(TunnelEvent::PlayerJoined {
            id,
            intent: header.intent,
        });
    }
    let started = Instant::now();
    let (s_in, s_out) = (shared.clone(), shared.clone());
    let result = splice(
        stream,
        local,
        move |n| {
            s_in.bytes_in.fetch_add(n as u64, Ordering::Relaxed);
            Flow::Go
        },
        move |n| {
            s_out.bytes_out.fetch_add(n as u64, Ordering::Relaxed);
            Flow::Go
        },
    )
    .await;
    if player {
        let n = shared
            .players
            .fetch_sub(1, Ordering::Relaxed)
            .saturating_sub(1);
        shared.status.send_modify(|s| s.stats.players = n);
        let (bytes_in, bytes_out) = result.unwrap_or_default();
        shared.emit(TunnelEvent::PlayerLeft {
            id,
            bytes_in,
            bytes_out,
            duration: started.elapsed(),
        });
    }
}

async fn unreachable(mut stream: mux::MuxStream, shared: &Shared, error: String) {
    let _ = stream.write_all(&[ACK_UNREACHABLE]).await;
    let _ = stream.shutdown().await;
    tracing::warn!(%error, "the local server did not accept a relayed connection");
    shared.emit(TunnelEvent::LocalServerUnreachable { error });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_to_its_cap_with_jitter() {
        let b = Backoff::default();
        let d1 = b.delay(1);
        assert!(d1 >= Duration::from_millis(800) && d1 <= Duration::from_millis(1200));
        let d3 = b.delay(3);
        assert!(d3 >= Duration::from_millis(3200) && d3 <= Duration::from_millis(4800));
        assert!(b.delay(40) <= b.max);
        assert!(b.delay(40) >= b.max.mul_f64(0.8));
    }

    #[test]
    fn hosts_of_relay_addresses() {
        assert_eq!(host_of("tunnel.example:7443"), "tunnel.example");
        assert_eq!(host_of("[::1]:7443"), "::1");
        assert_eq!(host_of("tunnel.example"), "tunnel.example");
    }

    #[test]
    fn refresh_comes_before_expiry() {
        let in_ten_minutes = unix_now() + 600;
        let d = refresh_delay(in_ten_minutes);
        assert!(d <= Duration::from_secs(400) && d >= Duration::from_secs(390));
        assert_eq!(refresh_delay(0), Duration::from_secs(5));
    }

    #[tokio::test]
    async fn connect_needs_a_relay_address() {
        let c = TunnelConfig::new(
            "",
            "127.0.0.1:1".parse().unwrap(),
            Arc::new(StaticToken("t".into())),
        );
        assert!(matches!(Tunnel::connect(c), Err(TunnelError::Config(_))));
    }
}
