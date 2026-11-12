//! Console autocomplete. The control API has no completion endpoint (horizon/docs/api/server.md
//! lists none), so suggestions come from the command tree the game's data generator wrote into
//! the prepared resources: `commands.json` beside `registries.json` (horizon/docs/hosting.md: the
//! prepared folder keeps both). Its shape is Brigadier's dump: `{"type": "root", "children":
//! {"name": {"type": "literal" | "argument", "parser"?, "properties"?, "children"?, "executable"?,
//! "redirect"?}}}`. Plugin commands registered at run time aren't in it (RISKS.md).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

/// One suggestion.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Completion {
    /// What replaces the last word (a literal), or a placeholder like `<targets>`.
    pub text: String,
    /// `literal`, `argument` or `player`.
    pub kind: String,
    /// The argument's parser (`minecraft:entity`, `brigadier:integer`...).
    pub parser: Option<String>,
}

/// The command tree.
#[derive(Clone, Debug, Default)]
pub struct CommandTree {
    root: Value,
}

impl CommandTree {
    pub fn from_json(root: Value) -> Self {
        Self { root }
    }

    /// `commands.json` in a prepared folder (searched one level down too, e.g. `reports/`).
    pub fn load(prepared: &Path) -> Option<Self> {
        let candidates = [
            prepared.join("commands.json"),
            prepared.join("reports/commands.json"),
        ];
        let text = candidates
            .iter()
            .find_map(|p| std::fs::read_to_string(p).ok())?;
        serde_json::from_str(&text).ok().map(Self::from_json)
    }

    fn node<'a>(&'a self, path: &[String]) -> Option<&'a Value> {
        let mut node = &self.root;
        for step in path {
            node = node.get("children")?.get(step)?;
        }
        Some(node)
    }

    /// Follows a `redirect` (e.g. `execute run` back to the root) to the node whose children apply.
    fn resolve<'a>(&'a self, node: &'a Value) -> &'a Value {
        match node.get("redirect").and_then(Value::as_array) {
            Some(path) => {
                let path: Vec<String> = path
                    .iter()
                    .filter_map(|s| s.as_str().map(str::to_owned))
                    .collect();
                self.node(&path).unwrap_or(node)
            }
            None if node.get("children").is_none()
                && node.get("type").and_then(Value::as_str) != Some("root")
                && node.get("executable").is_none() =>
            {
                // Brigadier's dump omits `redirect` to the root for `execute run`: a leaf that
                // isn't executable continues at the root.
                &self.root
            }
            None => node,
        }
    }

    /// Suggestions for the last word of `line` (without the leading `/`). `players` completes
    /// player arguments.
    pub fn complete(&self, line: &str, players: &[String]) -> Vec<Completion> {
        let line = line.trim_start().trim_start_matches('/');
        let mut words: Vec<&str> = line.split(' ').collect();
        let last = words.pop().unwrap_or("");
        let mut node = &self.root;
        for word in &words {
            if word.is_empty() {
                continue;
            }
            let Some(children) = node.get("children").and_then(Value::as_object) else {
                return Vec::new();
            };
            let next = children
                .get(*word)
                .filter(|c| c["type"] == "literal")
                .or_else(|| children.values().find(|c| c["type"] == "argument"));
            match next {
                Some(n) => {
                    // A greedy string swallows the rest of the line: nothing more to suggest.
                    if n["properties"]["type"] == "greedy" {
                        return Vec::new();
                    }
                    node = self.resolve(n);
                }
                None => return Vec::new(),
            }
        }
        let Some(children) = node.get("children").and_then(Value::as_object) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (name, child) in children {
            match child["type"].as_str() {
                Some("literal") if name.starts_with(last) => out.push(Completion {
                    text: name.clone(),
                    kind: "literal".into(),
                    parser: None,
                }),
                Some("argument") => {
                    let parser = child["parser"].as_str().map(str::to_owned);
                    let takes_players = matches!(
                        parser.as_deref(),
                        Some("minecraft:entity" | "minecraft:game_profile")
                    );
                    if takes_players {
                        for p in players
                            .iter()
                            .filter(|p| p.to_lowercase().starts_with(&last.to_lowercase()))
                        {
                            out.push(Completion {
                                text: p.clone(),
                                kind: "player".into(),
                                parser: parser.clone(),
                            });
                        }
                    }
                    if last.is_empty() {
                        out.push(Completion {
                            text: format!("<{name}>"),
                            kind: "argument".into(),
                            parser,
                        });
                    }
                }
                _ => {}
            }
        }
        out.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.text.cmp(&b.text)));
        out.dedup();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tree() -> CommandTree {
        CommandTree::from_json(json!({
            "type": "root",
            "children": {
                "kick": {"type": "literal", "children": {
                    "targets": {"type": "argument", "parser": "minecraft:entity", "executable": true,
                        "children": {"reason": {"type": "argument", "parser": "minecraft:message", "properties": {"type": "greedy"}, "executable": true}}}
                }},
                "kill": {"type": "literal", "executable": true},
                "time": {"type": "literal", "children": {
                    "set": {"type": "literal", "children": {
                        "day": {"type": "literal", "executable": true},
                        "time": {"type": "argument", "parser": "minecraft:time", "executable": true}
                    }},
                    "query": {"type": "literal"}
                }},
                "execute": {"type": "literal", "children": {
                    "run": {"type": "literal", "redirect": []}
                }}
            }
        }))
    }

    fn texts(c: Vec<Completion>) -> Vec<String> {
        c.into_iter().map(|c| c.text).collect()
    }

    #[test]
    fn completes_literals_arguments_and_players() {
        let t = tree();
        assert_eq!(texts(t.complete("ki", &[])), ["kick", "kill"]);
        assert_eq!(texts(t.complete("/time s", &[])), ["set"]);
        assert_eq!(texts(t.complete("time set ", &[])), ["<time>", "day"]);
        let players = vec!["Steve".to_owned(), "Alex".to_owned()];
        assert_eq!(texts(t.complete("kick St", &players)), ["Steve"]);
        assert!(texts(t.complete("kick Steve because ", &players)).is_empty());
        // `execute run` continues at the root.
        assert_eq!(texts(t.complete("execute run ti", &[])), ["time"]);
        assert!(t.complete("nothing here", &[]).is_empty());
    }
}
