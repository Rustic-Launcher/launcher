//! Rustic's own in-game texts (the game's own buttons keep the game's translations). One table per
//! language, English first and complete; a translation step adds a table for another language
//! (`("de", &[...])`) without touching the code. A text a table lacks falls back to the language
//! without its region (`fr_ca` -> `fr`), then to English.

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

type Table = &'static [(T, &'static str)];

const EN: Table = &[
    (T::Store, "Store"),
    (T::StoreNews, "Store news"),
    (T::Discover, "Discover"),
    (T::MyServers, "My servers"),
    (T::LanguageTitle, "Language"),
    (T::LanguageSearch, "Search languages"),
    (T::LanguageNone, "No language matches"),
    (T::Back, "Back"),
    (T::Search, "Search servers"),
    (T::Featured, "Featured"),
    (T::Trending, "Trending"),
    (T::RecentlyPlayed, "Recently played"),
    (T::AllServers, "All servers"),
    (T::Categories, "Categories"),
    (T::All, "All"),
    (T::Join, "Join"),
    (T::AddToList, "Add to list"),
    (T::Added, "In your list"),
    (T::Verified, "Verified"),
    (T::PlayersOnline, "{n} online"),
    (T::Offline, "Offline"),
    (T::Loading, "Loading servers…"),
    (T::EmptyTitle, "No servers listed yet"),
    (
        T::EmptyBody,
        "Servers published on rustic-launcher.com show up here.",
    ),
    (T::NoResults, "No server matches your search"),
    (T::ErrorTitle, "The server directory can't be reached"),
    (T::Retry, "Retry"),
    (
        T::PublishHint,
        "Publish yours at rustic-launcher.com/servers",
    ),
    (T::RusticEdition, "Rustic edition"),
];

const FR: Table = &[
    (T::Store, "Boutique"),
    (T::StoreNews, "Nouveautés boutique"),
    (T::Discover, "Découvrir"),
    (T::MyServers, "Mes serveurs"),
    (T::LanguageTitle, "Langue"),
    (T::LanguageSearch, "Rechercher une langue"),
    (T::LanguageNone, "Aucune langue ne correspond"),
    (T::Back, "Retour"),
    (T::Search, "Rechercher un serveur"),
    (T::Featured, "À la une"),
    (T::Trending, "Tendances"),
    (T::RecentlyPlayed, "Joués récemment"),
    (T::AllServers, "Tous les serveurs"),
    (T::Categories, "Catégories"),
    (T::All, "Tous"),
    (T::Join, "Rejoindre"),
    (T::AddToList, "Ajouter à la liste"),
    (T::Added, "Dans ta liste"),
    (T::Verified, "Vérifié"),
    (T::PlayersOnline, "{n} en ligne"),
    (T::Offline, "Hors ligne"),
    (T::Loading, "Chargement des serveurs…"),
    (T::EmptyTitle, "Aucun serveur pour l'instant"),
    (
        T::EmptyBody,
        "Les serveurs publiés sur rustic-launcher.com apparaissent ici.",
    ),
    (T::NoResults, "Aucun serveur ne correspond à ta recherche"),
    (T::ErrorTitle, "L'annuaire des serveurs est injoignable"),
    (T::Retry, "Réessayer"),
    (
        T::PublishHint,
        "Publie le tien sur rustic-launcher.com/servers",
    ),
    (T::RusticEdition, "Édition Rustic"),
];

/// Every table, by language code (`en`, `fr`, or a full `pt_br`). English must stay first.
const TABLES: &[(&str, Table)] = &[("en", EN), ("fr", FR)];

