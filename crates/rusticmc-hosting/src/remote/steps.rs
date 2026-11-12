//! The install plan for a VPS (launcher/docs/HOSTING.md, "What the installer does"). Each step
//! checks the VPS itself to know whether it is done, so a re-run resumes.

use super::layout::{Account, Layout, ServiceKind};
use super::{RemoteInstallOptions, sha256_file};
use crate::error::{HostingError, Result};
use crate::model::{EngineSource, ServerSpec};
use crate::plan::{Step, StepProgress};
use crate::platform::{Arch, Firewall, SystemInfo, check_requirements};
use crate::shell::{Cmd, quote};
use crate::ssh::SshSession;
use crate::systemd;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Everything the steps share.
pub struct InstallCtx {
    pub ssh: Arc<SshSession>,
    pub info: SystemInfo,
    pub layout: Layout,
    pub spec: ServerSpec,
    pub opts: RemoteInstallOptions,
    /// The address players use (for the final status ping from this computer).
    pub public_host: String,
    /// Set by the init step: the server folder was there before this run (keep it on rollback).
    pub server_dir_existed: AtomicBool,
}

impl InstallCtx {
    pub(crate) async fn ok(&self, command: &str) -> Result<bool> {
        Ok(self.ssh.exec(command, None).await?.success())
    }
    pub(crate) async fn run(&self, command: &str) -> Result<String> {
        self.ssh.run(command).await
    }
    fn server_dir(&self) -> String {
        self.layout.server_dir(&self.spec.name)
    }
    fn test_as_service(&self, test: &str) -> Result<String> {
        self.layout.as_service(test)
    }
}

/// The ordered plan.
pub fn install_plan(ctx: &InstallCtx) -> Vec<Box<dyn Step<InstallCtx>>> {
    let mut steps: Vec<Box<dyn Step<InstallCtx>>> = vec![Box::new(Requirements)];
    if matches!(ctx.layout.account, Account::Dedicated { .. }) {
        steps.push(Box::new(ServiceAccount));
    }
    steps.push(Box::new(Folders));
    steps.push(Box::new(InstallHorizon));
    if !matches!(ctx.spec.engine, EngineSource::Horizon) {
        steps.push(Box::new(InstallEngine));
    }
    steps.push(Box::new(InitServer));
    steps.push(Box::new(Configure));
    if ctx.spec.prepared_resources.is_some() {
        steps.push(Box::new(ImportResources));
    }
    steps.push(Box::new(Service));
    steps.push(Box::new(OpenFirewall));
    steps.push(Box::new(StartServer));
    steps.push(Box::new(HealthCheck));
    steps
}

pub struct Requirements;

#[async_trait]
impl Step<InstallCtx> for Requirements {
    fn id(&self) -> &'static str {
        "requirements"
    }
    fn title(&self) -> String {
        "Check the VPS (system, RAM, disk, port)".into()
    }
    async fn is_done(&self, _ctx: &InstallCtx) -> Result<bool> {
        Ok(false)
    }
    async fn apply(&self, ctx: &InstallCtx, p: &StepProgress<'_>) -> Result<()> {
        p.detail(format!(
            "{} ({} {}), {} CPU, {} MB RAM",
            if ctx.info.os.pretty_name.is_empty() {
                "Linux"
            } else {
                &ctx.info.os.pretty_name
            },
            ctx.info.kernel,
            ctx.info.arch.as_str(),
            ctx.info.cpus,
            ctx.info.mem_total_kb / 1024
        ));
        if !ctx.info.os.is_debian_like() {
            p.warn(format!(
                "{} is not Debian or Ubuntu, the systems Rustic tests on: the install should work, but report problems",
                if ctx.info.os.id.is_empty() { "This Linux" } else { &ctx.info.os.id }
            ));
        }
        if ctx.info.arch == Arch::Aarch64 && ctx.opts.horizon_archive.is_none() {
            return Err(HostingError::UnsupportedPlatform {
                os: "Linux".into(),
                arch: "aarch64".into(),
                detail: "Horizon publishes no Linux ARM release yet (docs/hosting.md: build `horizon-server` and `horizon-client` from source). Build them, pack them as a .tar.gz and give it as the Horizon archive".into(),
            });
        }
        let ours = ctx
            .ok(&ctx.test_as_service(&format!(
                "test -f {}",
                quote(&format!("{}/server.toml", ctx.server_dir()))?
            ))?)
            .await?;
        let disk = match ctx.layout.account {
            Account::Dedicated { .. } => ctx.info.disk_varlib_available_kb,
            Account::Login => ctx.info.disk_home_available_kb,
        };
        let report = check_requirements(&ctx.info, disk, ctx.spec.port, ours);
        for w in &report.warnings {
            p.warn(w.clone());
        }
        if !report.blocking.is_empty() {
            return Err(HostingError::Requirements(report.blocking.join("; ")));
        }
        Ok(())
    }
}

pub struct ServiceAccount;

