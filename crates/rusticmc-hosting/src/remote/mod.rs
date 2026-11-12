//! `RemoteSshProvider`: the user's own Linux VPS, reached over SSH.
//!
//! Installation is a resumable plan ([`steps`]); management runs Horizon's supervisor CLI over SSH
//! (`horizon-server --dir <server> status --json`, `cmd`, `players --json`, `backup`...), as the
//! account that owns the server; the live console and backup downloads use the supervisor's control
//! API through an SSH `direct-tcpip` forward to its 127.0.0.1 address, so nothing is exposed
//! publicly (docs/hosting.md: "Never put it on the open internet").

pub mod layout;
pub mod steps;

use crate::error::{HostingError, Result};
use crate::model::*;
use crate::plan::{Step, StepProgress, run_plan};
use crate::platform::{PROBE_SCRIPT, SystemInfo, parse_probe};
use crate::provider::{Capabilities, ConsoleSubscription, HostingProvider};
use crate::secret::{Secret, SecretStore, names};
use crate::shell::{Cmd, quote, validate_name};
use crate::ssh::hostkey::KnownHosts;
use crate::ssh::http::{self, ApiEndpoint};
use crate::ssh::{ConnectOptions, SshAuth, SshSession, SshTarget};
use async_trait::async_trait;
use layout::Layout;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use steps::InstallCtx;
use tokio::sync::Mutex;

/// Horizon's one-command installer (docs/hosting.md, "1. Install").
pub const HORIZON_INSTALLER_URL: &str =
    "https://raw.githubusercontent.com/horizon-rust/horizon/main/packaging/server/install.sh";

/// How the profile authenticates. Secrets themselves are in the keychain under
/// `hosting/<profile>/...` ([`names`]). There is no stored password (SECURITY §2.8): a password is
/// only used once, by [`bootstrap_key`], to install a launcher-generated key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AuthMethod {
    /// A launcher-generated key (recommended) or a private key the user pasted or imported, kept in the keychain (with its passphrase, if any).
    KeychainKey,
    /// A key file the user already has (`~/.ssh/id_ed25519`); only its passphrase, if any, is in
    /// the keychain. The file is read at connect time and never copied.
    KeyFile { path: PathBuf },
}

/// A VPS connection profile (no secrets: safe to store in the launcher's config).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteProfile {
    /// `[a-z][a-z0-9-]*`, the keychain and registry key.
    pub id: String,
    pub label: String,
    pub target: SshTarget,
    pub auth: AuthMethod,
}

/// Install choices for a VPS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RemoteInstallOptions {
    pub installer_url: String,
    /// Pin the installer script's SHA-256 (it is fetched from Horizon's `main` branch otherwise).
    pub installer_sha256: Option<String>,
    /// `--version TAG` (empty: the current Nightly, the only build Horizon ships).
    pub horizon_version: Option<String>,
    /// A Horizon server archive from this computer (`install.sh --archive`): required on aarch64,
    /// where Horizon publishes no release.
    pub horizon_archive: Option<PathBuf>,
    /// The user allowed opening the game port in ufw/firewalld.
    pub open_firewall: bool,
    pub dedicated_user: String,
    #[serde(with = "secs")]
    pub health_timeout: Duration,
    /// Skip the status ping from this computer (tests, or a VPS reachable only through a tunnel).
    pub skip_public_ping: bool,
}

impl Default for RemoteInstallOptions {
    fn default() -> Self {
        Self {
            installer_url: HORIZON_INSTALLER_URL.into(),
            installer_sha256: None,
            horizon_version: None,
            horizon_archive: None,
            open_firewall: false,
            dedicated_user: "rustic".into(),
            health_timeout: Duration::from_secs(300),
            skip_public_ping: false,
        }
    }
}

mod secs {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;
    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(d.as_secs())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        Ok(Duration::from_secs(u64::deserialize(d)?))
    }
}

pub(crate) fn sha256_file(path: &Path) -> Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    std::io::copy(&mut f, &mut h)?;
    Ok(hex::encode(h.finalize()))
}

/// Pack a local folder into a temporary `.tar` holding it as `<top>/...` (deleted on drop). Not
/// compressed: textures and class files barely shrink, and gzip in a debug build crawls.
pub fn pack_dir(local: &Path, top: &str) -> Result<tempfile::NamedTempFile> {
    let files = crate::ssh::collect_files(local)?;
    if files.is_empty() {
        return Err(HostingError::Invalid(format!(
            "{} is empty",
            local.display()
        )));
    }
    let tmp = tempfile::Builder::new()
        .prefix("rustic-upload-")
        .suffix(".tar")
        .tempfile()?;
    let out = std::io::BufWriter::with_capacity(1 << 20, tmp.reopen()?);
    let mut tar = tar::Builder::new(out);
    tar.mode(tar::HeaderMode::Deterministic);
    for (rel, _) in &files {
        let name = std::path::Path::new(top).join(rel);
        tar.append_path_with_name(local.join(rel), name)?;
    }
    std::io::Write::flush(&mut tar.into_inner()?)?;
    Ok(tmp)
}

