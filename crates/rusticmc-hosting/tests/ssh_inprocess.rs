//! The VPS provider against an in-process SSH server (russh's server side) on 127.0.0.1:
//! - a temp folder is the "remote filesystem" served over SFTP (so uploads really land);
//! - a fake executor answers `exec` requests like a Linux box with sudo, systemd and ufw would,
//!   records every command, and can be told to fail one;
//! - `direct-tcpip` forwards to a fake control API (HTTP + server-sent events) on 127.0.0.1.
//!
//! It checks: host key TOFU/pinning on a real handshake, the full install plan, idempotent re-runs,
//! rollback on failure, and the management calls (status, console, plugins, backups).

use russh::keys::{Algorithm, PrivateKey};
use russh::server::{Auth, ChannelOpenHandle, Msg, Server as _, Session};
use russh::{Channel, ChannelId};
use russh_sftp::protocol::{
    Attrs, Data, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode, Version,
};
use rusticmc_hosting::provider::HostingProvider;
use rusticmc_hosting::secret::names;
use rusticmc_hosting::ssh::{ConnectOptions, SshSession};
use rusticmc_hosting::*;
use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

// ---------------------------------------------------------------- the fake machine

/// What the fake VPS knows about itself (changes as commands "run").
#[derive(Default)]
struct Machine {
    commands: Mutex<Vec<String>>,
    /// Substring of a command that must fail (status 1).
    fail: Mutex<Option<String>>,
    started: AtomicBool,
    unit_enabled: AtomicBool,
    installed: AtomicBool,
    /// Pretend every "is it done?" check succeeds (an already installed server).
    everything_done: AtomicBool,
    engine_sha: Mutex<String>,
    api_port: AtomicU16,
    /// Keys appended to authorized_keys by a command.
    authorized: Mutex<Vec<russh::keys::PublicKey>>,
}

const PROBE: &str = "uname=Linux x86_64
uid=1000
user=tester
home=/home/tester
elevation=sudo
has=systemctl
has=loginctl
has=ufw
has=curl
has=tar
has=sha256sum
has=openssl
has=useradd
has=ss
has=install
systemd=running
mem_total_kb=4026532
mem_available_kb=3500000
cpus=2
disk_home_available_kb=40000000
disk_varlib_available_kb=40000000
--- os-release
PRETTY_NAME=\"Ubuntu 24.04.1 LTS\"
ID=ubuntu
ID_LIKE=debian
VERSION_ID=\"24.04\"
--- listening
0.0.0.0:22
--- end
";

impl Machine {
    fn commands(&self) -> Vec<String> {
        self.commands.lock().unwrap().clone()
    }

    fn position(&self, needle: &str) -> Option<usize> {
        self.commands().iter().position(|c| c.contains(needle))
    }

