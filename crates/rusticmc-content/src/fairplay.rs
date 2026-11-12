//! Fair play: X-ray, auto-clickers and hacks are never promoted, and Rustic never installs them
//! (launcher/AGENTS.md). The public policy is in launcher/docs/CONTENT.md.
//!
//! A project is **blocked** when its id or slug is on the deny list, or when its name, slug,
//! categories or tags name a cheat; it is **flagged** (not promoted, installable with a notice)
//! when only its summary does. "Anti-xray" and friends are protections, never matches.
use crate::model::{FairPlay, ProjectSummary};
use std::collections::BTreeSet;

/// Terms that name a cheat (matched on whole words, or as a whole hyphen/space-joined phrase).
pub const DEFAULT_TERMS: &[&str] = &[
    "xray",
    "x-ray",
    "x ray",
    "autoclicker",
    "auto clicker",
    "auto-clicker",
    "autoclick",
    "killaura",
    "kill aura",
    "aimbot",
    "aim bot",
    "aim assist",
    "triggerbot",
    "esp",
    "wallhack",
    "wall hack",
    "flyhack",
    "fly hack",
    "speedhack",
    "speed hack",
    "nofall",
    "no fall damage hack",
    "reach hack",
    "hacked client",
    "hack client",
    "cheat client",
    "cheats",
    "ghost client",
    "baritone",
    "auto totem",
    "autototem",
    "anti knockback",
    "antiknockback",
    "velocity hack",
    "tracers",
    "ore finder",
    "orefinder",
    "cave finder",
    "scaffold",
    "jesus hack",
    "dupe",
    "duplication glitch",
    "crasher",
    "server crasher",
    "session stealer",
    "token logger",
    "packet exploit",
];

/// Terms of features fair in single player but not on every server (launcher/docs/FAIR_PLAY.md):
/// never promoted, installable with a notice.
pub const DEFAULT_FLAG_TERMS: &[&str] = &["freecam", "fullbright", "full bright"];

/// Words before a term that make it a protection, not a cheat ("anti-xray").
const NEGATIONS: &[&str] = &[
    "anti",
    "no",
    "detect",
    "detects",
    "detection",
    "against",
    "block",
];

#[derive(Clone, Debug)]
pub struct FairPlayPolicy {
    /// Project ids or slugs never offered (any source), lowercase.
    pub denied: BTreeSet<String>,
    pub terms: Vec<String>,
    pub flag_terms: Vec<String>,
}