/// Read the secrets for a profile from the keychain.
pub fn load_auth(profile: &RemoteProfile, store: &dyn SecretStore) -> Result<SshAuth> {
    let missing = |what: &str| {
        HostingError::Keychain(format!(
            "no {what} saved for `{}`: enter it again",
            profile.label
        ))
    };
    Ok(match &profile.auth {
        AuthMethod::KeychainKey => SshAuth::Key {
            private_key: store
                .get(&names::private_key(&profile.id))?
                .ok_or_else(|| missing("private key"))?,
            passphrase: store.get(&names::passphrase(&profile.id))?,
        },
        AuthMethod::KeyFile { path } => SshAuth::Key {
            private_key: Secret::new(
                std::fs::read_to_string(path)
                    .map_err(|e| HostingError::Invalid(format!("{}: {e}", path.display())))?,
            ),
            passphrase: store.get(&names::passphrase(&profile.id))?,
        },
    })
}

/// The recommended setup (ARCHITECTURE §6.3): connect once with the password (never stored),
/// generate an Ed25519 key, append its public line to `~/.ssh/authorized_keys`, keep the private key
/// in the keychain, then check that the key alone logs in. Returns the profile and the public line.
pub async fn bootstrap_key(
    id: &str,
    label: &str,
    target: &SshTarget,
    password: &Secret,
    known_hosts: Arc<KnownHosts>,
    secrets: &dyn SecretStore,
    options: &ConnectOptions,
) -> Result<(RemoteProfile, String)> {
    validate_name(id)?;
    let key = crate::ssh::generate_key("rustic-launcher")?;
    let session = SshSession::connect(
        target,
        &SshAuth::Password(password.clone()),
        known_hosts.clone(),
        options,
    )
    .await?;
    let script = crate::ssh::authorize_key_script(&key.public_line)?;
    let r = session.run(&script).await;
    session.close().await;
    r?;
    secrets.set(&names::private_key(id), &key.private_openssh)?;
    let check = SshSession::connect(
        target,
        &SshAuth::Key {
            private_key: key.private_openssh.clone(),
            passphrase: None,
        },
        known_hosts,
        options,
    )
    .await;
    match check {
        Ok(s) => s.close().await,
        Err(e) => {
            let _ = secrets.delete(&names::private_key(id));
            return Err(HostingError::AuthFailed {
                user: target.user.clone(),
                host: target.host_id(),
                detail: format!(
                    "the key was added to ~/.ssh/authorized_keys but the server does not accept it ({e}); check `PubkeyAuthentication yes` in sshd_config"
                ),
            });
        }
    }
    Ok((
        RemoteProfile {
            id: id.to_owned(),
            label: label.to_owned(),
            target: target.clone(),
            auth: AuthMethod::KeychainKey,
        },
        key.public_line,
    ))
}

struct Connection {
    ssh: Arc<SshSession>,
    info: SystemInfo,
    layout: Layout,
}

/// The VPS provider for one connection profile.
pub struct RemoteSshProvider {
    profile: RemoteProfile,
    secrets: Arc<dyn SecretStore>,
    known_hosts: Arc<KnownHosts>,
    connect: ConnectOptions,
    install_options: std::sync::Mutex<RemoteInstallOptions>,
    conn: Mutex<Option<Arc<Connection>>>,
}

impl RemoteSshProvider {
    pub fn new(
        profile: RemoteProfile,
        secrets: Arc<dyn SecretStore>,
        known_hosts: Arc<KnownHosts>,
    ) -> Result<Self> {
        validate_name(&profile.id)?;
        Ok(Self {
            profile,
            secrets,
            known_hosts,
            connect: ConnectOptions::default(),
            install_options: std::sync::Mutex::new(RemoteInstallOptions::default()),
            conn: Mutex::new(None),
        })
    }

    pub fn profile(&self) -> &RemoteProfile {
        &self.profile
    }

    /// Options for the next `install` (firewall consent, Horizon version or archive...).
    pub fn set_install_options(&self, options: RemoteInstallOptions) {
        *self.install_options.lock().unwrap() = options;
    }

    pub fn set_connect_options(&mut self, options: ConnectOptions) {
        self.connect = options;
    }

    async fn connection(&self) -> Result<Arc<Connection>> {
        let mut guard = self.conn.lock().await;
        if let Some(c) = guard.as_ref() {
            return Ok(c.clone());
        }
        let auth = load_auth(&self.profile, self.secrets.as_ref())?;
        let ssh = Arc::new(
            SshSession::connect(
                &self.profile.target,
                &auth,
                self.known_hosts.clone(),
                &self.connect,
            )
            .await?,
        );
        let info = parse_probe(&ssh.run(PROBE_SCRIPT).await?)?;
        let dedicated = self.install_options.lock().unwrap().dedicated_user.clone();
        let layout = Layout::for_system(&info, &dedicated)?;
        let c = Arc::new(Connection { ssh, info, layout });
        *guard = Some(c.clone());
        Ok(c)
    }

    /// Forget the connection after a transport error, so the next call reconnects.
    async fn reset_on_transport_error<T>(&self, r: Result<T>) -> Result<T> {
        if matches!(r, Err(HostingError::Ssh(_) | HostingError::Sftp(_))) {
            *self.conn.lock().await = None;
        }
        r
    }