    /// (status, stdout, stderr) for a command, like the real box would answer.
    fn run(&self, cmd: &str) -> (u32, String, String) {
        self.commands.lock().unwrap().push(cmd.to_owned());
        if let Some(f) = self.fail.lock().unwrap().as_ref()
            && cmd.contains(f.as_str())
        {
            return (1, String::new(), format!("simulated failure of {f}"));
        }
        let ok = |out: &str| (0, out.to_owned(), String::new());
        let no = (1, String::new(), String::new());
        let done = self.everything_done.load(Ordering::SeqCst);
        if cmd.contains("--- os-release") {
            return ok(PROBE);
        }
        if cmd.contains("authorized_keys")
            && let Some(start) = cmd.find("ssh-ed25519 ")
        {
            let line: String = cmd[start..].chars().take_while(|c| *c != '\'').collect();
            let key = russh::keys::PublicKey::from_openssh(&line).unwrap();
            self.authorized.lock().unwrap().push(key);
            return ok("");
        }
        if cmd.contains("status --json") {
            return if self.started.load(Ordering::SeqCst) {
                ok(
                    r#"{"phase":"running","version":"nightly-2026-11-01-abc1234","engine_uptime_seconds":12,"crashes":0,"sample":{"rss_bytes":734003200,"cpu_percent":3.5,"game":{"tps":20.0,"mspt":{"average":1.2,"p95":3.1},"players":[{"name":"Alex"}],"max_players":20,"chunks":377}}}"#,
                )
            } else {
                ok(r#"{"phase":"stopped"}"#)
            };
        }
        if cmd.contains("systemctl start") {
            self.started.store(true, Ordering::SeqCst);
            return ok("");
        }
        if cmd.contains("disable --now") {
            self.started.store(false, Ordering::SeqCst);
            self.unit_enabled.store(false, Ordering::SeqCst);
            return ok("");
        }
        if cmd.contains("systemctl stop") {
            self.started.store(false, Ordering::SeqCst);
            return ok("");
        }
        if cmd.contains("is-enabled") {
            return if done || self.unit_enabled.load(Ordering::SeqCst) {
                ok("")
            } else {
                no
            };
        }
        if cmd.contains("systemctl enable") {
            self.unit_enabled.store(true, Ordering::SeqCst);
            return ok("");
        }
        if cmd.contains("ufw status | grep -q '\\''^Status: active")
            || cmd.contains("^Status: active")
        {
            return ok("");
        }
        if cmd.contains("ufw status | grep -qE") {
            return if done { ok("") } else { no };
        }
        if cmd.contains("id -u rustic") {
            return if done { ok("1001\n") } else { no };
        }
        if cmd.contains("test -d") || cmd.contains("test -x") {
            if cmd.contains("current/horizon-server") {
                return if done || self.installed.load(Ordering::SeqCst) {
                    ok("")
                } else {
                    no
                };
            }
            if cmd.contains("test -x") && cmd.contains("sha256sum") {
                // Engine present?
                return if done {
                    ok(&format!("{}  /x\n", self.engine_sha.lock().unwrap()))
                } else {
                    no
                };
            }
            return if done { ok("") } else { no };
        }
        if cmd.contains("test -f") {
            if cmd.contains("server.toml") {
                return if done || self.installed.load(Ordering::SeqCst) {
                    ok("")
                } else {
                    no
                };
            }
            if cmd.contains("horizon-resources.json") {
                return ok("");
            }
            return if done { ok("") } else { no };
        }
        if cmd.contains("sha256sum") {
            if cmd.contains("install.sh") {
                return ok("0f0f0f  install.sh\n");
            }
            if cmd.contains(".part") {
                return ok(&format!("{}  part\n", self.engine_sha.lock().unwrap()));
            }
            // The resources manifest on the VPS never matches: re-runs upload again.
            return no;
        }
        if cmd.contains(" init ") && cmd.contains("--eula") {
            self.installed.store(true, Ordering::SeqCst);
            return ok(
                "Wrote server.toml\nGame files: /var/lib/rustic/servers/survival/resources\n",
            );
        }
        if cmd.contains("install.sh") && cmd.contains(" sh ")
            || cmd.contains("sh /var/lib/rustic/staging/install.sh")
        {
            return ok(
                "Release signature OK (Horizon release key).\nInstalled Horizon nightly-2026-11-01-abc1234 in /var/lib/rustic/horizon/releases/x\n",
            );
        }
        if cmd.contains("config set") {
            return ok(r#"{"saved":["engine.path"],"applied":[],"restart":["engine.path"]}"#);
        }
        if cmd.contains("config --json") {
            return ok(r#"{"server":{"port":25565},"world":{"name":"world"}}"#);
        }
        if cmd.contains("import-resources") {
            return ok("Imported the game files into /var/lib/rustic/servers/survival/resources\n");
        }
        if cmd.contains(" cmd ") {
            return ok("There are 1 of a max of 20 players online: Alex\n");
        }
        if cmd.contains("players --json") {
            return ok(
                r#"{"online":[{"name":"Alex","uuid":"u","ping_ms":30}],"max_players":20,"ops":[],"whitelist":{"enabled":true,"players":[]},"bans":[],"ip_bans":[]}"#,
            );
        }
        if cmd.contains("backups --json") {
            return ok(r#"[{"name":"2026-11-10T12-00-00Z-manual.zip","bytes":5,"modified":1}]"#);
        }
        if cmd.contains(" backup") {
            return ok("Backed up to backups/2026-11-10T12-00-00Z-manual.zip\n");
        }
        if cmd.contains(".horizon/api.json") {
            let port = self.api_port.load(Ordering::SeqCst);
            return if port == 0 {
                no
            } else {
                ok(&format!(
                    r#"{{"address":"127.0.0.1:{port}","token":"tok","pid":1}}"#
                ))
            };
        }
        if cmd.contains("for f in *") {
            return ok("homes.rplugin\t120\nold.rplugin.disabled\t50\nconfig.toml\t3\n");
        }
        // mkdir, install, rm, mv, curl, useradd, ufw allow, printf > unit... succeed silently.
        ok("")
    }
}

// ---------------------------------------------------------------- SFTP over a temp folder

struct FsSftp {
    root: PathBuf,
    handles: HashMap<String, std::fs::File>,
    dirs: HashMap<String, Option<Vec<File>>>,
    next: u64,
}

impl FsSftp {
    fn local(&self, remote: &str) -> PathBuf {
        let rel = remote.trim_start_matches('/');
        if rel.is_empty() || rel == "." {
            self.root.clone()
        } else {
            self.root.join(rel)
        }
    }
    fn ok(id: u32) -> Status {
        Status {
            id,
            status_code: StatusCode::Ok,
            error_message: "Ok".into(),
            language_tag: "en-US".into(),
        }
    }
    fn handle(&mut self) -> String {
        self.next += 1;
        format!("h{}", self.next)
    }
}

fn io_status(e: std::io::Error) -> StatusCode {
    match e.kind() {
        std::io::ErrorKind::NotFound => StatusCode::NoSuchFile,
        std::io::ErrorKind::PermissionDenied => StatusCode::PermissionDenied,
        _ => StatusCode::Failure,
    }
}

impl russh_sftp::server::Handler for FsSftp {
    type Error = StatusCode;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported
    }

    async fn init(&mut self, _v: u32, _e: HashMap<String, String>) -> Result<Version, Self::Error> {
        Ok(Version::new())
    }

    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        let p = if path.starts_with('/') {
            path
        } else {
            "/home/tester".into()
        };
        Ok(Name {
            id,
            files: vec![File::dummy(p)],
        })
    }

    async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        let m = std::fs::metadata(self.local(&path)).map_err(io_status)?;
        Ok(Attrs {
            id,
            attrs: FileAttributes::from(&m),
        })
    }

    async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        self.stat(id, path).await
    }

