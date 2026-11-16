//! Rules of version files: a list of `allow`/`disallow` actions with conditions on the OS (name,
//! architecture, version pattern or range) and on launcher features (`is_demo_user`,
//! `has_custom_resolution`, quick play...). As in the official launcher: no rules allows; with
//! rules, nothing is allowed until a matching rule says so, and the last matching rule decides.

use crate::platform::{Platform, compare_versions};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub action: Action,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<OsRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<BTreeMap<String, bool>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    #[default]
    Allow,
    Disallow,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    /// A regular expression on the OS version (old files: `^10\\.`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// `{min, max}` on the OS version (recent files).
    #[serde(
        default,
        rename = "versionRange",
        skip_serializing_if = "Option::is_none"
    )]
    pub version_range: Option<VersionRange>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionRange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
}

/// The launcher features a launch turns on (the others are off).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Features {
    on: Vec<String>,
}

impl Features {
    pub fn none() -> Self {
        Self::default()
    }
    pub fn with(mut self, name: &str) -> Self {
        if !self.has(name) {
            self.on.push(name.to_owned());
        }
        self
    }
    pub fn has(&self, name: &str) -> bool {
        self.on.iter().any(|f| f == name)
    }
}

impl OsRule {
    fn matches(&self, platform: &Platform) -> bool {
        if let Some(name) = &self.name
            && name != platform.os_name()
        {
            return false;
        }
        if let Some(arch) = &self.arch
            && !arch_matches(arch, platform)
        {
            return false;
        }
        if let Some(pattern) = &self.version {
            // An unknown OS version, or a pattern we can't read, doesn't match.
            let Some(v) = &platform.os_version else {
                return false;
            };
            match regex::Regex::new(pattern) {
                Ok(re) if re.is_match(v) => {}
                _ => return false,
            }
        }
        if let Some(range) = &self.version_range {
            let Some(v) = &platform.os_version else {
                return false;
            };
            // `min` inclusive, `max` exclusive: the two halves of a split never both match.
            if let Some(min) = &range.min
                && compare_versions(v, min).is_lt()
            {
                return false;
            }
            if let Some(max) = &range.max
                && !compare_versions(v, max).is_lt()
            {
                return false;
            }
        }
        true
    }
}

fn arch_matches(arch: &str, platform: &Platform) -> bool {
    let arch = arch.to_ascii_lowercase();
    match platform.arch_name() {
        "x86_64" => matches!(arch.as_str(), "x86_64" | "amd64" | "x64"),
        "arm64" => matches!(arch.as_str(), "arm64" | "aarch64"),
        "x86" => matches!(arch.as_str(), "x86" | "i386" | "i686"),
        other => arch == other,
    }
}

impl Rule {
    fn matches(&self, platform: &Platform, features: &Features) -> bool {
        if let Some(os) = &self.os
            && !os.matches(platform)
        {
            return false;
        }
        if let Some(wanted) = &self.features {
            for (name, value) in wanted {
                if features.has(name) != *value {
                    return false;
                }
            }
        }
        true
    }
}

/// Whether `rules` let this platform with these features use the item.
pub fn allowed(rules: &[Rule], platform: &Platform, features: &Features) -> bool {
    if rules.is_empty() {
        return true;
    }
    rules
        .iter()
        .rfind(|r| r.matches(platform, features))
        .is_some_and(|r| r.action == Action::Allow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{Arch, Os};
    use serde_json::json;

    fn rules(v: serde_json::Value) -> Vec<Rule> {
        serde_json::from_value(v).unwrap()
    }

    fn pf(os: Os, arch: Arch, version: Option<&str>) -> Platform {
        Platform {
            os,
            arch,
            os_version: version.map(str::to_owned),
        }
    }

    #[test]
    fn no_rules_allow_and_the_last_match_decides() {
        let win = pf(Os::Windows, Arch::X86_64, None);
        let mac = pf(Os::Osx, Arch::Arm64, None);
        assert!(allowed(&[], &win, &Features::none()));
        let r = rules(json!([{"action": "allow"}, {"action": "disallow", "os": {"name": "osx"}}]));
        assert!(allowed(&r, &win, &Features::none()));
        assert!(!allowed(&r, &mac, &Features::none()));
        let only_mac = rules(json!([{"action": "allow", "os": {"name": "osx"}}]));
        assert!(!allowed(&only_mac, &win, &Features::none()));
        assert!(allowed(&only_mac, &mac, &Features::none()));
    }

    #[test]
    fn architecture_and_version_pattern() {
        let x86 = rules(json!([{"action": "allow", "os": {"arch": "x86"}}]));
        assert!(allowed(
            &x86,
            &pf(Os::Windows, Arch::X86, None),
            &Features::none()
        ));
        assert!(!allowed(
            &x86,
            &pf(Os::Windows, Arch::X86_64, None),
            &Features::none()
        ));
        let win10 =
            rules(json!([{"action": "allow", "os": {"name": "windows", "version": "^10\\."}}]));
        assert!(allowed(
            &win10,
            &pf(Os::Windows, Arch::X86_64, Some("10.0.26200")),
            &Features::none()
        ));
        assert!(!allowed(
            &win10,
            &pf(Os::Windows, Arch::X86_64, Some("6.1.7601")),
            &Features::none()
        ));
        // Unknown OS version: a version condition does not match.
        assert!(!allowed(
            &win10,
            &pf(Os::Windows, Arch::X86_64, None),
            &Features::none()
        ));
    }

    #[test]
    fn version_ranges_split_cleanly() {
        let zgc = rules(json!([
            {"action": "allow", "os": {"name": "osx"}},
            {"action": "allow", "os": {"name": "windows", "versionRange": {"min": "10.0.17134"}}}
        ]));
        let g1 = rules(json!([
            {"action": "allow", "os": {"name": "windows", "versionRange": {"max": "10.0.17134"}}}
        ]));
        let new = pf(Os::Windows, Arch::X86_64, Some("10.0.26200"));
        let old = pf(Os::Windows, Arch::X86_64, Some("10.0.16299"));
        let edge = pf(Os::Windows, Arch::X86_64, Some("10.0.17134"));
        let f = Features::none();
        assert!(allowed(&zgc, &new, &f) && !allowed(&g1, &new, &f));
        assert!(!allowed(&zgc, &old, &f) && allowed(&g1, &old, &f));
        assert!(allowed(&zgc, &edge, &f) && !allowed(&g1, &edge, &f));
    }

    #[test]
    fn features() {
        let r = rules(json!([{"action": "allow", "features": {"has_custom_resolution": true}}]));
        let p = pf(Os::Linux, Arch::X86_64, None);
        assert!(!allowed(&r, &p, &Features::none()));
        assert!(allowed(
            &r,
            &p,
            &Features::none().with("has_custom_resolution")
        ));
        let demo_off = rules(json!([{"action": "allow", "features": {"is_demo_user": false}}]));
        assert!(allowed(&demo_off, &p, &Features::none()));
    }
}
