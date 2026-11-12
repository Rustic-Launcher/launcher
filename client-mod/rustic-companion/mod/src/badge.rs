//! The Rustic badge in the game (ADR 0072): the Rustic logo left of the names of the players
//! who use Rustic, in the place of Horizon's mark.
//!
//! - The player list and the name tags over heads: `horizon_hooks::Hooks::name_picture`
//!   (Rustic's Horizon patch U14) answers the badge's sprite for a Rustic user; the game then
//!   draws it where Horizon's mark goes and leaves Horizon's mark out for that player. Players
//!   who are not Rustic users keep Horizon's own mark when Horizon says they play Horizon.
//! - Chat: a player's chat line (`client_chat_received`) from a Rustic user starts with the
//!   badge as an inline sprite (`minecraft:object`, atlas kind), which the game draws 8 GUI
//!   pixels square with the text's shadow; the line's own text (narration, log, copy) is left
//!   as the server sent it.
//! - The picture: the official mark (cropped from `assets/rustic-wordmark.png`) set at the screen
//!   size of the game's text for the current GUI scale (`rusticmc_companion_core::badge`), put
//!   into the hooks' sprite slot again when the scale changes.
use crate::state::State;
use horizon_client::text::{InlineObject, Span, Style, Text};
use horizon_hooks::players::SpriteStore;
use rusticmc_companion_core::badge::{self, BadgeCache};
use rusticmc_companion_core::title::Rgba;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// The listed players are read again this often (`Host::client_profiles`).
pub const LIST_EVERY: Duration = Duration::from_secs(2);
/// Chat senders remembered to ask about.
const SENDERS_MAX: usize = 64;

/// The mark alone, from the wordmark.
static MARK: LazyLock<Option<Rgba>> = LazyLock::new(|| {
    crate::title_menu::LOGO
        .as_ref()
        .and_then(badge::mark_of_wordmark)
});

/// The badge's part of the companion's state.
#[derive(Default)]
pub struct Badges {
    pub cache: BadgeCache,
    /// The Rustic session's player (the window's own, while signed in).
    pub own: Option<u128>,
    /// The GUI scale the HUD was last drawn at, and the render side the sprite holds.
    pub scale: Option<f32>,
    pub side_put: Option<u32>,
    /// The players the player list shows, as last read, and when.
    pub listed: Vec<u128>,
    pub listed_at: Option<Instant>,
    /// Recent chat senders, so a sender is asked about even out of the list.
    pub senders: Vec<u128>,
}

impl Badges {
    /// Whether `uuid` gets the badge now.
    pub fn shows(&self, uuid: u128) -> bool {
        badge::shows_badge(uuid, self.own, &self.cache)
    }

    /// Whether the list is due to be read again.
    pub fn list_due(&self, now: Instant) -> bool {
        self.listed_at
            .is_none_or(|t| now.saturating_duration_since(t) >= LIST_EVERY)
    }

    /// The next batch to ask the cloud about: the listed players, those in view, chat senders.
    pub fn next_batch(&mut self, in_view: &[u128], now: Instant) -> Vec<u128> {
        let mut wanted: Vec<u128> = Vec::with_capacity(self.listed.len() + in_view.len());
        wanted.extend(&self.listed);
        wanted.extend(in_view);
        wanted.extend(&self.senders);
        self.cache.next_batch(&wanted, now)
    }

    fn heard(&mut self, sender: u128) {
        if !self.senders.contains(&sender) {
            if self.senders.len() >= SENDERS_MAX {
                self.senders.remove(0);
            }
            self.senders.push(sender);
        }
    }
}

/// Puts the badge's sprite in the atlas when it is missing or the GUI scale changed (the `frame`
/// player hook, before each frame is drawn).
pub fn frame(st: &mut State, sprites: &mut dyn SpriteStore) {
    let side = badge::side_for(st.badges.scale.unwrap_or(2.0));
    if sprites.has(badge::SPRITE) && st.badges.side_put == Some(side) {
        return;
    }
    let Some(mark) = MARK.as_ref() else {
        return;
    };
    let image = badge::badge_image(mark, side);
    sprites.put(badge::SPRITE, badge::sprite_pixels(&image));
    st.badges.side_put = Some(side);
}

/// The badge's sprite for `uuid`'s name, if they use Rustic (`name_picture`).
pub fn picture(st: &State, uuid: u128) -> Option<u64> {
    st.badges.shows(uuid).then_some(badge::SPRITE)
}

/// The inline sprite a chat line starts with: the badge, white (no tint), "Rustic" on hover.
fn chat_span() -> Span {
    Span {
        text: "\u{fffc}".into(),
        style: Style {
            color: Some(0xFF_FFFF),
            hover: Some(Box::new(Text::literal("Rustic"))),
            inline_object: Some(InlineObject::Atlas {
                atlas: "minecraft:gui".into(),
                sprite: badge::sprite_name(),
                fallback: String::new(),
            }),
            ..Style::default()
        },
    }
}

/// A chat line from the server is about to show: a player's (not a system line, not a
/// disguised one) from a Rustic user starts with the badge. Unknown senders are asked about.
pub fn chat_line(st: &mut State, system: bool, sender: u128, message: &mut Text) {
    if system || sender == 0 {
        return;
    }
    if st.badges.shows(sender) {
        message.spans.insert(0, chat_span());
    } else if !st.badges.cache.knows(sender) {
        st.badges.heard(sender);
    }
}