    async fn fstat(&mut self, id: u32, handle: String) -> Result<Attrs, Self::Error> {
        let f = self.handles.get(&handle).ok_or(StatusCode::Failure)?;
        let m = f.metadata().map_err(io_status)?;
        Ok(Attrs {
            id,
            attrs: FileAttributes::from(&m),
        })
    }

    async fn setstat(
        &mut self,
        id: u32,
        _p: String,
        _a: FileAttributes,
    ) -> Result<Status, Self::Error> {
        Ok(Self::ok(id))
    }

    async fn fsetstat(
        &mut self,
        id: u32,
        _h: String,
        _a: FileAttributes,
    ) -> Result<Status, Self::Error> {
        Ok(Self::ok(id))
    }

    async fn mkdir(
        &mut self,
        id: u32,
        path: String,
        _a: FileAttributes,
    ) -> Result<Status, Self::Error> {
        std::fs::create_dir(self.local(&path)).map_err(io_status)?;
        Ok(Self::ok(id))
    }

    async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, Self::Error> {
        std::fs::remove_dir(self.local(&path)).map_err(io_status)?;
        Ok(Self::ok(id))
    }

    async fn remove(&mut self, id: u32, path: String) -> Result<Status, Self::Error> {
        std::fs::remove_file(self.local(&path)).map_err(io_status)?;
        Ok(Self::ok(id))
    }

    async fn rename(&mut self, id: u32, a: String, b: String) -> Result<Status, Self::Error> {
        std::fs::rename(self.local(&a), self.local(&b)).map_err(io_status)?;
        Ok(Self::ok(id))
    }

    async fn open(
        &mut self,
        id: u32,
        filename: String,
        flags: OpenFlags,
        _attrs: FileAttributes,
    ) -> Result<Handle, Self::Error> {
        let f = std::fs::OpenOptions::new()
            .read(flags.contains(OpenFlags::READ))
            .write(flags.contains(OpenFlags::WRITE))
            .create(flags.contains(OpenFlags::CREATE))
            .truncate(flags.contains(OpenFlags::TRUNCATE))
            .open(self.local(&filename))
            .map_err(io_status)?;
        let h = self.handle();
        self.handles.insert(h.clone(), f);
        Ok(Handle { id, handle: h })
    }

    async fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, Self::Error> {
        let f = self.handles.get_mut(&handle).ok_or(StatusCode::Failure)?;
        f.seek(SeekFrom::Start(offset)).map_err(io_status)?;
        let mut buf = vec![0u8; len as usize];
        let n = f.read(&mut buf).map_err(io_status)?;
        if n == 0 {
            return Err(StatusCode::Eof);
        }
        buf.truncate(n);
        Ok(Data { id, data: buf })
    }

    async fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, Self::Error> {
        let f = self.handles.get_mut(&handle).ok_or(StatusCode::Failure)?;
        f.seek(SeekFrom::Start(offset)).map_err(io_status)?;
        f.write_all(&data).map_err(io_status)?;
        Ok(Self::ok(id))
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
        self.handles.remove(&handle);
        self.dirs.remove(&handle);
        Ok(Self::ok(id))
    }

    async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, Self::Error> {
        let mut files = Vec::new();
        for e in std::fs::read_dir(self.local(&path)).map_err(io_status)? {
            let e = e.map_err(io_status)?;
            let m = e.metadata().map_err(io_status)?;
            files.push(File::new(
                e.file_name().to_string_lossy(),
                FileAttributes::from(&m),
            ));
        }
        let h = self.handle();
        self.dirs.insert(h.clone(), Some(files));
        Ok(Handle { id, handle: h })
    }

    async fn readdir(&mut self, id: u32, handle: String) -> Result<Name, Self::Error> {
        match self.dirs.get_mut(&handle).and_then(Option::take) {
            Some(files) => Ok(Name { id, files }),
            None => Err(StatusCode::Eof),
        }
    }
}

// ---------------------------------------------------------------- the SSH server

#[derive(Clone)]
struct FakeServer {
    machine: Arc<Machine>,
    root: PathBuf,
    client_key: russh::keys::PublicKey,
}

impl russh::server::Server for FakeServer {
    type Handler = Conn;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Conn {
        Conn {
            server: self.clone(),
            channels: HashMap::new(),
        }
    }
}

struct Conn {
    server: FakeServer,
    channels: HashMap<ChannelId, Channel<Msg>>,
}

