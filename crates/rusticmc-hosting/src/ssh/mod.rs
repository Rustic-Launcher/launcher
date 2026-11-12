//! SSH and SFTP to the user's VPS, over `russh` + `russh-sftp` (pure Rust, ADR 0120).
//!
//! - [`SshSession::connect`] verifies the host key against the pins **before** authenticating
//!   ([`hostkey`]), then authenticates with a private key (OpenSSH, PEM or PuTTY format, ed25519,
//!   ECDSA or RSA, optionally passphrase-protected) or a password.
//! - [`SshSession::exec`] runs one non-interactive command (no PTY), optionally feeding stdin.
//! - [`SshSession::direct_tcpip`] opens an SSH local forward to an address *on the VPS* (the
//!   supervisor's control API on 127.0.0.1), without listening on any local port.
//! - SFTP uploads land in a staging folder owned by the login user.

pub mod hostkey;
pub mod http;

use crate::error::{HostingError, Result};
use crate::secret::Secret;
use hostkey::{HostKey, KnownHosts, Verdict};
use russh::ChannelMsg;
use russh::client;
use russh::keys::{PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::OpenFlags;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::OnceCell;

/// Where to connect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTarget {
    pub host: String,
    pub port: u16,
    pub user: String,
}

impl SshTarget {
    pub fn host_id(&self) -> String {
        hostkey::host_id(&self.host, self.port)
    }
}

/// How to authenticate. The values come from the keychain right before connecting.
#[derive(Debug, Clone)]
pub enum SshAuth {
    Password(Secret),
    /// A private key's text (OpenSSH `-----BEGIN OPENSSH PRIVATE KEY-----`, PKCS#8/PEM, or PuTTY).
    Key {
        private_key: Secret,
        passphrase: Option<Secret>,
    },
}

/// Connection settings.
#[derive(Debug, Clone)]
pub struct ConnectOptions {
    pub connect_timeout: Duration,
    pub keepalive: Duration,
}

impl Default for ConnectOptions {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(15),
            keepalive: Duration::from_secs(20),
        }
    }
}

/// Output of a finished command.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExecOutput {
    pub status: u32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl ExecOutput {
    pub fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
    pub fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
    pub fn success(&self) -> bool {
        self.status == 0
    }
    /// Turn a non-zero status into [`HostingError::RemoteCommand`] (with the end of stderr, or of
    /// stdout when stderr is empty: `horizon-server` prints its errors on stderr).
    pub fn check(self, command: &str) -> Result<Self> {
        if self.success() {
            return Ok(self);
        }
        let err = self.stderr_text();
        let text = if err.trim().is_empty() {
            self.stdout_text()
        } else {
            err
        };
        Err(HostingError::RemoteCommand {
            command: redact_command(command),
            status: self.status,
            stderr: tail(&text, 1500),
        })
    }
}

/// Commands can be long (a unit file in a printf): keep the error readable.
fn redact_command(command: &str) -> String {
    let first = command.lines().next().unwrap_or_default();
    if first.len() > 200 || command.contains('\n') {
        format!("{}...", &first[..first.floor_char_boundary(200)])
    } else {
        first.to_owned()
    }
}

fn tail(text: &str, max: usize) -> String {
    let t = text.trim();
    if t.len() <= max {
        t.to_owned()
    } else {
        format!("...{}", &t[t.ceil_char_boundary(t.len() - max)..])
    }
}

/// The russh client handler: host key verification only.
struct Handler {
    host_id: String,
    known_hosts: Arc<KnownHosts>,
    seen: Arc<Mutex<Option<(HostKey, Verdict)>>>,
}

impl client::Handler for Handler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        key: &PublicKeyOrCertificate,
    ) -> std::result::Result<bool, Self::Error> {
        let PublicKeyOrCertificate::PublicKey { key, .. } = key else {
            // Host certificates need a CA the user configured: not supported yet (HOSTING.md).
            return Ok(false);
        };
        let presented = HostKey::from_public_key(key);
        let verdict = self.known_hosts.verify(&self.host_id, &presented);
        let ok = verdict == Verdict::Trusted;
        *self.seen.lock().unwrap() = Some((presented, verdict));
        Ok(ok)
    }
}

