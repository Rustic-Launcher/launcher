//! The multiplayer list shared by every instance, and server pings (Server List Ping).
//!
//! The list lives in `<data>/multiplayer.json` (the game's own `servers.dat` is NBT, per
//! instance; RISKS.md). Invite codes (`rustic://join/<code>` or a pasted code) resolve to the
//! first endpoint that answers.

use crate::dto::*;
use crate::state::{AppState, Res, State};
use crate::util::iso;
use rusticmc_launcher_core::{IpcError, IpcErrorCode, fsutil, time};
use std::time::Duration;

fn path(state: &AppState) -> std::path::PathBuf {
    state.paths.data().join("multiplayer.json")
}

fn load(state: &AppState) -> Vec<MultiplayerEntry> {
    fsutil::read_json(&path(state))
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn save(state: &AppState, list: &[MultiplayerEntry]) -> Res<()> {
    Ok(fsutil::write_json(&path(state), &list)?)
}

fn valid_address(a: &str) -> bool {
    let a = a.trim();
    !a.is_empty() && a.len() <= 255 && !a.contains(char::is_whitespace)
}

#[tauri::command]
pub fn multiplayer_list(state: State<'_>) -> Vec<MultiplayerEntry> {
    let mut list = load(&state);
    list.sort_by_key(|e| !e.favorite);
    list
}

#[tauri::command]
pub fn multiplayer_add(state: State<'_>, args: MultiplayerAddArgs) -> Res<MultiplayerEntry> {
    if !valid_address(&args.address) {
        return Err(IpcError::invalid("not a server address"));
    }
    let entry = MultiplayerEntry {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name: if args.name.trim().is_empty() {
            args.address.clone()
        } else {
            args.name.trim().into()
        },
        address: args.address.trim().into(),
        favorite: false,
        last_joined_at: None,
    };
    let mut list = load(&state);
    list.push(entry.clone());
    save(&state, &list)?;
    Ok(entry)
}

#[tauri::command]
pub fn multiplayer_update(state: State<'_>, args: MultiplayerUpdateArgs) -> Res<MultiplayerEntry> {
    let mut list = load(&state);
    let e = list
        .iter_mut()
        .find(|e| e.id == args.id)
        .ok_or_else(|| IpcError::not_found("no such entry"))?;
    if let Some(n) = args.patch.name {
        e.name = n;
    }
    if let Some(a) = args.patch.address {
        if !valid_address(&a) {
            return Err(IpcError::invalid("not a server address"));
        }
        e.address = a;
    }
    if let Some(f) = args.patch.favorite {
        e.favorite = f;
    }
    let out = e.clone();
    save(&state, &list)?;
    Ok(out)
}

#[tauri::command]
pub fn multiplayer_remove(state: State<'_>, args: IdArgs) -> Res<()> {
    let mut list = load(&state);
    list.retain(|e| e.id != args.id);
    save(&state, &list)
}

/// Records a join (the launch path calls it).
pub fn mark_joined(state: &AppState, address: &str) {
    let mut list = load(state);
    if let Some(e) = list
        .iter_mut()
        .find(|e| e.address.eq_ignore_ascii_case(address))
    {
        e.last_joined_at = Some(iso(time::now()));
        let _ = save(state, &list);
    }
}

/// `host[:port]`, `[v6]:port`.
pub fn split_address(address: &str) -> (String, u16) {
    let a = address.trim();
    if let Some(rest) = a.strip_prefix('[') {
        if let Some((h, p)) = rest.split_once("]:") {
            return (h.into(), p.parse().unwrap_or(25565));
        }
        return (rest.trim_end_matches(']').into(), 25565);
    }
    match a.rsplit_once(':') {
        Some((h, p)) if !h.contains(':') => (h.into(), p.parse().unwrap_or(25565)),
        _ => (a.into(), 25565),
    }
}

#[tauri::command]
pub async fn multiplayer_ping(args: AddressArgs) -> Res<ServerPing> {
    let mut address = args.address.trim().to_owned();
    // An invite code: the first endpoint that answers.
    if !address.contains('.')
        && !address.contains(':')
        && address.len() >= 10
        && let Ok(invite) = rusticmc_servers::invite::Invite::decode(&address)
    {
        address = invite
            .first_reachable(Duration::from_secs(3))
            .await
            .map(|e| e.address())
            .ok_or_else(|| {
                IpcError::new(IpcErrorCode::Network, "No address in this invite answers.")
            })?;
    }
    let (host, port) = split_address(&address);
    Ok(
        match rusticmc_hosting::slp::ping(&host, port, Duration::from_secs(5)).await {
            Ok(s) => ServerPing {
                online: true,
                latency_ms: Some(s.latency_ms),
                motd: s.motd.unwrap_or_default(),
                players_online: s.online.unwrap_or(0),
                players_max: s.max.unwrap_or(0),
                version: s.version.unwrap_or_default(),
            },
            Err(_) => ServerPing {
                online: false,
                latency_ms: None,
                motd: String::new(),
                players_online: 0,
                players_max: 0,
                version: String::new(),
            },
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert_eq!(
            split_address("play.example.com"),
            ("play.example.com".into(), 25565)
        );
        assert_eq!(split_address("1.2.3.4:25570"), ("1.2.3.4".into(), 25570));
        assert_eq!(
            split_address("[2001:db8::1]:30000"),
            ("2001:db8::1".into(), 30000)
        );
        assert_eq!(split_address("2001:db8::1"), ("2001:db8::1".into(), 25565));
    }
}
