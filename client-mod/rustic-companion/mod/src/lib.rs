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
        "horizon.mods.empty" => Some(t(T::NoMods, &horizon_client::lang::code()).to_owned()),
        _ => None,
    }
}