/// An authenticated connection.
pub struct SshSession {
    handle: client::Handle<Handler>,
    target: SshTarget,
    host_key: HostKey,
    sftp: OnceCell<SftpSession>,
}

/// What the presented key was, when the connection stopped at host key verification.
async fn handshake(
    target: &SshTarget,
    known_hosts: Arc<KnownHosts>,
    options: &ConnectOptions,
) -> Result<(client::Handle<Handler>, HostKey)> {
    let host_id = target.host_id();
    let seen = Arc::new(Mutex::new(None));
    let handler = Handler {
        host_id: host_id.clone(),
        known_hosts,
        seen: seen.clone(),
    };
    let config = Arc::new(client::Config {
        inactivity_timeout: None,
        keepalive_interval: Some(options.keepalive),
        keepalive_max: 3,
        nodelay: true,
        ..Default::default()
    });
    let addr = (target.host.trim_matches(['[', ']']).to_owned(), target.port);
    let connecting = client::connect(config, addr, handler);
    let result = tokio::time::timeout(options.connect_timeout, connecting)
        .await
        .map_err(|_| HostingError::Connect {
            host: host_id.clone(),
            detail: format!(
                "no answer within {} s (is the address right and port {} open?)",
                options.connect_timeout.as_secs(),
                target.port
            ),
        })?;
    let presented = seen.lock().unwrap().clone();
    match (result, presented) {
        (Ok(handle), Some((key, Verdict::Trusted))) => Ok((handle, key)),
        (_, Some((_, verdict))) if verdict != Verdict::Trusted => {
            verdict.into_result(&host_id).map(|_| unreachable!())
        }
        (Err(e), _) => Err(HostingError::Connect {
            host: host_id,
            detail: e.to_string(),
        }),
        (Ok(_), _) => Err(HostingError::Ssh(
            "the server presented no host key we could check".into(),
        )),
    }
}

impl SshSession {
    /// Learn the host key without authenticating (for the "trust this host?" dialog). Never sends
    /// credentials. Returns the key and whether it is already pinned.
    pub async fn probe_host_key(
        target: &SshTarget,
        known_hosts: Arc<KnownHosts>,
        options: &ConnectOptions,
    ) -> Result<(HostKey, bool)> {
        match handshake(target, known_hosts, options).await {
            Ok((handle, key)) => {
                let _ = handle
                    .disconnect(russh::Disconnect::ByApplication, "", "en")
                    .await;
                Ok((key, true))
            }
            Err(HostingError::HostKeyUnknown {
                algorithm,
                fingerprint,
                ..
            }) => Ok((
                HostKey {
                    algorithm,
                    fingerprint,
                },
                false,
            )),
            Err(e) => Err(e),
        }
    }

    /// Connect, verify the host key against the pins, authenticate.
    pub async fn connect(
        target: &SshTarget,
        auth: &SshAuth,
        known_hosts: Arc<KnownHosts>,
        options: &ConnectOptions,
    ) -> Result<Self> {
        let (mut handle, host_key) = handshake(target, known_hosts, options).await?;
        let auth_failed = |detail: String| HostingError::AuthFailed {
            user: target.user.clone(),
            host: target.host_id(),
            detail,
        };
        let ok = match auth {
            SshAuth::Password(password) => handle
                .authenticate_password(&target.user, password.expose())
                .await
                .map_err(|e| auth_failed(e.to_string()))?
                .success(),
            SshAuth::Key {
                private_key,
                passphrase,
            } => {
                let key = russh::keys::decode_secret_key(
                    private_key.expose(),
                    passphrase.as_ref().map(Secret::expose),
                )
                .map_err(|e| {
                    auth_failed(format!(
                        "the private key can't be read ({e}){}",
                        if passphrase.is_none() {
                            "; if it is protected, give its passphrase"
                        } else {
                            ""
                        }
                    ))
                })?;
                let hash = handle
                    .best_supported_rsa_hash()
                    .await
                    .ok()
                    .flatten()
                    .flatten();
                handle
                    .authenticate_publickey(
                        &target.user,
                        PrivateKeyWithHashAlg::new(Arc::new(key), hash),
                    )
                    .await
                    .map_err(|e| auth_failed(e.to_string()))?
                    .success()
            }
        };
        if !ok {
            return Err(auth_failed("the server refused the credentials".to_owned()));
        }
        Ok(Self {
            handle,
            target: target.clone(),
            host_key,
            sftp: OnceCell::new(),
        })
    }