#[async_trait]
impl Step<InstallCtx> for ServiceAccount {
    fn id(&self) -> &'static str {
        "service-account"
    }
    fn title(&self) -> String {
        "Create the dedicated `rustic` account".into()
    }
    async fn is_done(&self, ctx: &InstallCtx) -> Result<bool> {
        let user = ctx.layout.service_user().unwrap_or_default();
        ctx.ok(&format!("id -u {} >/dev/null 2>&1", quote(user)?))
            .await
    }
    async fn apply(&self, ctx: &InstallCtx, _p: &StepProgress<'_>) -> Result<()> {
        let user = quote(ctx.layout.service_user().unwrap_or_default())?;
        let home = quote(&ctx.layout.base)?;
        let script = if ctx.info.has("useradd") {
            format!(
                "useradd --system --create-home --home-dir {home} --shell /usr/sbin/nologin --user-group {user}"
            )
        } else if ctx.info.has("adduser") {
            // BusyBox (Alpine): system account, no password, own group.
            format!(
                "addgroup -S {user} && adduser -S -D -h {home} -s /sbin/nologin -G {user} {user}"
            )
        } else {
            return Err(HostingError::Precondition(
                "neither useradd nor adduser exists on the VPS: create an account `rustic` by hand, or log in without sudo to install under your own account".into(),
            ));
        };
        ctx.run(
            &ctx.layout
                .as_root(&format!("{script} && chmod 0750 {home}"))?,
        )
        .await?;
        Ok(())
    }
    async fn rollback(&self, ctx: &InstallCtx) -> Result<()> {
        let user = quote(ctx.layout.service_user().unwrap_or_default())?;
        let home = quote(&ctx.layout.base)?;
        let script = format!(
            "(userdel {user} 2>/dev/null || deluser {user} 2>/dev/null); (groupdel {user} 2>/dev/null || delgroup {user} 2>/dev/null); rm -rf {home}; true"
        );
        ctx.run(&ctx.layout.as_root(&script)?).await?;
        Ok(())
    }
}

pub struct Folders;

#[async_trait]
impl Step<InstallCtx> for Folders {
    fn id(&self) -> &'static str {
        "folders"
    }
    fn title(&self) -> String {
        "Prepare the folders".into()
    }
    async fn is_done(&self, ctx: &InstallCtx) -> Result<bool> {
        let l = &ctx.layout;
        let test = format!(
            "test -d {} && test -d {} && test -d {}",
            quote(&l.servers_dir())?,
            quote(&l.engines_dir())?,
            quote(&l.service_staging())?
        );
        Ok(ctx.ok(&l.as_service(&test)?).await?
            && ctx
                .ok(&format!(
                    "test -d {}",
                    quote(&l.upload_dir(&ctx.spec.name))?
                ))
                .await?)
    }
    async fn apply(&self, ctx: &InstallCtx, _p: &StepProgress<'_>) -> Result<()> {
        let l = &ctx.layout;
        let mk = Cmd::new("mkdir")?.args([
            "-p",
            &l.servers_dir(),
            &l.engines_dir(),
            &l.service_staging(),
            &l.bin_dir(),
        ])?;
        ctx.run(&l.as_service(mk.as_str())?).await?;
        // The login user's upload folder, private to it.
        let up = Cmd::new("mkdir")?.args(["-p", &l.upload_dir(&ctx.spec.name)])?;
        ctx.run(&format!(
            "{} && chmod 0700 {}",
            up,
            quote(&format!("{}/.cache/rustic-hosting", l.login_home))?
        ))
        .await?;
        Ok(())
    }
}

pub struct InstallHorizon;

#[async_trait]
impl Step<InstallCtx> for InstallHorizon {
    fn id(&self) -> &'static str {
        "install-horizon"
    }
    fn title(&self) -> String {
        "Install Horizon's server (horizon-server)".into()
    }
    async fn is_done(&self, ctx: &InstallCtx) -> Result<bool> {
        let l = &ctx.layout;
        let test = format!(
            "test -x {} && {} version >/dev/null 2>&1",
            quote(&format!("{}/current/horizon-server", l.horizon_dir()))?,
            quote(&l.horizon_server())?
        );
        ctx.ok(&l.as_service(&test)?).await
    }
    async fn apply(&self, ctx: &InstallCtx, p: &StepProgress<'_>) -> Result<()> {
        let l = &ctx.layout;
        let script = format!("{}/install.sh", l.service_staging());
        // 1. The installer script, from Horizon's repository (docs/hosting.md, "1. Install").
        p.detail(format!("Downloading {}", ctx.opts.installer_url));
        let fetch = Cmd::new("curl")?.args([
            "-fsSL",
            "--proto",
            "=https",
            "-o",
            &script,
            &ctx.opts.installer_url,
        ])?;
        ctx.run(&l.as_service(fetch.as_str())?).await?;
        let sum = ctx
            .run(&l.as_service(Cmd::new("sha256sum")?.arg(&script)?.as_str())?)
            .await?;
        let sum = sum.split_whitespace().next().unwrap_or_default().to_owned();
        if let Some(expected) = &ctx.opts.installer_sha256
            && !expected.eq_ignore_ascii_case(&sum)
        {
            return Err(HostingError::Checksum {
                what: "Horizon's install.sh".into(),
                expected: expected.clone(),
                actual: sum,
            });
        }
        p.detail(format!("install.sh sha256 {sum}"));
        // 2. Run it: it downloads the release, checks its signature (OpenSSL 3) or GitHub's SHA-256,
        //    and never asks for sudo. --dir/--bin keep everything in our tree.
        let mut run =
            Cmd::new("sh")?.args([&script, "--dir", &l.horizon_dir(), "--bin", &l.bin_dir()])?;
        if let Some(archive) = &ctx.opts.horizon_archive {
            let file = archive
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| "horizon.tar.gz".into());
            let upload = format!("{}/{file}", l.upload_dir(&ctx.spec.name));
            let staged = format!("{}/{file}", l.service_staging());
            p.detail(format!("Uploading {file}"));
            ctx.ssh
                .sftp_upload(archive, &upload, &|d, t| p.bytes(d, t))
                .await?;
            ctx.run(&l.place(&upload, &staged, "0644")?).await?;
            let version = ctx
                .opts
                .horizon_version
                .clone()
                .unwrap_or_else(|| "local".into());
            run = run.args(["--archive", &staged, "--version", &version])?;
        } else if let Some(v) = ctx.opts.horizon_version.as_ref().filter(|v| !v.is_empty()) {
            run = run.args(["--version", v])?;
        }
        p.detail("Running Horizon's installer");
        let out = ctx.run(&l.as_service(run.as_str())?).await?;
        for line in out.lines().filter(|l| {
            l.starts_with("Release signature")
                || l.starts_with("Note:")
                || l.starts_with("Installed")
        }) {
            p.detail(line.to_owned());
        }
        Ok(())
    }
    async fn rollback(&self, ctx: &InstallCtx) -> Result<()> {
        let l = &ctx.layout;
        let rm = Cmd::new("rm")?.args([
            "-rf",
            &l.horizon_dir(),
            &l.horizon_server(),
            &format!("{}/horizon-prepare", l.bin_dir()),
        ])?;
        ctx.run(&l.as_service(rm.as_str())?).await?;
        Ok(())
    }
}