    /// Connect and describe the VPS (`hosting_ssh_test`).
    pub async fn test_connection(&self) -> Result<SystemInfo> {
        let r = self.connection().await.map(|c| c.info.clone());
        self.reset_on_transport_error(r).await
    }

    pub async fn disconnect(&self) {
        if let Some(c) = self.conn.lock().await.take() {
            c.ssh.close().await;
        }
    }

    async fn ensure_server(&self, c: &Connection, name: &str) -> Result<()> {
        validate_name(name)?;
        let test = format!(
            "test -f {}",
            quote(&format!("{}/server.toml", c.layout.server_dir(name)))?
        );
        if c.ssh
            .exec(&c.layout.as_service(&test)?, None)
            .await?
            .success()
        {
            Ok(())
        } else {
            Err(HostingError::UnknownServer(name.to_owned()))
        }
    }

    /// `horizon-server --dir <server> args...`, its stdout.
    async fn horizon(&self, name: &str, args: &[&str]) -> Result<String> {
        let c = self.connection().await?;
        self.ensure_server(&c, name).await?;
        let r = c.ssh.run(&c.layout.horizon(name, args)?).await;
        self.reset_on_transport_error(r).await
    }

    async fn horizon_json(&self, name: &str, args: &[&str]) -> Result<serde_json::Value> {
        let out = self.horizon(name, args).await?;
        serde_json::from_str(out.trim())
            .map_err(|e| HostingError::Protocol(format!("{}: {e}: {}", args.join(" "), out.trim())))
    }

    /// The control API's address and token (`.horizon/api.json`, readable only by the server's
    /// account), or `None` while the supervisor is not running.
    async fn api_endpoint(&self, c: &Connection, name: &str) -> Result<Option<ApiEndpoint>> {
        let path = format!("{}/.horizon/api.json", c.layout.server_dir(name));
        let out = c
            .ssh
            .exec(
                &c.layout
                    .as_service(&format!("cat {} 2>/dev/null", quote(&path)?))?,
                None,
            )
            .await?;
        if !out.success() || out.stdout.is_empty() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_slice(&out.stdout)?))
    }

    /// Upload a local file to `dest` on the VPS (through the login user's upload folder).
    async fn upload_to(&self, c: &Connection, name: &str, local: &Path, dest: &str) -> Result<()> {
        let file = local
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| "upload".into());
        let upload = format!("{}/{}", c.layout.upload_dir(name), sanitize_file(&file)?);
        c.ssh
            .run(
                Cmd::new("mkdir")?
                    .args(["-p", &c.layout.upload_dir(name)])?
                    .as_str(),
            )
            .await?;
        c.ssh.sftp_upload(local, &upload, &|_, _| {}).await?;
        c.ssh.run(&c.layout.place(&upload, dest, "0644")?).await?;
        Ok(())
    }

    /// The files (name, size) in a server sub-folder.
    async fn list_dir(&self, c: &Connection, dir: &str) -> Result<Vec<(String, u64)>> {
        let script = format!(
            "cd {} 2>/dev/null || exit 0; for f in *; do [ -f \"$f\" ] && printf '%s\\t%s\\n' \"$f\" \"$(wc -c < \"$f\")\"; done; true",
            quote(dir)?
        );
        let out = c.ssh.run(&c.layout.as_service(&script)?).await?;
        Ok(out
            .lines()
            .filter_map(|l| {
                let (n, s) = l.split_once('\t')?;
                Some((n.to_owned(), s.trim().parse().unwrap_or(0)))
            })
            .collect())
    }

    async fn world_dir(&self, c: &Connection, name: &str) -> Result<String> {
        let cfg = self.horizon_json(name, &["config", "--json"]).await?;
        let world = cfg["world"]["name"].as_str().unwrap_or("world");
        if world.contains('/') || world.starts_with('.') || world.is_empty() {
            return Err(HostingError::Protocol(format!("odd world name `{world}`")));
        }
        Ok(format!("{}/{world}", c.layout.server_dir(name)))
    }

    /// Install with explicit options (the trait's `install` uses the stored ones).
    pub async fn install_with(
        &self,
        spec: &ServerSpec,
        options: RemoteInstallOptions,
        progress: &dyn ProgressSink,
    ) -> Result<ServerRef> {
        validate_name(&spec.name)?;
        steps::init_args(spec)?;
        self.set_install_options(options.clone());
        emit(
            progress,
            "install",
            "connect",
            "Connect over SSH",
            StepState::Running,
            None,
        );
        let c = match self.connection().await {
            Ok(c) => c,
            Err(e) => {
                emit(
                    progress,
                    "install",
                    "connect",
                    "Connect over SSH",
                    StepState::Failed,
                    Some(e.to_string()),
                );
                return Err(e);
            }
        };
        emit(
            progress,
            "install",
            "connect",
            "Connect over SSH",
            StepState::Done,
            Some(format!(
                "{}@{} ({}, {})",
                c.info.user,
                self.profile.target.host,
                c.ssh.host_key().fingerprint,
                match c.layout.account {
                    layout::Account::Dedicated { .. } => "dedicated account",
                    layout::Account::Login => "your account",
                }
            )),
        );
        let ctx = InstallCtx {
            ssh: c.ssh.clone(),
            info: c.info.clone(),
            layout: c.layout.clone(),
            spec: spec.clone(),
            opts: options,
            public_host: self.profile.target.host.clone(),
            server_dir_existed: AtomicBool::new(false),
        };
        let plan = steps::install_plan(&ctx);
        let r = run_plan("install", &plan, &ctx, progress).await;
        self.reset_on_transport_error(r).await?;
        Ok(ServerRef {
            provider: ProviderKind::RemoteSsh,
            target: self.profile.id.clone(),
            name: spec.name.clone(),
        })
    }

    /// Upload prepared game files to an installed server later (when the install had none).
    pub async fn import_resources(
        &self,
        name: &str,
        local: &Path,
        progress: &dyn ProgressSink,
    ) -> Result<()> {
        let c = self.connection().await?;
        self.ensure_server(&c, name).await?;
        let mut spec = placeholder_spec(name);
        spec.prepared_resources = Some(local.to_owned());
        let ctx = InstallCtx {
            ssh: c.ssh.clone(),
            info: c.info.clone(),
            layout: c.layout.clone(),
            spec,
            opts: self.install_options.lock().unwrap().clone(),
            public_host: self.profile.target.host.clone(),
            server_dir_existed: AtomicBool::new(true),
        };
        let plan: Vec<Box<dyn Step<InstallCtx>>> = vec![Box::new(steps::ImportResources)];
        run_plan("import-resources", &plan, &ctx, progress).await?;
        Ok(())
    }
}