impl Default for FairPlayPolicy {
    fn default() -> Self {
        Self {
            denied: BTreeSet::new(),
            terms: DEFAULT_TERMS.iter().map(|s| s.to_string()).collect(),
            flag_terms: DEFAULT_FLAG_TERMS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

impl FairPlayPolicy {
    /// With a deny list (ids or slugs).
    pub fn with_denied<I: IntoIterator<Item = S>, S: Into<String>>(mut self, ids: I) -> Self {
        self.denied
            .extend(ids.into_iter().map(|s| s.into().to_lowercase()));
        self
    }

    /// The first term `text` names as a cheat, if any.
    pub fn matches(&self, text: &str) -> Option<String> {
        Self::find(&self.terms, text)
    }

    fn find(terms: &[String], text: &str) -> Option<String> {
        let w = words(text);
        for term in terms {
            let t = words(term);
            if t.is_empty() || t.len() > w.len() {
                continue;
            }
            for start in 0..=w.len() - t.len() {
                if w[start..start + t.len()] == t[..] {
                    let negated = start > 0 && NEGATIONS.contains(&w[start - 1].as_str());
                    // "antixray": a protection written as one word.
                    if !negated {
                        return Some(term.clone());
                    }
                }
            }
            // A term glued to a negation ("antixray", "antiesp") never matches; a term glued
            // to other words ("xrayplus") does.
            let joined = t.concat();
            if joined.len() >= 5
                && w.iter().any(|word| {
                    word.contains(&joined)
                        && !NEGATIONS
                            .iter()
                            .any(|n| word.starts_with(&format!("{n}{joined}")))
                        && word.as_str() != joined
                })
            {
                return Some(term.clone());
            }
        }
        None
    }

    /// The standing of `p`.
    pub fn judge(&self, p: &ProjectSummary) -> FairPlay {
        if self.denied.contains(&p.id.to_lowercase())
            || self.denied.contains(&p.slug.to_lowercase())
        {
            return FairPlay::Blocked {
                reason: "on Rustic's fair-play deny list".into(),
            };
        }
        let strong = [p.name.as_str(), p.slug.as_str()]
            .into_iter()
            .chain(p.categories.iter().map(String::as_str))
            .chain(p.tags.iter().map(String::as_str));
        for text in strong {
            if let Some(term) = self.matches(text) {
                return FairPlay::Blocked {
                    reason: format!("named as a cheat (\"{term}\")"),
                };
            }
        }
        let strong = [p.name.as_str(), p.slug.as_str()]
            .into_iter()
            .chain(p.tags.iter().map(String::as_str));
        for text in strong {
            if let Some(term) = Self::find(&self.flag_terms, text) {
                return FairPlay::Flagged {
                    reason: format!("fair in single player, not on every server (\"{term}\")"),
                };
            }
        }
        if let Some(term) = self.matches(&p.summary) {
            return FairPlay::Flagged {
                reason: format!("its description mentions \"{term}\""),
            };
        }
        FairPlay::Clean
    }

    /// Sets each result's standing.
    pub fn apply(&self, results: &mut [ProjectSummary]) {
        for r in results {
            r.fair_play = self.judge(r);
        }
    }

    /// Only what may be promoted (featured, recommended, "popular" rows): clean projects.
    pub fn promotable<'a>(&self, results: &'a [ProjectSummary]) -> Vec<&'a ProjectSummary> {
        results
            .iter()
            .filter(|r| self.judge(r) == FairPlay::Clean)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ContentKind, SourceKind};

    fn project(name: &str, summary: &str) -> ProjectSummary {
        ProjectSummary {
            source: SourceKind::Modrinth,
            id: "abc".into(),
            slug: name.to_lowercase().replace(' ', "-"),
            kind: ContentKind::Mod,
            name: name.into(),
            summary: summary.into(),
            authors: vec![],
            icon_url: None,
            downloads: 0,
            categories: vec![],
            tags: vec![],
            license: None,
            loaders: vec![],
            game_versions: vec![],
            fair_play: FairPlay::Clean,
        }
    }

    #[test]
    fn cheats_are_blocked_protections_are_not() {
        let p = FairPlayPolicy::default();
        assert!(matches!(
            p.judge(&project("Xray Ultimate", "See ores")),
            FairPlay::Blocked { .. }
        ));
        assert!(matches!(
            p.judge(&project("Simple AutoClicker", "")),
            FairPlay::Blocked { .. }
        ));
        assert!(matches!(
            p.judge(&project("Mob ESP", "")),
            FairPlay::Blocked { .. }
        ));
        assert_eq!(
            p.judge(&project("Anti-Xray", "hides ores from x-ray cheats")),
            FairPlay::Flagged {
                reason: "its description mentions \"x-ray\"".into()
            }
        );
        assert_eq!(p.judge(&project("AntiXray", "")), FairPlay::Clean);
        assert_eq!(
            p.judge(&project("Sodium", "Rendering engine")),
            FairPlay::Clean
        );
        assert!(matches!(
            p.judge(&project("Freecam", "")),
            FairPlay::Flagged { .. }
        ));
        assert!(matches!(
            p.judge(&project("Ore Finder", "")),
            FairPlay::Blocked { .. }
        ));
        // "esp" inside another word is not ESP.
        assert_eq!(p.judge(&project("Respawn Anchors+", "")), FairPlay::Clean);
    }

    #[test]
    fn deny_list_and_promotion() {
        let p = FairPlayPolicy::default().with_denied(["bad-mod"]);
        let mut list = vec![project("Bad Mod", ""), project("Waystones", "")];
        list[0].slug = "bad-mod".into();
        p.apply(&mut list);
        assert!(matches!(list[0].fair_play, FairPlay::Blocked { .. }));
        let promoted = p.promotable(&list);
        assert_eq!(promoted.len(), 1);
        assert_eq!(promoted[0].name, "Waystones");
    }
}