pub struct InstallEngine;

impl InstallEngine {
    fn sha(spec: &ServerSpec) -> Result<String> {
        match &spec.engine {
            EngineSource::RusticUpload { sha256, .. }
            | EngineSource::RusticRelease { sha256, .. } => {
                let s = sha256.to_ascii_lowercase();
                if s.len() != 64 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(HostingError::Invalid(
                        "the engine's sha256 must be 64 hex digits".into(),
                    ));
                }
                Ok(s)
            }
            EngineSource::Horizon => Err(HostingError::Invalid("no Rustic engine".into())),
        }
    }
}

/// Install a Rustic engine at its content-addressed path; shared with `update_engine`.
pub(crate) async fn install_engine(
    ssh: &SshSession,
    layout: &Layout,
    server: &str,
    engine: &EngineSource,
    sha: &str,
    p: Option<&StepProgress<'_>>,
) -> Result<String> {
    let target = layout.engine_path(sha);
    let part = format!("{target}.part");
    match engine {
        EngineSource::RusticUpload { path, .. } => {
            let local = sha256_file(path)?;
            if local != sha {
                return Err(HostingError::Checksum {
                    what: path.display().to_string(),
                    expected: sha.to_owned(),
                    actual: local,
                });
            }
            let upload = format!("{}/rustic-server", layout.upload_dir(server));
            if let Some(p) = p {
                p.detail("Uploading rustic-server");
            }
            ssh.sftp_upload(path, &upload, &|d, t| {
                if let Some(p) = p {
                    p.bytes(d, t)
                }
            })
            .await?;
            ssh.run(&layout.place(&upload, &part, "0755")?).await?;
        }
        EngineSource::RusticRelease { url, .. } => {
            if !url.starts_with("https://") {
                return Err(HostingError::Invalid(
                    "the engine release URL must be https://".into(),
                ));
            }
            if let Some(p) = p {
                p.detail(format!("Downloading {url} on the VPS"));
            }
            let fetch = Cmd::new("mkdir")?
                .args(["-p", super::layout::parent_of(&target)])?
                .and(Cmd::new("curl")?.args(["-fL", "--proto", "=https", "-o", &part, url])?)
                .and(Cmd::new("chmod")?.args(["0755", &part])?);
            ssh.run(&layout.as_service(fetch.as_str())?).await?;
        }
        EngineSource::Horizon => return Err(HostingError::Invalid("no Rustic engine".into())),
    }
    // Check on the VPS, then move into place.
    let got = ssh
        .run(&layout.as_service(Cmd::new("sha256sum")?.arg(&part)?.as_str())?)
        .await?;
    let got = got
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if got != sha {
        let _ = ssh
            .run(&layout.as_service(Cmd::new("rm")?.args(["-f", &part])?.as_str())?)
            .await;
        return Err(HostingError::Checksum {
            what: "rustic-server on the VPS".into(),
            expected: sha.to_owned(),
            actual: got,
        });
    }
    ssh.run(&layout.as_service(Cmd::new("mv")?.args(["-f", &part, &target])?.as_str())?)
        .await?;
    Ok(target)
}