impl russh::server::Handler for Conn {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        Ok(if user == "tester" && password == "correct horse" {
            Auth::Accept
        } else {
            Auth::reject()
        })
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        key: russh::server::VerifiedPublicKey<'_>,
    ) -> Result<Auth, Self::Error> {
        let known = key.public_key.key_data() == self.server.client_key.key_data()
            || self
                .server
                .machine
                .authorized
                .lock()
                .unwrap()
                .iter()
                .any(|k| k.key_data() == key.public_key.key_data());
        Ok(if user == "tester" && known {
            Auth::Accept
        } else {
            Auth::reject()
        })
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channels.insert(channel.id(), channel);
        reply.accept().await;
        Ok(())
    }

    async fn channel_open_direct_tcpip(
        &mut self,
        channel: Channel<Msg>,
        host: &str,
        port: u32,
        _oa: &str,
        _op: u32,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        // Only the VPS's loopback, like a real sshd with the API on 127.0.0.1.
        if host != "127.0.0.1" {
            reply
                .reject(russh::ChannelOpenFailure::AdministrativelyProhibited)
                .await;
            return Ok(());
        }
        reply.accept().await;
        let port = port as u16;
        tokio::spawn(async move {
            if let Ok(mut tcp) = tokio::net::TcpStream::connect(("127.0.0.1", port)).await {
                let mut stream = channel.into_stream();
                let _ = tokio::io::copy_bidirectional(&mut stream, &mut tcp).await;
            }
        });
        Ok(())
    }

    async fn exec_request(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let cmd = String::from_utf8_lossy(data).into_owned();
        let (status, out, err) = self.server.machine.run(&cmd);
        session.channel_success(channel)?;
        let handle = session.handle();
        tokio::spawn(async move {
            if !out.is_empty() {
                let _ = handle.data(channel, out.into_bytes()).await;
            }
            if !err.is_empty() {
                let _ = handle.extended_data(channel, 1, err.into_bytes()).await;
            }
            let _ = handle.exit_status_request(channel, status).await;
            let _ = handle.eof(channel).await;
            let _ = handle.close(channel).await;
        });
        Ok(())
    }

    async fn subsystem_request(
        &mut self,
        channel_id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if name != "sftp" {
            session.channel_failure(channel_id)?;
            return Ok(());
        }
        let Some(channel) = self.channels.remove(&channel_id) else {
            session.channel_failure(channel_id)?;
            return Ok(());
        };
        session.channel_success(channel_id)?;
        let sftp = FsSftp {
            root: self.server.root.clone(),
            handles: HashMap::new(),
            dirs: HashMap::new(),
            next: 0,
        };
        tokio::spawn(russh_sftp::server::run(channel.into_stream(), sftp));
        Ok(())
    }
}

struct Harness {
    machine: Arc<Machine>,
    root: tempfile::TempDir,
    port: u16,
    host_key: PrivateKey,
    client_key: PrivateKey,
    _server: tokio::task::JoinHandle<()>,
}

