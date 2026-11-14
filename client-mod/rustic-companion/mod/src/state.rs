//! The companion's state, shared by the trait mod's hooks (game thread) and the player hooks
//! (render thread): one mutex, held briefly, never across a network call.
use crate::net::{Done, Net};
use horizon_hooks::players::{BodyPose, Mesh};
use rusticmc_companion_core::config::CompanionConfig;
use rusticmc_companion_core::notify::{Note, Notifier};
use rusticmc_companion_core::overlay::{Layout, Tab};
use rusticmc_companion_core::protocol::{Endpoints, WornItem};
use rusticmc_companion_core::social::{News, Social};
use rusticmc_companion_core::store::CosmeticStore;
use rusticmc_companion_core::trail::Emitter;
use rusticmc_companion_core::worn::WornCache;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, OnceLock};
use std::time::Instant;

/// The mod's id: its own screen, its toasts, its data folder.
pub const MOD_ID: &str = "rustic_companion";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Overlay,
    Wheel,
}

#[derive(Default)]
pub struct Ui {
    /// Which of the mod's screens is open (None: closed).
    pub mode: Option<Mode>,
    pub tab: Tab,
    /// The overlay as last drawn, for clicks.
    pub layout: Layout,
    /// Close the mod's screen at the next tick (clicks only get a read-only host).
    pub pending_close: bool,
}

/// The emote the window's own player plays.
pub struct OwnEmote {
    pub item: WornItem,
    pub pose: Option<Arc<dyn BodyPose>>,
}

#[derive(Default)]
pub struct State {
    pub cfg: CompanionConfig,
    /// Where the player is, told to the launcher for Discord (`presence`).
    pub presence: rusticmc_companion_core::presence::Reporter,
    pub started: bool,
    pub worn: WornCache,
    pub store: CosmeticStore,
    pub social: Social,
    pub notes: Notifier,
    pub ui: Ui,
    pub own_emote: Option<OwnEmote>,
    /// Poses laid over other players' models, by UUID: the bundle hash and the pose.
    pub others: HashMap<u128, (String, Arc<dyn BodyPose>)>,
    /// Players in view (from the player models of the last client tick).
    pub in_view: Vec<u128>,
    pub connection: u64,
    pub trails: HashMap<u128, Emitter>,
    /// Meshes handed to the renderer, by (sprite, root): built once, kept while drawn.
    pub meshes: HashMap<(u64, usize), Arc<Mesh>>,
    /// The Rustic main menu over the title screen.
    pub title: crate::title_menu::TitleMenu,
    /// Who uses Rustic, and the badge's sprite (ADR 0072).
    pub badges: crate::badge::Badges,
    /// Rustic's language screen over the title (ADR 0160).
    pub languages: crate::screens::language::LangScreen,
    /// The multiplayer screen's Discover tab and the server directory (ADR 0160).
    pub discover: crate::screens::DiscoverUi,
    pub directory: crate::directory::Directory,
    /// Rustic's server list and world list pages, and its pause menu (ADR 0164).
    pub servers_page: crate::screens::pages::PageUi,
    pub worlds_page: crate::screens::pages::PageUi,
    pub pause: crate::screens::pause::PauseUi,
}

pub struct Shared {
    pub state: Mutex<State>,
    pub net: OnceLock<Net>,
}

pub static SHARED: LazyLock<Shared> = LazyLock::new(|| Shared {
    state: Mutex::new(State::default()),
    net: OnceLock::new(),
});

/// The state, even after a panic poisoned the lock (a hook must never panic in turn).
pub fn lock() -> MutexGuard<'static, State> {
    SHARED.state.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn net() -> Option<&'static Net> {
    SHARED.net.get()
}

/// Runs a hook's body; a panic is logged and swallowed, `fallback` returned: a panic in a hook
/// would end the game (horizon/docs/native-mods.md). Never call a `Host` helper that may run
/// hooks while holding [`lock`]: the mutex is not reentrant.
pub fn guard_or<R>(hook: &str, fallback: R, f: impl FnOnce() -> R) -> R {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(_) => {
            log::error!("Rustic companion: hook {hook} failed; skipped");
            fallback
        }
    }
}

pub fn guard(hook: &str, f: impl FnOnce()) {
    guard_or(hook, (), f)
}

impl State {
    /// Loads the config and starts the network once, with the launcher's one-time companion
    /// token (ADR 0008).
    pub fn start(&mut self, config_text: Option<String>) {
        if self.started {
            return;
        }
        self.started = true;
        self.cfg = CompanionConfig::load(config_text.as_deref(), |k| std::env::var(k).ok());
        let net = Net::start(Endpoints {
            api: self.cfg.api_base.clone(),
            cdn: self.cfg.cdn_base.clone(),
            local: std::env::var("RUSTIC_COMPANION_ENDPOINT").ok(),
        });
        if let Ok(token) = std::env::var("RUSTIC_COMPANION_TOKEN")
            && !token.is_empty()
        {
            net.send(crate::net::Job::Session(token));
        }
        let _ = SHARED.net.set(net);
    }

    /// Applies what the network finished.
    pub fn apply(&mut self, done: Done, now: Instant) {
        match done {
            Done::Session(Ok(s)) => {
                self.badges.own = rusticmc_companion_core::parse_uuid(&s.uuid);
                self.social.signed_in = true;
                self.social.refresh_soon();
            }
            Done::Session(Err(e)) => log::warn!("Rustic: no session ({e}): signed out"),
            Done::Worn(Ok(a)) => self.worn.answered(a, now),
            Done::Worn(Err(e)) => {
                log::debug!("Rustic: worn: {e}");
                self.worn.failed(now);
            }
            Done::Badges(Ok(rustic)) => self.badges.cache.answered(&rustic, now),
            Done::Badges(Err(e)) => {
                log::debug!("Rustic: badges: {e}");
                self.badges.cache.failed(now);
            }
            Done::Bundle { hash, bytes } => {
                let r = bytes.and_then(|b| self.store.arrived(&hash, &b, now));
                if let Err(e) = r {
                    log::warn!("Rustic: cosmetic {hash}: {e}");
                    self.store.failed(&hash, now);
                }
            }
            Done::Social(Ok(s)) => {
                for n in self.social.arrived(s, now) {
                    let note = match n {
                        News::FriendOnline(name) => Note {
                            id: format!("{MOD_ID}:friend"),
                            title: "Rustic".into(),
                            body: format!("{name} is online"),
                        },
                        News::Invite { from, server } => Note {
                            id: format!("{MOD_ID}:invite"),
                            title: "Rustic invite".into(),
                            body: format!("{from} invites you to {server}"),
                        },
                        News::Message { from, text } => Note {
                            id: format!("{MOD_ID}:message"),
                            title: from,
                            body: text.chars().take(80).collect(),
                        },
                    };
                    if self.cfg.notifications {
                        self.notes.push(note, now);
                    }
                }
            }
            Done::Social(Err(e)) => {
                log::debug!("Rustic: social: {e}");
                self.social.failed(now);
            }
            Done::Action { what, result } => {
                let body = match (what, result) {
                    ("invite", Ok(())) => "Invite sent".to_owned(),
                    ("accept", Ok(())) => "Rustic Launcher will take you there".to_owned(),
                    (w, Err(e)) => format!("{w} failed: {e}"),
                    (w, Ok(())) => format!("{w}: done"),
                };
                self.notes.push(
                    Note {
                        id: format!("{MOD_ID}:action"),
                        title: "Rustic".into(),
                        body,
                    },
                    now,
                );
                self.social.refresh_soon();
            }
        }
    }
}
