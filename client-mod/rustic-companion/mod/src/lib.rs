//! The Rustic companion inside Horizon (ADR 0070):
//!
//! - [`hooks`]: the trait mod (`horizon_client::mod_api::Hooks`): network and caches on the
//!   client tick, the social overlay and the emote wheel on a screen of the mod's own, a HUD line
//!   for pending invites, discreet toasts, and the Rustic main menu over the title screen.
//! - [`register_platform_hooks`]: Horizon's platform player hooks (`horizon_hooks::Hooks`):
//!   Rustic cosmetics drawn on the players in view (meshes on bones, companions, auras, trail
//!   particles), name tag styles, emote body poses, and the Rustic badge in the place of
//!   Horizon's mark ([`badge`], ADR 0072).
//!
//! The Rustic edition's executable calls both before `horizon_client::entry::main_with`.
//! Nothing here changes what the game sends to a server: everything is drawn by this client
//! only, from the Rustic cloud's answers.
#![forbid(unsafe_code)]

mod badge;
mod canvas;
mod directory;
mod frame;
mod game;
mod net;
mod players;
mod screens;
mod skin;
mod state;
mod title_menu;

pub use state::MOD_ID;
pub use title_menu::ICON_PNG;

/// Installs the Rustic look of every screen's widgets (Rustic's Horizon patch U15, ADR 0160).
/// Call it before the game starts (from `main`); a second skin is refused.
pub fn install_skin() {
    if !horizon_client::ui::skin::install(&skin::SKIN) {
        log::error!("Rustic: another widget skin is installed");
    }
    // Horizon's own settings pages and overlays in Rustic's palette (before the game gives
    // Horizon's: the first look given is kept).
    if !horizon_client::menu_style::install(skin::menu_style()) {
        log::error!("Rustic: the menus' look was given before Rustic's");
    }
    // Every settings page in the layout of Horizon's settings screens, under Rustic's name; the
    // language is chosen on Rustic's own screen (Rustic's Horizon patch U18, ADR 0164).
    horizon_client::options_shell::set_brand("Rustic");
    horizon_client::options_shell::install(true, false);
    horizon_client::options_shell::set_pages_scroll(true);
    horizon_client::options_shell::set_texts(options_texts);
    // Where each installed mod comes from, on the Mods page, and the way to find more
    // (Rustic's Horizon patch U19, ADR 0165).
    horizon_client::options_shell::set_mod_tags(mod_tags);
    horizon_client::options_shell::set_find_mods(find_mods_label, find_mods);
}

/// "Find mods in the launcher", in the game's language.
fn find_mods_label() -> String {
    use rusticmc_companion_core::i18n::{T, t};
    t(T::FindMods, &horizon_client::lang::code()).to_owned()
}

/// Opens the launcher's Explore on its mods.
fn find_mods() {
    title_menu::open_url(screens::FIND_MODS_URL.to_owned());
}

/// The tags of installed mod `id` on the Mods page: where it comes from (the launcher's content
/// ledger, the game's browser record, its file) and "Native" for one built into the game. What
/// the game folder says is read again at most every 2 s.
fn mod_tags(id: &str, native: bool) -> Vec<(String, u32)> {
    use rusticmc_companion_core::i18n::{T, t};
    use rusticmc_companion_core::origins::{self, Origin, Origins};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static READ: Mutex<Option<(Instant, Origins)>> = Mutex::new(None);
    let dir = std::env::current_dir().unwrap_or_default();
    let known = {
        let mut read = READ.lock().unwrap_or_else(|e| e.into_inner());
        if read
            .as_ref()
            .is_none_or(|(at, _)| at.elapsed() > Duration::from_secs(2))
        {
            *read = Some((Instant::now(), Origins::read(&dir)));
        }
        read.as_ref().map(|(_, o)| o.clone()).unwrap_or_default()
    };
    let origin = if id == state::MOD_ID || id == "rustic" {
        // Rustic's own companion, built into the edition.
        Some(Origin::Rustic)
    } else {
        known.of(id, origins::on_disk(&dir, id))
    };
    let lang = horizon_client::lang::code();
    let own = id == state::MOD_ID || id == "rustic";
    origins::tags(origin, native && !own, |o| match o {
        Some(Origin::Horizon) => "Horizon".to_owned(),
        Some(Origin::Rustic) => "Rustic".to_owned(),
        Some(Origin::Modrinth) => "Modrinth".to_owned(),
        Some(Origin::Local) => t(T::OriginLocal, &lang).to_owned(),
        None => t(T::OriginNative, &lang).to_owned(),
    })
}

/// The game window's title: "Rustic · Horizon 26.3".
pub fn window_title() -> String {
    format!("Rustic · Horizon {}", horizon_core::VANILLA_VERSION)
}

/// The trait mod's hooks, for `main_with`'s mod list.
pub fn hooks() -> Box<dyn horizon_client::mod_api::Hooks> {
    Box::new(game::Companion)
}

/// Registers the player-drawing hooks. Call it before the game starts (from `main`): once the
/// game dispatches, the list is sealed and this logs an error.
pub fn register_platform_hooks() {
    if let Err(e) = horizon_hooks::register(&players::PLAYERS) {
        log::error!("Rustic: player hooks registered too late: {e}");
    }
}

/// Rustic's wording for some of the settings screens' texts (the Mods page with no mod), in the
/// game's language.
fn options_texts(key: &str) -> Option<String> {
    use rusticmc_companion_core::i18n::{T, t};
    match key {
        // Its first sentence on a line of its own (the page wraps each line in its row).
        "horizon.mods.empty" => Some(first_sentence_apart(t(
            T::NoMods,
            &horizon_client::lang::code(),
        ))),
        _ => None,
    }
}

/// `text` with a line break after its first sentence (". ", "。", "! ", "? "), if it has more.
fn first_sentence_apart(text: &str) -> String {
    let ends = [". ", "! ", "? ", "。", "！", "？", "। "];
    let at = ends
        .iter()
        .filter_map(|e| text.find(e).map(|i| i + e.len()))
        .min();
    match at {
        Some(i) if i < text.len() => {
            format!("{}\n{}", text[..i].trim_end(), text[i..].trim_start())
        }
        _ => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_empty_mods_line_breaks_after_its_first_sentence() {
        assert_eq!(
            first_sentence_apart("No mods. Find some."),
            "No mods.\nFind some."
        );
        assert_eq!(first_sentence_apart("模组。去找。"), "模组。\n去找。");
        assert_eq!(first_sentence_apart("One sentence."), "One sentence.");
    }
}
