//! Where things go on the VPS and as whom commands run (ADR 0122).
//!
//! - With root or passwordless sudo: a dedicated unprivileged system account (`rustic` by default,
//!   no login shell) owns everything under its home `/var/lib/rustic`; the supervisor runs as it
//!   under a system unit. The login user only stages uploads in its own `~/.cache`.
//! - Without: everything goes under the login user's `~/rustic`, run as that user, with Horizon's
//!   own user unit (`horizon-server service install`) when systemd is there.
//!
//! ```text
//! <base>/horizon/               Horizon's install.sh --dir (releases/, current, game-files/)
//! <base>/bin/horizon-server     install.sh --bin launcher script (runs current/)
//! <base>/engines/<sha16>/rustic-server
//! <base>/servers/<name>/        the server folder (server.toml, world, plugins/, backups/)
//! <base>/staging/               files handed from the login user to the service account
//! <login home>/.cache/rustic-hosting/<name>/   SFTP uploads land here first
//! ```

use crate::error::{HostingError, Result};
use crate::platform::{Elevation, SystemInfo};
use crate::shell::{Cmd, quote};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Account {
    /// A dedicated system account owns the files and runs the server.
    Dedicated { user: String },
    /// The login user does everything (no root, no sudo).
    Login,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub account: Account,
    pub elevation: Elevation,
    pub base: String,
    pub login_home: String,
    pub systemd: bool,
}

/// How the supervisor is kept running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceKind {
    /// Our system unit `rustic-<name>`.
    System { unit: String },
    /// Horizon's user unit `horizon-server-<folder>`.
    User { unit: String },
    /// No systemd: `horizon-server start` (does not survive a reboot).
    Background,
}

pub const DEDICATED_HOME: &str = "/var/lib/rustic";

impl Layout {
    pub fn for_system(info: &SystemInfo, dedicated_user: &str) -> Result<Self> {
        crate::shell::validate_name(dedicated_user)?;
        if info.home.is_empty() || !info.home.starts_with('/') {
            return Err(HostingError::Protocol(format!(
                "the login user's home is not an absolute path: `{}`",
                info.home
            )));
        }
        let (account, base) = match info.elevation {
            Elevation::Root | Elevation::Sudo => (
                Account::Dedicated {
                    user: dedicated_user.to_owned(),
                },
                DEDICATED_HOME.to_owned(),
            ),
            Elevation::None => (
                Account::Login,
                format!("{}/rustic", info.home.trim_end_matches('/')),
            ),
        };
        Ok(Self {
            account,
            elevation: info.elevation,
            base,
            login_home: info.home.trim_end_matches('/').to_owned(),
            systemd: info.systemd,
        })
    }

    pub fn horizon_dir(&self) -> String {
        format!("{}/horizon", self.base)
    }
    pub fn bin_dir(&self) -> String {
        format!("{}/bin", self.base)
    }
    pub fn horizon_server(&self) -> String {
        format!("{}/bin/horizon-server", self.base)
    }
    pub fn servers_dir(&self) -> String {
        format!("{}/servers", self.base)
    }
    pub fn server_dir(&self, name: &str) -> String {
        format!("{}/servers/{name}", self.base)
    }
    pub fn engines_dir(&self) -> String {
        format!("{}/engines", self.base)
    }
    pub fn engine_path(&self, sha256: &str) -> String {
        format!(
            "{}/engines/{}/rustic-server",
            self.base,
            &sha256[..16.min(sha256.len())]
        )
    }
    /// Where the service account picks staged files up.
    pub fn service_staging(&self) -> String {
        format!("{}/staging", self.base)
    }
    /// Where the login user's SFTP uploads land.
    pub fn upload_dir(&self, name: &str) -> String {
        format!("{}/.cache/rustic-hosting/{name}", self.login_home)
    }

    pub fn service_user(&self) -> Option<&str> {
        match &self.account {
            Account::Dedicated { user } => Some(user),
            Account::Login => None,
        }
    }

