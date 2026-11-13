//! Rustic Launcher's desktop app: the Tauri v2 shell around the launcher crates. Every command
//! and event of launcher/docs/IPC.md is here; commands take one `args` struct and reject with
//! `IpcError {code, message, details}`.

mod cloud_link;
mod cmd;
mod discord;
mod dto;
mod relay;
mod skins;
mod state;
mod tasks;
mod util;

use state::AppState;
use std::sync::Arc;
use tauri::{Emitter, Manager};

/// Handles `rustic://...` links: `rustic://join/<address-or-invite>` opens the join flow,
/// `rustic://auth?code=...` completes a Rustic account login (the website's handoff, ADR 0090),
/// `rustic://content/rustic/<project>[?version=<project>@<version>]` opens the project's page with
/// an install prompt (never an install: docs/SECURITY.md), `rustic://server/<slug>` opens a server
/// of the directory (ADR 0158; never joins from the link alone). A content or server link that
/// arrives before the webview listens (the launcher started by the link) waits in
/// `PENDING_CONTENT` until the UI takes it (`app_take_deep_link`).
/// A link can arrive twice (the single-instance plugin forwards it to the deep-link plugin, then to
/// its own callback): each sign-in code is exchanged once.
fn handle_deep_link(app: &tauri::AppHandle, url: &str) {
    let Some(rest) = url.strip_prefix("rustic://") else {
        tracing::warn!("ignored a link that is not rustic://");
        return;
    };
    if let Some(target) = rest.strip_prefix("join/") {
        tracing::info!("deep link: join");
        let target = target.trim_end_matches('/').to_owned();
        let _ = app.emit(
            "deep-link",
            serde_json::json!({ "action": "join", "target": target, "url": url }),
        );
    } else if rest.starts_with("content/") || rest == "content" {
        match content_link(rest) {
            Ok(link) if repeated_content_link(url) => {
                tracing::debug!(project = %link.project, "deep link: content (repeat ignored)");
            }
            Ok(link) => {
                tracing::info!(project = %link.project, "deep link: content");
                let payload = serde_json::json!({
                    "action": "content", "source": link.source, "project": link.project,
                    "version": link.version, "url": url
                });
                *PENDING_CONTENT.lock().unwrap_or_else(|e| e.into_inner()) = Some(payload.clone());
                let _ = app.emit("deep-link", payload);
            }
            Err(why) => tracing::warn!("ignored a rustic://content link: {why}"),
        }
    } else if rest.starts_with("server/") || rest == "server" {
        match server_link(rest) {
            Ok(listing) if repeated_content_link(url) => {
                tracing::debug!(%listing, "deep link: server (repeat ignored)");
            }
            Ok(listing) => {
                tracing::info!(%listing, "deep link: server");
                let payload =
                    serde_json::json!({ "action": "server", "listing": listing, "url": url });
                *PENDING_CONTENT.lock().unwrap_or_else(|e| e.into_inner()) = Some(payload.clone());
                let _ = app.emit("deep-link", payload);
            }
            Err(why) => tracing::warn!("ignored a rustic://server link: {why}"),
        }
    } else if let Some(code) = auth_code(rest) {
        static SEEN: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        {
            let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
            if seen.contains(&code) {
                return;
            }
            seen.push(code.clone());
        }
        tracing::info!("deep link: Rustic sign-in from the website");
        let Some(state) = app.try_state::<Arc<AppState>>() else {
            tracing::warn!("deep link before the app state: sign-in dropped");
            return;
        };
        let state = state.inner().clone();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let got = state.rustic.complete_oauth(&code).await;
            if let Err(e) = &got {
                tracing::warn!(error = %e, "Rustic sign-in from the website failed");
            } else {
                tracing::info!("Rustic sign-in from the website completed");
                crate::cmd::auth::autolink_minecraft(&state).await;
            }
            let _ = app.emit(
                "deep-link",
                serde_json::json!({
                    "action": "auth",
                    "ok": got.is_ok(),
                    "error": got.err().map(|e| e.to_string()),
                }),
            );
        });
    } else {
        tracing::warn!("ignored an unknown rustic:// link");
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// The newest content or server link not taken by the UI yet.
static PENDING_CONTENT: std::sync::Mutex<Option<serde_json::Value>> = std::sync::Mutex::new(None);

/// Takes the content link waiting for the UI, if any (once).
pub(crate) fn take_pending_content_link() -> Option<serde_json::Value> {
    PENDING_CONTENT
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
}

/// The same content link again within a few seconds: the single-instance plugin and the
/// deep-link plugin both forward it, and the page must open (and toast) once.
fn repeated_content_link(url: &str) -> bool {
    static LAST: std::sync::Mutex<Option<(String, std::time::Instant)>> =
        std::sync::Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    let repeated = last
        .as_ref()
        .is_some_and(|(u, at)| u == url && at.elapsed() < std::time::Duration::from_secs(5));
    *last = Some((url.to_owned(), std::time::Instant::now()));
    repeated
}

/// A parsed `rustic://content/...` link.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ContentLink {
    source: &'static str,
    project: String,
    /// `<project>@<version>`.
    version: Option<String>,
}