    pub fn target(&self) -> &SshTarget {
        &self.target
    }

    pub fn host_key(&self) -> &HostKey {
        &self.host_key
    }

    /// Run `command` (a POSIX shell command line built with [`crate::shell`]), feeding `stdin`.
    pub async fn exec(&self, command: &str, stdin: Option<&[u8]>) -> Result<ExecOutput> {
        let mut out = ExecOutput::default();
        let mut stdout = Vec::new();
        let status = self
            .exec_streaming(command, stdin, &mut stdout, Some(&mut out.stderr))
            .await?;
        out.stdout = stdout;
        out.status = status;
        Ok(out)
    }

    /// `exec`, then fail on a non-zero status.
    pub async fn run(&self, command: &str) -> Result<String> {
        Ok(self
            .exec(command, None)
            .await?
            .check(command)?
            .stdout_text())
    }

    /// Run a command and copy its stdout into `sink` as it comes (downloads with `cat`).
    pub async fn exec_streaming<W: AsyncWrite + Unpin + Send>(
        &self,
        command: &str,
        stdin: Option<&[u8]>,
        sink: &mut W,
        mut stderr: Option<&mut Vec<u8>>,
    ) -> Result<u32> {
        let mut channel = self.handle.channel_open_session().await?;
        channel.exec(true, command).await?;
        if let Some(input) = stdin {
            channel.data(input).await?;
        }
        channel.eof().await?;
        let mut status = None;
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { data } => sink.write_all(&data).await?,
                ChannelMsg::ExtendedData { data, ext: 1 } => {
                    if let Some(buf) = stderr.as_deref_mut()
                        && buf.len() < 1 << 20
                    {
                        buf.extend_from_slice(&data);
                    }
                }
                ChannelMsg::ExitStatus { exit_status } => status = Some(exit_status),
                ChannelMsg::ExitSignal { signal_name, .. } => {
                    tracing::warn!(?signal_name, "remote command killed by a signal");
                    status = Some(128);
                }
                _ => {}
            }
        }
        sink.flush().await?;
        Ok(status.unwrap_or(255))
    }

    /// Run a long-lived command and send its stdout line by line (console fallback, `logs -f`).
    pub async fn exec_lines(&self, command: &str) -> Result<tokio::sync::mpsc::Receiver<String>> {
        let mut channel = self.handle.channel_open_session().await?;
        channel.exec(true, command).await?;
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        tokio::spawn(async move {
            let mut pending = Vec::new();
            while let Some(msg) = channel.wait().await {
                if let ChannelMsg::Data { data } = msg {
                    pending.extend_from_slice(&data);
                    while let Some(i) = pending.iter().position(|b| *b == b'\n') {
                        let line: Vec<u8> = pending.drain(..=i).collect();
                        let text = String::from_utf8_lossy(&line).trim_end().to_owned();
                        if tx.send(text).await.is_err() {
                            let _ = channel.close().await;
                            return;
                        }
                    }
                }
            }
        });
        Ok(rx)
    }

    /// An SSH local forward to `host:port` as seen from the VPS (RFC 4254 `direct-tcpip`).
    pub async fn direct_tcpip(
        &self,
        host: &str,
        port: u16,
    ) -> Result<russh::ChannelStream<client::Msg>> {
        let channel = self
            .handle
            .channel_open_direct_tcpip(host, u32::from(port), "127.0.0.1", 0)
            .await?;
        Ok(channel.into_stream())
    }

    async fn sftp(&self) -> Result<&SftpSession> {
        self.sftp
            .get_or_try_init(|| async {
                let channel = self.handle.channel_open_session().await?;
                channel.request_subsystem(true, "sftp").await?;
                let sftp = SftpSession::new(channel.into_stream()).await?;
                Ok::<_, HostingError>(sftp)
            })
            .await
    }

    /// `mkdir -p` over SFTP.
    pub async fn sftp_mkdir_p(&self, dir: &str) -> Result<()> {
        let sftp = self.sftp().await?;
        let mut path = String::new();
        for part in dir.split('/').filter(|p| !p.is_empty()) {
            path.push('/');
            path.push_str(part);
            if !sftp.try_exists(path.clone()).await.unwrap_or(false)
                && let Err(e) = sftp.create_dir(path.clone()).await
                && !sftp.try_exists(path.clone()).await.unwrap_or(false)
            {
                return Err(HostingError::Sftp(format!("mkdir {path}: {e}")));
            }
        }
        Ok(())
    }

    /// Write bytes to a remote file (created or truncated).
    pub async fn sftp_write(&self, remote: &str, bytes: &[u8]) -> Result<()> {
        let sftp = self.sftp().await?;
        let mut file = sftp
            .open_with_flags(
                remote,
                OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
            )
            .await
            .map_err(|e| HostingError::Sftp(format!("{remote}: {e}")))?;
        file.write_all(bytes).await?;
        file.shutdown().await?;
        Ok(())
    }

    /// Upload a local file; `on_bytes(done, total)` after each chunk.
    pub async fn sftp_upload(
        &self,
        local: &Path,
        remote: &str,
        on_bytes: &(dyn Fn(u64, u64) + Send + Sync),
    ) -> Result<u64> {
        let sftp = self.sftp().await?;
        let mut src = tokio::fs::File::open(local).await.map_err(|e| {
            HostingError::Io(std::io::Error::new(
                e.kind(),
                format!("{}: {e}", local.display()),
            ))
        })?;
        let total = src.metadata().await?.len();
        if let Some((parent, _)) = remote.rsplit_once('/')
            && !parent.is_empty()
        {
            self.sftp_mkdir_p(parent).await?;
        }
        let mut dst = sftp
            .open_with_flags(
                remote,
                OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
            )
            .await
            .map_err(|e| HostingError::Sftp(format!("{remote}: {e}")))?;
        let mut buf = vec![0u8; 64 * 1024];
        let mut done = 0u64;
        loop {
            let n = src.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            dst.write_all(&buf[..n]).await?;
            done += n as u64;
            on_bytes(done, total);
        }
        dst.shutdown().await?;
        Ok(done)
    }

    pub async fn sftp_remove_file(&self, remote: &str) -> Result<()> {
        self.sftp().await?.remove_file(remote).await?;
        Ok(())
    }

    pub async fn sftp_exists(&self, remote: &str) -> Result<bool> {
        Ok(self.sftp().await?.try_exists(remote).await?)
    }

    pub async fn sftp_read(&self, remote: &str) -> Result<Vec<u8>> {
        Ok(self.sftp().await?.read(remote).await?)
    }

    pub async fn close(&self) {
        if let Some(s) = self.sftp.get() {
            let _ = s.close().await;
        }
        let _ = self
            .handle
            .disconnect(russh::Disconnect::ByApplication, "", "en")
            .await;
    }
}