async fn harness() -> Harness {
    let root = tempfile::tempdir().unwrap();
    let machine = Arc::new(Machine::default());
    let host_key = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).unwrap();
    let client_key = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).unwrap();
    let config = Arc::new(russh::server::Config {
        keys: vec![host_key.clone()],
        auth_rejection_time: std::time::Duration::from_millis(10),
        auth_rejection_time_initial: Some(std::time::Duration::from_millis(0)),
        ..Default::default()
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let mut server = FakeServer {
        machine: machine.clone(),
        root: root.path().to_path_buf(),
        client_key: client_key.public_key().clone(),
    };
    let task = tokio::spawn(async move {
        let _ = server.run_on_socket(config, &listener).await;
    });
    Harness {
        machine,
        root,
        port,
        host_key,
        client_key,
        _server: task,
    }
}

impl Harness {
    fn target(&self) -> SshTarget {
        SshTarget {
            host: "127.0.0.1".into(),
            port: self.port,
            user: "tester".into(),
        }
    }

    fn host_key(&self) -> HostKey {
        HostKey::from_public_key(self.host_key.public_key())
    }

    /// A provider whose keychain holds the client key and whose pins trust the server.
    fn provider(&self) -> (RemoteSshProvider, Arc<MemoryStore>) {
        let secrets = Arc::new(MemoryStore::default());
        let pem = self
            .client_key
            .to_openssh(russh::keys::ssh_key::LineEnding::LF)
            .unwrap();
        secrets
            .set(
                &names::private_key("test-vps"),
                &Secret::new(pem.to_string()),
            )
            .unwrap();
        let known = Arc::new(KnownHosts::in_memory());
        known
            .trust(&self.target().host_id(), &self.host_key())
            .unwrap();
        let p = RemoteSshProvider::new(
            RemoteProfile {
                id: "test-vps".into(),
                label: "Test VPS".into(),
                target: self.target(),
                auth: AuthMethod::KeychainKey,
            },
            secrets.clone(),
            known,
        )
        .unwrap();
        (p, secrets)
    }

    fn remote(&self, path: &str) -> PathBuf {
        self.root.path().join(path.trim_start_matches('/'))
    }
}

fn engine_file(dir: &Path) -> (PathBuf, String) {
    let path = dir.join("rustic-server");
    std::fs::write(&path, b"\x7fELF fake rustic-server").unwrap();
    let sha = {
        use sha2::Digest;
        hex::encode(sha2::Sha256::digest(std::fs::read(&path).unwrap()))
    };
    (path, sha)
}

fn prepared_dir(dir: &Path) -> PathBuf {
    let p = dir.join("26.3");
    std::fs::create_dir_all(p.join("assets/minecraft/textures/block")).unwrap();
    std::fs::write(p.join("horizon-resources.json"), br#"{"minecraft":"26.3"}"#).unwrap();
    std::fs::write(p.join("facts.json"), b"{}").unwrap();
    std::fs::write(p.join("assets/minecraft/textures/block/stone.png"), b"PNG").unwrap();
    p
}

fn spec(engine: EngineSource, resources: Option<PathBuf>) -> ServerSpec {
    ServerSpec {
        name: "survival".into(),
        port: 25565,
        world: "world".into(),
        seed: None,
        whitelist: true,
        players: vec!["Alex".into()],
        op: Some("Alex".into()),
        online_mode: true,
        eula_accepted: true,
        engine,
        prepared_resources: resources,
        settings: BTreeMap::from([("server.motd".to_owned(), "Rustic test".to_owned())]),
    }
}

fn options() -> RemoteInstallOptions {
    RemoteInstallOptions {
        open_firewall: true,
        skip_public_ping: true,
        health_timeout: std::time::Duration::from_secs(10),
        ..Default::default()
    }
}

fn collect() -> (
    Arc<Mutex<Vec<ProgressEvent>>>,
    impl Fn(ProgressEvent) + Send + Sync,
) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let e2 = events.clone();
    (events, move |e: ProgressEvent| e2.lock().unwrap().push(e))
}

// ---------------------------------------------------------------- tests

#[tokio::test]
async fn host_key_is_confirmed_then_pinned() {
    let h = harness().await;
    let known = Arc::new(KnownHosts::in_memory());
    // Unknown host: the probe reports the fingerprint without authenticating.
    let (key, pinned) =
        SshSession::probe_host_key(&h.target(), known.clone(), &ConnectOptions::default())
            .await
            .unwrap();
    assert!(!pinned);
    assert_eq!(key, h.host_key());
    // Connecting before trusting is refused with the fingerprint to show.
    let auth = SshAuth::Password(Secret::new("correct horse"));
    let err = SshSession::connect(
        &h.target(),
        &auth,
        known.clone(),
        &ConnectOptions::default(),
    )
    .await
    .err()
    .unwrap();
    assert_eq!(err.code(), "host_key_unknown");
    assert_eq!(
        err.to_payload().fingerprint.as_deref(),
        Some(key.fingerprint.as_str())
    );
    assert!(h.machine.commands().is_empty(), "nothing ran before trust");
    // Trust, then password auth works.
    known.trust(&h.target().host_id(), &key).unwrap();
    let s = SshSession::connect(
        &h.target(),
        &auth,
        known.clone(),
        &ConnectOptions::default(),
    )
    .await
    .unwrap();
    let out = s.exec("echo hi", None).await.unwrap();
    assert!(out.success());
    s.close().await;
    // Wrong password.
    let bad = SshAuth::Password(Secret::new("nope"));
    let err = SshSession::connect(&h.target(), &bad, known.clone(), &ConnectOptions::default())
        .await
        .err()
        .unwrap();
    assert_eq!(err.code(), "auth_failed");
    // A different key pinned for this host: hard mismatch, before any auth.
    let other = Arc::new(KnownHosts::in_memory());
    other
        .trust(
            &h.target().host_id(),
            &HostKey {
                algorithm: "ssh-ed25519".into(),
                fingerprint: "SHA256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into(),
            },
        )
        .unwrap();
    let err = SshSession::connect(&h.target(), &auth, other, &ConnectOptions::default())
        .await
        .err()
        .unwrap();
    assert_eq!(err.code(), "host_key_mismatch");
}

#[tokio::test]
async fn password_is_used_once_to_install_a_generated_key() {
    let h = harness().await;
    let known = Arc::new(KnownHosts::in_memory());
    known.trust(&h.target().host_id(), &h.host_key()).unwrap();
    let secrets = MemoryStore::default();
    let (profile, public) = rusticmc_hosting::remote::bootstrap_key(
        "my-vps",
        "My VPS",
        &h.target(),
        &Secret::new("correct horse"),
        known.clone(),
        &secrets,
        &ConnectOptions::default(),
    )
    .await
    .unwrap();
    assert!(public.starts_with("ssh-ed25519 "));
    assert_eq!(profile.auth, AuthMethod::KeychainKey);
    // Only the private key is kept; the profile carries no secret.
    assert!(
        secrets
            .get(&names::private_key("my-vps"))
            .unwrap()
            .is_some()
    );
    assert!(!serde_json::to_string(&profile).unwrap().contains("horse"));
    let cmd = h.machine.position("authorized_keys").unwrap();
    assert!(h.machine.commands()[cmd].contains("grep -qxF"));
    // The provider now logs in with the key alone.
    let p = RemoteSshProvider::new(profile, Arc::new(secrets), known).unwrap();
    p.test_connection().await.unwrap();
    // A wrong password stores nothing.
    let s2 = MemoryStore::default();
    let err = rusticmc_hosting::remote::bootstrap_key(
        "other",
        "x",
        &h.target(),
        &Secret::new("wrong"),
        Arc::new(KnownHosts::in_memory()),
        &s2,
        &ConnectOptions::default(),
    )
    .await
    .unwrap_err();
    assert_eq!(err.code(), "host_key_unknown");
    assert!(s2.get(&names::private_key("other")).unwrap().is_none());
}

#[tokio::test]
async fn full_install_runs_every_step_and_uploads_land() {
    let h = harness().await;
    let local = tempfile::tempdir().unwrap();
    let (engine, sha) = engine_file(local.path());
    *h.machine.engine_sha.lock().unwrap() = sha.clone();
    let resources = prepared_dir(local.path());
    let (p, _) = h.provider();
    let (events, sink) = collect();
    let server = p
        .install_with(
            &spec(
                EngineSource::RusticUpload {
                    path: engine.clone(),
                    sha256: sha.clone(),
                },
                Some(resources),
            ),
            options(),
            &sink,
        )
        .await
        .unwrap();
    assert_eq!(server.name, "survival");
    assert_eq!(server.target, "test-vps");

    let cmds = h.machine.commands();
    let order = [
        "--- os-release",
        "useradd --system",
        "mkdir",
        "curl -fsSL --proto =https -o /var/lib/rustic/staging/install.sh https://raw.githubusercontent.com/horizon-rust/horizon/main/packaging/server/install.sh",
        "/var/lib/rustic/staging/install.sh --dir /var/lib/rustic/horizon --bin /var/lib/rustic/bin",
        "install -D -o rustic -g rustic -m 0755",
        " init --eula --world world --port 25565 --whitelist --player Alex --op Alex --yes",
        "config set",
        "tar -xf",
        "import-resources",
        "/etc/systemd/system/rustic-survival.service",
        "ufw allow 25565/tcp",
        "systemctl start",
    ];
    let mut last = 0;
    for needle in order {
        let at = cmds
            .iter()
            .skip(last)
            .position(|c| c.contains(needle))
            .unwrap_or_else(|| {
                panic!("`{needle}` not run after step {last}:\n{}", cmds.join("\n"))
            });
        last += at;
    }
    // Everything after the probe runs through sudo, as the dedicated account or as root.
    assert!(
        cmds.iter()
            .any(|c| c.starts_with("sudo -n -H -u rustic sh -c"))
    );
    // The unit runs as the dedicated account and points at our install.
    let unit = cmds
        .iter()
        .find(|c| c.contains("rustic-survival.service") && c.contains("printf %s"))
        .unwrap();
    assert!(unit.contains("User=rustic"));
    assert!(unit.contains("/var/lib/rustic/bin/horizon-server"));
    assert!(unit.contains("ProtectSystem=strict"));
    // engine.path points at the content-addressed engine, and motd is set.
    let cfg = cmds.iter().find(|c| c.contains("config set")).unwrap();
    assert!(cfg.contains(&format!(
        "engine.path=/var/lib/rustic/engines/{}/rustic-server",
        &sha[..16]
    )));
    assert!(cfg.contains("server.motd=Rustic test") || cfg.contains("'server.motd=Rustic test'"));
    assert!(
        cfg.contains("panel.bind="),
        "the web panel is turned off: {cfg}"
    );

    // SFTP uploads landed in the login user's upload folder.
    let up = h.remote("/home/tester/.cache/rustic-hosting/survival");
    assert_eq!(
        std::fs::read(up.join("rustic-server")).unwrap(),
        std::fs::read(&engine).unwrap()
    );
    let archive = std::fs::File::open(up.join("game-files.tar")).unwrap();
    let mut tar = tar::Archive::new(archive);
    let names: Vec<String> = tar
        .entries()
        .unwrap()
        .map(|e| {
            e.unwrap()
                .path()
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    assert!(
        names.contains(&"prepared/horizon-resources.json".to_owned()),
        "{names:?}"
    );
    assert!(names.contains(&"prepared/assets/minecraft/textures/block/stone.png".to_owned()));

    // Progress: every step reported, ending with the health check.
    let ev = events.lock().unwrap();
    for step in [
        "connect",
        "requirements",
        "service-account",
        "folders",
        "install-horizon",
        "install-engine",
        "init-server",
        "configure",
        "import-resources",
        "service",
        "firewall",
        "start",
        "health-check",
    ] {
        assert!(
            ev.iter()
                .any(|e| e.step == step && matches!(e.state, StepState::Done)),
            "{step} not done: {:?}",
            ev.iter().map(|e| (&e.step, e.state)).collect::<Vec<_>>()
        );
    }
}

#[tokio::test]
async fn rerun_on_an_installed_server_skips_done_steps() {
    let h = harness().await;
    h.machine.everything_done.store(true, Ordering::SeqCst);
    h.machine.started.store(true, Ordering::SeqCst);
    let local = tempfile::tempdir().unwrap();
    let (engine, sha) = engine_file(local.path());
    *h.machine.engine_sha.lock().unwrap() = sha.clone();
    let (p, _) = h.provider();
    let (events, sink) = collect();
    p.install_with(
        &spec(
            EngineSource::RusticUpload {
                path: engine,
                sha256: sha,
            },
            None,
        ),
        options(),
        &sink,
    )
    .await
    .unwrap();
    let cmds = h.machine.commands();
    for never in [
        "useradd --system",
        "install.sh --dir",
        " init ",
        "ufw allow",
        "systemctl start",
        "rustic-survival.service.rustic-tmp",
    ] {
        assert!(
            !cmds.iter().any(|c| c.contains(never)),
            "`{never}` ran again"
        );
    }
    let ev = events.lock().unwrap();
    for step in [
        "service-account",
        "folders",
        "install-horizon",
        "install-engine",
        "init-server",
        "service",
        "firewall",
        "start",
    ] {
        assert!(
            ev.iter()
                .any(|e| e.step == step && e.state == StepState::Skipped),
            "{step} not skipped"
        );
    }
}

#[tokio::test]
async fn a_failing_step_rolls_back_what_this_run_did() {
    let h = harness().await;
    *h.machine.fail.lock().unwrap() = Some("systemctl start".into());
    let local = tempfile::tempdir().unwrap();
    let resources = prepared_dir(local.path());
    let (p, _) = h.provider();
    let (events, sink) = collect();
    let err = p
        .install_with(
            &spec(EngineSource::Horizon, Some(resources)),
            options(),
            &sink,
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "step_failed");
    assert!(
        err.to_string()
            .contains("simulated failure of systemctl start"),
        "{err}"
    );
    // Undo in reverse order: firewall rule, unit, game files, server folder, Horizon, account.
    let undo = [
        "ufw delete allow 25565/tcp",
        "disable --now rustic-survival.service",
        "rm -rf /var/lib/rustic/servers/survival/resources",
        "rm -rf /var/lib/rustic/servers/survival",
        "rm -rf /var/lib/rustic/horizon",
        "userdel rustic",
    ];
    let start = h.machine.position("systemctl start").unwrap();
    let mut last = start;
    let cmds = h.machine.commands();
    for needle in undo {
        let at = cmds
            .iter()
            .skip(last)
            .position(|c| c.contains(needle))
            .unwrap_or_else(|| panic!("`{needle}` not undone:\n{}", cmds[start..].join("\n")));
        last += at;
    }
    let ev = events.lock().unwrap();
    assert!(
        ev.iter()
            .any(|e| e.step == "start" && e.state == StepState::Failed)
    );
    assert!(
        ev.iter()
            .any(|e| e.step == "service-account" && e.state == StepState::RolledBack)
    );
}

#[tokio::test]
async fn refuses_without_eula_before_connecting() {
    let h = harness().await;
    let (p, _) = h.provider();
    let mut s = spec(EngineSource::Horizon, None);
    s.eula_accepted = false;
    let err = p.install_with(&s, options(), &|_| {}).await.unwrap_err();
    assert_eq!(err.code(), "precondition");
    assert!(h.machine.commands().is_empty());
}

#[tokio::test]
async fn management_calls() {
    let h = harness().await;
    h.machine.installed.store(true, Ordering::SeqCst);
    h.machine.started.store(true, Ordering::SeqCst);
    h.machine.unit_enabled.store(true, Ordering::SeqCst);
    let (p, _) = h.provider();

    let st = p.status("survival").await.unwrap();
    assert!(st.is_running());
    assert_eq!(st.metrics.tps, Some(20.0));
    assert_eq!(st.players_online, vec!["Alex"]);

    let out = p.send_command("survival", "/list").await.unwrap();
    assert_eq!(out.output.len(), 1);
    let cmd = h
        .machine
        .commands()
        .into_iter()
        .find(|c| c.contains(" cmd "))
        .unwrap();
    assert!(cmd.contains("cmd list"), "{cmd}");
    assert!(p.send_command("survival", "say a\nop me").await.is_err());

    let players = p.players("survival").await.unwrap();
    assert_eq!(players.online[0].ping_ms, Some(30));

    p.player_action(
        "survival",
        &PlayerAction::Ban {
            name: "Griefer".into(),
            reason: Some("x; rm -rf /".into()),
        },
    )
    .await
    .unwrap();
    let ban = h
        .machine
        .commands()
        .into_iter()
        .find(|c| c.contains("Griefer"))
        .unwrap();
    // The reason is a single quoted word inside the sudo'd script, never shell syntax.
    assert!(ban.contains("ban Griefer '\\''x; rm -rf /'\\''"), "{ban}");

    // Plugins: list classifies, upload validates and lands, toggles rename.
    let plugins = p.plugins_list("survival").await.unwrap();
    assert_eq!(plugins.len(), 2);
    assert!(plugins.iter().any(|x| x.id == "old" && !x.enabled));
    let local = tempfile::tempdir().unwrap();
    let rp = local.path().join("Homes-1.2.rplugin");
    {
        let f = std::fs::File::create(&rp).unwrap();
        let mut z = zip::ZipWriter::new(f);
        z.start_file(
            "rustic.plugin.toml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        z.write_all(
            b"[plugin]\nid = \"homes\"\nname = \"Homes\"\nversion = \"1.2.0\"\napi = \"0.1\"\n",
        )
        .unwrap();
        z.finish().unwrap();
    }
    let info = p.plugin_upload("survival", &rp).await.unwrap();
    assert_eq!(info.id, "homes");
    assert!(
        h.remote("/home/tester/.cache/rustic-hosting/survival/Homes-1.2.rplugin")
            .is_file()
    );
    assert!(
        h.machine
            .commands()
            .iter()
            .any(|c| c.contains("/var/lib/rustic/servers/survival/plugins/homes.rplugin"))
    );
    p.plugin_set_enabled("survival", "homes", false)
        .await
        .unwrap();
    assert!(
        h.machine
            .commands()
            .iter()
            .any(|c| c.contains("homes.rplugin.disabled"))
    );
    assert!(
        p.plugin_set_enabled("survival", "../x", false)
            .await
            .is_err()
    );

    // Config.
    let out = p
        .config_write("survival", &[("world.difficulty".into(), "hard".into())])
        .await
        .unwrap();
    assert_eq!(out.saved, vec!["engine.path"]);

    // Backups.
    let made = p
        .backup_create("survival", Some("before-update"))
        .await
        .unwrap();
    assert_eq!(made, "2026-11-10T12-00-00Z-manual.zip");
    let list = p.backups_list("survival").await.unwrap();
    assert_eq!(list.len(), 1);

    // Restart while running goes through the supervisor (the unit stays up).
    p.restart("survival").await.unwrap();
    assert!(
        h.machine
            .commands()
            .iter()
            .any(|c| c.contains("survival restart"))
    );

    // Unknown server.
    h.machine.installed.store(false, Ordering::SeqCst);
    assert_eq!(
        p.status("other").await.unwrap_err().code(),
        "unknown_server"
    );
}

/// A fake control API: SSE for events, a zip for downloads; checks Host and token like Horizon.
async fn fake_api() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = l.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let n = s.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).into_owned();
                let host_ok = req.contains(&format!("Host: 127.0.0.1:{port}\r\n"));
                let token_ok = req.contains("Authorization: Bearer tok\r\n");
                if !host_ok || !token_ok {
                    let _ = s.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 20\r\n\r\n{\"error\":\"wrong Ho\"}").await;
                    return;
                }
                if req.starts_with("GET /api/v1/events") {
                    let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n").await;
                    let _ = s.write_all(b"id: 7\nevent: log\ndata: {\"id\":7,\"type\":\"log\",\"line\":\"[Server thread/INFO]: Done (2.1s)!\"}\n\n").await;
                    let _ = s.write_all(b"id: 8\nevent: join\ndata: {\"id\":8,\"type\":\"join\",\"name\":\"Alex\"}\n\n").await;
                    let _ = s.shutdown().await;
                } else if req.starts_with(
                    "GET /api/v1/backups/download?name=2026-11-10T12-00-00Z-manual.zip",
                ) {
                    let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Length: 5\r\n\r\nPK\x05\x06!").await;
                } else {
                    let _ = s
                        .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 2\r\n\r\n{}")
                        .await;
                }
            });
        }
    });
    port
}

