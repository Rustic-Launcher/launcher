//! The companion's settings: `config/rustic-companion.json` in the instance (Horizon's shared
//! config folder, `Host::config_dir`), overridden by the environment the launcher starts the game
//! with. No secret is ever stored here: the session comes from a one-time launch ticket.
use serde::{Deserialize, Serialize};

pub const FILE: &str = "rustic-companion.json";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CompanionConfig {
    /// Rustic Cloud API base (`RUSTIC_CLOUD_API`).
    pub api_base: String,
    /// The cosmetics CDN base (`RUSTIC_CDN`).
    pub cdn_base: String,
    /// Vanilla key names (`key.keyboard.o`), as Horizon's `key` hook gives them.
    pub overlay_key: String,
    pub emote_key: String,
    /// Draw other players' Rustic cosmetics.
    pub show_others: bool,
    /// Show friend/invite/message notifications.
    pub notifications: bool,
    /// Hide other players' trails and auras.
    pub hide_effects: bool,
    /// Draw the Rustic main menu over the title screen (false: the game's own title).
    pub title_menu: bool,
}

impl Default for CompanionConfig {
    fn default() -> Self {
        Self {
            // The production hosts (launcher AGENTS.md, user decision 2026-11-10).
            api_base: "https://api.rustic-launcher.com".into(),
            cdn_base: "https://cdn.rustic-launcher.com".into(),
            overlay_key: "key.keyboard.o".into(),
            emote_key: "key.keyboard.g".into(),
            show_others: true,
            notifications: true,
            hide_effects: false,
            title_menu: true,
        }
    }
}

/// An endpoint the environment may set: https, or plain http on this computer only (a local
/// development cloud).
fn allowed_base(v: &str) -> bool {
    v.starts_with("https://")
        || ["http://127.0.0.1:", "http://localhost:", "http://[::1]:"]
            .iter()
            .any(|p| v.starts_with(p))
}

impl CompanionConfig {
    /// The file's settings (defaults for what it lacks), then the environment's endpoints.
    pub fn load(file: Option<&str>, env: impl Fn(&str) -> Option<String>) -> Self {
        let mut c: CompanionConfig = file
            .and_then(|t| serde_json::from_str(t).ok())
            .unwrap_or_default();
        if let Some(a) = env("RUSTIC_CLOUD_API").filter(|v| allowed_base(v)) {
            c.api_base = a;
        }
        if let Some(a) = env("RUSTIC_CDN").filter(|v| allowed_base(v)) {
            c.cdn_base = a;
        }
        c.api_base = c.api_base.trim_end_matches('/').to_owned();
        c.cdn_base = c.cdn_base.trim_end_matches('/').to_owned();
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_then_environment() {
        let c = CompanionConfig::load(Some(r#"{"overlay_key": "key.keyboard.p"}"#), |k| {
            (k == "RUSTIC_CLOUD_API").then(|| "https://api.example.org/".to_owned())
        });
        assert_eq!(c.overlay_key, "key.keyboard.p");
        assert_eq!(c.emote_key, "key.keyboard.g");
        assert_eq!(c.api_base, "https://api.example.org");
        // Plain http from the environment is ignored, but for this computer's own.
        let c = CompanionConfig::load(None, |_| Some("http://evil".into()));
        assert!(c.api_base.starts_with("https://"));
        let c = CompanionConfig::load(None, |_| Some("http://localhost.evil.example:80".into()));
        assert!(c.api_base.starts_with("https://"));
        let c = CompanionConfig::load(None, |_| Some("http://127.0.0.1:8080".into()));
        assert_eq!(c.api_base, "http://127.0.0.1:8080");
    }
}