/// A launcher-generated Ed25519 key pair (SECURITY §2.8): the private key goes to the keychain, the
/// public line to the VPS's `~/.ssh/authorized_keys`.
pub struct GeneratedKey {
    pub private_openssh: Secret,
    /// `ssh-ed25519 AAAA... rustic-launcher`, shown to the user and appended on the VPS.
    pub public_line: String,
    pub fingerprint: String,
}

pub fn generate_key(comment: &str) -> Result<GeneratedKey> {
    let key = russh::keys::PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519)
        .map_err(|e| HostingError::Ssh(e.to_string()))?;
    let private = key
        .to_openssh(russh::keys::ssh_key::LineEnding::LF)
        .map_err(|e| HostingError::Ssh(e.to_string()))?;
    let public = key
        .public_key()
        .to_openssh()
        .map_err(|e| HostingError::Ssh(e.to_string()))?;
    let comment: String = comment
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '@'))
        .collect();
    Ok(GeneratedKey {
        private_openssh: Secret::new(private.to_string()),
        public_line: format!("{public} {comment}").trim().to_owned(),
        fingerprint: key
            .public_key()
            .fingerprint(russh::keys::HashAlg::Sha256)
            .to_string(),
    })
}

/// Shell that appends `public_line` to the login user's `authorized_keys` once (idempotent),
/// with the permissions sshd requires.
pub fn authorize_key_script(public_line: &str) -> Result<String> {
    if !public_line.starts_with("ssh-") || public_line.contains(['\n', '\r']) {
        return Err(HostingError::Invalid(
            "not an OpenSSH public key line".into(),
        ));
    }
    let q = crate::shell::quote(public_line)?;
    Ok(format!(
        "umask 077 && mkdir -p \"$HOME/.ssh\" && touch \"$HOME/.ssh/authorized_keys\" && (grep -qxF {q} \"$HOME/.ssh/authorized_keys\" || printf '%s\\n' {q} >> \"$HOME/.ssh/authorized_keys\") && chmod 700 \"$HOME/.ssh\" && chmod 600 \"$HOME/.ssh/authorized_keys\""
    ))
}