fn placeholder_spec(name: &str) -> ServerSpec {
    ServerSpec {
        name: name.to_owned(),
        port: 0,
        world: String::new(),
        seed: None,
        whitelist: true,
        players: Vec::new(),
        op: None,
        online_mode: true,
        eula_accepted: true,
        engine: EngineSource::Horizon,
        prepared_resources: None,
        settings: Default::default(),
    }
}

fn emit(
    sink: &dyn ProgressSink,
    operation: &str,
    step: &str,
    title: &str,
    state: StepState,
    detail: Option<String>,
) {
    sink.emit(ProgressEvent {
        operation: operation.into(),
        step: step.into(),
        title: title.into(),
        index: 0,
        total: 0,
        state,
        detail,
        bytes: None,
    });
}

/// A file name we put on the VPS: no folders, no hidden files, a conservative character set.
pub fn sanitize_file(name: &str) -> Result<String> {
    let ok = !name.is_empty()
        && name.len() <= 128
        && !name.starts_with('.')
        && name.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' ' | '+' | '(' | ')')
        });
    if ok {
        Ok(name.to_owned())
    } else {
        Err(HostingError::Invalid(format!(
            "`{name}` is not a usable file name (letters, digits, spaces and . _ - + ( ) only)"
        )))
    }
}

/// Map a control-API event to a console event.
pub fn console_event(ev: &http::SseEvent) -> ConsoleEvent {
    let data: serde_json::Value =
        serde_json::from_str(&ev.data).unwrap_or(serde_json::Value::String(ev.data.clone()));
    match ev.event.as_str() {
        "log" => ConsoleEvent::Log {
            line: data["line"].as_str().unwrap_or_default().to_owned(),
        },
        "phase" => ConsoleEvent::Phase {
            phase: data["phase"].as_str().unwrap_or_default().to_owned(),
        },
        other => ConsoleEvent::Game {
            kind: other.to_owned(),
            data,
        },
    }
}