/// Parses `content/<source>/<project>[?version=<project>@<version>]` (what follows `rustic://`)
/// strictly: the `rustic` source only, a project id `[a-z0-9_-]{1,64}`, no other path segment,
/// query key or fragment, and a version of that same project (`[0-9A-Za-z.+_-]{1,64}` after the
/// `@`, percent-decoded: the website encodes `@` and `+`).
fn content_link(rest: &str) -> Result<ContentLink, &'static str> {
    let rest = rest.strip_prefix("content/").ok_or("not a content link")?;
    if rest.contains('#') {
        return Err("a fragment");
    }
    let (path, query) = match rest.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (rest, None),
    };
    // One trailing slash, as some browsers add it.
    let path = path.strip_suffix('/').unwrap_or(path);
    let mut parts = path.split('/');
    let (Some(source), Some(project), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err("expected content/<source>/<project>");
    };
    let source = match source {
        "rustic" => "rustic",
        _ => return Err("unknown source"),
    };
    let decode = |s: &str| {
        percent_encoding::percent_decode_str(s)
            .decode_utf8()
            .map(|c| c.into_owned())
            .map_err(|_| "not UTF-8")
    };
    let project = decode(project)?;
    if !rusticmc_content::sources::rustic::valid_project_id(&project) {
        return Err("invalid project id");
    }
    let mut version = None;
    if let Some(q) = query.filter(|q| !q.is_empty()) {
        for kv in q.split('&') {
            let Some(v) = kv.strip_prefix("version=") else {
                return Err("unknown query key");
            };
            if version.is_some() {
                return Err("version given twice");
            }
            let v = decode(v)?;
            let (p, ver) = v
                .split_once('@')
                .ok_or("version is not <project>@<version>")?;
            let ok = (1..=64).contains(&ver.len())
                && ver
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'+' | b'_' | b'-'));
            if p != project || !ok {
                return Err("invalid version");
            }
            version = Some(v);
        }
    }
    Ok(ContentLink {
        source,
        project,
        version,
    })
}

/// Parses `server/<slug or id>` (what follows `rustic://`) strictly: a directory slug
/// `[a-z0-9-]{3,48}` or a listing UUID, percent-decoded, one optional trailing slash, no other
/// path segment, query or fragment.
fn server_link(rest: &str) -> Result<String, &'static str> {
    let rest = rest.strip_prefix("server/").ok_or("not a server link")?;
    if rest.contains(['#', '?']) {
        return Err("a query or fragment");
    }
    let path = rest.strip_suffix('/').unwrap_or(rest);
    if path.contains('/') {
        return Err("expected server/<slug>");
    }
    let id = percent_encoding::percent_decode_str(path)
        .decode_utf8()
        .map_err(|_| "not UTF-8")?;
    let slug = (3..=48).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-');
    if slug || uuid::Uuid::parse_str(&id).is_ok_and(|u| u.hyphenated().to_string() == id) {
        Ok(id.into_owned())
    } else {
        Err("invalid server slug")
    }
}

/// The sign-in code of `auth?code=` (also `auth/?code=`, as some browsers rewrite it, and the older
/// `ticket=`), percent-decoded.
fn auth_code(rest: &str) -> Option<String> {
    let query = rest
        .strip_prefix("auth?")
        .or_else(|| rest.strip_prefix("auth/?"))?;
    query
        .split('&')
        .find_map(|kv| {
            kv.strip_prefix("code=")
                .or_else(|| kv.strip_prefix("ticket="))
        })
        .filter(|c| !c.is_empty())
        .map(|c| {
            percent_encoding::percent_decode_str(c)
                .decode_utf8_lossy()
                .into_owned()
        })
}

