//! Which mod a compiler error belongs to, so a mod that no longer compiles on this Horizon can be
//! left out and the others built (Horizon's launcher, `native_mods/diagnostics.rs`, rewritten).
//!
//! horizon-mixins leaves a plan in the staged source (`.horizon-mixins-stage.json`, schema 6): for
//! each injection the file and the helper function it brought, for each module the file and the
//! module's name. An error belongs to a mod only when its primary span is inside what that mod
//! brought: the helper function (or the block horizon-mixins wraps around it with its call) or the
//! module, and the pristine source has no function or module of that name. A file being touched
//! is not enough.
use serde_json::Value;
use std::collections::BTreeSet;
use std::ops::Range;
use std::path::{Path, PathBuf};

pub const PLAN: &str = ".horizon-mixins-stage.json";

/// The plan's entries: `(mod id, file relative to the workspace, kind)`.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    entries: Vec<Entry>,
}

#[derive(Clone, Debug)]
struct Entry {
    mod_id: String,
    file: String,
    /// `fn` for a helper (its name), `mod` for a module (its name).
    item: Item,
}

#[derive(Clone, Debug)]
enum Item {
    Helper(String),
    Module(String),
}

impl Plan {
    /// The plan in the staged Horizon at `horizon`, if horizon-mixins left one it can read.
    pub fn read(horizon: &Path) -> Option<Self> {
        let bytes = std::fs::read(horizon.join(PLAN)).ok()?;
        let value: Value = serde_json::from_slice(&bytes).ok()?;
        Self::from_value(&value)
    }

    pub fn from_value(value: &Value) -> Option<Self> {
        if value["schema"].as_u64() != Some(6) {
            return None;
        }
        let mut entries = Vec::new();
        for e in value["injections"].as_array().into_iter().flatten() {
            let (Some(id), Some(file), Some(helper)) = (
                e["mod_id"].as_str(),
                e["file"].as_str(),
                e["helper_source"].as_str(),
            ) else {
                continue;
            };
            if let Some(name) = fn_name(helper) {
                entries.push(Entry {
                    mod_id: id.to_owned(),
                    file: file.replace('\\', "/"),
                    item: Item::Helper(name),
                });
            }
        }
        for e in value["modules"].as_array().into_iter().flatten() {
            let (Some(id), Some(file), Some(name)) =
                (e["mod_id"].as_str(), e["file"].as_str(), e["name"].as_str())
            else {
                continue;
            };
            entries.push(Entry {
                mod_id: id.to_owned(),
                file: file.replace('\\', "/"),
                item: Item::Module(name.to_owned()),
            });
        }
        Some(Self { entries })
    }

    /// The mods owning an error whose primary span is `[start, end)` in `file` (relative to the
    /// staged workspace), with the staged and pristine texts of that file.
    pub fn owners(
        &self,
        file: &str,
        start: usize,
        end: usize,
        staged: &str,
        pristine: &str,
    ) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let inside = |r: &Range<usize>| r.start <= start && end <= r.end;
        for e in self.entries.iter().filter(|e| e.file == file) {
            let (keyword, name) = match &e.item {
                Item::Helper(n) => ("fn", n),
                Item::Module(n) => ("mod", n),
            };
            if !items(pristine, keyword, name).is_empty() {
                continue;
            }
            let mut ranges = items(staged, keyword, name);
            if keyword == "fn" {
                // The wrapper block `{ fn helper(..) {..} helper(..) }`: its call too.
                ranges = ranges
                    .into_iter()
                    .map(|r| r.start..wrapper_end(staged, r.end))
                    .collect();
            }
            if ranges.iter().any(inside) {
                out.insert(e.mod_id.clone());
            }
        }
        out
    }
}