#[async_trait]
impl Step<InstallCtx> for InstallEngine {
    fn id(&self) -> &'static str {
        "install-engine"
    }
    fn title(&self) -> String {
        "Install the Rustic Server engine (rustic-server)".into()
    }
    async fn is_done(&self, ctx: &InstallCtx) -> Result<bool> {
        let sha = Self::sha(&ctx.spec)?;
        let target = ctx.layout.engine_path(&sha);
        let check = format!("test -x {t} && sha256sum {t}", t = quote(&target)?);
        let out = ctx.ssh.exec(&ctx.layout.as_service(&check)?, None).await?;
        Ok(out.success() && out.stdout_text().split_whitespace().next() == Some(sha.as_str()))
    }
    async fn apply(&self, ctx: &InstallCtx, p: &StepProgress<'_>) -> Result<()> {
        let sha = Self::sha(&ctx.spec)?;
        install_engine(
            &ctx.ssh,
            &ctx.layout,
            &ctx.spec.name,
            &ctx.spec.engine,
            &sha,
            Some(p),
        )
        .await?;
        Ok(())
    }
    async fn rollback(&self, ctx: &InstallCtx) -> Result<()> {
        let sha = Self::sha(&ctx.spec)?;
        let dir = super::layout::parent_of(&ctx.layout.engine_path(&sha)).to_owned();
        ctx.run(
            &ctx.layout
                .as_service(Cmd::new("rm")?.args(["-rf", &dir])?.as_str())?,
        )
        .await?;
        Ok(())
    }
}

pub struct InitServer;

/// `horizon-server init` flags from the spec (docs/hosting.md, "3. First run").
pub fn init_args(spec: &ServerSpec) -> Result<Vec<String>> {
    if !spec.eula_accepted {
        return Err(HostingError::Precondition(
            "the Minecraft EULA (https://aka.ms/MinecraftEULA) must be accepted for this server first".into(),
        ));
    }
    let mut a = vec![
        "init".to_owned(),
        "--eula".into(),
        "--world".into(),
        spec.world.clone(),
        "--port".into(),
        spec.port.to_string(),
        if spec.whitelist {
            "--whitelist".into()
        } else {
            "--no-whitelist".into()
        },
    ];
    if let Some(seed) = spec.seed.as_ref().filter(|s| !s.is_empty()) {
        a.extend(["--seed".into(), seed.clone()]);
    }
    for player in &spec.players {
        a.extend(["--player".into(), player.clone()]);
    }
    if let Some(op) = spec.op.as_ref().filter(|s| !s.is_empty()) {
        a.extend(["--op".into(), op.clone()]);
    }
    if !spec.online_mode {
        a.push("--offline".into());
    }
    a.push("--yes".into());
    Ok(a)
}

#[async_trait]
impl Step<InstallCtx> for InitServer {
    fn id(&self) -> &'static str {
        "init-server"
    }
    fn title(&self) -> String {
        "Create the server (horizon-server init)".into()
    }
    async fn is_done(&self, ctx: &InstallCtx) -> Result<bool> {
        let test = format!(
            "test -f {}",
            quote(&format!("{}/server.toml", ctx.server_dir()))?
        );
        ctx.ok(&ctx.layout.as_service(&test)?).await
    }
    async fn apply(&self, ctx: &InstallCtx, p: &StepProgress<'_>) -> Result<()> {
        let dir = ctx.server_dir();
        let existed = ctx
            .ok(&ctx
                .layout
                .as_service(&format!("test -d {}", quote(&dir)?))?)
            .await?;
        ctx.server_dir_existed.store(existed, Ordering::SeqCst);
        ctx.run(
            &ctx.layout
                .as_service(Cmd::new("mkdir")?.args(["-p", &dir])?.as_str())?,
        )
        .await?;
        let args = init_args(&ctx.spec)?;
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = ctx.run(&ctx.layout.horizon(&ctx.spec.name, &args)?).await?;
        for line in out.lines().filter(|l| !l.trim().is_empty()).take(8) {
            p.detail(line.to_owned());
        }
        Ok(())
    }
    async fn rollback(&self, ctx: &InstallCtx) -> Result<()> {
        let dir = ctx.server_dir();
        let cmd = if ctx.server_dir_existed.load(Ordering::SeqCst) {
            // The folder was there (an existing world): only take back what init wrote.
            Cmd::new("rm")?.args(["-f", &format!("{dir}/server.toml")])?
        } else {
            Cmd::new("rm")?.args(["-rf", &dir])?
        };
        ctx.run(&ctx.layout.as_service(cmd.as_str())?).await?;
        Ok(())
    }
}

/// `section.key=value` pairs, validated (keys are TOML paths, values one line).
pub fn config_pairs(changes: &[(String, String)]) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for (k, v) in changes {
        let ok_key = !k.is_empty()
            && k.contains('.')
            && k.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '"'));
        if !ok_key {
            return Err(HostingError::Invalid(format!("bad setting name `{k}`")));
        }
        if v.contains(['\n', '\r', '\0']) {
            return Err(HostingError::Invalid(format!(
                "the value of `{k}` must be one line"
            )));
        }
        out.push(format!("{k}={v}"));
    }
    Ok(out)
}

pub struct Configure;