pub fn run() {
    let paths = match rusticmc_launcher_core::LauncherPaths::from_os() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Rustic Launcher: {e}");
            std::process::exit(1);
        }
    };
    let level = rusticmc_launcher_core::LauncherConfig::load(&paths.config_file())
        .map(|c| c.logging)
        .unwrap_or_default();
    let _log_guard =
        rusticmc_launcher_core::logging::init(&paths.logs(), &level.level, level.keep_files)
            .map_err(|e| eprintln!("logging: {e}"))
            .ok();
    tracing::info!(
        version = rusticmc_launcher_core::VERSION,
        "Rustic Launcher starting"
    );

    let builder = tauri::Builder::default()
        // First, so a second launch only forwards its arguments (deep links) to this one.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // The `deep-link` feature has already handed the links to `on_open_url`; handling
            // them here too is harmless (a code is exchanged once) and covers a missed one.
            for arg in argv.iter().filter(|a| a.starts_with("rustic://")) {
                handle_deep_link(app, arg);
            }
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        // The webview's log calls reach our tracing files (the Rust side already set the logger).
        .plugin(tauri_plugin_log::Builder::new().skip_logger().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            let handle = app.handle().clone();
            let state =
                Arc::new(AppState::new(handle.clone(), paths.clone()).map_err(|e| e.message)?);
            app.manage(state.clone());

            // Background work: downloads into the task list, game events, server watch, renewals.
            {
                let st = state.clone();
                let mut rx = st.downloads.subscribe();
                tauri::async_runtime::spawn(async move {
                    while let Ok(e) = rx.recv().await {
                        st.tasks.on_download(&e);
                    }
                });
            }
            cmd::instances::forward_game_events(state.clone());
            cmd::servers::watch_servers(state.clone());
            cmd::servers::autostart(state.clone());
            {
                let st = state.clone();
                tauri::async_runtime::spawn(async move {
                    for (uuid, r) in st.accounts.refresh_due().await {
                        if let Err(e) = r {
                            tracing::info!(account = %uuid, error = %e, "background renewal");
                        }
                    }
                });
            }

            use tauri_plugin_deep_link::DeepLinkExt;
            #[cfg(any(windows, target_os = "linux"))]
            if let Err(e) = app.deep_link().register_all() {
                tracing::warn!(error = %e, "could not register the rustic:// scheme");
            }
            let dl = handle.clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    handle_deep_link(&dl, url.as_str());
                }
            });
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                for url in urls {
                    handle_deep_link(&handle, url.as_str());
                }
            }
            state.discord.set(discord::Presence {
                place: discord::Place::Launcher,
                game: None,
                since: rusticmc_launcher_core::time::now(),
            });
            {
                let st = state.clone();
                tauri::async_runtime::spawn(async move {
                    cmd::auth::autolink_minecraft(&st).await;
                });
            }
            tracing::info!(data = %state.paths.data().display(), "ready");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // app / system
            cmd::app::app_info,
            cmd::app::player_skin_url,
            cmd::app::system_info,
            cmd::app::settings_get,
            cmd::app::settings_set,
            cmd::app::java_runtimes_list,
            cmd::app::rustic_server_status,
            cmd::app::rustic_server_set,
            cmd::app::app_open_path,
            cmd::app::app_open_url,
            cmd::app::app_take_deep_link,
            cmd::app::app_pick_path,
            cmd::app::app_check_update,
            cmd::app::app_install_update,
            cmd::app::downloads_list,
            cmd::app::download_cancel,
            cmd::app::game_detect_install,
            cmd::app::game_prepare_status,
            cmd::app::game_prepare,
            // auth
            cmd::auth::auth_accounts_list,
            cmd::auth::auth_microsoft_login_start,
            cmd::auth::auth_microsoft_browser_login_start,
            cmd::auth::auth_microsoft_login_poll,
            cmd::auth::auth_ownership_check,
            cmd::auth::auth_set_active,
            cmd::auth::auth_logout,
            cmd::auth::auth_rustic_session,
            cmd::auth::auth_rustic_login,
            cmd::auth::auth_rustic_logout,
            cmd::auth::auth_rustic_access_token,
            cmd::auth::auth_rustic_signup,
            cmd::auth::auth_rustic_login_password,
            cmd::auth::auth_rustic_login_mfa,
            cmd::auth::auth_rustic_resend_verification,
            // versions & instances
            cmd::instances::horizon_versions_list,
            cmd::instances::instances_list,
            cmd::instances::instance_get,
            cmd::instances::instance_create,
            cmd::instances::instance_import,
            cmd::instances::instance_update,
            cmd::instances::instance_delete,
            cmd::instances::instance_duplicate,
            cmd::instances::instance_set_version,
            cmd::instances::instance_open_folder,
            cmd::instances::instance_worlds_list,
            cmd::instances::instance_content_list,
            cmd::instances::instance_content_set_enabled,
            cmd::instances::instance_content_remove,
            cmd::instances::instance_content_update,
            cmd::instances::instance_resource_packs_reorder,
            // launch
            cmd::instances::instance_launch,
            cmd::instances::instance_launch_cancel,
            cmd::instances::instance_stop,
            cmd::instances::launch_status,
            // content
            cmd::content::content_search,
            cmd::content::content_get,
            cmd::content::content_categories,
            cmd::content::content_install,
            // native mods (ADR 0157)
            cmd::native::native_status,
            cmd::native::native_overview,
            cmd::native::native_consent,
            cmd::native::native_toolchain_install,
            cmd::native::native_build,
            cmd::native::native_cache_info,
            cmd::native::native_cache_clear,
            cmd::native::native_cache_set_limit,
            // multiplayer
            cmd::multiplayer::multiplayer_list,
            cmd::multiplayer::multiplayer_add,
            cmd::multiplayer::multiplayer_update,
            cmd::multiplayer::multiplayer_remove,
            cmd::multiplayer::multiplayer_ping,
            // servers
            cmd::servers::servers_list,
            cmd::servers::server_get,
            cmd::servers::server_create,
            cmd::servers::server_delete,
            cmd::servers::server_start,
            cmd::servers::server_stop,
            cmd::servers::server_restart,
            cmd::servers::server_command,
            cmd::servers::server_console_history,
            cmd::servers::server_complete,
            cmd::servers::server_players,
            cmd::servers::server_player_action,
            cmd::servers::server_whitelist_set_enabled,
            cmd::servers::server_config_get,
            cmd::servers::server_config_set,
            cmd::servers::server_config_raw_get,
            cmd::servers::server_config_raw_set,
            cmd::servers::server_plugins_list,
            cmd::servers::server_plugin_install,
            cmd::servers::server_plugin_set_enabled,
            cmd::servers::server_plugin_update,
            cmd::servers::server_plugin_remove,
            cmd::servers::server_datapacks_list,
            cmd::servers::server_datapack_set_enabled,
            cmd::servers::server_datapack_remove,
            cmd::servers::server_backups_list,
            cmd::servers::server_backup_create,
            cmd::servers::server_backup_restore,
            cmd::servers::server_backup_delete,
            cmd::servers::server_backup_schedule_get,
            cmd::servers::server_backup_schedule_set,
            cmd::servers::server_stats_history,
            cmd::servers::server_access_get,
            cmd::servers::server_relay_set,
            cmd::servers::server_invite_create,
            cmd::servers::server_invite_revoke,
            cmd::servers::server_upnp_set,
            cmd::servers::servers_starter_plugins,
            // hosting
            cmd::hosting::hosting_providers,
            cmd::hosting::hosting_waitlist_join,
            cmd::hosting::hosting_vps_test,
            cmd::hosting::hosting_vps_trust,
            cmd::hosting::hosting_vps_list,
            cmd::hosting::hosting_vps_remove,
            cmd::hosting::hosting_vps_check,
            cmd::hosting::hosting_vps_install,
            cmd::hosting::hosting_vps_generate_key,
            // domains
            cmd::dns::dns_porkbun_status,
            cmd::dns::dns_connect_porkbun,
            cmd::dns::dns_disconnect,
            cmd::dns::dns_list_domains,
            cmd::dns::dns_check_domain,
            cmd::dns::dns_buy_domain,
            cmd::dns::dns_configure_server,
            cmd::dns::dns_status,
            cmd::dns::dns_required_records,
            cmd::dns::dns_verify,
            cmd::dns::dns_remove_binding,
        ]);

    if let Err(e) = builder.run(tauri::generate_context!()) {
        tracing::error!(error = %e, "the app stopped");
        eprintln!("Rustic Launcher: {e}");
    }
}