    /// Run a shell command line as the account that owns the server.
    pub fn as_service(&self, command: &str) -> Result<String> {
        match (&self.account, self.elevation) {
            (Account::Login, _) => Ok(command.to_owned()),
            (Account::Dedicated { user }, Elevation::Root) => Ok(Cmd::new("su")?
                .args(["-s", "/bin/sh", "-c", command, user])?
                .into_string()),
            (Account::Dedicated { user }, Elevation::Sudo) => Ok(Cmd::new("sudo")?
                .args(["-n", "-H", "-u", user, "sh", "-c", command])?
                .into_string()),
            (Account::Dedicated { .. }, Elevation::None) => Err(HostingError::Precondition(
                "this server belongs to a dedicated account, but this login has neither root nor passwordless sudo".into(),
            )),
        }
    }

    /// Run a shell command line as root (unit files, firewall, account creation).
    pub fn as_root(&self, command: &str) -> Result<String> {
        match self.elevation {
            Elevation::Root => Ok(command.to_owned()),
            Elevation::Sudo => Ok(Cmd::new("sudo")?
                .args(["-n", "sh", "-c", command])?
                .into_string()),
            Elevation::None => Err(HostingError::Precondition(
                "this needs root or passwordless sudo on the VPS".into(),
            )),
        }
    }

    /// `<horizon-server> --dir <server> args...`, as the service account.
    pub fn horizon(&self, server: &str, args: &[&str]) -> Result<String> {
        let cmd = Cmd::new(&self.horizon_server())?
            .args(["--dir", &self.server_dir(server)])?
            .args(args)?;
        self.as_service(cmd.as_str())
    }

    /// Move a staged upload (owned by the login user) into the server's tree with `mode`, owned by
    /// the service account. Creates parent folders. Removes the staged file.
    pub fn place(&self, staged: &str, dest: &str, mode: &str) -> Result<String> {
        if !mode.chars().all(|c| c.is_ascii_digit()) || mode.len() != 4 {
            return Err(HostingError::Invalid(format!("bad mode {mode}")));
        }
        match &self.account {
            Account::Login => Ok(format!(
                "install -D -m {mode} {src} {dst} && rm -f {src}",
                src = quote(staged)?,
                dst = quote(dest)?
            )),
            Account::Dedicated { user } => self.as_root(&format!(
                "install -D -o {u} -g {u} -m {mode} {src} {dst} && rm -f {src}",
                u = quote(user)?,
                src = quote(staged)?,
                dst = quote(dest)?
            )),
        }
    }

    /// Copy a staged folder into the service account's staging, readable by it (prepared game
    /// files: the login user's home may be closed to other accounts).
    pub fn place_dir(&self, staged: &str, dest: &str) -> Result<String> {
        let body = format!(
            "rm -rf {dst} && mkdir -p {parent} && cp -R {src} {dst}",
            src = quote(staged)?,
            dst = quote(dest)?,
            parent = quote(parent_of(dest))?,
        );
        match &self.account {
            Account::Login => Ok(body),
            Account::Dedicated { user } => self.as_root(&format!(
                "{body} && chown -R {u}:{u} {dst}",
                u = quote(user)?,
                dst = quote(dest)?
            )),
        }
    }

    /// The service kind a server would use on this system.
    pub fn planned_service(&self, name: &str) -> ServiceKind {
        if !self.systemd {
            return ServiceKind::Background;
        }
        match self.account {
            Account::Dedicated { .. } => ServiceKind::System {
                unit: crate::systemd::rustic_unit_name(name),
            },
            Account::Login => ServiceKind::User {
                unit: crate::systemd::horizon_unit_name(&self.server_dir(name)),
            },
        }
    }
}