/// Tag names of the server directory (`GET /v1/server-listings/catalog`), by language; an unknown
/// tag shows as itself, capitalised.
const TAGS: &[(&str, &[(&str, &str)])] = &[
    (
        "en",
        &[
            ("survival", "Survival"),
            ("smp", "SMP"),
            ("creative", "Creative"),
            ("minigames", "Minigames"),
            ("pvp", "PvP"),
            ("skyblock", "Skyblock"),
            ("oneblock", "OneBlock"),
            ("roleplay", "Roleplay"),
            ("factions", "Factions"),
            ("prison", "Prison"),
            ("parkour", "Parkour"),
            ("adventure", "Adventure"),
            ("modded", "Modded"),
            ("anarchy", "Anarchy"),
            ("economy", "Economy"),
            ("towny", "Towny"),
            ("hardcore", "Hardcore"),
            ("vanilla", "Vanilla"),
            ("bedwars", "BedWars"),
            ("building", "Building"),
        ],
    ),
    (
        "fr",
        &[
            ("survival", "Survie"),
            ("creative", "Créatif"),
            ("minigames", "Mini-jeux"),
            ("roleplay", "Jeu de rôle"),
            ("adventure", "Aventure"),
            ("economy", "Économie"),
            ("building", "Construction"),
        ],
    ),
];

/// The tables to look in for `lang` (a game code such as `fr_fr`), most specific first, English
/// last.
fn chain(lang: &str) -> impl Iterator<Item = &'static str> {
    let lang = lang.to_ascii_lowercase();
    let base = lang.split(['_', '-']).next().unwrap_or("").to_owned();
    let mut out: Vec<&'static str> = Vec::new();
    for want in [lang.as_str(), base.as_str(), "en"] {
        if let Some((code, _)) = TABLES.iter().find(|(c, _)| *c == want)
            && !out.contains(code)
        {
            out.push(code);
        }
    }
    out.into_iter()
}

/// Text `key` in the game's language `lang` (English when no table has it).
pub fn t(key: T, lang: &str) -> &'static str {
    for code in chain(lang) {
        let table = TABLES.iter().find(|(c, _)| *c == code).map(|(_, t)| *t);
        if let Some(text) = table.and_then(|t| t.iter().find(|(k, _)| *k == key)) {
            return text.1;
        }
    }
    // English is complete (tested); never reached.
    ""
}

/// [`T::PlayersOnline`] with its number.
pub fn players_online(n: i64, lang: &str) -> String {
    t(T::PlayersOnline, lang).replace("{n}", &group_digits(n, lang))
}

/// `12345` as `12,345` (English) or `12 345` (French, with a narrow no-break space).
pub fn group_digits(n: i64, lang: &str) -> String {
    let sep = if lang.starts_with("fr") {
        '\u{202F}'
    } else {
        ','
    };
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(sep);
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// A directory tag's name in `lang`.
pub fn tag_name(tag: &str, lang: &str) -> String {
    let base = lang.split(['_', '-']).next().unwrap_or("");
    for code in [lang, base, "en"] {
        if let Some((_, names)) = TAGS.iter().find(|(c, _)| *c == code)
            && let Some((_, name)) = names.iter().find(|(k, _)| *k == tag)
        {
            return (*name).to_owned();
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [T; 29] = [
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

    #[test]
    fn english_and_french_are_complete() {
        for key in ALL {
            assert!(EN.iter().any(|(k, _)| *k == key), "{key:?} in English");
            assert!(FR.iter().any(|(k, _)| *k == key), "{key:?} in French");
        }
        assert_eq!(TABLES[0].0, "en");
    }

    #[test]
    fn regions_fall_back_to_their_language_then_english() {
        assert_eq!(t(T::Discover, "fr_fr"), "Découvrir");
        assert_eq!(t(T::Discover, "fr_ca"), "Découvrir");
        assert_eq!(t(T::MyServers, "fr_fr"), "Mes serveurs");
        assert_eq!(t(T::Discover, "en_us"), "Discover");
        assert_eq!(t(T::Discover, "de_de"), "Discover");
        assert_eq!(t(T::Discover, ""), "Discover");
        assert_eq!(players_online(1234, "en_us"), "1,234 online");
        assert_eq!(players_online(5, "fr_fr"), "5 en ligne");
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
