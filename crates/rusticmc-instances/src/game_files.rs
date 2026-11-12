//! The player's own copy of the game files a build is pinned to, downloaded from Mojang for a
//! verified owner when no launcher on this computer has them whole (ADR 0005: "downloading Mojang
//! files ... only for a signed-in owner"; ADR 0147). Same idea as Horizon's own launcher
//! (`horizon/launcher/src/mojang.rs`, MIT), rewritten: the version JSON, the client jar, this
//! system's libraries, the asset index and its objects, each checked against the sha1 Mojang lists,
//! in the official launcher's layout under the launcher's data folder. `horizon-prepare` then reads
//! that folder like any launcher's. Nothing of Mojang's is part of Rustic.
//!
//! Files the player's own launcher folder already holds are linked (or copied) instead of
//! downloaded; the download manager re-checks every one and fetches what does not match.

use crate::error::{InstanceError, Result};
use rusticmc_auth::VerifiedOwner;
use rusticmc_launcher_core::download::hash_file;
use rusticmc_launcher_core::{
    CancellationToken, Checksum, DownloadError, DownloadManager, DownloadRequest, HashAlgo,
};
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

/// Where Mojang serves version files and asset objects (another host only in tests).
#[derive(Clone, Copy, Debug)]
struct Hosts<'a> {
    piston_meta: &'a str,
    objects: &'a str,
}

const MOJANG: Hosts<'static> = Hosts {
    piston_meta: "https://piston-meta.mojang.com/v1/packages",
    objects: "https://resources.download.minecraft.net",
};

/// The game version a build is pinned to (its `research-lock.json`, `minecraft`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pins {
    pub version: String,
    /// The client jar's sha1.
    pub client_sha1: String,
    /// The version JSON's sha1, then the other ones Mojang republished (accepted too).
    pub manifests: Vec<String>,
}

impl Pins {
    /// Reads the pins of an installed build; `None` when it has no lock file or no pins.
    pub fn of_build(build_dir: &Path) -> Option<Pins> {
        let text = std::fs::read_to_string(build_dir.join("research-lock.json")).ok()?;
        Self::parse(&serde_json::from_str(&text).ok()?)
    }

    pub fn parse(lock: &Value) -> Option<Pins> {
        let game = &lock["minecraft"];
        let hex = |v: &Value| v.as_str().filter(|s| is_sha1(s)).map(str::to_owned);
        let mut manifests = vec![hex(&game["manifest_sha1"])?];
        if let Some(more) = game["accepted_manifest_sha1"].as_array() {
            manifests.extend(more.iter().filter_map(hex));
        }
        let version = game["version"].as_str()?;
        relative(version).ok()?;
        Some(Pins {
            version: version.to_owned(),
            client_sha1: hex(&game["client_sha1"])?,
            manifests,
        })
    }

    fn manifest_accepted(&self, path: &Path) -> bool {
        hash_file(path, HashAlgo::Sha1).is_ok_and(|h| self.manifests.contains(&h))
    }
}

/// One file of the game, where it goes under a game folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameFile {
    pub url: String,
    pub sha1: String,
    pub size: Option<u64>,
    /// Relative to the game folder (`libraries/...`, `assets/objects/ab/ab...`).
    pub path: PathBuf,
}

/// Mojang's name for this system in version files' rules.
pub fn os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

/// The version JSON of the pinned version in a game folder.
fn manifest_path(dir: &Path, version: &str) -> PathBuf {
    dir.join("versions")
        .join(version)
        .join(format!("{version}.json"))
}

/// Whether `dir` holds every file of the pinned version (an accepted version JSON, the client jar,
/// this system's libraries, the asset index and its objects), by size. Cheap: no hashing but the
/// version JSON's; `horizon-prepare` checks the rest.
pub fn is_complete(dir: &Path, pins: &Pins) -> bool {
    let manifest = manifest_path(dir, &pins.version);
    if !pins.manifest_accepted(&manifest) {
        return false;
    }
    let Ok(files) = read_json(&manifest).and_then(|m| game_files(&m, pins, os_name())) else {
        return false;
    };
    let index = files.last().cloned();
    let present = |f: &GameFile| {
        std::fs::metadata(dir.join(&f.path))
            .is_ok_and(|m| m.is_file() && f.size.is_none_or(|s| s == m.len()))
    };
    if !files.iter().all(present) {
        return false;
    }
    let Some(index) = index else { return false };
    read_json(&dir.join(&index.path))
        .and_then(|i| objects(&i, MOJANG.objects))
        .is_ok_and(|objects| objects.iter().all(present))
}

