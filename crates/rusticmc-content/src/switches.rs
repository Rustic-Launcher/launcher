//! Turning content on and off the way Horizon reads it (horizon/docs/api/content.md, "Where
//! content goes"). Change these only while the game is stopped: it reads them at start.
//!
//! | Kind | File | Switch |
//! |---|---|---|
//! | Content / native mod | `horizon-mods.txt` | `mod.<id>:true` / `false` |
//! | Resource pack | `options.txt` | `resourcePacks:["vanilla","file/<name>"]` |
//! | Shader pack | `horizon-graphics.txt` | `pickedShaderPack:<name>` (empty: none) |
//! | Data pack | `saves/<world>/datapacks.json` | `{"enabled": [...], "disabled": [...]}` |
use crate::error::Result;
use std::path::Path;

/// Reads a `key:value` file (as `options.txt`): lines in order, the first `:` splits.
pub fn read_kv(path: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            Some((k.to_owned(), v.to_owned()))
        })
        .collect()
}

/// Sets `key` to `value` in a `key:value` file, keeping every other line and its order.
pub fn set_kv(path: &Path, key: &str, value: &str) -> Result<()> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut found = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|l| match l.split_once(':') {
            Some((k, _)) if k == key => {
                found = true;
                format!("{key}:{value}")
            }
            _ => l.to_owned(),
        })
        .collect();
    if !found {
        lines.push(format!("{key}:{value}"));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, lines.join("\n") + "\n")?;
    Ok(())
}

pub fn get_kv(path: &Path, key: &str) -> Option<String> {
    read_kv(path)
        .into_iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
}

/// `horizon-mods.txt`: `mod.<id>:<on>`.
pub fn set_mod(game_dir: &Path, id: &str, on: bool) -> Result<()> {
    set_kv(
        &game_dir.join("horizon-mods.txt"),
        &format!("mod.{id}"),
        if on { "true" } else { "false" },
    )
}

pub fn mod_on(game_dir: &Path, id: &str) -> bool {
    get_kv(&game_dir.join("horizon-mods.txt"), &format!("mod.{id}")).as_deref() == Some("true")
}

/// The resource packs turned on, lowest priority first.
pub fn resource_packs(game_dir: &Path) -> Vec<String> {
    get_kv(&game_dir.join("options.txt"), "resourcePacks")
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_else(|| vec!["vanilla".to_owned()])
}

/// Turns `file/<name>` on (highest priority) or off.
pub fn set_resource_pack(game_dir: &Path, name: &str, on: bool) -> Result<()> {
    let id = format!("file/{name}");
    let mut packs = resource_packs(game_dir);
    packs.retain(|p| p != &id);
    if on {
        packs.push(id);
    }
    set_kv(
        &game_dir.join("options.txt"),
        "resourcePacks",
        &serde_json::to_string(&packs)?,
    )
}

/// Picks shader pack `name` (or none with `None`).
pub fn set_shader_pack(game_dir: &Path, name: Option<&str>) -> Result<()> {
    set_kv(
        &game_dir.join("horizon-graphics.txt"),
        "pickedShaderPack",
        name.unwrap_or(""),
    )
}

pub fn shader_pack(game_dir: &Path) -> Option<String> {
    get_kv(&game_dir.join("horizon-graphics.txt"), "pickedShaderPack").filter(|v| !v.is_empty())
}

/// `datapacks.json` of a world folder: a pack in neither list is on.
pub fn set_data_pack(world_dir: &Path, name: &str, on: bool) -> Result<()> {
    let path = world_dir.join("datapacks.json");
    let mut v: serde_json::Value = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    if !v.is_object() {
        v = serde_json::json!({});
    }
    let id = format!("file/{name}");
    for list in ["enabled", "disabled"] {
        let arr = v[list].as_array().cloned().unwrap_or_default();
        let mut arr: Vec<serde_json::Value> = arr
            .into_iter()
            .filter(|x| x.as_str() != Some(&id))
            .collect();
        if (list == "enabled") == on {
            arr.push(serde_json::Value::String(id.clone()));
        }
        v[list] = serde_json::Value::Array(arr);
    }
    std::fs::create_dir_all(world_dir)?;
    std::fs::write(path, serde_json::to_vec_pretty(&v)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switches_keep_other_lines() {
        let dir = tempfile::tempdir().unwrap();
        let g = dir.path();
        std::fs::write(
            g.join("options.txt"),
            "fov:0.5\nresourcePacks:[\"vanilla\"]\nlang:fr_fr\n",
        )
        .unwrap();
        set_resource_pack(g, "Faithful.zip", true).unwrap();
        let text = std::fs::read_to_string(g.join("options.txt")).unwrap();
        assert!(text.starts_with("fov:0.5\n"));
        assert!(text.contains(r#"resourcePacks:["vanilla","file/Faithful.zip"]"#));
        assert!(text.contains("lang:fr_fr"));
        set_resource_pack(g, "Faithful.zip", false).unwrap();
        assert_eq!(resource_packs(g), vec!["vanilla"]);

        set_mod(g, "coords_hud", true).unwrap();
        assert!(mod_on(g, "coords_hud"));
        set_mod(g, "coords_hud", false).unwrap();
        assert!(!mod_on(g, "coords_hud"));

        set_shader_pack(g, Some("glow.hshader")).unwrap();
        assert_eq!(shader_pack(g).as_deref(), Some("glow.hshader"));
        set_shader_pack(g, None).unwrap();
        assert_eq!(shader_pack(g), None);

        let w = g.join("saves/World");
        set_data_pack(&w, "Trades.zip", false).unwrap();
        let v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(w.join("datapacks.json")).unwrap()).unwrap();
        assert_eq!(v["disabled"][0], "file/Trades.zip");
        set_data_pack(&w, "Trades.zip", true).unwrap();
        let v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(w.join("datapacks.json")).unwrap()).unwrap();
        assert_eq!(v["enabled"][0], "file/Trades.zip");
        assert!(v["disabled"].as_array().unwrap().is_empty());
    }
}