#[async_trait]
impl Step<InstallCtx> for Configure {
    fn id(&self) -> &'static str {
        "configure"
    }
    fn title(&self) -> String {
        "Write the settings (server.toml)".into()
    }
    async fn is_done(&self, _ctx: &InstallCtx) -> Result<bool> {
        // `config set` is idempotent and checks everything first: always run it.
        Ok(false)
    }
    async fn apply(&self, ctx: &InstallCtx, p: &StepProgress<'_>) -> Result<()> {
        let mut changes: Vec<(String, String)> = ctx
            .spec
            .settings
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if let EngineSource::RusticUpload { sha256, .. }
        | EngineSource::RusticRelease { sha256, .. } = &ctx.spec.engine
        {
            changes.push((
                "engine.path".into(),
                ctx.layout.engine_path(&sha256.to_ascii_lowercase()),
            ));
        }
        // The web panel is not needed (the launcher is the panel) and stays off (SECURITY §2.8);
        // RCON and the management protocol are off by default and left so.
        if !ctx.spec.settings.contains_key("panel.bind") {
            changes.push(("panel.bind".into(), String::new()));
        }
        if ctx.info.mem_total_kb > 0 && ctx.info.mem_total_kb < crate::platform::RECOMMENDED_RAM_KB
        {
            changes.push(("engine.low_memory".into(), "true".into()));
        }
        if changes.is_empty() {
            return Ok(());
        }
        let mut args = vec!["config".to_owned(), "set".to_owned()];
        args.extend(config_pairs(&changes)?);
        args.push("--json".into());
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = ctx.run(&ctx.layout.horizon(&ctx.spec.name, &args)?).await?;
        p.detail(format!("server.toml: {}", out.trim()));
        Ok(())
    }
}

pub struct ImportResources;

/// The folder name the prepared files are unpacked under on the VPS (any name: import-resources
/// checks the content, not the name).
pub const PREPARED_DIR: &str = "prepared";

#[async_trait]
impl Step<InstallCtx> for ImportResources {
    fn id(&self) -> &'static str {
        "import-resources"
    }
    fn title(&self) -> String {
        "Upload your prepared game files (horizon-server import-resources)".into()
    }
    async fn is_done(&self, ctx: &InstallCtx) -> Result<bool> {
        let Some(local) = &ctx.spec.prepared_resources else {
            return Ok(true);
        };
        let manifest = local.join("horizon-resources.json");
        let local_sum = sha256_file(&manifest).map_err(|e| {
            HostingError::Precondition(format!(
                "{} is not a prepared folder (no horizon-resources.json: prepare the game files first): {e}",
                local.display()
            ))
        })?;
        let remote = format!("{}/resources/horizon-resources.json", ctx.server_dir());
        let out = ctx
            .ssh
            .exec(
                &ctx.layout
                    .as_service(Cmd::new("sha256sum")?.arg(&remote)?.as_str())?,
                None,
            )
            .await?;
        Ok(
            out.success()
                && out.stdout_text().split_whitespace().next() == Some(local_sum.as_str()),
        )
    }
    async fn apply(&self, ctx: &InstallCtx, p: &StepProgress<'_>) -> Result<()> {
        let local = ctx
            .spec
            .prepared_resources
            .clone()
            .expect("planned only with resources");
        // One archive instead of thousands of small SFTP writes.
        p.detail(format!("Packing {}", local.display()));
        let archive = tokio::task::spawn_blocking(move || super::pack_dir(&local, PREPARED_DIR))
            .await
            .map_err(|e| HostingError::Io(std::io::Error::other(e.to_string())))??;
        let upload = format!("{}/game-files.tar", ctx.layout.upload_dir(&ctx.spec.name));
        p.detail("Uploading the prepared game files");
        let sent = ctx
            .ssh
            .sftp_upload(archive.path(), &upload, &|d, t| p.bytes(d, t))
            .await;
        drop(archive);
        sent?;
        let staging = ctx.layout.service_staging();
        let staged_archive = format!("{staging}/{}-game-files.tar", ctx.spec.name);
        let staged = format!("{staging}/{}-game-files", ctx.spec.name);
        ctx.run(&ctx.layout.place(&upload, &staged_archive, "0644")?)
            .await?;
        let unpack = Cmd::new("rm")?
            .args(["-rf", &staged])?
            .and(Cmd::new("mkdir")?.args(["-p", &staged])?)
            .and(Cmd::new("tar")?.args(["-xf", &staged_archive, "-C", &staged])?)
            .and(Cmd::new("rm")?.args(["-f", &staged_archive])?);
        ctx.run(&ctx.layout.as_service(unpack.as_str())?).await?;
        // `resources/` in the server folder: import-resources then puts the files there, and the engine
        // looks there first whatever `engine.path` is (horizon-server engine::resource_candidates).
        let dir = ctx.server_dir();
        ctx.run(
            &ctx.layout.as_service(
                Cmd::new("mkdir")?
                    .args(["-p", &format!("{dir}/resources")])?
                    .as_str(),
            )?,
        )
        .await?;
        let source = format!("{staged}/{PREPARED_DIR}");
        let out = ctx
            .run(
                &ctx.layout
                    .horizon(&ctx.spec.name, &["import-resources", &source])?,
            )
            .await?;
        p.detail(out.lines().next().unwrap_or_default().to_owned());
        let _ = ctx
            .run(
                &ctx.layout
                    .as_service(Cmd::new("rm")?.args(["-rf", &staged])?.as_str())?,
            )
            .await;
        Ok(())
    }
    async fn rollback(&self, ctx: &InstallCtx) -> Result<()> {
        let dir = ctx.server_dir();
        ctx.run(
            &ctx.layout.as_service(
                Cmd::new("rm")?
                    .args(["-rf", &format!("{dir}/resources")])?
                    .as_str(),
            )?,
        )
        .await?;
        Ok(())
    }
}

