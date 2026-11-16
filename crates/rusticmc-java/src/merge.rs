//! `inheritsFrom`: a loader's profile (Fabric, Quilt, Forge, NeoForge) names the version it builds
//! on and lists only what it adds. Merged as the official launcher does: the child's main class,
//! type and id win; its libraries come first and replace the parent's copy of the same library
//! (`group:artifact[:classifier]`); argument lists are the parent's then the child's; legacy
//! `minecraftArguments`, the asset index, downloads, Java version and logging are the child's when
//! it has them.

use crate::maven::Coordinate;
use crate::version::{Arguments, VersionJson};
use crate::{JavaError, Result};
use std::collections::HashSet;

/// `child` over `parent`.
pub fn merge(parent: VersionJson, child: VersionJson) -> VersionJson {
    let child_ids: HashSet<String> = child
        .libraries
        .iter()
        .filter_map(|l| Coordinate::parse(&l.name).map(|c| c.identity()))
        .collect();
    let mut libraries = child.libraries;
    libraries.extend(
        parent.libraries.into_iter().filter(|l| {
            Coordinate::parse(&l.name).is_none_or(|c| !child_ids.contains(&c.identity()))
        }),
    );
    let arguments = match (parent.arguments, child.arguments) {
        (Some(p), Some(c)) => Some(Arguments {
            game: p.game.into_iter().chain(c.game).collect(),
            jvm: p.jvm.into_iter().chain(c.jvm).collect(),
            default_user_jvm: if c.default_user_jvm.is_empty() {
                p.default_user_jvm
            } else {
                c.default_user_jvm
            },
        }),
        (p, c) => c.or(p),
    };
    VersionJson {
        id: child.id,
        inherits_from: None,
        // The client jar stays the parent's (or the one it names).
        jar: child.jar.or(parent.jar).or_else(|| Some(parent.id.clone())),
        kind: child.kind.or(parent.kind),
        main_class: child.main_class.or(parent.main_class),
        minecraft_arguments: child.minecraft_arguments.or(parent.minecraft_arguments),
        arguments,
        libraries,
        asset_index: child.asset_index.or(parent.asset_index),
        assets: child.assets.or(parent.assets),
        downloads: child.downloads.or(parent.downloads),
        java_version: child.java_version.or(parent.java_version),
        logging: child.logging.or(parent.logging),
        release_time: child.release_time.or(parent.release_time),
        time: child.time.or(parent.time),
    }
}