/// The version's own files (client jar, libraries for `os`, logging setup) and, last, its asset
/// index; paths relative to a game folder. Unsafe paths are refused.
pub fn game_files(manifest: &Value, pins: &Pins, os: &str) -> Result<Vec<GameFile>> {
    let v = &pins.version;
    let client = &manifest["downloads"]["client"];
    let mut out = vec![GameFile {
        url: text(client, "url")?,
        // The pin wins over the version file: a rewritten file can't swap the game.
        sha1: pins.client_sha1.clone(),
        size: client["size"].as_u64(),
        path: Path::new("versions").join(v).join(format!("{v}.jar")),
    }];
    let libraries = manifest["libraries"]
        .as_array()
        .ok_or_else(|| bad("the version file lists no libraries"))?;
    for library in libraries.iter().filter(|l| allowed(l, os)) {
        let native = native_classifier(library, os)
            .map(|key| &library["downloads"]["classifiers"][key.as_str()]);
        for artifact in [Some(&library["downloads"]["artifact"]), native]
            .into_iter()
            .flatten()
            .filter(|a| a.is_object())
        {
            out.push(GameFile {
                url: text(artifact, "url")?,
                sha1: sha1_of(artifact)?,
                size: artifact["size"].as_u64(),
                path: Path::new("libraries").join(relative(&text(artifact, "path")?)?),
            });
        }
    }
    let logging = &manifest["logging"]["client"]["file"];
    if logging.is_object() {
        out.push(GameFile {
            url: text(logging, "url")?,
            sha1: sha1_of(logging)?,
            size: logging["size"].as_u64(),
            path: Path::new("assets/log_configs").join(relative(&text(logging, "id")?)?),
        });
    }
    let index = &manifest["assetIndex"];
    out.push(GameFile {
        url: text(index, "url")?,
        sha1: sha1_of(index)?,
        size: index["size"].as_u64(),
        path: Path::new("assets/indexes").join(relative(&format!("{}.json", text(index, "id")?))?),
    });
    Ok(out)
}

/// The asset index's objects, once each, served from `host`.
fn objects(index: &Value, host: &str) -> Result<Vec<GameFile>> {
    let map = index["objects"]
        .as_object()
        .ok_or_else(|| bad("the asset index lists no objects"))?;
    let mut out: Vec<GameFile> = Vec::with_capacity(map.len());
    for object in map.values() {
        let hash = object["hash"]
            .as_str()
            .filter(|h| is_sha1(h))
            .ok_or_else(|| bad("an asset has no valid hash"))?;
        let path = format!("{}/{hash}", &hash[..2]);
        out.push(GameFile {
            url: format!("{host}/{path}"),
            sha1: hash.to_owned(),
            size: object["size"].as_u64(),
            path: Path::new("assets/objects").join(path),
        });
    }
    out.sort_by(|a, b| a.sha1.cmp(&b.sha1));
    out.dedup_by(|a, b| a.sha1 == b.sha1);
    Ok(out)
}

/// Progress of [`download`]: bytes done of the total known so far.
pub type Progress<'a> = &'a (dyn Fn(u64, u64) + Send + Sync);