pub struct Service;

pub(crate) async fn service_enabled(
    ctx_ssh: &SshSession,
    layout: &Layout,
    kind: &ServiceKind,
) -> Result<bool> {
    match kind {
        ServiceKind::System { unit } => Ok(ctx_ssh
            .exec(
                &format!(
                    "test -f {} && systemctl is-enabled --quiet {}",
                    quote(&format!("/etc/systemd/system/{unit}.service"))?,
                    quote(unit)?
                ),
                None,
            )
            .await?
            .success()),
        ServiceKind::User { unit } => Ok(ctx_ssh
            .exec(
                &layout.as_service(&format!(
                    "systemctl --user is-enabled --quiet {}",
                    quote(unit)?
                ))?,
                None,
            )
            .await?
            .success()),
        ServiceKind::Background => Ok(false),
    }
}

#[async_trait]
impl Step<InstallCtx> for Service {
    fn id(&self) -> &'static str {
        "service"
    }
    fn title(&self) -> String {
        "Start the server with the VPS (systemd)".into()
    }
    async fn is_done(&self, ctx: &InstallCtx) -> Result<bool> {
        service_enabled(
            &ctx.ssh,
            &ctx.layout,
            &ctx.layout.planned_service(&ctx.spec.name),
        )
        .await
    }
    async fn apply(&self, ctx: &InstallCtx, p: &StepProgress<'_>) -> Result<()> {
        let l = &ctx.layout;
        match l.planned_service(&ctx.spec.name) {
            ServiceKind::System { unit } => {
                let dir = ctx.server_dir();
                let program = l.horizon_server();
                let text = systemd::render_unit(&systemd::UnitSpec {
                    description: &format!("Rustic server {} (Horizon)", ctx.spec.name),
                    program: &program,
                    server_dir: &dir,
                    user: l.service_user(),
                    writable: Some(&l.base),
                });
                ctx.run(&l.as_root(&systemd::install_unit_script(&unit, &text)?)?)
                    .await?;
                p.detail(format!("/etc/systemd/system/{unit}.service"));
            }
            ServiceKind::User { .. } => {
                // Horizon's own command: a user unit, enabled and started now.
                ctx.run(&l.horizon(&ctx.spec.name, &["service", "install"])?)
                    .await?;
                let linger = ctx
                    .ssh
                    .exec("loginctl enable-linger \"$(id -un)\"", None)
                    .await?;
                if !linger.success() {
                    p.warn("`loginctl enable-linger` was refused: the server stops when you log out of the VPS. Ask your provider to allow lingering, or install with sudo.");
                }
            }
            ServiceKind::Background => {
                p.warn("No systemd on this VPS: the server runs in the background but won't start again by itself after a reboot.");
            }
        }
        Ok(())
    }
    async fn rollback(&self, ctx: &InstallCtx) -> Result<()> {
        remove_service(&ctx.ssh, &ctx.layout, &ctx.spec.name).await
    }
}

pub(crate) async fn remove_service(ssh: &SshSession, l: &Layout, name: &str) -> Result<()> {
    match l.planned_service(name) {
        ServiceKind::System { unit } => {
            ssh.run(&l.as_root(&systemd::uninstall_unit_script(&unit)?)?)
                .await?;
        }
        ServiceKind::User { .. } => {
            let _ = ssh
                .exec(&l.horizon(name, &["service", "uninstall"])?, None)
                .await?;
        }
        ServiceKind::Background => {}
    }
    Ok(())
}

pub struct OpenFirewall;

/// The firewall commands for a port: (is the front-end active?, is the rule there?, add, remove).
pub(crate) fn firewall_commands(fw: Firewall, port: u16, name: &str) -> Result<[String; 4]> {
    let rule = format!("{port}/tcp");
    Ok(match fw {
        Firewall::Ufw => [
            "ufw status | grep -q '^Status: active'".into(),
            format!("ufw status | grep -qE '^{port}/tcp[[:space:]]'"),
            format!(
                "ufw allow {} comment {}",
                quote(&rule)?,
                quote(&format!("rustic {name}"))?
            ),
            format!("ufw delete allow {} >/dev/null 2>&1; true", quote(&rule)?),
        ],
        Firewall::Firewalld => [
            "firewall-cmd --state >/dev/null 2>&1".into(),
            format!("firewall-cmd --query-port={}", quote(&rule)?),
            format!(
                "firewall-cmd --permanent --add-port={r} && firewall-cmd --add-port={r}",
                r = quote(&rule)?
            ),
            format!(
                "firewall-cmd --permanent --remove-port={r} >/dev/null 2>&1; firewall-cmd --remove-port={r} >/dev/null 2>&1; true",
                r = quote(&rule)?
            ),
        ],
    })
}