#[tokio::test]
async fn console_and_downloads_go_through_the_ssh_forward() {
    let h = harness().await;
    h.machine.installed.store(true, Ordering::SeqCst);
    h.machine.started.store(true, Ordering::SeqCst);
    let (p, _) = h.provider();
    // Not running: no api.json, no console.
    assert_eq!(
        p.console_subscribe("survival").await.err().unwrap().code(),
        "precondition"
    );
    let api = fake_api().await;
    h.machine.api_port.store(api, Ordering::SeqCst);
    let mut sub = p.console_subscribe("survival").await.unwrap();
    let first = sub.events.recv().await.unwrap();
    assert_eq!(
        first,
        ConsoleEvent::Log {
            line: "[Server thread/INFO]: Done (2.1s)!".into()
        }
    );
    match sub.events.recv().await.unwrap() {
        ConsoleEvent::Game { kind, data } => {
            assert_eq!(kind, "join");
            assert_eq!(data["name"], "Alex");
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        sub.events.recv().await.unwrap(),
        ConsoleEvent::Closed { .. }
    ));

    let dest = tempfile::tempdir().unwrap();
    let file = dest.path().join("backup.zip");
    let n = p
        .backup_download("survival", "2026-11-10T12-00-00Z-manual.zip", &file)
        .await
        .unwrap();
    assert_eq!(n, 5);
    assert_eq!(std::fs::read(&file).unwrap(), b"PK\x05\x06!");
    // Only names the supervisor lists.
    assert!(
        p.backup_download("survival", "../../etc/shadow", &file)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn uninstall_removes_service_firewall_and_files() {
    let h = harness().await;
    h.machine.installed.store(true, Ordering::SeqCst);
    h.machine.started.store(true, Ordering::SeqCst);
    h.machine.unit_enabled.store(true, Ordering::SeqCst);
    let (p, _) = h.provider();
    p.uninstall(
        "survival",
        &UninstallOptions {
            delete_files: true,
            remove_runtime: false,
        },
        &|_| {},
    )
    .await
    .unwrap();
    let cmds = h.machine.commands();
    for needle in [
        "systemctl stop rustic-survival",
        "disable --now rustic-survival.service",
        "ufw delete allow 25565/tcp",
        "rm -rf /var/lib/rustic/servers/survival",
    ] {
        assert!(
            cmds.iter().any(|c| c.contains(needle)),
            "`{needle}` missing:\n{}",
            cmds.join("\n")
        );
    }
    assert!(!cmds.iter().any(|c| c.contains("userdel rustic")));
}