/// Makes `dest` a complete game folder for the pinned version: files `seed` (the player's own
/// launcher folder) holds at the same place are linked or copied, the rest downloaded from Mojang,
/// every one checked against its sha1. Only for a verified owner. Stops at the first failure or
/// when `cancel` fires; what is done stays, so the next call fetches only the rest.
pub async fn download(
    owner: &VerifiedOwner,
    pins: &Pins,
    dest: &Path,
    seed: Option<&Path>,
    downloads: &DownloadManager,
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> Result<()> {
    download_from(MOJANG, owner, pins, dest, seed, downloads, cancel, progress).await
}

#[allow(clippy::too_many_arguments)]
async fn download_from(
    hosts: Hosts<'_>,
    owner: &VerifiedOwner,
    pins: &Pins,
    dest: &Path,
    seed: Option<&Path>,
    downloads: &DownloadManager,
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> Result<()> {
    tracing::info!(
        owner = owner.uuid(),
        version = %pins.version,
        "downloading the game files from Mojang for a verified owner"
    );
    let v = &pins.version;
    // The version JSON: the player's if it is an accepted one, else Mojang's (by the pinned sha1).
    let manifest = manifest_path(dest, v);
    if !pins.manifest_accepted(&manifest) {
        let from_seed = seed
            .map(|s| manifest_path(s, v))
            .filter(|p| pins.manifest_accepted(p));
        match from_seed {
            Some(p) => copy_file(&p, &manifest)?,
            None => {
                let sha1 = &pins.manifests[0];
                let req = DownloadRequest::new(
                    format!("{}/{sha1}/{v}.json", hosts.piston_meta),
                    &manifest,
                )
                .checksum(Checksum::sha1(sha1).ok_or_else(|| bad("bad pinned sha1"))?)
                .label(format!("Game {v}"))
                .no_cache();
                fetch(downloads, vec![req], cancel, &|_, _| {}).await?;
            }
        }
    }
    let files = game_files(&read_json(&manifest)?, pins, os_name())?;
    let total: u64 = files.iter().filter_map(|f| f.size).sum();
    run(downloads, files.clone(), dest, seed, cancel, &|done, _| {
        progress(done, total)
    })
    .await?;
    let index = files.last().expect("the asset index is listed last");
    let objects = objects(&read_json(&dest.join(&index.path))?, hosts.objects)?;
    let assets: u64 = objects.iter().filter_map(|f| f.size).sum();
    run(downloads, objects, dest, seed, cancel, &|done, _| {
        progress(total + done, total + assets)
    })
    .await
}

/// Seeds then downloads `files` into `dest`.
async fn run(
    downloads: &DownloadManager,
    files: Vec<GameFile>,
    dest: &Path,
    seed: Option<&Path>,
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> Result<()> {
    if let Some(seed) = seed.filter(|s| *s != dest) {
        let (files, dest, seed) = (files.clone(), dest.to_owned(), seed.to_owned());
        tokio::task::spawn_blocking(move || seed_from(&files, &dest, &seed))
            .await
            .map_err(|e| InstanceError::Prepare(e.to_string()))?;
    }
    let reqs = files
        .iter()
        .map(|f| {
            let mut req = DownloadRequest::new(&f.url, dest.join(&f.path))
                .checksum(Checksum::sha1(&f.sha1).ok_or_else(|| bad("bad sha1"))?)
                .no_cache();
            if let Some(size) = f.size {
                req = req.size(size);
            }
            Ok(req)
        })
        .collect::<Result<Vec<_>>>()?;
    fetch(downloads, reqs, cancel, progress).await
}

/// Links (or copies) every file the seed folder has with the listed size, unless already there.
fn seed_from(files: &[GameFile], dest: &Path, seed: &Path) {
    for f in files {
        let to = dest.join(&f.path);
        let from = seed.join(&f.path);
        let same_size = |p: &Path| {
            std::fs::metadata(p).is_ok_and(|m| m.is_file() && f.size.is_none_or(|s| s == m.len()))
        };
        if same_size(&to) || !same_size(&from) {
            continue;
        }
        if let Some(parent) = to.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::remove_file(&to);
        if std::fs::hard_link(&from, &to).is_err() {
            let _ = std::fs::copy(&from, &to);
        }
    }
}

/// Runs the requests; on the first failure (or `cancel`) the others are cancelled.
async fn fetch(
    downloads: &DownloadManager,
    reqs: Vec<DownloadRequest>,
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> Result<()> {
    let total: u64 = reqs.iter().filter_map(|r| r.size).sum();
    let sizes: Vec<u64> = reqs.iter().map(|r| r.size.unwrap_or(0)).collect();
    let handles: Vec<_> = reqs.into_iter().map(|r| downloads.enqueue(r)).collect();
    let tokens: Vec<CancellationToken> = handles.iter().map(|h| h.token()).collect();
    let stop = || tokens.iter().for_each(CancellationToken::cancel);
    let mut done = 0u64;
    let mut first_error = None;
    for (handle, size) in handles.into_iter().zip(sizes) {
        let got = tokio::select! {
            r = handle.wait() => r,
            _ = cancel.cancelled() => {
                stop();
                return Err(DownloadError::Cancelled.into());
            }
        };
        match got {
            Ok(_) => {
                done += size;
                progress(done, total);
            }
            Err(e) if first_error.is_none() => {
                stop();
                first_error = Some(e);
            }
            Err(_) => {}
        }
    }
    match first_error {
        Some(e) => Err(e.into()),
        None => Ok(()),
    }
}

fn copy_file(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| InstanceError::io(parent, e))?;
    }
    std::fs::copy(from, to).map_err(|e| InstanceError::io(to, e))?;
    Ok(())
}

fn read_json(path: &Path) -> Result<Value> {
    let text = std::fs::read_to_string(path).map_err(|e| InstanceError::io(path, e))?;
    serde_json::from_str(&text).map_err(|e| bad(&format!("{}: {e}", path.display())))
}

fn bad(message: &str) -> InstanceError {
    InstanceError::Prepare(format!("game files: {message}"))
}

fn text(v: &Value, key: &str) -> Result<String> {
    v[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| bad(&format!("the version file has no {key}")))
}

fn sha1_of(v: &Value) -> Result<String> {
    v["sha1"]
        .as_str()
        .filter(|h| is_sha1(h))
        .map(str::to_owned)
        .ok_or_else(|| bad("a file in the version file has no valid sha1"))
}

fn is_sha1(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A path from the version file, refused if it could leave the folder.
fn relative(path: &str) -> Result<PathBuf> {
    let safe = !path.is_empty()
        && !path.contains('\\')
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)));
    safe.then(|| PathBuf::from(path))
        .ok_or_else(|| bad(&format!("unsafe path {path:?}")))
}