#[async_trait]
impl Step<InstallCtx> for OpenFirewall {
    fn id(&self) -> &'static str {
        "firewall"
    }
    fn title(&self) -> String {
        "Open the game port in the firewall".into()
    }
    async fn is_done(&self, ctx: &InstallCtx) -> Result<bool> {
        let (Some(fw), true) = (ctx.info.firewall(), ctx.opts.open_firewall) else {
            return Ok(false);
        };
        if ctx.layout.as_root("true").is_err() {
            return Ok(false);
        }
        let [active, has, ..] = firewall_commands(fw, ctx.spec.port, &ctx.spec.name)?;
        Ok(!ctx.ok(&ctx.layout.as_root(&active)?).await?
            || ctx.ok(&ctx.layout.as_root(&has)?).await?)
    }
    async fn apply(&self, ctx: &InstallCtx, p: &StepProgress<'_>) -> Result<()> {
        let cloud_note = "Your provider may also have a firewall in its console (security group): allow TCP port there too.";
        let Some(fw) = ctx.info.firewall() else {
            p.warn(format!("No ufw or firewalld on the VPS. {cloud_note}"));
            return Ok(());
        };
        if !ctx.opts.open_firewall {
            p.warn(format!(
                "Not opening TCP port {} (you did not allow it): players can't join until it is open.",
                ctx.spec.port
            ));
            return Ok(());
        }
        let Ok(_) = ctx.layout.as_root("true") else {
            p.warn("Opening the firewall needs root or sudo: open the port by hand.");
            return Ok(());
        };
        let [_, _, add, _] = firewall_commands(fw, ctx.spec.port, &ctx.spec.name)?;
        ctx.run(&ctx.layout.as_root(&add)?).await?;
        p.detail(format!("TCP {} allowed. {cloud_note}", ctx.spec.port));
        Ok(())
    }
    async fn rollback(&self, ctx: &InstallCtx) -> Result<()> {
        if let (Some(fw), true, Ok(_)) = (
            ctx.info.firewall(),
            ctx.opts.open_firewall,
            ctx.layout.as_root("true"),
        ) {
            let [.., remove] = firewall_commands(fw, ctx.spec.port, &ctx.spec.name)?;
            ctx.run(&ctx.layout.as_root(&remove)?).await?;
        }
        Ok(())
    }
}

/// Read the supervisor's phase (`status --json`).
pub(crate) async fn phase(ssh: &SshSession, l: &Layout, name: &str) -> Result<String> {
    let out = ssh.run(&l.horizon(name, &["status", "--json"])?).await?;
    let v: serde_json::Value = serde_json::from_str(out.trim())
        .map_err(|e| HostingError::Protocol(format!("status --json: {e}: {}", out.trim())))?;
    Ok(v["phase"].as_str().unwrap_or("unknown").to_owned())
}

pub(crate) async fn start_server(ssh: &SshSession, l: &Layout, name: &str) -> Result<()> {
    match l.planned_service(name) {
        ServiceKind::System { unit }
            if service_enabled(ssh, l, &ServiceKind::System { unit: unit.clone() }).await? =>
        {
            ssh.run(&l.as_root(&format!("systemctl start {}", quote(&unit)?))?)
                .await?;
        }
        ServiceKind::User { unit }
            if service_enabled(ssh, l, &ServiceKind::User { unit: unit.clone() }).await? =>
        {
            ssh.run(&l.as_service(&format!("systemctl --user start {}", quote(&unit)?))?)
                .await?;
        }
        _ => {
            ssh.run(&l.horizon(name, &["start"])?).await?;
        }
    }
    Ok(())
}

pub(crate) async fn stop_server(ssh: &SshSession, l: &Layout, name: &str) -> Result<()> {
    match l.planned_service(name) {
        ServiceKind::System { unit }
            if service_enabled(ssh, l, &ServiceKind::System { unit: unit.clone() }).await? =>
        {
            ssh.run(&l.as_root(&format!("systemctl stop {}", quote(&unit)?))?)
                .await?;
        }
        ServiceKind::User { unit }
            if service_enabled(ssh, l, &ServiceKind::User { unit: unit.clone() }).await? =>
        {
            ssh.run(&l.as_service(&format!("systemctl --user stop {}", quote(&unit)?))?)
                .await?;
        }
        _ => {
            let out = ssh.exec(&l.horizon(name, &["stop"])?, None).await?;
            // "no server is running" is fine for a stop.
            if !out.success() && !out.stderr_text().contains("no server is running") {
                out.check("horizon-server stop")?;
            }
        }
    }
    Ok(())
}

pub struct StartServer;

#[async_trait]
impl Step<InstallCtx> for StartServer {
    fn id(&self) -> &'static str {
        "start"
    }
    fn title(&self) -> String {
        "Start the server".into()
    }
    async fn is_done(&self, ctx: &InstallCtx) -> Result<bool> {
        Ok(phase(&ctx.ssh, &ctx.layout, &ctx.spec.name).await? == "running")
    }
    async fn apply(&self, ctx: &InstallCtx, p: &StepProgress<'_>) -> Result<()> {
        if ctx.spec.prepared_resources.is_none() {
            let have = ctx
                .ok(&ctx.layout.as_service(&format!(
                    "test -f {}",
                    quote(&format!(
                        "{}/resources/horizon-resources.json",
                        ctx.server_dir()
                    ))?
                ))?)
                .await?;
            if !have {
                p.warn("Not started: the server has no prepared game files yet. Import them from this computer (Server > Game files), then start it.");
                return Ok(());
            }
        }
        start_server(&ctx.ssh, &ctx.layout, &ctx.spec.name).await
    }
    async fn rollback(&self, ctx: &InstallCtx) -> Result<()> {
        stop_server(&ctx.ssh, &ctx.layout, &ctx.spec.name).await
    }
}

pub struct HealthCheck;

