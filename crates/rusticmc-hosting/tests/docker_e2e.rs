//! End to end against a real Linux box: an Ubuntu 24.04 container running OpenSSH, with a `tester`
//! user that has passwordless sudo (no systemd: the server runs in the background).
//! The real flow runs: host key TOFU, probe, dedicated account, Horizon's real installer from
//! GitHub (signature/SHA-256 checked by it), `init`, `config set`, and, when
//! `RUSTIC_E2E_RESOURCES` names a prepared `26.3` folder from your own game, the upload, import,
//! start, status ping from this computer, console, then uninstall.
//!
//! Opt in (needs Docker and network; downloads ~100 MB):
//!   RUSTIC_SSH_E2E=1 [RUSTIC_E2E_RESOURCES=path/to/26.3] cargo test -p rusticmc-hosting --test docker_e2e -- --ignored --nocapture

use rusticmc_hosting::provider::HostingProvider;
use rusticmc_hosting::secret::names;
use rusticmc_hosting::ssh::{ConnectOptions, SshSession};
use rusticmc_hosting::*;
use std::collections::BTreeMap;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

const IMAGE: &str = "rustic-hosting-e2e:ubuntu24";

fn docker(args: &[&str]) -> String {
    let out = Command::new("docker").args(args).output().expect("docker");
    assert!(
        out.status.success(),
        "docker {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

struct Container(String);
impl Drop for Container {
    fn drop(&mut self) {
        if std::env::var("RUSTIC_E2E_KEEP").is_err() {
            let _ = Command::new("docker").args(["rm", "-f", &self.0]).output();
        }
    }
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[tokio::test]
#[ignore = "needs Docker and network: RUSTIC_SSH_E2E=1"]
async fn real_install_on_a_container() {
    if std::env::var("RUSTIC_SSH_E2E").as_deref() != Ok("1") {
        eprintln!("skipped: set RUSTIC_SSH_E2E=1");
        return;
    }
    // 1. Image: sshd + sudo + curl + openssl, user `tester` with our key.
    let key =
        russh::keys::PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519).unwrap();
    let ctx = tempfile::tempdir().unwrap();
    std::fs::write(
        ctx.path().join("authorized_keys"),
        key.public_key().to_openssh().unwrap(),
    )
    .unwrap();
    std::fs::write(
        ctx.path().join("Dockerfile"),
        "FROM ubuntu:24.04
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \\
      openssh-server sudo curl ca-certificates openssl tar coreutils iproute2 \\
    && rm -rf /var/lib/apt/lists/* && mkdir -p /run/sshd \\
    && useradd -m -s /bin/bash tester && echo 'tester ALL=(ALL) NOPASSWD:ALL' > /etc/sudoers.d/tester \\
    && mkdir -p /home/tester/.ssh
COPY authorized_keys /home/tester/.ssh/authorized_keys
RUN chown -R tester:tester /home/tester/.ssh && chmod 700 /home/tester/.ssh && chmod 600 /home/tester/.ssh/authorized_keys
CMD [\"/usr/sbin/sshd\", \"-D\", \"-e\"]
",
    )
    .unwrap();
    docker(&[
        "build",
        "-q",
        "-t",
        IMAGE,
        &ctx.path().display().to_string(),
    ]);
    let ssh_port = free_port();
    let game_port = free_port();
    let id = docker(&[
        "run",
        "-d",
        "-p",
        &format!("127.0.0.1:{ssh_port}:22"),
        "-p",
        &format!("127.0.0.1:{game_port}:{game_port}"),
        IMAGE,
    ]);
    let _container = Container(id);
    let target = SshTarget {
        host: "127.0.0.1".into(),
        port: ssh_port,
        user: "tester".into(),
    };

    // 2. TOFU: probe the key, "confirm" it, pin it.
    let known = Arc::new(KnownHosts::in_memory());
    let mut probed = None;
    for _ in 0..30 {
        match SshSession::probe_host_key(&target, known.clone(), &ConnectOptions::default()).await {
            Ok(k) => {
                probed = Some(k);
                break;
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(500)).await,
        }
    }
    let (host_key, pinned) = probed.expect("sshd came up");
    assert!(!pinned);
    eprintln!("host key {} {}", host_key.algorithm, host_key.fingerprint);
    known.trust(&target.host_id(), &host_key).unwrap();

    let secrets = Arc::new(MemoryStore::default());
    secrets
        .set(
            &names::private_key("e2e"),
            &Secret::new(
                key.to_openssh(russh::keys::ssh_key::LineEnding::LF)
                    .unwrap()
                    .to_string(),
            ),
        )
        .unwrap();
    let p = RemoteSshProvider::new(
        RemoteProfile {
            id: "e2e".into(),
            label: "Docker".into(),
            target: target.clone(),
            auth: AuthMethod::KeychainKey,
        },
        secrets,
        known,
    )
    .unwrap();
    let info = p.test_connection().await.unwrap();
    eprintln!(
        "VPS: {} {} as {} ({:?}), systemd={}",
        info.os.pretty_name,
        info.arch.as_str(),
        info.user,
        info.elevation,
        info.systemd
    );

    // 3. Install for real.
    let resources = std::env::var_os("RUSTIC_E2E_RESOURCES").map(std::path::PathBuf::from);
    let spec = ServerSpec {
        name: "e2e".into(),
        port: game_port,
        world: "world".into(),
        seed: Some("12345".into()),
        whitelist: false,
        players: vec![],
        op: None,
        online_mode: false,
        eula_accepted: true,
        engine: EngineSource::Horizon,
        prepared_resources: resources.clone(),
        settings: BTreeMap::from([("server.motd".to_owned(), "Rustic e2e".to_owned())]),
    };
    let sink = |e: ProgressEvent| {
        eprintln!(
            "[{}] {:?} {}{}",
            e.step,
            e.state,
            e.title,
            e.detail.map(|d| format!(": {d}")).unwrap_or_default()
        )
    };
    let options = RemoteInstallOptions {
        open_firewall: true,
        health_timeout: Duration::from_secs(240),
        ..Default::default()
    };
    let started = std::time::Instant::now();
    p.install_with(&spec, options.clone(), &sink).await.unwrap();
    eprintln!("installed in {:?}", started.elapsed());

    // 4. Idempotent re-run: nothing re-installed.
    p.install_with(&spec, options, &sink).await.unwrap();

    let status = p.status("e2e").await.unwrap();
    eprintln!("status: {status:?}");
    let cfg = p.config_read("e2e").await.unwrap();
    assert_eq!(cfg["server"]["motd"], "Rustic e2e");
    if resources.is_some() {
        assert!(status.is_running(), "{status:?}");
        let ping = rusticmc_hosting::slp::ping("127.0.0.1", game_port, Duration::from_secs(10))
            .await
            .unwrap();
        eprintln!("ping: {ping:?}");
        let mut console = p.console_subscribe("e2e").await.unwrap();
        let out = p.send_command("e2e", "list").await.unwrap();
        eprintln!("list: {:?}", out.output);
        let ev = tokio::time::timeout(Duration::from_secs(10), console.events.recv()).await;
        eprintln!("console event: {ev:?}");
        let backup = p.backup_create("e2e", Some("e2e")).await.unwrap();
        let dest = ctx.path().join("backup.zip");
        let n = p.backup_download("e2e", &backup, &dest).await.unwrap();
        assert!(n > 0);
        eprintln!("backup {backup}: {n} bytes");
    }

    // 5. Uninstall everything.
    p.uninstall(
        "e2e",
        &UninstallOptions {
            delete_files: true,
            remove_runtime: true,
        },
        &sink,
    )
    .await
    .unwrap();
    assert_eq!(p.status("e2e").await.unwrap_err().code(), "unknown_server");
}
