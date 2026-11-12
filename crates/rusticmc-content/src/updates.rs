//! Update checks over what a ledger lists: a newer version from the same source that runs on
//! the instance's Horizon. Nothing is installed here; the UI offers "Update to ..." and installs
//! through the normal path. Never offered: an older version, a modpack's pinned mods, data
//! packs inside a world, native mods (Horizon's launcher rebuilds those).
use crate::compat;
use crate::error::Result;
use crate::horizon_env::HorizonEnv;
use crate::ledger::Ledger;
use crate::model::{ContentKind, ContentVersion, SourceKind};
use crate::sources::horizon::Index;
use crate::sources::modrinth::Modrinth;

#[derive(Clone, Debug, PartialEq)]
pub struct Update {
    pub project_id: String,
    pub source: SourceKind,
    pub installed: String,
    pub available: ContentVersion,
}

fn newer(a: &str, b: &str) -> bool {
    match (semver::Version::parse(a), semver::Version::parse(b)) {
        (Ok(x), Ok(y)) => x > y,
        _ => a != b,
    }
}

/// Native mods (rebuilt by Horizon's launcher), packs inside one world, and a modpack's mods
/// (pinned by the pack: it updates as a whole) are never updated one by one.
fn skip(kind: ContentKind, target: &str, needed_by: &[String], ledger: &Ledger) -> bool {
    kind == ContentKind::NativeMod
        || target.starts_with("world:")
        || needed_by.iter().any(|p| {
            ledger
                .entries
                .iter()
                .any(|e| &e.project_id == p && e.kind == ContentKind::Modpack)
        })
}

/// Updates from Horizon's registry (no network: the index is already fetched).
pub fn horizon_updates(ledger: &Ledger, index: &Index, env: &HorizonEnv) -> Vec<Update> {
    ledger
        .entries
        .iter()
        .filter(|e| {
            e.source == SourceKind::Horizon && !skip(e.kind, &e.target, &e.needed_by, ledger)
        })
        .filter_map(|e| {
            let best = index.installable(&e.project_id, env)?;
            newer(&best.version, &e.version).then(|| Update {
                project_id: e.project_id.clone(),
                source: SourceKind::Horizon,
                installed: e.version.clone(),
                available: best,
            })
        })
        .collect()
}

/// Updates from Modrinth for the instance's game version (resource packs, shader packs and data
/// packs for new worlds: the Modrinth kinds Rustic installs).
pub async fn modrinth_updates(
    ledger: &Ledger,
    modrinth: &Modrinth,
    env: &HorizonEnv,
) -> Result<Vec<Update>> {
    let mut out = Vec::new();
    for e in ledger.entries.iter().filter(|e| {
        e.source == SourceKind::Modrinth && !skip(e.kind, &e.target, &e.needed_by, ledger)
    }) {
        let versions = modrinth
            .versions(&e.project_id, &[], &[env.game.as_str()], None)
            .await?;
        let best = versions
            .into_iter()
            .find(|v| v.kind == e.kind && compat::check(v, None, env).installable());
        if let Some(best) = best
            && best.id != e.version_id
            && newer(&best.version, &e.version)
        {
            out.push(Update {
                project_id: e.project_id.clone(),
                source: SourceKind::Modrinth,
                installed: e.version.clone(),
                available: best,
            });
        }
    }
    Ok(out)
}
