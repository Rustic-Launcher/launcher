//! Discreet notifications: shown through the game's own toast slots (`Host::add_toast`), at most
//! one every few seconds, never the same twice in a minute, and none while the player has them
//! off. Fair to the game: never over a screen, never with sound (not a challenge toast).
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub const GAP: Duration = Duration::from_secs(4);
pub const REPEAT: Duration = Duration::from_secs(60);
pub const QUEUE: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// The toast id (`rustic:<kind>`), which a pack can hide (`native_toast_add`).
    pub id: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Default)]
pub struct Notifier {
    queue: VecDeque<Note>,
    last_shown: Option<Instant>,
    recent: VecDeque<(String, Instant)>,
}

impl Notifier {
    pub fn push(&mut self, note: Note, now: Instant) {
        let key = format!("{}|{}|{}", note.id, note.title, note.body);
        self.recent
            .retain(|(_, at)| now.saturating_duration_since(*at) < REPEAT);
        if self.recent.iter().any(|(k, _)| k == &key) || self.queue.contains(&note) {
            return;
        }
        if self.queue.len() == QUEUE {
            self.queue.pop_front();
        }
        self.recent.push_back((key, now));
        self.queue.push_back(note);
    }

    /// The next note to show at `now`, if one may show.
    pub fn next(&mut self, now: Instant) -> Option<Note> {
        if self
            .last_shown
            .is_some_and(|t| now.saturating_duration_since(t) < GAP)
        {
            return None;
        }
        let n = self.queue.pop_front()?;
        self.last_shown = Some(now);
        Some(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(t: &str) -> Note {
        Note {
            id: "rustic:friend".into(),
            title: t.into(),
            body: String::new(),
        }
    }

    #[test]
    fn spaced_and_deduplicated() {
        let t = Instant::now();
        let mut n = Notifier::default();
        n.push(note("Ana is online"), t);
        n.push(note("Ana is online"), t);
        n.push(note("Bob is online"), t);
        assert_eq!(n.next(t).unwrap().title, "Ana is online");
        assert!(n.next(t + Duration::from_secs(1)).is_none());
        assert_eq!(n.next(t + GAP).unwrap().title, "Bob is online");
        assert!(n.next(t + GAP * 3).is_none());
        n.push(note("Ana is online"), t + Duration::from_secs(30));
        assert!(n.next(t + GAP * 4).is_none(), "not again within a minute");
    }
}
