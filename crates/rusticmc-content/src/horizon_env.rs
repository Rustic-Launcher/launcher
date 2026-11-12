//! What the instance's Horizon build loads: `horizon-client --launch-api`'s `content` block
//! (horizon/docs/api/launch-reference.md, docs/api/content.md). Every verdict is made against
//! this, never against a guess.
use serde::{Deserialize, Serialize};

/// The game version Horizon plays.
pub const GAME: &str = "26.3";
/// 26.3's resource pack format `[major, minor]` (`horizon_core::version::RESOURCE_PACK_FORMAT`).
pub const RESOURCE_PACK_FORMAT: (i64, i64) = (97, 1);
/// 26.3's data pack format (`horizon_core::version::DATA_PACK_FORMAT`).
pub const DATA_PACK_FORMAT: (i64, i64) = (121, 0);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HorizonEnv {
    /// `minecraft` of the launch API (`26.3`).
    pub game: String,
    /// The build's version (`horizon`).
    pub horizon: String,
    /// The exact commit the instance plays, when the launcher knows it (a native build's
    /// `native-mods.json`, or the release's): compared with native mods' pins and `broken_on`.
    pub commit: Option<String>,
    /// Content mod API lines this build loads (`content.mod.api`).
    pub mod_api: Vec<String>,
    /// The native hook points version (`content.native_mod.hooks`), and whether native mods
    /// need the game rebuilt (`rebuild`).
    pub native_hooks: Option<u32>,
    pub native_rebuild: bool,
    pub shader_api: Vec<String>,
    pub shader_schema: u32,
    pub resourcepack_format: (i64, i64),
    pub datapack_format: (i64, i64),
    pub modpack_format: u32,
    pub index_schema: u32,
}

impl Default for HorizonEnv {
    /// Horizon at the pinned commit (a84f637d, `NATIVE_HOOKS_API` 415), as its docs list it.
    fn default() -> Self {
        Self {
            game: GAME.into(),
            horizon: String::new(),
            commit: None,
            mod_api: vec!["0.2".into(), "0.3".into()],
            native_hooks: Some(415),
            native_rebuild: true,
            shader_api: vec!["0.1".into()],
            shader_schema: 1,
            resourcepack_format: RESOURCE_PACK_FORMAT,
            datapack_format: DATA_PACK_FORMAT,
            modpack_format: 1,
            index_schema: 1,
        }
    }
}

fn pair(v: &serde_json::Value) -> Option<(i64, i64)> {
    let a = v.as_array()?;
    Some((
        a.first()?.as_i64()?,
        a.get(1).and_then(|m| m.as_i64()).unwrap_or(0),
    ))
}

fn strings(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s.as_str().map(str::to_owned))
        .collect()
}

impl HorizonEnv {
    /// Reads `horizon-client --launch-api`'s JSON. Fields it lacks keep the defaults.
    pub fn from_launch_api(json: &str) -> crate::Result<HorizonEnv> {
        let v: serde_json::Value = serde_json::from_str(json)?;
        let mut env = HorizonEnv::default();
        if let Some(g) = v["minecraft"].as_str() {
            env.game = g.into();
        }
        if let Some(h) = v["horizon"].as_str() {
            env.horizon = h.into();
        }
        let c = &v["content"];
        if c.is_object() {
            let api = strings(&c["mod"]["api"]);
            if !api.is_empty() {
                env.mod_api = api;
            }
            let hooks = &c["native_mod"]["hooks"];
            env.native_hooks = hooks
                .as_u64()
                .or_else(|| hooks.as_str().and_then(|s| s.parse().ok()))
                .map(|h| h as u32);
            env.native_rebuild = c["native_mod"]["rebuild"].as_bool().unwrap_or(true);
            let shader = strings(&c["shader"]["api"]);
            if !shader.is_empty() {
                env.shader_api = shader;
            }
            if let Some(s) = c["shader"]["schema"].as_u64() {
                env.shader_schema = s as u32;
            }
            if let Some(f) = pair(&c["resourcepack"]["format"]) {
                env.resourcepack_format = f;
            }
            if let Some(f) = pair(&c["datapack"]["format"]) {
                env.datapack_format = f;
            }
            if let Some(f) = c["modpack"]["format"].as_u64() {
                env.modpack_format = f as u32;
            }
            if let Some(s) = c["index"]["schema"].as_u64() {
                env.index_schema = s as u32;
            }
        }
        Ok(env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_documented_launch_api() {
        let json = r#"{ "content": { "datapack": { "format": [121, 0] }, "index": { "schema": 1 },
          "mod": { "api": ["0.2", "0.3"] }, "modpack": { "format": 1 },
          "native_mod": { "hooks": 415, "rebuild": true }, "resourcepack": { "format": [97, 1] },
          "shader": { "api": ["0.1"], "schema": 1 } }, "horizon": "nightly-11-09",
          "launch_api": "0.1.0", "minecraft": "26.3" }"#;
        let env = HorizonEnv::from_launch_api(json).unwrap();
        assert_eq!(env.native_hooks, Some(415));
        assert_eq!(env.resourcepack_format, (97, 1));
        assert_eq!(env.mod_api, vec!["0.2", "0.3"]);
        assert_eq!(env.horizon, "nightly-11-09");
    }
}