#[cfg(test)]
mod deep_link_tests {
    use super::{ContentLink, auth_code, content_link, server_link};

    #[test]
    fn reads_server_links() {
        assert_eq!(
            server_link("server/arena-nord").as_deref(),
            Ok("arena-nord")
        );
        assert_eq!(
            server_link("server/arena-nord/").as_deref(),
            Ok("arena-nord")
        );
        assert_eq!(
            server_link("server/arena%2Dnord").as_deref(),
            Ok("arena-nord")
        );
        assert_eq!(
            server_link("server/0190f0aa-0000-7000-8000-000000000001").as_deref(),
            Ok("0190f0aa-0000-7000-8000-000000000001")
        );
        for bad in [
            "server",
            "server/",
            "server/ab",
            "server/Arena",
            "server/arena_nord",
            "server/-arena",
            "server/arena/extra",
            "server/arena?join=1",
            "server/arena#x",
            "server/..",
            "server/%2E%2E",
            "server/arena%2Fx",
            "server/%FF",
            "server/0190F0AA-0000-7000-8000-000000000001",
        ] {
            assert!(server_link(bad).is_err(), "{bad}");
        }
        assert!(server_link(&format!("server/{}", "a".repeat(49))).is_err());
    }

    fn link(project: &str, version: Option<&str>) -> ContentLink {
        ContentLink {
            source: "rustic",
            project: project.into(),
            version: version.map(Into::into),
        }
    }

