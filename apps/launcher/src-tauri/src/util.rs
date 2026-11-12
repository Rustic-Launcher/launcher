//! Small helpers shared by the commands.

use crate::state::AppState;
use rusticmc_instances::InstalledBuild;
use rusticmc_launcher_core::{IpcError, IpcErrorCode};

/// ISO-8601 for unix seconds.
pub fn iso(secs: u64) -> String {
    rusticmc_launcher_core::time::iso8601(secs)
}

/// The build new instances, servers and the default preparation use: the newest installed of
/// the default source, else the newest installed of any source.
pub fn default_build(state: &AppState) -> Option<InstalledBuild> {
    let default = state.config().builds.default_source;
    let installed = state.instances.builds.installed();
    installed
        .iter()
        .find(|b| b.source == default)
        .or_else(|| installed.first())
        .cloned()
}

/// A label for a build version: `Nightly · 4baa571 (Rustic edition)`, `v2026.10.12`.
pub fn build_label(version: &str, source: &str) -> String {
    let base = match version.strip_prefix("nightly-") {
        Some(rest) if rest.len() > 11 => format!("Nightly · {}", &rest[11..]),
        Some(rest) => format!("Nightly {rest}"),
        None => version.to_owned(),
    };
    if source == "rustic-edition" {
        format!("Horizon {base} (Rustic edition)")
    } else {
        format!("Horizon {base}")
    }
}

/// A feature waiting on another component (the cloud client, a crate not wired yet).
pub fn not_available_yet(what: &str) -> IpcError {
    IpcError::with_reason(
        IpcErrorCode::Unavailable,
        format!("{what} is not available yet."),
        "not_available_yet",
    )
}

/// Files directly in `dir` (names), skipping hidden ones.
pub fn files_in(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .map(|e| {
            e.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    !p.file_name()
                        .map(|n| n.to_string_lossy().starts_with('.'))
                        .unwrap_or(true)
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// A file's modification time as unix seconds.
pub fn mtime(path: &std::path::Path) -> Option<u64> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}