#[async_trait]
impl HostingProvider for RemoteSshProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::RemoteSsh
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::ALL
    }

    async fn install(&self, spec: &ServerSpec, progress: &dyn ProgressSink) -> Result<ServerRef> {
        let opts = self.install_options.lock().unwrap().clone();
        self.install_with(spec, opts, progress).await
    }

    async fn start(&self, server: &str) -> Result<()> {
        let c = self.connection().await?;
        self.ensure_server(&c, server).await?;
        let r = steps::start_server(&c.ssh, &c.layout, server).await;
        self.reset_on_transport_error(r).await
    }

    async fn stop(&self, server: &str) -> Result<()> {
        let c = self.connection().await?;
        self.ensure_server(&c, server).await?;
        let r = steps::stop_server(&c.ssh, &c.layout, server).await;
        self.reset_on_transport_error(r).await
    }

    async fn restart(&self, server: &str) -> Result<()> {
        if self.status(server).await?.is_running() {
            // Restarts the game; the supervisor (and its unit) stay up.
            self.horizon(server, &["restart"]).await.map(|_| ())
        } else {
            self.start(server).await
        }
    }

    async fn status(&self, server: &str) -> Result<ServerStatus> {
        let v = self.horizon_json(server, &["status", "--json"]).await?;
        Ok(ServerStatus::from_horizon(&v))
    }

    async fn console_subscribe(&self, server: &str) -> Result<ConsoleSubscription> {
        let c = self.connection().await?;
        self.ensure_server(&c, server).await?;
        let endpoint = self.api_endpoint(&c, server).await?.ok_or_else(|| {
            HostingError::Precondition(
                "the server is not running: start it to see its console".into(),
            )
        })?;
        let (host, port) = endpoint.host_port()?;
        let stream = c.ssh.direct_tcpip(&host, port).await?;
        let (tx, rx) = tokio::sync::mpsc::channel(512);
        let task = tokio::spawn(async move {
            let (sse_tx, mut sse_rx) = tokio::sync::mpsc::channel(512);
            let forward_tx = tx.clone();
            let forward = tokio::spawn(async move {
                while let Some(ev) = sse_rx.recv().await {
                    if forward_tx.send(console_event(&ev)).await.is_err() {
                        return;
                    }
                }
            });
            let reason = match http::events(
                stream,
                "/api/v1/events?types=log,phase,join,leave,kick,chat,death",
                &endpoint,
                sse_tx,
            )
            .await
            {
                Ok(()) => "the server closed the stream".to_owned(),
                Err(e) => e.to_string(),
            };
            let _ = forward.await;
            let _ = tx.send(ConsoleEvent::Closed { reason }).await;
        });
        Ok(ConsoleSubscription::new(rx, task))
    }

    async fn send_command(&self, server: &str, line: &str) -> Result<CommandOutcome> {
        let line = line.trim().trim_start_matches('/');
        if line.is_empty() || line.contains(['\n', '\r', '\0']) {
            return Err(HostingError::Invalid(
                "one console command, on one line".into(),
            ));
        }
        let out = self.horizon(server, &["cmd", line]).await?;
        Ok(CommandOutcome {
            output: out.lines().map(str::to_owned).collect(),
        })
    }

    async fn logs(&self, server: &str, lines: usize) -> Result<Vec<String>> {
        let n = lines.clamp(1, 5000).to_string();
        let out = self.horizon(server, &["logs", "-n", &n]).await?;
        Ok(out.lines().map(str::to_owned).collect())
    }

    async fn players(&self, server: &str) -> Result<PlayersSnapshot> {
        Ok(PlayersSnapshot::from_horizon(
            &self.horizon_json(server, &["players", "--json"]).await?,
        ))
    }

    async fn player_action(&self, server: &str, action: &PlayerAction) -> Result<CommandOutcome> {
        let args = action.horizon_args();
        for a in &args {
            if a.contains(['\n', '\r', '\0']) {
                return Err(HostingError::Invalid(
                    "names and reasons are one line".into(),
                ));
            }
        }
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = self.horizon(server, &args).await?;
        Ok(CommandOutcome {
            output: out.lines().map(str::to_owned).collect(),
        })
    }

    async fn config_read(&self, server: &str) -> Result<serde_json::Value> {
        self.horizon_json(server, &["config", "--json"]).await
    }

    async fn config_write(
        &self,
        server: &str,
        changes: &[(String, String)],
    ) -> Result<ConfigOutcome> {
        if changes.is_empty() {
            return Ok(ConfigOutcome::default());
        }
        let mut args = vec!["config".to_owned(), "set".to_owned()];
        args.extend(steps::config_pairs(changes)?);
        args.push("--json".into());
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        Ok(ConfigOutcome::from_horizon(
            &self.horizon_json(server, &args).await?,
        ))
    }

    async fn plugins_list(&self, server: &str) -> Result<Vec<PluginInfo>> {
        let c = self.connection().await?;
        self.ensure_server(&c, server).await?;
        let dir = format!("{}/plugins", c.layout.server_dir(server));
        Ok(self
            .list_dir(&c, &dir)
            .await?
            .into_iter()
            .filter_map(|(file, bytes)| {
                let (id, enabled) = crate::rplugin::classify(&file)?;
                Some(PluginInfo {
                    id,
                    file,
                    enabled,
                    bytes,
                })
            })
            .collect())
    }

    async fn plugin_upload(&self, server: &str, rplugin: &Path) -> Result<PluginInfo> {
        let manifest = crate::rplugin::read_manifest(rplugin)?;
        let c = self.connection().await?;
        self.ensure_server(&c, server).await?;
        let dir = format!("{}/plugins", c.layout.server_dir(server));
        let dest = format!("{dir}/{}.rplugin", manifest.id);
        self.upload_to(&c, server, rplugin, &dest).await?;
        // A disabled copy of the same plugin would be a stale duplicate: remove it.
        c.ssh
            .run(
                &c.layout.as_service(
                    Cmd::new("rm")?
                        .args([
                            "-f",
                            &format!("{dir}/{}{}", manifest.id, crate::rplugin::DISABLED_SUFFIX),
                        ])?
                        .as_str(),
                )?,
            )
            .await?;
        Ok(PluginInfo {
            id: manifest.id.clone(),
            file: format!("{}.rplugin", manifest.id),
            enabled: true,
            bytes: std::fs::metadata(rplugin)?.len(),
        })
    }

    async fn plugin_set_enabled(&self, server: &str, id: &str, enabled: bool) -> Result<()> {
        crate::rplugin::validate_id(id)?;
        let c = self.connection().await?;
        self.ensure_server(&c, server).await?;
        let dir = format!("{}/plugins", c.layout.server_dir(server));
        let on = format!("{dir}/{id}.rplugin");
        let off = format!("{dir}/{id}{}", crate::rplugin::DISABLED_SUFFIX);
        let (from, to) = if enabled { (&off, &on) } else { (&on, &off) };
        let script = format!(
            "if [ -f {to} ] && [ ! -f {from} ]; then exit 0; fi; mv -f {from} {to}",
            from = quote(from)?,
            to = quote(to)?
        );
        c.ssh.run(&c.layout.as_service(&script)?).await?;
        Ok(())
    }

    async fn plugin_remove(&self, server: &str, id: &str) -> Result<()> {
        crate::rplugin::validate_id(id)?;
        let c = self.connection().await?;
        self.ensure_server(&c, server).await?;
        let dir = format!("{}/plugins", c.layout.server_dir(server));
        let rm = Cmd::new("rm")?.args([
            "-f",
            &format!("{dir}/{id}.rplugin"),
            &format!("{dir}/{id}{}", crate::rplugin::DISABLED_SUFFIX),
        ])?;
        c.ssh.run(&c.layout.as_service(rm.as_str())?).await?;
        Ok(())
    }

    async fn datapacks_list(&self, server: &str) -> Result<Vec<DatapackInfo>> {
        let c = self.connection().await?;
        self.ensure_server(&c, server).await?;
        let dir = format!("{}/datapacks", self.world_dir(&c, server).await?);
        Ok(self
            .list_dir(&c, &dir)
            .await?
            .into_iter()
            .map(|(file, bytes)| DatapackInfo { file, bytes })
            .collect())
    }

    async fn datapack_upload(&self, server: &str, zip: &Path) -> Result<DatapackInfo> {
        let file = zip
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let file = sanitize_file(&file)?;
        if !file.to_ascii_lowercase().ends_with(".zip") {
            return Err(HostingError::Invalid("a datapack is a .zip file".into()));
        }
        let c = self.connection().await?;
        self.ensure_server(&c, server).await?;
        let dest = format!("{}/datapacks/{file}", self.world_dir(&c, server).await?);
        self.upload_to(&c, server, zip, &dest).await?;
        Ok(DatapackInfo {
            file,
            bytes: std::fs::metadata(zip)?.len(),
        })
    }

    async fn datapack_remove(&self, server: &str, file: &str) -> Result<()> {
        let file = sanitize_file(file)?;
        let c = self.connection().await?;
        self.ensure_server(&c, server).await?;
        let path = format!("{}/datapacks/{file}", self.world_dir(&c, server).await?);
        c.ssh
            .run(
                &c.layout
                    .as_service(Cmd::new("rm")?.args(["-f", &path])?.as_str())?,
            )
            .await?;
        Ok(())
    }

    async fn backup_create(&self, server: &str, name: Option<&str>) -> Result<String> {
        let mut args = vec!["backup"];
        if let Some(n) = name.filter(|n| !n.is_empty()) {
            args.push(n);
        }
        let out = self.horizon(server, &args).await?;
        // "Backed up to backups/<file>"
        out.lines()
            .find_map(|l| l.split_once("backups/").map(|(_, f)| f.trim().to_owned()))
            .ok_or_else(|| HostingError::Protocol(format!("backup: {}", out.trim())))
    }

    async fn backups_list(&self, server: &str) -> Result<Vec<BackupInfo>> {
        Ok(BackupInfo::list_from_horizon(
            &self.horizon_json(server, &["backups", "--json"]).await?,
        ))
    }

    async fn backup_restore(
        &self,
        server: &str,
        backup: &str,
        dimension: Option<&str>,
    ) -> Result<()> {
        let mut args = vec!["restore", backup, "--yes"];
        if let Some(d) = dimension {
            args.extend(["--dimension", d]);
        }
        self.horizon(server, &args).await.map(|_| ())
    }

    async fn backup_download(&self, server: &str, backup: &str, dest: &Path) -> Result<u64> {
        // Only names the supervisor lists (no path tricks).
        if !self
            .backups_list(server)
            .await?
            .iter()
            .any(|b| b.name == backup)
        {
            return Err(HostingError::Invalid(format!("no backup named {backup}")));
        }
        let c = self.connection().await?;
        let part = dest.with_extension("part");
        let mut file = tokio::fs::File::create(&part).await?;
        let written = match self.api_endpoint(&c, server).await? {
            Some(endpoint) => {
                let (host, port) = endpoint.host_port()?;
                let stream = c.ssh.direct_tcpip(&host, port).await?;
                let path = format!("/api/v1/backups/download?name={}", url_encode(backup));
                http::download(stream, &path, &endpoint, &mut file, &|_, _| {}).await?
            }
            None => {
                // Stopped: read the file itself as the server's account.
                let path = format!("{}/backups/{backup}", c.layout.server_dir(server));
                let cmd = c.layout.as_service(Cmd::new("cat")?.arg(&path)?.as_str())?;
                let status = c.ssh.exec_streaming(&cmd, None, &mut file, None).await?;
                if status != 0 {
                    return Err(HostingError::RemoteCommand {
                        command: "cat backup".into(),
                        status,
                        stderr: String::new(),
                    });
                }
                tokio::fs::metadata(&part).await?.len()
            }
        };
        drop(file);
        tokio::fs::rename(&part, dest).await?;
        Ok(written)
    }

    async fn update_engine(
        &self,
        server: &str,
        request: &UpdateRequest,
        progress: &dyn ProgressSink,
    ) -> Result<()> {
        let c = self.connection().await?;
        self.ensure_server(&c, server).await?;
        if let Some(v) = &request.horizon_version {
            emit(
                progress,
                "update",
                "horizon",
                "Update Horizon",
                StepState::Running,
                None,
            );
            let mut args = vec!["update"];
            if !v.is_empty() {
                args.extend(["--version", v.as_str()]);
            }
            let out = self.horizon(server, &args).await?;
            emit(
                progress,
                "update",
                "horizon",
                "Update Horizon",
                StepState::Done,
                out.lines().last().map(str::to_owned),
            );
        }
        if let Some(engine) = &request.engine {
            let sha = match engine {
                EngineSource::RusticUpload { sha256, .. }
                | EngineSource::RusticRelease { sha256, .. } => sha256.to_ascii_lowercase(),
                EngineSource::Horizon => String::new(),
            };
            emit(
                progress,
                "update",
                "engine",
                "Update the Rustic engine",
                StepState::Running,
                None,
            );
            let path = if sha.is_empty() {
                String::new()
            } else {
                steps::install_engine(&c.ssh, &c.layout, server, engine, &sha, None).await?
            };
            let set = format!("engine.path={path}");
            self.horizon(server, &["config", "set", &set, "--json"])
                .await?;
            if self.status(server).await?.is_running() {
                self.horizon(server, &["restart"]).await?;
            }
            emit(
                progress,
                "update",
                "engine",
                "Update the Rustic engine",
                StepState::Done,
                Some(path),
            );
        }
        Ok(())
    }

    async fn uninstall(
        &self,
        server: &str,
        options: &UninstallOptions,
        progress: &dyn ProgressSink,
    ) -> Result<()> {
        validate_name(server)?;
        let c = self.connection().await?;
        let mut spec = placeholder_spec(server);
        // The port, for the firewall rule (best effort: the folder may be half-installed).
        if let Ok(cfg) = self.horizon_json(server, &["config", "--json"]).await {
            spec.port = cfg["server"]["port"].as_u64().unwrap_or(0) as u16;
        }
        let ctx = UninstallCtx {
            ssh: c.ssh.clone(),
            info: c.info.clone(),
            layout: c.layout.clone(),
            name: server.to_owned(),
            port: spec.port,
            options: options.clone(),
        };
        let plan: Vec<Box<dyn Step<UninstallCtx>>> = vec![
            Box::new(UStop),
            Box::new(UService),
            Box::new(UFirewall),
            Box::new(UFiles),
            Box::new(URuntime),
        ];
        run_plan("uninstall", &plan, &ctx, progress).await?;
        Ok(())
    }
}