/// Whether a library's rules let `os` use it: the last rule that matches decides.
fn allowed(library: &Value, os: &str) -> bool {
    let Some(rules) = library["rules"].as_array() else {
        return true;
    };
    rules
        .iter()
        .rfind(|rule| rule["os"]["name"].as_str().is_none_or(|name| name == os))
        .is_some_and(|rule| rule["action"] == "allow")
}

/// The classifier holding a library's natives for `os` (versions before 1.19).
fn native_classifier(library: &Value, os: &str) -> Option<String> {
    let arch = if cfg!(target_pointer_width = "64") {
        "64"
    } else {
        "32"
    };
    library["natives"][os]
        .as_str()
        .map(|key| key.replace("${arch}", arch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn pins() -> Pins {
        Pins {
            version: "9.9".into(),
            client_sha1: A.into(),
            manifests: vec![B.into()],
        }
    }

    #[test]
    fn reads_a_build_lock() {
        let lock = json!({"minecraft": {
            "version": "26.3", "manifest_sha1": A, "accepted_manifest_sha1": [B, "nope"],
            "client_sha1": B
        }});
        let p = Pins::parse(&lock).unwrap();
        assert_eq!(p.version, "26.3");
        assert_eq!(p.manifests, vec![A.to_string(), B.to_string()]);
        assert!(
            Pins::parse(
                &json!({"minecraft": {"version": "../x", "manifest_sha1": A, "client_sha1": B}})
            )
            .is_none()
        );
        assert!(Pins::parse(&json!({})).is_none());
    }

    fn manifest() -> Value {
        json!({
            "downloads": {"client": {"url": "https://x/client.jar", "sha1": B, "size": 10}},
            "libraries": [
                {"downloads": {"artifact": {"url": "https://x/a.jar", "sha1": A, "size": 1, "path": "a/a.jar"}}},
                {"downloads": {"artifact": {"url": "https://x/w.jar", "sha1": A, "size": 1, "path": "w/w.jar"}},
                 "rules": [{"action": "allow", "os": {"name": "windows"}}]},
                {"downloads": {"artifact": {"url": "https://x/n.jar", "sha1": A, "size": 1, "path": "n/n.jar"}},
                 "rules": [{"action": "allow"}, {"action": "disallow", "os": {"name": "osx"}}]}
            ],
            "assetIndex": {"id": "34", "url": "https://x/34.json", "sha1": A}
        })
    }

    #[test]
    fn lists_this_system_files_with_the_pinned_client() {
        let files = game_files(&manifest(), &pins(), "linux").unwrap();
        let paths: Vec<_> = files.iter().map(|f| f.path.clone()).collect();
        assert_eq!(
            paths,
            vec![
                PathBuf::from("versions/9.9/9.9.jar"),
                PathBuf::from("libraries/a/a.jar"),
                PathBuf::from("libraries/n/n.jar"),
                PathBuf::from("assets/indexes/34.json"),
            ]
        );
        // The client's sha1 is the pin, not the version file's.
        assert_eq!(files[0].sha1, A);
        assert_eq!(
            game_files(&manifest(), &pins(), "windows").unwrap().len(),
            5
        );
        assert_eq!(game_files(&manifest(), &pins(), "osx").unwrap().len(), 3);
    }

    #[test]
    fn refuses_paths_that_leave_the_folder() {
        let mut m = manifest();
        m["libraries"][0]["downloads"]["artifact"]["path"] = json!("../../evil.jar");
        assert!(game_files(&m, &pins(), "linux").is_err());
        m["libraries"][0]["downloads"]["artifact"]["path"] = json!("/abs.jar");
        assert!(game_files(&m, &pins(), "linux").is_err());
    }

    #[test]
    fn asset_objects_once_each_and_checked() {
        let index = json!({"objects": {
            "a.ogg": {"hash": A, "size": 1}, "b.ogg": {"hash": A, "size": 1}, "c.ogg": {"hash": B, "size": 2}
        }});
        let o = objects(&index, MOJANG.objects).unwrap();
        assert_eq!(o.len(), 2);
        assert_eq!(o[0].path, Path::new("assets/objects/aa").join(A));
        assert_eq!(
            o[0].url,
            format!("https://resources.download.minecraft.net/aa/{A}")
        );
        assert!(objects(&json!({"objects": {"x": {"hash": "../x"}}}), MOJANG.objects).is_err());
    }

    #[test]
    fn completeness_needs_every_file_by_size() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let manifest_text = serde_json::to_vec(&manifest()).unwrap();
        let sha1 = {
            let p = root.join("m.json");
            std::fs::write(&p, &manifest_text).unwrap();
            hash_file(&p, HashAlgo::Sha1).unwrap()
        };
        let pins = Pins {
            manifests: vec![sha1],
            ..pins()
        };
        let put = |rel: &str, len: usize| {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, vec![0u8; len]).unwrap();
        };
        assert!(!is_complete(root, &pins));
        std::fs::create_dir_all(root.join("versions/9.9")).unwrap();
        std::fs::write(root.join("versions/9.9/9.9.json"), &manifest_text).unwrap();
        put("versions/9.9/9.9.jar", 10);
        for lib in game_files(&manifest(), &pins, os_name()).unwrap() {
            if lib.path.starts_with("libraries") {
                put(lib.path.to_str().unwrap(), 1);
            }
        }
        std::fs::create_dir_all(root.join("assets/indexes")).unwrap();
        let index = json!({"objects": {"a": {"hash": A, "size": 2}}});
        std::fs::write(root.join("assets/indexes/34.json"), index.to_string()).unwrap();
        assert!(!is_complete(root, &pins), "the asset object is missing");
        put(&format!("assets/objects/aa/{A}"), 1);
        assert!(!is_complete(root, &pins), "wrong size");
        put(&format!("assets/objects/aa/{A}"), 2);
        assert!(is_complete(root, &pins));
        // A version JSON that is not an accepted one is not complete.
        std::fs::write(root.join("versions/9.9/9.9.json"), b"{}").unwrap();
        assert!(!is_complete(root, &pins));
    }

    #[test]
    fn seeds_from_the_player_folder_by_size() {
        let seed = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();
        let file = |rel: &str, size| GameFile {
            url: String::new(),
            sha1: A.into(),
            size: Some(size),
            path: PathBuf::from(rel),
        };
        std::fs::create_dir_all(seed.path().join("libraries/a")).unwrap();
        std::fs::write(seed.path().join("libraries/a/a.jar"), b"abc").unwrap();
        std::fs::write(seed.path().join("libraries/a/b.jar"), b"abc").unwrap();
        seed_from(
            &[file("libraries/a/a.jar", 3), file("libraries/a/b.jar", 9)],
            dest.path(),
            seed.path(),
        );
        assert_eq!(
            std::fs::read(dest.path().join("libraries/a/a.jar")).unwrap(),
            b"abc"
        );
        assert!(
            !dest.path().join("libraries/a/b.jar").exists(),
            "wrong size: downloaded"
        );
    }

    fn sha1_of_bytes(bytes: &[u8]) -> String {
        let f = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(f.path(), bytes).unwrap();
        hash_file(f.path(), HashAlgo::Sha1).unwrap()
    }

    /// A fake Mojang serving a version: its JSON, client (`client` bytes), one library, the asset
    /// index and two objects. Returns the server, the pins and the version JSON's bytes.
    async fn fake_mojang(client: &[u8]) -> (wiremock::MockServer, Pins, Vec<u8>) {
        use wiremock::matchers::path;
        use wiremock::{Mock, ResponseTemplate};
        let server = wiremock::MockServer::start().await;
        let (o1, o2) = (b"sound one".to_vec(), b"sound two!".to_vec());
        let index = json!({"objects": {
            "minecraft/sounds/a.ogg": {"hash": sha1_of_bytes(&o1), "size": o1.len()},
            "minecraft/sounds/b.ogg": {"hash": sha1_of_bytes(&o2), "size": o2.len()}
        }})
        .to_string()
        .into_bytes();
        let lib = b"library bytes".to_vec();
        let real_client = b"the client jar".to_vec();
        let url = |p: &str| format!("{}{p}", server.uri());
        let manifest = serde_json::to_vec(&json!({
            "downloads": {"client": {
                "url": url("/client.jar"), "sha1": sha1_of_bytes(&real_client), "size": client.len()
            }},
            "libraries": [{"downloads": {"artifact": {
                "url": url("/lib.jar"), "sha1": sha1_of_bytes(&lib), "size": lib.len(),
                "path": "org/x/lib.jar"
            }}}],
            "assetIndex": {
                "id": "34", "url": url("/34.json"), "sha1": sha1_of_bytes(&index), "size": index.len()
            }
        }))
        .unwrap();
        let manifest_sha1 = sha1_of_bytes(&manifest);
        let object = |o: &[u8]| {
            let h = sha1_of_bytes(o);
            format!("/objects/{}/{h}", &h[..2])
        };
        for (route, body) in [
            (
                format!("/packages/{manifest_sha1}/9.9.json"),
                manifest.clone(),
            ),
            ("/client.jar".to_owned(), client.to_vec()),
            ("/lib.jar".to_owned(), lib),
            ("/34.json".to_owned(), index),
            (object(&o1), o1.clone()),
            (object(&o2), o2.clone()),
        ] {
            Mock::given(path(route))
                .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
                .mount(&server)
                .await;
        }
        let pins = Pins {
            version: "9.9".into(),
            client_sha1: sha1_of_bytes(&real_client),
            manifests: vec![manifest_sha1],
        };
        (server, pins, manifest)
    }

    async fn run_download(
        server: &wiremock::MockServer,
        pins: &Pins,
        dest: &Path,
        seed: Option<&Path>,
    ) -> (Result<()>, Vec<(u64, u64)>) {
        let piston = format!("{}/packages", server.uri());
        let objects = format!("{}/objects", server.uri());
        let hosts = Hosts {
            piston_meta: &piston,
            objects: &objects,
        };
        let cache = tempfile::tempdir().unwrap();
        let downloads = DownloadManager::new(cache.path(), 4, 0).unwrap();
        let seen = std::sync::Mutex::new(Vec::new());
        let got = download_from(
            hosts,
            &VerifiedOwner::for_tests("u", "Owner"),
            pins,
            dest,
            seed,
            &downloads,
            &CancellationToken::new(),
            &|done, total| seen.lock().unwrap().push((done, total)),
        )
        .await;
        (got, seen.into_inner().unwrap())
    }

    #[tokio::test]
    async fn downloads_a_whole_version_for_an_owner() {
        let (server, pins, _) = fake_mojang(b"the client jar").await;
        let dest = tempfile::tempdir().unwrap();
        let (got, seen) = run_download(&server, &pins, dest.path(), None).await;
        got.unwrap();
        assert!(is_complete(dest.path(), &pins));
        assert_eq!(
            std::fs::read(dest.path().join("versions/9.9/9.9.jar")).unwrap(),
            b"the client jar"
        );
        let last = *seen.last().unwrap();
        assert_eq!(last.0, last.1, "progress ends at the total");
        // Nothing is fetched again once complete.
        let before = server.received_requests().await.unwrap().len();
        run_download(&server, &pins, dest.path(), None)
            .await
            .0
            .unwrap();
        assert_eq!(server.received_requests().await.unwrap().len(), before);
    }

    #[tokio::test]
    async fn reuses_the_player_files_and_fetches_the_rest() {
        let (server, pins, manifest) = fake_mojang(b"the client jar").await;
        // The player's launcher has the version JSON and the library, not the client nor sounds.
        let seed = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(seed.path().join("versions/9.9")).unwrap();
        std::fs::write(seed.path().join("versions/9.9/9.9.json"), &manifest).unwrap();
        std::fs::create_dir_all(seed.path().join("libraries/org/x")).unwrap();
        std::fs::write(
            seed.path().join("libraries/org/x/lib.jar"),
            b"library bytes",
        )
        .unwrap();
        assert!(!is_complete(seed.path(), &pins));
        let dest = tempfile::tempdir().unwrap();
        run_download(&server, &pins, dest.path(), Some(seed.path()))
            .await
            .0
            .unwrap();
        assert!(is_complete(dest.path(), &pins));
        let paths: Vec<String> = server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|r| r.url.path().to_owned())
            .collect();
        assert!(
            !paths.iter().any(|p| p.starts_with("/packages")),
            "{paths:?}"
        );
        assert!(!paths.iter().any(|p| p == "/lib.jar"), "{paths:?}");
        assert!(paths.iter().any(|p| p == "/client.jar"));
        // The player's own files are untouched.
        assert_eq!(
            std::fs::read(seed.path().join("libraries/org/x/lib.jar")).unwrap(),
            b"library bytes"
        );
    }

    #[tokio::test]
    async fn refuses_a_client_that_is_not_the_pinned_one() {
        let (server, pins, _) = fake_mojang(b"a swapped client").await;
        let dest = tempfile::tempdir().unwrap();
        let (got, _) = run_download(&server, &pins, dest.path(), None).await;
        assert!(got.is_err());
        assert!(!dest.path().join("versions/9.9/9.9.jar").exists());
        assert!(!is_complete(dest.path(), &pins));
    }

    /// Against Mojang itself, on a computer with an installed build (manual check):
    /// `RUSTIC_REAL_BUILD=<build dir> RUSTIC_REAL_DEST=<folder> [RUSTIC_REAL_SEED=<.minecraft>]
    /// cargo test -p rusticmc-instances real_mojang -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore = "network: downloads from Mojang"]
    async fn real_mojang_download() {
        let var = |k: &str| std::env::var_os(k).map(PathBuf::from);
        let pins = Pins::of_build(&var("RUSTIC_REAL_BUILD").expect("RUSTIC_REAL_BUILD")).unwrap();
        let dest = var("RUSTIC_REAL_DEST").expect("RUSTIC_REAL_DEST");
        let seed = var("RUSTIC_REAL_SEED");
        let cache = tempfile::tempdir().unwrap();
        let downloads = DownloadManager::new(cache.path(), 8, 2).unwrap();
        let started = std::time::Instant::now();
        let last = std::sync::Mutex::new(0u64);
        download(
            &VerifiedOwner::for_tests("manual", "Owner"),
            &pins,
            &dest,
            seed.as_deref(),
            &downloads,
            &CancellationToken::new(),
            &|done, total| {
                let mut l = last.lock().unwrap();
                if done >= *l + 50_000_000 || done == total {
                    *l = done;
                    println!("{} / {} MB", done / 1_000_000, total / 1_000_000);
                }
            },
        )
        .await
        .unwrap();
        println!("done in {:?}", started.elapsed());
        assert!(is_complete(&dest, &pins));
    }
}
