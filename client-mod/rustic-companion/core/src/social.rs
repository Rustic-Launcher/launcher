//! Friends, invites and messages as the overlay shows them, refreshed from the cloud at most
//! every 30 s while the overlay is open (every 2 minutes otherwise, for notifications), and what
//! changed since the last snapshot (for discreet notifications).
use crate::backoff::Backoff;
use crate::protocol::{Invite, Message, SocialSnapshot};
use std::time::{Duration, Instant};

pub const OPEN_EVERY: Duration = Duration::from_secs(30);
pub const CLOSED_EVERY: Duration = Duration::from_secs(120);

/// Something new since the last snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum News {
    FriendOnline(String),
    Invite { from: String, server: String },
    Message { from: String, text: String },
}

#[derive(Debug, Default)]
pub struct Social {
    pub snapshot: SocialSnapshot,
    last: Option<Instant>,
    asking: bool,
    backoff: Backoff,
    /// Signed in (a session exists); without one the overlay says to sign in from the launcher.
    pub signed_in: bool,
    force: bool,
}

impl Social {
    /// Whether to ask now.
    pub fn due(&mut self, overlay_open: bool, now: Instant) -> bool {
        if !self.signed_in || self.asking || !self.backoff.ready(now) {
            return false;
        }
        let every = if overlay_open {
            OPEN_EVERY
        } else {
            CLOSED_EVERY
        };
        let due = self.force
            || self
                .last
                .is_none_or(|l| now.saturating_duration_since(l) >= every);
        self.force = false;
        if due {
            self.asking = true;
        }
        due
    }

    /// A snapshot arrived: returns what is new (nothing on the first one, to stay discreet).
    pub fn arrived(&mut self, new: SocialSnapshot, now: Instant) -> Vec<News> {
        let first = self.last.is_none();
        self.asking = false;
        self.backoff.succeeded();
        self.last = Some(now);
        let mut news = Vec::new();
        if !first {
            for f in new.friends.iter().filter(|f| f.online) {
                let was = self
                    .snapshot
                    .friends
                    .iter()
                    .any(|o| o.uuid == f.uuid && o.online);
                if !was {
                    news.push(News::FriendOnline(f.name.clone()));
                }
            }
            for Invite { id, from, server } in &new.invites {
                if !self.snapshot.invites.iter().any(|o| &o.id == id) {
                    news.push(News::Invite {
                        from: from.clone(),
                        server: server.clone(),
                    });
                }
            }
            for Message { id, from, text } in &new.messages {
                if !self.snapshot.messages.iter().any(|o| &o.id == id) {
                    news.push(News::Message {
                        from: from.clone(),
                        text: text.clone(),
                    });
                }
            }
        }
        self.snapshot = new;
        news
    }

    pub fn failed(&mut self, now: Instant) {
        self.asking = false;
        self.backoff.failed(now);
    }

    /// Ask again at the next check (after an action, so the overlay reflects it).
    pub fn refresh_soon(&mut self) {
        self.force = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Friend;

    fn friend(name: &str, online: bool) -> Friend {
        Friend {
            uuid: name.into(),
            name: name.into(),
            online,
            activity: None,
        }
    }

    #[test]
    fn news_are_what_changed() {
        let t = Instant::now();
        let mut s = Social {
            signed_in: true,
            ..Social::default()
        };
        assert!(s.due(false, t));
        assert!(!s.due(false, t), "one request at a time");
        let first = SocialSnapshot {
            friends: vec![friend("ana", false)],
            ..SocialSnapshot::default()
        };
        assert!(
            s.arrived(first, t).is_empty(),
            "nothing on the first snapshot"
        );
        assert!(!s.due(true, t + Duration::from_secs(10)));
        assert!(s.due(true, t + Duration::from_secs(31)));
        let second = SocialSnapshot {
            friends: vec![friend("ana", true)],
            invites: vec![Invite {
                id: "i1".into(),
                from: "ana".into(),
                server: "Rustic SMP".into(),
            }],
            messages: vec![],
            emotes: vec![],
        };
        let news = s.arrived(second, t + Duration::from_secs(31));
        assert_eq!(news.len(), 2);
        assert_eq!(news[0], News::FriendOnline("ana".into()));
    }

    #[test]
    fn signed_out_asks_nothing() {
        let mut s = Social::default();
        assert!(!s.due(true, Instant::now()));
    }
}