fn url_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Uninstall context. Uninstall steps are each idempotent and never rolled back.
pub struct UninstallCtx {
    ssh: Arc<SshSession>,
    info: SystemInfo,
    layout: Layout,
    name: String,
    port: u16,
    options: UninstallOptions,
}

struct UStop;
#[async_trait]
impl Step<UninstallCtx> for UStop {
    fn id(&self) -> &'static str {
        "stop"
    }
    fn title(&self) -> String {
        "Stop the server".into()
    }
    async fn is_done(&self, c: &UninstallCtx) -> Result<bool> {
        Ok(steps::phase(&c.ssh, &c.layout, &c.name)
            .await
            .map(|p| p == "stopped")
            .unwrap_or(true))
    }
    async fn apply(&self, c: &UninstallCtx, _p: &StepProgress<'_>) -> Result<()> {
        steps::stop_server(&c.ssh, &c.layout, &c.name).await
    }
}

struct UService;
#[async_trait]
impl Step<UninstallCtx> for UService {
    fn id(&self) -> &'static str {
        "service"
    }
    fn title(&self) -> String {
        "Remove the service".into()
    }
    async fn is_done(&self, c: &UninstallCtx) -> Result<bool> {
        Ok(!steps::service_enabled(&c.ssh, &c.layout, &c.layout.planned_service(&c.name)).await?)
    }
    async fn apply(&self, c: &UninstallCtx, _p: &StepProgress<'_>) -> Result<()> {
        steps::remove_service(&c.ssh, &c.layout, &c.name).await
    }
}