/// Every regular file under `root`, as (relative path, length), sorted.
pub fn collect_files(root: &Path) -> Result<Vec<(std::path::PathBuf, u64)>> {
    fn walk(
        root: &Path,
        dir: &Path,
        out: &mut Vec<(std::path::PathBuf, u64)>,
    ) -> std::io::Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<std::io::Result<_>>()?;
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let path = e.path();
            let meta = std::fs::metadata(&path)?;
            if meta.is_dir() {
                walk(root, &path, out)?;
            } else if meta.is_file() {
                out.push((
                    path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
                    meta.len(),
                ));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_reports_stderr_tail_or_stdout() {
        let e = ExecOutput {
            status: 2,
            stdout: b"out".to_vec(),
            stderr: b"horizon-server: no prepared game files".to_vec(),
        }
        .check("horizon-server start")
        .unwrap_err();
        match e {
            HostingError::RemoteCommand {
                status,
                stderr,
                command,
            } => {
                assert_eq!(status, 2);
                assert_eq!(command, "horizon-server start");
                assert!(stderr.contains("no prepared game files"));
            }
            other => panic!("{other:?}"),
        }
        let e = ExecOutput {
            status: 1,
            stdout: b"only stdout".to_vec(),
            stderr: Vec::new(),
        }
        .check("x")
        .unwrap_err();
        assert!(e.to_string().contains("only stdout"));
    }

    #[test]
    fn long_commands_are_shortened_in_errors() {
        let cmd = format!("printf %s '{}'\n more", "x".repeat(500));
        let short = redact_command(&cmd);
        assert!(short.len() < 220);
        assert!(short.ends_with("..."));
        assert_eq!(
            tail(&"y".repeat(3000), 10),
            format!("...{}", "y".repeat(10))
        );
    }

    #[test]
    fn generated_keys_round_trip() {
        let k = generate_key("rustic-launcher@pc; rm -rf /").unwrap();
        assert!(k.public_line.starts_with("ssh-ed25519 AAAA"));
        assert!(k.public_line.ends_with(" rustic-launcher@pcrm-rf"));
        assert!(k.fingerprint.starts_with("SHA256:"));
        let back = russh::keys::decode_secret_key(k.private_openssh.expose(), None).unwrap();
        assert_eq!(
            back.public_key()
                .fingerprint(russh::keys::HashAlg::Sha256)
                .to_string(),
            k.fingerprint
        );
        let s = authorize_key_script(&k.public_line).unwrap();
        assert!(s.contains("grep -qxF '"));
        assert!(authorize_key_script("evil\nssh-ed25519 x").is_err());
        assert!(authorize_key_script("command=\"x\" ssh-ed25519 x").is_err());
    }

    #[test]
    fn collects_files_recursively() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
        std::fs::write(dir.path().join("a/b/c.json"), b"12").unwrap();
        std::fs::write(dir.path().join("top"), b"1").unwrap();
        let files = collect_files(dir.path()).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files.iter().map(|f| f.1).sum::<u64>(), 3);
    }
}
