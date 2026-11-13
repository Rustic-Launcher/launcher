//! Other players' skins, for their heads in lists (a server's players, the overview): the skin URL
//! of a game profile, from Mojang's session service (`/session/minecraft/profile/<uuid>`, its
//! base64 `textures` property), cached an hour (and a miss ten minutes) so a list asks once.
//! Only `textures.minecraft.net` URLs are returned, always as https.

use base64::Engine;
use rusticmc_launcher_core::reqwest;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const PROFILE: &str = "https://sessionserver.mojang.com/session/minecraft/profile";
const HIT_TTL: Duration = Duration::from_secs(3600);
const MISS_TTL: Duration = Duration::from_secs(600);

#[derive(Default)]
pub struct SkinCache {
    entries: Mutex<HashMap<String, (Option<String>, Instant)>>,
}

impl SkinCache {
    /// The skin URL of the profile `uuid` (dashed or not), or `None` (no such profile, an offline
    /// profile, no custom skin, Mojang unreachable).
    pub async fn skin_url(&self, http: &reqwest::Client, uuid: &str) -> Option<String> {
        let id: String = uuid.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if id.len() != 32 {
            return None;
        }
        let id = id.to_ascii_lowercase();
        if let Some((url, at)) = self.entries.lock().expect("skins").get(&id).cloned() {
            let ttl = if url.is_some() { HIT_TTL } else { MISS_TTL };
            if at.elapsed() < ttl {
                return url;
            }
        }
        let url = fetch(http, &id).await;
        self.entries
            .lock()
            .expect("skins")
            .insert(id, (url.clone(), Instant::now()));
        url
    }
}

async fn fetch(http: &reqwest::Client, id: &str) -> Option<String> {
    let resp = http
        .get(format!("{PROFILE}/{id}"))
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let profile: serde_json::Value = resp.json().await.ok()?;
    skin_from_profile(&profile)
}

/// The skin URL in a session profile's `textures` property.
pub fn skin_from_profile(profile: &serde_json::Value) -> Option<String> {
    let encoded = profile["properties"]
        .as_array()?
        .iter()
        .find(|p| p["name"] == "textures")?["value"]
        .as_str()?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let textures: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    let url = textures["textures"]["SKIN"]["url"].as_str()?;
    let rest = url
        .strip_prefix("http://textures.minecraft.net/")
        .or_else(|| url.strip_prefix("https://textures.minecraft.net/"))?;
    (!rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '/'))
        .then(|| format!("https://textures.minecraft.net/{rest}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn profile(textures: serde_json::Value) -> serde_json::Value {
        let value = base64::engine::general_purpose::STANDARD.encode(textures.to_string());
        json!({ "id": "e800e4e48c8b48a391fe0b5884e457bd", "name": "Bob",
                "properties": [{ "name": "textures", "value": value }] })
    }

    #[test]
    fn reads_the_skin_url_as_https() {
        let p = profile(json!({ "textures": { "SKIN": {
            "url": "http://textures.minecraft.net/texture/abc123" } } }));
        assert_eq!(
            skin_from_profile(&p).as_deref(),
            Some("https://textures.minecraft.net/texture/abc123")
        );
    }

    #[test]
    fn refuses_other_hosts_and_missing_skins() {
        let other = profile(json!({ "textures": { "SKIN": { "url": "https://evil.example/x" } } }));
        assert_eq!(skin_from_profile(&other), None);
        assert_eq!(skin_from_profile(&profile(json!({ "textures": {} }))), None);
        assert_eq!(skin_from_profile(&json!({ "properties": [] })), None);
    }
}
