//! Rustic Server `.rplugin` validation, dependency resolution and install.
mod common;
use common::zip;
use rusticmc_content::ContentError;
use rusticmc_content::plugins::*;
use std::path::Path;

const WASM: &[u8] = b"\0asm\x01\0\0\0";

fn manifest(id: &str, version: &str, deps: &str) -> String {
    format!(
        "[plugin]\nid = \"{id}\"\nname = \"{id}\"\nversion = \"{version}\"\napi = \"0.1\"\nauthors = [\"Rustic\"]\nlicense = \"MIT\"\nentry = \"plugin.wasm\"\n{deps}"
    )
}

fn rplugin(dir: &Path, name: &str, manifest: &str, entry: &[u8]) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(
        &p,
        zip(&[
            ("rustic.plugin.toml", manifest.as_bytes()),
            ("plugin.wasm", entry),
            ("config.toml", b"greeting = \"hi\"\n"),
            ("resources/lang/en.json", b"{}"),
        ]),
    )
    .unwrap();
    p
}

fn rustic_server(dir: &Path) {
    std::fs::write(
        dir.join("server.toml"),
        "[engine]\npath = \"C:/rustic/bin/rustic-server.exe\"\n",
    )
    .unwrap();
}

#[test]
fn validates_the_contract() {
    let d = tempfile::tempdir().unwrap();
    let ok = read_rplugin(&rplugin(
        d.path(),
        "ok.rplugin",
        &manifest(
            "essentials_lite",
            "1.2.0",
            "[dependencies]\nrequired = { perms = \"^0.3\" }\n[capabilities]\nnetwork = [\"api.example.com\"]\nfilesystem = \"data-folder\"\n",
        ),
        WASM,
    ))
    .unwrap();
    assert_eq!(ok.manifest.plugin.id, "essentials_lite");
    assert_eq!(ok.version.minor, 2);
    assert_eq!(ok.manifest.capabilities.filesystem, Filesystem::DataFolder);

    let cases = [
        (
            manifest("x", "1.0.0", "").replace("0.1", "0.2"),
            WASM,
            "doesn't run",
        ),
        (manifest("rustic", "1.0.0", ""), WASM, "id"),
        (
            manifest(
                "x",
                "1.0.0",
                "[capabilities]
network = true
",
            ),
            WASM,
            "list the domains",
        ),
        (
            manifest(
                "x",
                "1.0.0",
                "[dependencies]
required = { y = \"*\" }
load_before = [\"y\"]
",
            ),
            WASM,
            "load_before",
        ),
        (manifest("x", "one", ""), WASM, "not semver"),
        (manifest("X Bad", "1.0.0", ""), WASM, "id"),
        (
            manifest("x", "1.0.0", ""),
            b"MZ\0\0".as_slice(),
            "WebAssembly",
        ),
        (
            manifest(
                "x",
                "1.0.0",
                "[dependencies]\nrequired = { y = \"not a req\" }\n",
            ),
            WASM,
            "dependency y",
        ),
        (
            manifest(
                "x",
                "1.0.0",
                "[capabilities]\nnetwork = [\"not a host!\"]\n",
            ),
            WASM,
            "network domain",
        ),
        (
            manifest("x", "1.0.0", "[plugin.extra]\nfoo = 1\n"),
            WASM,
            "rustic.plugin.toml",
        ),
    ];
    for (i, (m, entry, want)) in cases.iter().enumerate() {
        let err = read_rplugin(&rplugin(d.path(), &format!("{i}.rplugin"), m, entry)).unwrap_err();
        assert!(err.to_string().contains(want), "{i}: {err}");
    }
}

#[test]
fn resolves_load_order_and_finds_problems() {
    let d = tempfile::tempdir().unwrap();
    let read = |name: &str, m: String| {
        read_rplugin(&rplugin(d.path(), name, &m, WASM))
            .unwrap()
            .manifest
    };
    let perms = read("perms", manifest("perms", "0.3.1", ""));
    let chat = read(
        "chat",
        manifest(
            "chat_format",
            "1.0.0",
            "[dependencies]\noptional = { perms = \">=0.3\" }\nload_before = [\"menus\"]\n",
        ),
    );
    let menus = read("menus", manifest("menus", "2.0.0", ""));
    let ess = read(
        "ess",
        manifest(
            "essentials_lite",
            "1.0.0",
            "[dependencies]\nrequired = { perms = \"^0.3\" }\n",
        ),
    );
    let r = resolve(&[ess.clone(), menus.clone(), chat.clone(), perms.clone()]);
    assert!(r.refused.is_empty(), "{r:?}");
    let order = r.order;
    let pos = |id: &str| order.iter().position(|o| o == id).unwrap();
    assert!(pos("perms") < pos("essentials_lite"));
    assert!(pos("perms") < pos("chat_format"));
    assert!(pos("chat_format") < pos("menus"));

    // Missing, wrong version (refused); a mismatched optional one only warns.
    let r = resolve(std::slice::from_ref(&ess));
    assert!(r.refused[0].1.contains("not installed"));
    let old_perms = read("op", manifest("perms", "0.2.0", ""));
    let r = resolve(&[ess.clone(), old_perms.clone(), chat.clone()]);
    assert_eq!(r.refused.len(), 1);
    assert_eq!(r.refused[0].0, "essentials_lite");
    assert!(r.warnings.iter().any(|w| w.contains("optional dependency")));
    // Transitive refusal and required cycles.
    let a = read(
        "a",
        manifest(
            "a",
            "1.0.0",
            "[dependencies]
required = { b = \"*\" }
",
        ),
    );
    let b = read(
        "b",
        manifest(
            "b",
            "1.0.0",
            "[dependencies]
required = { a = \"*\" }
",
        ),
    );
    let c = read(
        "c",
        manifest(
            "c",
            "1.0.0",
            "[dependencies]
required = { a = \"*\" }
",
        ),
    );
    let r = resolve(&[a, b, c]);
    assert_eq!(r.refused.len(), 3);
    assert!(
        r.refused
            .iter()
            .any(|(_, why)| why.contains("cycle between a, b"))
    );
    // Duplicate ids: none loaded.
    let r = resolve(&[perms.clone(), perms.clone(), menus.clone()]);
    assert_eq!(r.order, vec!["menus"]);
    assert!(r.refused[0].1.contains("two packages"));
    // A soft cycle is broken with a warning.
    let x = read(
        "x",
        manifest(
            "x",
            "1.0.0",
            "[dependencies]
load_before = [\"y\"]
",
        ),
    );
    let y = read(
        "y",
        manifest(
            "y",
            "1.0.0",
            "[dependencies]
load_before = [\"x\"]
",
        ),
    );
    let r = resolve(&[x, y]);
    assert_eq!(r.order.len(), 2);
    assert!(r.warnings[0].contains("cycle"));
}

#[test]
fn installs_only_into_a_rustic_server() {
    let src = tempfile::tempdir().unwrap();
    let server = tempfile::tempdir().unwrap();
    let perms = rplugin(
        src.path(),
        "perms.rplugin",
        &manifest("perms", "0.3.0", ""),
        WASM,
    );
    let ess = rplugin(
        src.path(),
        "ess.rplugin",
        &manifest(
            "essentials_lite",
            "1.0.0",
            "[dependencies]\nrequired = { perms = \"^0.3\" }\n",
        ),
        WASM,
    );
    // Vanilla Horizon (no engine path): refused.
    assert!(matches!(
        install(server.path(), &perms),
        Err(ContentError::Incompatible(_))
    ));
    rustic_server(server.path());
    assert!(ServerInfo::detect(server.path()).rustic);
    // Its dependency isn't there yet.
    assert!(matches!(
        install(server.path(), &ess),
        Err(ContentError::Dependency(_))
    ));
    install(server.path(), &perms).unwrap();
    let done = install(server.path(), &ess).unwrap();
    assert_eq!(done.load_order, vec!["perms", "essentials_lite"]);
    assert!(
        server
            .path()
            .join("plugins/essentials_lite.rplugin")
            .exists()
    );
    // A newer perms replaces the old one.
    let perms2 = rplugin(
        src.path(),
        "perms2.rplugin",
        &manifest("perms", "0.3.5", ""),
        WASM,
    );
    assert_eq!(
        install(server.path(), &perms2).unwrap().replaced.as_deref(),
        Some("0.3.0")
    );
    // perms is needed: not removable before essentials_lite.
    assert!(uninstall(server.path(), "perms").is_err());
    uninstall(server.path(), "essentials_lite").unwrap();
    uninstall(server.path(), "perms").unwrap();
    assert!(installed(server.path()).unwrap().is_empty());
}
