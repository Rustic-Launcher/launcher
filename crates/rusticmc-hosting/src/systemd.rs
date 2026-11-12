//! Running the supervisor at boot.
//!
//! Horizon's own `horizon-server service install` writes a systemd **user** unit (or, with
//! `--system`, a system unit running as `$USER`, which under `sudo` is root). For a dedicated
//! unprivileged account we need a system unit with `User=<that account>`, so with root or sudo the
//! launcher renders the same unit Horizon renders (`horizon-server/src/service.rs`,
//! `systemd_unit`: same `ExecStart`, `TimeoutStopSec=180`, `Restart=on-failure`) with our user and
//! name, and writes it to `/etc/systemd/system` (ADR 0122). Without privileges, Horizon's own
//! `service install` is used as is, plus `loginctl enable-linger`.

use crate::shell::quote;

/// Horizon's unit name for a server folder (`service::name`): `horizon-server-<folder>` with the
/// folder lowercased and every other character turned into `-`. Ported so `systemctl --user`
/// commands find the unit `horizon-server service install` created.
pub fn horizon_unit_name(server_dir: &str) -> String {
    let folder = server_dir
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    let clean: String = folder
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let clean = clean.trim_matches('-');
    format!(
        "horizon-server-{}",
        if clean.is_empty() { "server" } else { clean }
    )
}

/// Our system unit's name for a server: `rustic-<name>` (names are validated: `[a-z0-9-]`).
pub fn rustic_unit_name(server: &str) -> String {
    format!("rustic-{server}")
}

/// What the unit runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitSpec<'a> {
    pub description: &'a str,
    /// `<install>/bin/horizon-server`: the installer's launcher script, which always runs the
    /// `current` release, so an update applies at the next restart.
    pub program: &'a str,
    pub server_dir: &'a str,
    /// The account it runs as (a system unit); `None` for a user unit.
    pub user: Option<&'a str>,
    /// With a dedicated account: the only tree the service may write (its home). Everything else is
    /// read-only (`ProtectSystem=strict`), and /home, /root are hidden.
    pub writable: Option<&'a str>,
}