#[async_trait]
impl Step<InstallCtx> for HealthCheck {
    fn id(&self) -> &'static str {
        "health-check"
    }
    fn title(&self) -> String {
        "Check that players can reach it".into()
    }
    async fn is_done(&self, _ctx: &InstallCtx) -> Result<bool> {
        Ok(false)
    }
    async fn apply(&self, ctx: &InstallCtx, p: &StepProgress<'_>) -> Result<()> {
        let deadline = Instant::now() + ctx.opts.health_timeout;
        let mut last = String::new();
        loop {
            let ph = phase(&ctx.ssh, &ctx.layout, &ctx.spec.name).await?;
            if ph == "running" {
                break;
            }
            if ph == "stopped" && last.is_empty() {
                // Not started on purpose (no game files yet): nothing to check.
                let have = ctx
                    .ok(&ctx.layout.as_service(&format!(
                        "test -f {}",
                        quote(&format!(
                            "{}/resources/horizon-resources.json",
                            ctx.server_dir()
                        ))?
                    ))?)
                    .await?;
                if !have {
                    return Ok(());
                }
            }
            if Instant::now() > deadline {
                return Err(HostingError::Timeout(format!(
                    "the server is `{ph}` after {} s; its log says why (Server > Console)",
                    ctx.opts.health_timeout.as_secs()
                )));
            }
            if ph != last {
                p.detail(format!("Server is {ph}..."));
                last = ph;
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        p.detail("The game is up on the VPS");
        if ctx.opts.skip_public_ping {
            return Ok(());
        }
        match crate::slp::ping(&ctx.public_host, ctx.spec.port, Duration::from_secs(8)).await {
            Ok(st) => p.detail(format!(
                "Reachable at {}:{} ({} ms, {})",
                ctx.public_host,
                ctx.spec.port,
                st.latency_ms,
                st.version.unwrap_or_default()
            )),
            Err(e) => p.warn(format!(
                "The server runs, but this computer can't reach {}:{} ({e}). Check your provider's firewall (security group) for TCP {}.",
                ctx.public_host, ctx.spec.port, ctx.spec.port
            )),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn spec() -> ServerSpec {
        ServerSpec {
            name: "survival".into(),
            port: 25565,
            world: "world".into(),
            seed: Some("123".into()),
            whitelist: true,
            players: vec!["Alex".into(), "Steve".into()],
            op: Some("Alex".into()),
            online_mode: true,
            eula_accepted: true,
            engine: EngineSource::Horizon,
            prepared_resources: None,
            settings: BTreeMap::new(),
        }
    }

    #[test]
    fn init_flags_follow_horizon() {
        let a = init_args(&spec()).unwrap();
        assert_eq!(
            a,
            vec![
                "init",
                "--eula",
                "--world",
                "world",
                "--port",
                "25565",
                "--whitelist",
                "--seed",
                "123",
                "--player",
                "Alex",
                "--player",
                "Steve",
                "--op",
                "Alex",
                "--yes"
            ]
        );
        let mut s = spec();
        s.online_mode = false;
        s.whitelist = false;
        s.seed = None;
        s.players.clear();
        s.op = None;
        assert_eq!(
            init_args(&s).unwrap(),
            vec![
                "init",
                "--eula",
                "--world",
                "world",
                "--port",
                "25565",
                "--no-whitelist",
                "--offline",
                "--yes"
            ]
        );
        s.eula_accepted = false;
        assert!(matches!(init_args(&s), Err(HostingError::Precondition(_))));
    }

    #[test]
    fn config_pairs_are_checked() {
        let ok = config_pairs(&[
            ("world.difficulty".into(), "hard".into()),
            ("properties.spawn-protection".into(), "0".into()),
        ])
        .unwrap();
        assert_eq!(
            ok,
            vec!["world.difficulty=hard", "properties.spawn-protection=0"]
        );
        assert!(config_pairs(&[("nodot".into(), "x".into())]).is_err());
        assert!(config_pairs(&[("a.b;rm".into(), "x".into())]).is_err());
        assert!(config_pairs(&[("a.b".into(), "x\ny".into())]).is_err());
    }

    #[test]
    fn firewall_rules() {
        let [active, has, add, remove] =
            firewall_commands(Firewall::Ufw, 25565, "survival").unwrap();
        assert!(active.contains("Status: active"));
        assert!(has.contains("^25565/tcp"));
        assert_eq!(add, "ufw allow 25565/tcp comment 'rustic survival'");
        assert!(remove.starts_with("ufw delete allow 25565/tcp"));
        let [_, _, add, _] = firewall_commands(Firewall::Firewalld, 25570, "x").unwrap();
        assert_eq!(
            add,
            "firewall-cmd --permanent --add-port=25570/tcp && firewall-cmd --add-port=25570/tcp"
        );
    }

    #[test]
    fn engine_sha_is_validated() {
        let mut s = spec();
        s.engine = EngineSource::RusticRelease {
            url: "https://x".into(),
            sha256: "AB".repeat(32),
        };
        assert_eq!(InstallEngine::sha(&s).unwrap(), "ab".repeat(32));
        s.engine = EngineSource::RusticRelease {
            url: "https://x".into(),
            sha256: "zz".into(),
        };
        assert!(InstallEngine::sha(&s).is_err());
    }
}
