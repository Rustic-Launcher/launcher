# 0122. VPS layout, dedicated account and service unit

- Status: accepted (hosting, 2026-11-10)

## Context

Horizon's `horizon-server service install` writes a systemd **user** unit, or with `--system` a system unit running
as `$USER` (root under sudo). Its installer never asks for sudo and takes `--dir/--bin`. SECURITY §2.8 asks for a
dedicated unprivileged `rustic` account and a hardened unit.

## Decision

- With root or passwordless sudo: system account `rustic` (`useradd --system`, nologin shell), home and install tree
  `/var/lib/rustic` (FHS variable state, rather than ARCHITECTURE's `/opt/rustic`), mode 0750. Commands run as it
  through `sudo -n -H -u rustic sh -c` or, as root, `su -s /bin/sh -c ... rustic`.
- The unit `rustic-<server>.service` is rendered by the launcher with Horizon's `ExecStart="<bin>/horizon-server"
  --dir "<server>" run`, `TimeoutStopSec=180`, `Restart=on-failure`, plus `User/Group=rustic`, `NoNewPrivileges`,
  `PrivateTmp`, `ProtectSystem=strict`, `ProtectHome`, `ReadWritePaths=/var/lib/rustic`.
- Without privileges: `~/rustic`, Horizon's own `service install` (user unit) + `loginctl enable-linger`. Without
  systemd: background `horizon-server start` with a warning.
- Uploads land in the login user's `~/.cache/rustic-hosting/<server>` (0700) and are moved in with
  `install -D -o rustic -g rustic -m MODE` as root.
- The Rustic engine is content-addressed (`engines/<sha256[..16]>/rustic-server`), `engine.path` set with
  `config set`; the web panel is turned off (`panel.bind = ""`).

## Consequences

The supervisor and game run without root and cannot write outside their tree; `horizon-server update` still works
(releases live inside `/var/lib/rustic`).

## Alternatives

`sudo horizon-server service install --system` (would run as root); Docker on the VPS (heavier; Horizon publishes no
image).