/// The name of the first `fn` in a helper's source.
fn fn_name(source: &str) -> Option<String> {
    let code = strip_comments(source);
    let mut rest = code.as_str();
    while let Some(i) = rest.find("fn ") {
        let before = rest[..i].chars().last();
        let after = &rest[i + 3..];
        if before.is_none_or(|c| !(c.is_alphanumeric() || c == '_')) {
            let name: String = after
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
        rest = after;
    }
    None
}

/// `text` with comments blanked (same length, so offsets stay).
fn strip_comments(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    let mut in_str = false;
    while i < b.len() {
        if in_str {
            if b[i] == b'\\' {
                i += 2;
                continue;
            }
            if b[i] == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if b[i] == b'"' {
            in_str = true;
            i += 1;
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                out[i] = b' ';
                i += 1;
            }
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            let mut depth = 0;
            while i < b.len() {
                if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                    depth += 1;
                    out[i] = b' ';
                    out[i + 1] = b' ';
                    i += 2;
                } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                    depth -= 1;
                    out[i] = b' ';
                    out[i + 1] = b' ';
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if b[i] != b'\n' {
                        out[i] = b' ';
                    }
                    i += 1;
                }
            }
        } else {
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
}

/// Byte ranges of `<keyword> <name>` items in `text`, from the keyword to their closing brace.
fn items(text: &str, keyword: &str, name: &str) -> Vec<Range<usize>> {
    let code = strip_comments(text);
    let needle = format!("{keyword} {name}");
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = code[from..].find(&needle) {
        let at = from + i;
        from = at + needle.len();
        let before_ok = code[..at]
            .chars()
            .last()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        let after_ok = code[from..]
            .chars()
            .next()
            .is_some_and(|c| !(c.is_alphanumeric() || c == '_'));
        if !(before_ok && after_ok) {
            continue;
        }
        // `mod name;` has no body.
        let Some(open) = code[from..].find(['{', ';']).map(|j| from + j) else {
            continue;
        };
        if code.as_bytes()[open] == b';' {
            continue;
        }
        if let Some(close) = matching(&code, open) {
            out.push(at..close + 1);
        }
    }
    out
}

/// The `}` matching the `{` at `open`, skipping strings and character literals.
fn matching(code: &str, open: usize) -> Option<usize> {
    let b = code.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    while i < b.len() {
        match b[i] {
            b'"' => {
                // A raw string `r#"..."#` ends at `"#`; a plain one at an unescaped `"`.
                let raw_hashes = raw_prefix(b, i);
                i += 1;
                while i < b.len() {
                    if raw_hashes.is_none() && b[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if b[i] == b'"' {
                        let n = raw_hashes.unwrap_or(0);
                        if b[i + 1..].iter().take(n).filter(|&&c| c == b'#').count() == n {
                            i += n;
                            break;
                        }
                    }
                    i += 1;
                }
            }
            b'\'' => {
                // 'x' or '\n' is a character; 'a (a lifetime) is not.
                if b.get(i + 2) == Some(&b'\'') {
                    i += 2;
                } else if b.get(i + 1) == Some(&b'\\')
                    && let Some(j) = b[i + 2..].iter().position(|&c| c == b'\'')
                {
                    i += 2 + j;
                }
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// `Some(n)` when the `"` at `i` opens a raw string with `n` hashes.
fn raw_prefix(b: &[u8], i: usize) -> Option<usize> {
    let mut j = i;
    let mut n = 0;
    while j > 0 && b[j - 1] == b'#' {
        j -= 1;
        n += 1;
    }
    (j > 0 && b[j - 1] == b'r').then_some(n)
}

/// Where the block a helper ends in ends: the `}` closing its wrapper block, else `end`.
fn wrapper_end(staged: &str, end: usize) -> usize {
    let code = strip_comments(staged);
    let b = code.as_bytes();
    let mut depth = 0i32;
    for (i, &c) in b.iter().enumerate().skip(end) {
        match c {
            b'{' => depth += 1,
            b'}' => {
                if depth == 0 {
                    return i + 1;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    end
}

/// The mods a Cargo `compiler-message` (JSON) belongs to. `horizon` is the staged Horizon,
/// `pristine` the source's; span file names are relative to the package or absolute.
pub fn owners_of(
    plan: &Plan,
    message: &Value,
    horizon: &Path,
    pristine: &Path,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let manifest_dir = message["manifest_path"]
        .as_str()
        .and_then(|m| Path::new(m).parent().map(Path::to_path_buf));
    let Some(spans) = message["message"]["spans"].as_array() else {
        return out;
    };
    let horizon_c = canonical(horizon);
    for span in spans.iter().filter(|s| s["is_primary"] == true) {
        let (Some(file), Some(start), Some(end)) = (
            span["file_name"].as_str(),
            span["byte_start"].as_u64(),
            span["byte_end"].as_u64(),
        ) else {
            continue;
        };
        let candidates: Vec<PathBuf> = if Path::new(file).is_absolute() {
            vec![PathBuf::from(file)]
        } else {
            [manifest_dir.clone(), Some(horizon.to_path_buf())]
                .into_iter()
                .flatten()
                .map(|d| d.join(file))
                .collect()
        };
        for path in candidates {
            let full = canonical(&path);
            let Ok(rel) = full.strip_prefix(&horizon_c) else {
                continue;
            };
            let rel = rel.to_string_lossy().replace('\\', "/");
            let (Ok(staged), pristine) = (
                std::fs::read_to_string(&full),
                std::fs::read_to_string(pristine.join(&rel)).unwrap_or_default(),
            ) else {
                continue;
            };
            out.extend(plan.owners(&rel, start as usize, end as usize, &staged, &pristine));
            break;
        }
    }
    out
}

fn canonical(p: &Path) -> PathBuf {
    let c = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    // Windows' canonical form starts with \\?\; compare without it.
    PathBuf::from(c.to_string_lossy().trim_start_matches(r"\\?\").to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRISTINE: &str =
        "impl Game {\n    fn native_hud(&mut self) {\n        let x = 1;\n    }\n}\n";

    fn staged() -> String {
        "impl Game {\n    fn native_hud(&mut self) {\n        {\n            // the hud\n            fn official_fps_overlay_hud(game: &mut Game) { let y: u8 = \"no\"; }\n            official_fps_overlay_hud(&mut *self)\n        };\n        let x = 1;\n    }\n}\nmod official_fps_overlay {\n    pub fn tick() { let s = \"}\"; broken() }\n}\n".to_owned()
    }

    fn plan() -> Plan {
        Plan::from_value(&serde_json::json!({
            "schema": 6,
            "injections": [{
                "mod_id": "fps_overlay", "file": "crates/horizon-client/src/game/native_hooks.rs",
                "helper_source": "// The box\nfn official_fps_overlay_hud(game: &mut Game) { let y: u8 = \"no\"; }"
            }],
            "modules": [{
                "mod_id": "fps_overlay", "file": "crates/horizon-client/src/game/native_hooks.rs",
                "name": "official_fps_overlay"
            }]
        }))
        .unwrap()
    }

    #[test]
    fn an_error_in_a_helper_names_its_mod() {
        let s = staged();
        let file = "crates/horizon-client/src/game/native_hooks.rs";
        let at = s.find("\"no\"").unwrap();
        let owners = plan().owners(file, at, at + 4, &s, PRISTINE);
        assert_eq!(owners.into_iter().collect::<Vec<_>>(), ["fps_overlay"]);
        // The generated call to it, too.
        let call = s.find("official_fps_overlay_hud(&mut").unwrap();
        assert_eq!(plan().owners(file, call, call + 10, &s, PRISTINE).len(), 1);
    }

    #[test]
    fn an_error_in_a_module_names_its_mod_but_horizons_own_code_does_not() {
        let s = staged();
        let file = "crates/horizon-client/src/game/native_hooks.rs";
        let at = s.find("broken()").unwrap();
        assert_eq!(plan().owners(file, at, at + 8, &s, PRISTINE).len(), 1);
        let horizon = s.find("let x = 1").unwrap();
        assert!(
            plan()
                .owners(file, horizon, horizon + 5, &s, PRISTINE)
                .is_empty()
        );
        // Another file: nobody.
        assert!(
            plan()
                .owners("crates/other.rs", at, at + 8, &s, PRISTINE)
                .is_empty()
        );
    }

    #[test]
    fn a_name_horizon_already_has_is_not_blamed() {
        let s = staged();
        let pristine = format!("{PRISTINE}fn official_fps_overlay_hud() {{}}\n");
        let file = "crates/horizon-client/src/game/native_hooks.rs";
        let at = s.find("\"no\"").unwrap();
        assert!(plan().owners(file, at, at + 4, &s, &pristine).is_empty());
    }

    #[test]
    fn helper_names_and_plans() {
        assert_eq!(
            fn_name("// fn not_me\nfn real(x: u8) {}").as_deref(),
            Some("real")
        );
        assert_eq!(fn_name("pub(crate) fn a() {}").as_deref(), Some("a"));
        assert!(fn_name("let afn = 1;").is_none());
        assert!(Plan::from_value(&serde_json::json!({"schema": 5})).is_none());
        let code = "fn a() { let c = '{'; let s = r#\"}\"#; }";
        assert_eq!(items(code, "fn", "a"), vec![0..code.len()]);
        assert!(items("mod a;", "mod", "a").is_empty());
    }

    #[test]
    fn a_cargo_message_is_traced_through_the_staged_files() {
        let t = tempfile::tempdir().unwrap();
        let horizon = t.path().join("stage/Rustic-MC/horizon");
        let pristine = t.path().join("source/Rustic-MC/horizon");
        let rel = "crates/horizon-client/src/game/native_hooks.rs";
        for (root, text) in [(&horizon, staged()), (&pristine, PRISTINE.to_owned())] {
            std::fs::create_dir_all(root.join(rel).parent().unwrap()).unwrap();
            std::fs::write(root.join(rel), text).unwrap();
        }
        let s = staged();
        let at = s.find("\"no\"").unwrap();
        let message = serde_json::json!({
            "reason": "compiler-message",
            "manifest_path": horizon.join("crates/horizon-client/Cargo.toml"),
            "message": {"level": "error", "spans": [{
                "is_primary": true,
                "file_name": "src/game/native_hooks.rs",
                "byte_start": at, "byte_end": at + 4
            }]}
        });
        let owners = owners_of(&plan(), &message, &horizon, &pristine);
        assert_eq!(owners.into_iter().collect::<Vec<_>>(), ["fps_overlay"]);
    }
}