    #[test]
    fn reads_content_links() {
        assert_eq!(
            content_link("content/rustic/sky_islands"),
            Ok(link("sky_islands", None))
        );
        assert_eq!(
            content_link("content/rustic/sky-islands/"),
            Ok(link("sky-islands", None))
        );
        assert_eq!(
            content_link("content/rustic/sky_islands?version=sky_islands%401.2.0%2Bb3"),
            Ok(link("sky_islands", Some("sky_islands@1.2.0+b3")))
        );
        assert_eq!(
            content_link("content/rustic/sky_islands?version=sky_islands@1.2.0"),
            Ok(link("sky_islands", Some("sky_islands@1.2.0")))
        );
        assert_eq!(content_link("content/rustic/a?"), Ok(link("a", None)));
        assert_eq!(
            content_link(&format!("content/rustic/{}", "a".repeat(64))),
            Ok(link(&"a".repeat(64), None))
        );
    }

    #[test]
    fn a_link_forwarded_twice_opens_once() {
        let url = "rustic://content/rustic/twice_test";
        assert!(!super::repeated_content_link(url));
        assert!(super::repeated_content_link(url));
        assert!(!super::repeated_content_link(
            "rustic://content/rustic/other_test"
        ));
    }

    #[test]
    fn refuses_anything_else() {
        for bad in [
            "content",
            "content/",
            "content/rustic",
            "content/rustic/",
            "content/modrinth/AANobbMI",
            "content/horizon/abelha_skull",
            "content/Rustic/sky",
            "content/rustic/Sky",
            "content/rustic/sky/extra",
            "content/rustic/sky//",
            "content/rustic//sky",
            "content/rustic/../sky",
            "content/rustic/sky%2F..",
            "content/rustic/sk%20y",
            "content/rustic/sky.islands",
            "content/rustic/sky#frag",
            "content/rustic/sky?install=1",
            "content/rustic/sky?version=sky@1.0&install=1",
            "content/rustic/sky?version=sky@1.0&version=sky@1.1",
            "content/rustic/sky?version=other@1.0",
            "content/rustic/sky?version=1.0",
            "content/rustic/sky?version=sky@",
            "content/rustic/sky?version=sky@1.0%20rm",
            "content/rustic/sky?version=sky@1.0/../x",
            "content/rustic/%FF",
        ] {
            assert!(content_link(bad).is_err(), "{bad}");
        }
        assert!(content_link(&format!("content/rustic/{}", "a".repeat(65))).is_err());
        assert!(content_link(&format!("content/rustic/a?version=a@{}", "1".repeat(65))).is_err());
    }

    #[test]
    fn reads_the_sign_in_code_in_every_form() {
        assert_eq!(auth_code("auth?code=abc").as_deref(), Some("abc"));
        assert_eq!(auth_code("auth/?code=abc").as_deref(), Some("abc"));
        assert_eq!(auth_code("auth?x=1&code=a%2Bb").as_deref(), Some("a+b"));
        assert_eq!(auth_code("auth?ticket=t").as_deref(), Some("t"));
        assert_eq!(auth_code("auth?code=").as_deref(), None);
        assert_eq!(auth_code("join/play.example.com"), None);
    }
}