struct UFirewall;
#[async_trait]
impl Step<UninstallCtx> for UFirewall {
    fn id(&self) -> &'static str {
        "firewall"
    }
    fn title(&self) -> String {
        "Close the game port".into()
    }
    async fn is_done(&self, c: &UninstallCtx) -> Result<bool> {
        Ok(c.port == 0 || c.info.firewall().is_none() || c.layout.as_root("true").is_err())
    }
    async fn apply(&self, c: &UninstallCtx, _p: &StepProgress<'_>) -> Result<()> {
        let fw = c.info.firewall().expect("checked in is_done");
        let [.., remove] = steps::firewall_commands(fw, c.port, &c.name)?;
        c.ssh.run(&c.layout.as_root(&remove)?).await?;
        Ok(())
    }
}

struct UFiles;
#[async_trait]
impl Step<UninstallCtx> for UFiles {
    fn id(&self) -> &'static str {
        "files"
    }
    fn title(&self) -> String {
        "Delete the server folder (world, backups)".into()
    }
    async fn is_done(&self, c: &UninstallCtx) -> Result<bool> {
        Ok(!c.options.delete_files)
    }
    async fn apply(&self, c: &UninstallCtx, _p: &StepProgress<'_>) -> Result<()> {
        let dir = c.layout.server_dir(&c.name);
        c.ssh
            .run(
                &c.layout
                    .as_service(Cmd::new("rm")?.args(["-rf", &dir])?.as_str())?,
            )
            .await?;
        let up = c.layout.upload_dir(&c.name);
        c.ssh
            .run(Cmd::new("rm")?.args(["-rf", &up])?.as_str())
            .await?;
        Ok(())
    }
}