/// systemd's own quoting for `ExecStart` arguments: double quotes with `\` and `"` escaped, and `%`
/// doubled (specifiers), `$` doubled (variables).
fn systemd_quote(arg: &str) -> String {
    let mut out = String::from("\"");
    for c in arg.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '%' => out.push_str("%%"),
            '$' => out.push_str("$$"),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Render the unit. Same shape as Horizon's `service::systemd_unit`, plus hardening that does not
/// change behaviour for the supervisor (no new privileges, private /tmp).
pub fn render_unit(spec: &UnitSpec<'_>) -> String {
    let (run_as, wanted_by, mut hardening) = match spec.user {
        Some(user) => (
            format!("User={user}\nGroup={user}\n"),
            "multi-user.target",
            "NoNewPrivileges=true\nPrivateTmp=true\n".to_owned(),
        ),
        None => (String::new(), "default.target", String::new()),
    };
    if let (Some(_), Some(dir)) = (spec.user, spec.writable) {
        hardening.push_str(&format!(
            "ProtectSystem=strict\nProtectHome=true\nReadWritePaths={}\n",
            systemd_quote(dir)
        ));
    }
    format!(
        "# Written by Rustic Launcher (launcher/docs/HOSTING.md). Same as `horizon-server service install`
# with a dedicated account; `systemctl edit` to override.
[Unit]
Description={description}
Documentation=https://github.com/horizon-rust/horizon/blob/main/docs/server.md
Wants=network-online.target
After=network-online.target

[Service]
Type=simple
{run_as}WorkingDirectory={dir_q}
ExecStart={program} --dir {dir} run
# SIGTERM makes the supervisor stop the game cleanly; saving a big world takes a moment.
TimeoutStopSec=180
Restart=on-failure
RestartSec=10
{hardening}
[Install]
WantedBy={wanted_by}
",
        description = spec.description.replace(['\n', '\r'], " "),
        dir_q = systemd_quote(spec.server_dir),
        program = systemd_quote(spec.program),
        dir = systemd_quote(spec.server_dir),
    )
}

/// Shell to install a rendered unit as root: written through a temp file, then moved, then
/// `daemon-reload` and `enable` (not `--now`: the start step starts it).
pub fn install_unit_script(unit_name: &str, unit_text: &str) -> crate::Result<String> {
    let path = format!("/etc/systemd/system/{unit_name}.service");
    // The unit text goes through a quoted heredoc-free printf: one quoted argument.
    Ok(format!(
        "printf %s {text} > {tmp} && chmod 0644 {tmp} && mv -f {tmp} {path} && systemctl daemon-reload && systemctl enable {unit}",
        text = quote(unit_text)?,
        tmp = quote(&format!("{path}.rustic-tmp"))?,
        path = quote(&path)?,
        unit = quote(&format!("{unit_name}.service"))?,
    ))
}

/// Shell to remove a unit installed by [`install_unit_script`] (idempotent).
pub fn uninstall_unit_script(unit_name: &str) -> crate::Result<String> {
    let unit = quote(&format!("{unit_name}.service"))?;
    let path = quote(&format!("/etc/systemd/system/{unit_name}.service"))?;
    Ok(format!(
        "systemctl disable --now {unit} >/dev/null 2>&1; rm -f {path}; systemctl daemon-reload"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_names_match_horizon() {
        assert_eq!(
            horizon_unit_name("/home/alex/rustic/servers/survival"),
            "horizon-server-survival"
        );
        assert_eq!(
            horizon_unit_name("/srv/My Server!/"),
            "horizon-server-my-server"
        );
        assert_eq!(horizon_unit_name("/"), "horizon-server-server");
        assert_eq!(rustic_unit_name("survival"), "rustic-survival");
    }

    #[test]
    fn system_unit_runs_as_the_dedicated_user() {
        let text = render_unit(&UnitSpec {
            description: "Rustic server survival",
            program: "/var/lib/rustic/bin/horizon-server",
            server_dir: "/var/lib/rustic/servers/survival",
            user: Some("rustic"),
            writable: Some("/var/lib/rustic"),
        });
        assert!(text.contains("User=rustic\nGroup=rustic\n"));
        assert!(text.contains("ProtectSystem=strict\n"));
        assert!(text.contains("ReadWritePaths=\"/var/lib/rustic\"\n"));
        assert!(text.contains(
            "ExecStart=\"/var/lib/rustic/bin/horizon-server\" --dir \"/var/lib/rustic/servers/survival\" run"
        ));
        assert!(text.contains("WorkingDirectory=\"/var/lib/rustic/servers/survival\""));
        assert!(text.contains("TimeoutStopSec=180"));
        assert!(text.contains("Restart=on-failure"));
        assert!(text.contains("NoNewPrivileges=true"));
        assert!(text.contains("WantedBy=multi-user.target"));
        assert!(text.contains("After=network-online.target"));
    }

    #[test]
    fn user_unit_has_no_user_line() {
        let text = render_unit(&UnitSpec {
            description: "x",
            program: "/home/a/rustic/bin/horizon-server",
            server_dir: "/home/a/rustic/servers/x",
            user: None,
            writable: None,
        });
        assert!(!text.contains("User="));
        assert!(!text.contains("ProtectSystem"));
        assert!(text.contains("WantedBy=default.target"));
    }

    #[test]
    fn specials_in_paths_are_escaped_for_systemd() {
        assert_eq!(systemd_quote("/a b/%h/$X\"\\"), "\"/a b/%%h/$$X\\\"\\\\\"");
        let text = render_unit(&UnitSpec {
            description: "line\nbreak",
            program: "/p",
            server_dir: "/d",
            user: None,
            writable: None,
        });
        assert!(text.contains("Description=line break\n"));
    }

    #[test]
    fn install_script_is_quoted() {
        let s = install_unit_script("rustic-x", "[Unit]\nDescription='q'\n").unwrap();
        assert!(s.starts_with("printf %s '[Unit]\nDescription='\\''q'\\''\n' > "));
        assert!(s.contains("/etc/systemd/system/rustic-x.service"));
        assert!(s.ends_with("systemctl enable rustic-x.service"));
        let u = uninstall_unit_script("rustic-x").unwrap();
        assert!(u.contains("systemctl disable --now rustic-x.service"));
    }
}