/// Follows `inheritsFrom` from `id` with `load` (reads a version file by id), merging every step.
/// A chain longer than 8 or a loop is refused.
pub fn resolve(id: &str, mut load: impl FnMut(&str) -> Result<VersionJson>) -> Result<VersionJson> {
    let mut chain = vec![load(id)?];
    let mut seen = HashSet::from([id.to_owned()]);
    while let Some(parent) = chain.last().and_then(|v| v.inherits_from.clone()) {
        if !seen.insert(parent.clone()) || chain.len() >= 8 {
            return Err(JavaError::bad(format!(
                "{id}: inheritsFrom loops or is too deep"
            )));
        }
        chain.push(load(&parent)?);
    }
    let mut merged = chain.pop().expect("one at least");
    while let Some(child) = chain.pop() {
        merged = merge(merged, child);
    }
    if merged.jar.is_none() {
        merged.jar = Some(merged.id.clone());
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::Argument;
    use std::collections::HashMap;

    fn v(json: serde_json::Value) -> VersionJson {
        serde_json::from_value(json).unwrap()
    }

    fn vanilla() -> VersionJson {
        v(serde_json::json!({
            "id": "1.21.1", "type": "release", "mainClass": "net.minecraft.client.main.Main",
            "arguments": {"game": ["--username", "${auth_player_name}"], "jvm": ["-cp", "${classpath}"]},
            "libraries": [
                {"name": "org.ow2.asm:asm:9.3", "downloads": {"artifact": {"path": "org/ow2/asm/asm/9.3/asm-9.3.jar", "url": "https://libraries.minecraft.net/org/ow2/asm/asm/9.3/asm-9.3.jar", "sha1": "a", "size": 1}}},
                {"name": "org.lwjgl:lwjgl:3.3.3", "downloads": {"artifact": {"path": "p", "url": "https://libraries.minecraft.net/p", "sha1": "b", "size": 1}}}
            ],
            "assetIndex": {"id": "17", "url": "https://piston-meta.mojang.com/17.json"},
            "javaVersion": {"component": "java-runtime-delta", "majorVersion": 21}
        }))
    }

    /// The shape of Fabric's profile JSON (written for this test).
    fn fabric() -> VersionJson {
        v(serde_json::json!({
            "id": "fabric-loader-0.17.2-1.21.1", "inheritsFrom": "1.21.1", "type": "release",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "arguments": {"game": [], "jvm": ["-DFabricMcEmu= net.minecraft.client.main.Main "]},
            "libraries": [
                {"name": "org.ow2.asm:asm:9.8", "url": "https://maven.fabricmc.net/", "sha1": "c", "size": 2},
                {"name": "net.fabricmc:fabric-loader:0.17.2", "url": "https://maven.fabricmc.net/"}
            ]
        }))
    }

    #[test]
    fn a_loader_profile_merges_over_vanilla() {
        let mut files = HashMap::from([
            ("1.21.1".to_string(), vanilla()),
            ("fabric-loader-0.17.2-1.21.1".to_string(), fabric()),
        ]);
        let m = resolve("fabric-loader-0.17.2-1.21.1", |id| {
            files
                .remove(id)
                .ok_or_else(|| JavaError::UnknownVersion(id.into()))
        })
        .unwrap();
        assert_eq!(m.id, "fabric-loader-0.17.2-1.21.1");
        assert_eq!(m.jar.as_deref(), Some("1.21.1"));
        assert_eq!(
            m.main_class.as_deref(),
            Some("net.fabricmc.loader.impl.launch.knot.KnotClient")
        );
        let names: Vec<_> = m.libraries.iter().map(|l| l.name.as_str()).collect();
        // Fabric's newer ASM replaces the game's; the rest of the game's stays.
        assert_eq!(
            names,
            vec![
                "org.ow2.asm:asm:9.8",
                "net.fabricmc:fabric-loader:0.17.2",
                "org.lwjgl:lwjgl:3.3.3"
            ]
        );
        let args = m.arguments.unwrap();
        assert_eq!(args.jvm.len(), 3);
        assert_eq!(args.game[0], Argument::Plain("--username".into()));
        assert_eq!(m.asset_index.unwrap().id, "17");
        assert_eq!(m.java_version.unwrap().major_version, Some(21));
        assert!(m.inherits_from.is_none());
    }

    #[test]
    fn same_library_twice_in_one_file_stays() {
        // Old vanilla files list one library per OS under the same name; only a child replaces.
        let mut p = vanilla();
        p.libraries.push(p.libraries[1].clone());
        let m = merge(p, v(serde_json::json!({"id": "c", "libraries": []})));
        assert_eq!(m.libraries.len(), 3);
    }

    #[test]
    fn legacy_arguments_and_loops() {
        let parent =
            v(serde_json::json!({"id": "1.12.2", "minecraftArguments": "--a b", "libraries": []}));
        let child = v(
            serde_json::json!({"id": "f", "inheritsFrom": "1.12.2", "minecraftArguments": "--a b --tweakClass x", "libraries": []}),
        );
        let m = merge(parent, child);
        assert_eq!(
            m.minecraft_arguments.as_deref(),
            Some("--a b --tweakClass x")
        );
        let looped = resolve("a", |id| {
            Ok(v(
                serde_json::json!({"id": id, "inheritsFrom": if id == "a" {"b"} else {"a"}, "libraries": []}),
            ))
        });
        assert!(looped.is_err());
    }
}