struct URuntime;
#[async_trait]
impl Step<UninstallCtx> for URuntime {
    fn id(&self) -> &'static str {
        "runtime"
    }
    fn title(&self) -> String {
        "Remove Horizon and the Rustic account (when no server is left)".into()
    }
    async fn is_done(&self, c: &UninstallCtx) -> Result<bool> {
        if !c.options.remove_runtime {
            return Ok(true);
        }
        // Another server still there: keep the runtime.
        let others = c
            .ssh
            .exec(
                &c.layout.as_service(&format!(
                    "ls -A {} 2>/dev/null | grep -qv '^{}$'",
                    quote(&c.layout.servers_dir())?,
                    c.name
                ))?,
                None,
            )
            .await?;
        Ok(others.success())
    }
    async fn apply(&self, c: &UninstallCtx, _p: &StepProgress<'_>) -> Result<()> {
        match c.layout.service_user() {
            Some(user) => {
                let script = format!(
                    "(userdel {u} 2>/dev/null || deluser {u} 2>/dev/null); (groupdel {u} 2>/dev/null || delgroup {u} 2>/dev/null); rm -rf {home}; true",
                    u = quote(user)?,
                    home = quote(&c.layout.base)?
                );
                c.ssh.run(&c.layout.as_root(&script)?).await?;
            }
            None => {
                c.ssh
                    .run(Cmd::new("rm")?.args(["-rf", &c.layout.base])?.as_str())
                    .await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret::MemoryStore;

    #[test]
    fn file_names() {
        assert!(sanitize_file("my pack (1).zip").is_ok());
        assert!(sanitize_file("../x.zip").is_err());
        assert!(sanitize_file(".hidden").is_err());
        assert!(sanitize_file("a/b.zip").is_err());
        assert!(sanitize_file("a$b.zip").is_err());
        assert_eq!(
            url_encode("2026-10-31T12-00-00Z-manual.zip"),
            "2026-10-31T12-00-00Z-manual.zip"
        );
        assert_eq!(url_encode("a b&c"), "a%20b%26c");
    }

    #[test]
    fn console_events_from_sse() {
        let ev = |event: &str, data: &str| http::SseEvent {
            id: Some("1".into()),
            event: event.into(),
            data: data.into(),
        };
        assert_eq!(
            console_event(&ev("log", r#"{"id":1,"type":"log","line":"Done (2.1s)!"}"#)),
            ConsoleEvent::Log {
                line: "Done (2.1s)!".into()
            }
        );
        assert_eq!(
            console_event(&ev("phase", r#"{"phase":"running"}"#)),
            ConsoleEvent::Phase {
                phase: "running".into()
            }
        );
        match console_event(&ev("chat", r#"{"name":"Alex","message":"hi"}"#)) {
            ConsoleEvent::Game { kind, data } => {
                assert_eq!(kind, "chat");
                assert_eq!(data["message"], "hi");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn auth_comes_from_the_keychain_only() {
        let store = MemoryStore::default();
        let mut profile = RemoteProfile {
            id: "my-vps".into(),
            label: "My VPS".into(),
            target: SshTarget {
                host: "203.0.113.10".into(),
                port: 22,
                user: "alex".into(),
            },
            auth: AuthMethod::KeychainKey,
        };
        assert!(matches!(
            load_auth(&profile, &store),
            Err(HostingError::Keychain(_))
        ));
        store
            .set(&names::private_key("my-vps"), &"KEY".into())
            .unwrap();
        match load_auth(&profile, &store).unwrap() {
            SshAuth::Key {
                private_key,
                passphrase,
            } => {
                assert_eq!(private_key.expose(), "KEY");
                assert!(passphrase.is_none());
            }
            other => panic!("{other:?}"),
        }
        // Profiles serialise without any secret.
        let json = serde_json::to_string(&profile).unwrap();
        assert!(!json.contains("KEY"));
        profile.auth = AuthMethod::KeyFile {
            path: "/nonexistent/id_ed25519".into(),
        };
        assert!(matches!(
            load_auth(&profile, &store),
            Err(HostingError::Invalid(_))
        ));
    }

    #[test]
    fn options_round_trip() {
        let o = RemoteInstallOptions::default();
        let v = serde_json::to_value(&o).unwrap();
        assert_eq!(v["healthTimeout"], 300);
        assert_eq!(v["installerUrl"], HORIZON_INSTALLER_URL);
        let back: RemoteInstallOptions =
            serde_json::from_value(serde_json::json!({ "openFirewall": true })).unwrap();
        assert!(back.open_firewall);
        assert_eq!(back.dedicated_user, "rustic");
    }
}
