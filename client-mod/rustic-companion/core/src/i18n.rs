//! Rustic's own in-game texts (the game's own buttons keep the game's translations). One JSON
//! table per language in `core/i18n/<code>.json` (`en`, `fr`, `de`, `pt_br`...), embedded at build
//! time (`build.rs`), so a translation batch only adds files (ADR 0161, docs/TRANSLATING.md). A
//! text a table lacks falls back to the language without its region (`fr_ca` -> `fr`), to a table
//! that `covers` the game's code or its language (`pt_pt` -> `pt_br`, `bar` -> `de`), then to
//! English, which is complete (tested).
//!
//! A table: `{"language": "Deutsch", "covers": ["bar"], "digit_separator": ".",
//! "texts": {"<key>": "…"}, "tags": {"<tag>": "…"}}`; keys are [`T::key`].
use std::collections::HashMap;
use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/i18n_files.rs"));

/// A Rustic text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum T {
    Store,
    StoreNews,
    Discover,
    MyServers,
    LanguageTitle,
    LanguageSearch,
    LanguageNone,
    Back,
    Search,
    Featured,
    Trending,
    RecentlyPlayed,
    AllServers,
    Categories,
    All,
    Join,
    AddToList,
    Added,
    Verified,
    /// `{n}` is the number of players.
    PlayersOnline,
    Offline,
    Loading,
    EmptyTitle,
    EmptyBody,
    NoResults,
    ErrorTitle,
    Retry,
    PublishHint,
    RusticEdition,
}

impl T {
    /// Every text.
    pub const ALL: [T; 29] = [
        T::Store,
        T::StoreNews,
        T::Discover,
        T::MyServers,
        T::LanguageTitle,
        T::LanguageSearch,
        T::LanguageNone,
        T::Back,
        T::Search,
        T::Featured,
        T::Trending,
        T::RecentlyPlayed,
        T::AllServers,
        T::Categories,
        T::All,
        T::Join,
        T::AddToList,
        T::Added,
        T::Verified,
        T::PlayersOnline,
        T::Offline,
        T::Loading,
        T::EmptyTitle,
        T::EmptyBody,
        T::NoResults,
        T::ErrorTitle,
        T::Retry,
        T::PublishHint,
        T::RusticEdition,
    ];

    /// Its key in the JSON tables.
    pub const fn key(self) -> &'static str {
        match self {
            T::Store => "store",
            T::StoreNews => "store_news",
            T::Discover => "discover",
            T::MyServers => "my_servers",
            T::LanguageTitle => "language_title",
            T::LanguageSearch => "language_search",
            T::LanguageNone => "language_none",
            T::Back => "back",
            T::Search => "search",
            T::Featured => "featured",
            T::Trending => "trending",
            T::RecentlyPlayed => "recently_played",
            T::AllServers => "all_servers",
            T::Categories => "categories",
            T::All => "all",
            T::Join => "join",
            T::AddToList => "add_to_list",
            T::Added => "added",
            T::Verified => "verified",
            T::PlayersOnline => "players_online",
            T::Offline => "offline",
            T::Loading => "loading",
            T::EmptyTitle => "empty_title",
            T::EmptyBody => "empty_body",
            T::NoResults => "no_results",
            T::ErrorTitle => "error_title",
            T::Retry => "retry",
            T::PublishHint => "publish_hint",
            T::RusticEdition => "rustic_edition",
        }
    }
}

/// One language's table, as its JSON file says.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct Table {
    /// Game codes or languages this table also serves (`pt` for `pt_br`, `bar` for `de`).
    covers: Vec<String>,
    /// Groups thousands (`,` in English, a narrow no-break space in French).
    digit_separator: Option<String>,
    texts: HashMap<String, String>,
    tags: HashMap<String, String>,
}

/// Every table that parses, by code, English first; a broken file is left out (tested).
fn tables() -> &'static [(&'static str, Table)] {
    static TABLES: OnceLock<Vec<(&'static str, Table)>> = OnceLock::new();
    TABLES.get_or_init(|| {
        FILES
            .iter()
            .filter_map(|(code, json)| Some((*code, serde_json::from_str(json).ok()?)))
            .collect()
    })
}

/// The tables to look in for `lang` (a game code such as `fr_fr`), most specific first, English
/// last.
fn chain(lang: &str) -> Vec<&'static Table> {
    let lang = lang.to_ascii_lowercase();
    let base = lang.split(['_', '-']).next().unwrap_or("").to_owned();
    let all = tables();
    let mut out: Vec<&'static Table> = Vec::new();
    let mut push = |t: &'static Table| {
        if !out.iter().any(|o| std::ptr::eq(*o, t)) {
            out.push(t);
        }
    };
    for want in [lang.as_str(), base.as_str()] {
        if want.is_empty() {
            continue;
        }
        if let Some((_, t)) = all.iter().find(|(c, _)| *c == want) {
            push(t);
        }
        for (_, t) in all
            .iter()
            .filter(|(_, t)| t.covers.iter().any(|c| c == want))
        {
            push(t);
        }
    }
    if let Some((_, en)) = all.iter().find(|(c, _)| *c == "en") {
        push(en);
    }
    out
}

