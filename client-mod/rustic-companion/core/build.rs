//! Embeds every in-game text table (`i18n/<code>.json`, English first) so that a translation is
//! one more file in that folder, with no code change (ADR 0161).
use std::fmt::Write as _;
use std::path::Path;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut codes: Vec<String> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    name.strip_suffix(".json").map(str::to_owned)
                })
                .filter(|code| {
                    !code.is_empty()
                        && code
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                })
                .collect()
        })
        .unwrap_or_default();
    // English first: it is the fallback of every other table.
    codes.sort_by_key(|c| (c != "en", c.clone()));
    let mut out = String::from("/// Every text table: its code and its JSON, English first.\n");
    out.push_str("const FILES: &[(&str, &str)] = &[\n");
    for code in &codes {
        let path = dir.join(format!("{code}.json"));
        println!("cargo:rerun-if-changed={}", path.display());
        let _ = writeln!(
            out,
            "    ({code:?}, include_str!({:?})),",
            path.display().to_string()
        );
    }
    out.push_str("];\n");
    let target = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("i18n_files.rs");
    std::fs::write(target, out).expect("write i18n_files.rs");
}
