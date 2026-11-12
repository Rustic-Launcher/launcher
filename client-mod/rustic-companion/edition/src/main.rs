//! `rustic-client`: the Rustic edition, "based on Horizon": Horizon's game with the Rustic
//! companion linked in (ADRs 0006, 0070). Mirrors `horizon-official`'s `main` and the executable
//! Horizon's launcher generates for trait mods (`name_mods`, `main_with` and the mod list).
//! Rustic's window look (ADR 0148): its icon, and on Windows its title bar ([`chrome`], the one
//! module allowed `unsafe`, for the Win32 calls).
#![deny(unsafe_code)]

mod chrome;

#[global_allocator]
static ALLOCATOR: horizon_game::Allocator = horizon_game::ALLOCATOR;

/// The mod API level this game serves, readable without running it (launchers check it).
static MOD_API_LEVEL: [u8; horizon_game::MOD_API_LEVEL.len()] = horizon_game::MOD_API_LEVEL;

fn main() {
    std::hint::black_box(&MOD_API_LEVEL);
    // Rustic's icon for the window and the task bar, before the game gives Horizon's (the first
    // one given is kept).
    horizon_client::window::set_icon_png(rusticmc_companion::ICON_PNG);
    chrome::start(rusticmc_companion::window_title());
    // Play with Friends (`friend:` addresses), as Horizon's executable registers it. Horizon's
    // heartbeat, official-server entry and Discord status are left out (ADR 0006). Horizon's own
    // cosmetics stay on (the user's decision): they register inside the game, after ours.
    if let Err(e) = horizon_hooks::register(&horizon_friends::game::HOOKS) {
        eprintln!("Play with Friends did not register: {e}");
    }
    // Rustic's player drawing, before the game seals the hook list.
    rusticmc_companion::register_platform_hooks();
    let mods: horizon_client::mod_api::Mods = vec![rusticmc_companion::hooks()].into();
    horizon_client::mod_api::name_mods(&[rusticmc_companion::MOD_ID]);
    horizon_client::entry::main_with(mods);
}