pub(crate) fn parent_of(path: &str) -> &str {
    match path.trim_end_matches('/').rsplit_once('/') {
        Some(("", _)) => "/",
        Some((p, _)) => p,
        None => ".",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::parse_probe;

    fn info(elevation: &str, user: &str, home: &str, systemd: bool) -> SystemInfo {
        parse_probe(&format!(
            "uname=Linux x86_64\nuser={user}\nhome={home}\nelevation={elevation}\n{}--- end\n",
            if systemd { "systemd=running\n" } else { "" }
        ))
        .unwrap()
    }

    #[test]
    fn sudo_gets_a_dedicated_account() {
        let l = Layout::for_system(&info("sudo", "alex", "/home/alex", true), "rustic").unwrap();
        assert_eq!(l.base, "/var/lib/rustic");
        assert_eq!(l.server_dir("survival"), "/var/lib/rustic/servers/survival");
        assert_eq!(
            l.upload_dir("survival"),
            "/home/alex/.cache/rustic-hosting/survival"
        );
        assert_eq!(
            l.as_service("echo 'hi'").unwrap(),
            "sudo -n -H -u rustic sh -c 'echo '\\''hi'\\'''"
        );
        assert_eq!(
            l.as_root("ufw status").unwrap(),
            "sudo -n sh -c 'ufw status'"
        );
        assert_eq!(
            l.planned_service("survival"),
            ServiceKind::System {
                unit: "rustic-survival".into()
            }
        );
        let h = l.horizon("survival", &["cmd", "say hi; reboot"]).unwrap();
        assert!(h.starts_with("sudo -n -H -u rustic sh -c "));
        assert!(h.contains(
            "/var/lib/rustic/bin/horizon-server --dir /var/lib/rustic/servers/survival cmd"
        ));
        // The console line stays one quoted argument, quoted twice (outer sh -c, inner word).
        assert!(h.contains("say hi; reboot"));
        assert!(!h.contains(" say hi; reboot"));
    }

    #[test]
    fn root_uses_su_to_drop_privileges() {
        let l = Layout::for_system(&info("root", "root", "/root", true), "rustic").unwrap();
        assert_eq!(l.as_service("id").unwrap(), "su -s /bin/sh -c id rustic");
        assert_eq!(l.as_root("id").unwrap(), "id");
        let p = l
            .place(
                "/root/.cache/rustic-hosting/s/x.rplugin",
                "/var/lib/rustic/servers/s/plugins/x.rplugin",
                "0644",
            )
            .unwrap();
        assert_eq!(
            p,
            "install -D -o rustic -g rustic -m 0644 /root/.cache/rustic-hosting/s/x.rplugin /var/lib/rustic/servers/s/plugins/x.rplugin && rm -f /root/.cache/rustic-hosting/s/x.rplugin"
        );
    }

    #[test]
    fn no_privileges_stays_in_home() {
        let l = Layout::for_system(&info("none", "bob", "/home/bob/", false), "rustic").unwrap();
        assert_eq!(l.base, "/home/bob/rustic");
        assert_eq!(l.as_service("id").unwrap(), "id");
        assert!(l.as_root("id").is_err());
        assert_eq!(l.planned_service("s"), ServiceKind::Background);
        let l2 = Layout::for_system(&info("none", "bob", "/home/bob", true), "rustic").unwrap();
        assert_eq!(
            l2.planned_service("s"),
            ServiceKind::User {
                unit: "horizon-server-s".into()
            }
        );
        assert!(l2.place("a", "b", "644").is_err());
        assert!(l2.place("/a b", "/c", "0644").unwrap().contains("'/a b'"));
    }

    #[test]
    fn bad_inputs() {
        assert!(Layout::for_system(&info("none", "x", "relative", false), "rustic").is_err());
        assert!(Layout::for_system(&info("sudo", "x", "/home/x", false), "Root!").is_err());
        assert_eq!(parent_of("/a/b/c"), "/a/b");
        assert_eq!(parent_of("/a"), "/");
    }
}