/// Text `key` in the game's language `lang` (English when no table has it).
pub fn t(key: T, lang: &str) -> &'static str {
    chain(lang)
        .into_iter()
        .find_map(|t| t.texts.get(key.key()))
        .map(String::as_str)
        // English is complete (tested); never reached.
        .unwrap_or("")
}

/// [`T::PlayersOnline`] with its number.
pub fn players_online(n: i64, lang: &str) -> String {
    t(T::PlayersOnline, lang).replace("{n}", &group_digits(n, lang))
}

/// `12345` as `12,345` (English), `12 345` (French, with a narrow no-break space), `12.345`
/// (German): the separator of the first table in `lang`'s chain that names one.
pub fn group_digits(n: i64, lang: &str) -> String {
    let sep = chain(lang)
        .into_iter()
        .find_map(|t| t.digit_separator.as_deref())
        .unwrap_or(",");
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push_str(sep);
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// A directory tag's name in `lang`; an unknown tag shows as itself, capitalised.
pub fn tag_name(tag: &str, lang: &str) -> String {
    if let Some(name) = chain(lang).into_iter().find_map(|t| t.tags.get(tag)) {
        return name.clone();
    }
    let mut c = tag.chars();
    match c.next() {
        Some(f) => f
            .to_uppercase()
            .chain(c)
            .collect::<String>()
            .replace('-', " "),
        None => String::new(),
    }
}

/// The codes of the embedded tables, English first.
pub fn languages() -> Vec<&'static str> {
    tables().iter().map(|(c, _)| *c).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tables that must be complete: every text and every tag English names.
    const COMPLETE: &[&str] = &["en", "fr", "de", "es", "it", "pt_br", "pl"];

    fn placeholders(s: &str) -> Vec<String> {
        let mut out: Vec<String> = s
            .split('{')
            .skip(1)
            .filter_map(|p| p.split_once('}').map(|(name, _)| name.to_owned()))
            .collect();
        out.sort();
        out
    }

    #[test]
    fn every_file_parses_and_english_comes_first() {
        assert_eq!(tables().len(), FILES.len(), "a table doesn't parse");
        assert_eq!(FILES[0].0, "en");
        for code in COMPLETE {
            assert!(languages().contains(code), "{code} table");
        }
    }

    #[test]
    fn tables_know_only_real_keys_with_the_same_placeholders() {
        let en = &tables()[0].1;
        for key in T::ALL {
            assert!(en.texts.contains_key(key.key()), "{key:?} in English");
        }
        assert_eq!(en.texts.len(), T::ALL.len(), "English has unused keys");
        for (code, table) in tables() {
            for (key, text) in &table.texts {
                let english = en.texts.get(key);
                assert!(english.is_some(), "{code}: unknown key {key}");
                assert!(!text.trim().is_empty(), "{code}: {key} is empty");
                assert_eq!(
                    placeholders(text),
                    placeholders(english.map(String::as_str).unwrap_or("")),
                    "{code}: {key}"
                );
            }
            for tag in table.tags.keys() {
                assert!(en.tags.contains_key(tag), "{code}: unknown tag {tag}");
            }
            if COMPLETE.contains(code) {
                for key in T::ALL {
                    assert!(table.texts.contains_key(key.key()), "{code}: {key:?}");
                }
            }
        }
    }

    #[test]
    fn regions_fall_back_to_their_language_then_english() {
        assert_eq!(t(T::Discover, "fr_fr"), "Découvrir");
        assert_eq!(t(T::Discover, "fr_ca"), "Découvrir");
        assert_eq!(t(T::MyServers, "fr_fr"), "Mes serveurs");
        assert_eq!(t(T::Discover, "en_us"), "Discover");
        assert_eq!(t(T::Discover, "ja_jp"), "Discover");
        assert_eq!(t(T::Discover, ""), "Discover");
        assert_eq!(t(T::Discover, "de_at"), t(T::Discover, "de_de"));
        assert_eq!(t(T::Join, "pt_pt"), t(T::Join, "pt_br"));
        assert_ne!(t(T::Join, "pt_br"), "Join");
        assert_eq!(t(T::Back, "bar"), t(T::Back, "de_de"));
        assert_eq!(players_online(1234, "en_us"), "1,234 online");
        assert_eq!(players_online(5, "fr_fr"), "5 en ligne");
        assert_eq!(group_digits(12_345, "de_de"), "12.345");
    }

    #[test]
    fn tags_have_names() {
        assert_eq!(tag_name("smp", "fr_fr"), "SMP");
        assert_eq!(tag_name("survival", "fr_fr"), "Survie");
        assert_eq!(tag_name("survival", "en_gb"), "Survival");
        assert_eq!(tag_name("tower-defense", "en_us"), "Tower defense");
        assert_eq!(group_digits(1_000_000, "en_us"), "1,000,000");
        assert_eq!(group_digits(3410, "fr_fr"), "3\u{202F}410");
        assert_eq!(group_digits(-12, "en_us"), "-12");
    }
}
