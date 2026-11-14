//! Play statistics for the Rustic profile (ADR 0168): the launcher tells Rustic Cloud when a game
//! runs, on which edition, and where the player is (menu, singleplayer, or a server's address).
//! The cloud times everything with its own clock; worlds are never named and LAN or private
//! addresses are dropped by the cloud (only public hosts are sent at all).
//!
//! Off with Settings › Privacy › "Share play statistics" (kept in step with the account setting,
//! which the cloud also enforces), and silent when no Rustic account is signed in.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use rusticmc_api_client::types::{PlayPlaceKind, PlaySessionReport};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::cloud_link::ApiCloud;
use crate::discord::Place;

/// A heartbeat this often keeps an open run alive (the cloud closes one after 15 minutes).
pub const HEARTBEAT: Duration = Duration::from_secs(5 * 60);

/// What the launcher saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The game of an instance started (`edition`: `rustic` or `horizon`).
    Started {
        instance: String,
        edition: String,
        game_version: Option<String>,
    },
    /// The player moved (a line of the game's output).
    Moved { instance: String, place: Place },
    /// The game exited.
    Exited { instance: String },
}

/// What to tell the cloud.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Report(PlaySessionReport),
    Heartbeat(Uuid),
    End(Uuid),
}

#[derive(Debug, Clone)]
struct Run {
    id: Uuid,
    edition: String,
    game_version: Option<String>,
    place: (PlayPlaceKind, Option<String>),
}

/// The edition id the cloud keeps for a build source.
pub fn edition_of(source: &str) -> &'static str {
    if source.starts_with("rustic") {
        "rustic"
    } else {
        "horizon"
    }
}

/// Where a place counts: `None` for the launcher itself (not a game place).
fn place_of(place: &Place) -> Option<(PlayPlaceKind, Option<String>)> {
    match place {
        Place::Launcher => None,
        Place::Menu => Some((PlayPlaceKind::Menu, None)),
        Place::Singleplayer => Some((PlayPlaceKind::Singleplayer, None)),
        Place::Server(host) => Some((PlayPlaceKind::Server, Some(host.trim().to_owned()))),
    }
}

/// The runs in progress, one per running instance. Pure: the network is elsewhere.
#[derive(Default)]
pub struct Tracker {
    runs: HashMap<String, Run>,
}

impl Tracker {
    fn report(run: &Run) -> PlaySessionReport {
        PlaySessionReport {
            session_id: run.id,
            edition: run.edition.clone(),
            game_version: run.game_version.clone(),
            place: run.place.0,
            server_address: run.place.1.clone(),
        }
    }

    /// The actions an event leads to. `enabled`: the player's switch (off: nothing starts, and a
    /// run in progress is ended).
    pub fn on(&mut self, event: Event, enabled: bool) -> Vec<Action> {
        match event {
            Event::Started {
                instance,
                edition,
                game_version,
            } => {
                let mut out = Vec::new();
                if let Some(old) = self.runs.remove(&instance) {
                    out.push(Action::End(old.id));
                }
                if !enabled {
                    return out;
                }
                let run = Run {
                    id: Uuid::new_v4(),
                    edition,
                    game_version,
                    place: (PlayPlaceKind::Menu, None),
                };
                out.push(Action::Report(Self::report(&run)));
                self.runs.insert(instance, run);
                out
            }
            Event::Moved { instance, place } => {
                let Some(run) = self.runs.get_mut(&instance) else {
                    return Vec::new();
                };
                if !enabled {
                    let id = run.id;
                    self.runs.remove(&instance);
                    return vec![Action::End(id)];
                }
                match place_of(&place) {
                    Some(p) if p != run.place => {
                        run.place = p;
                        vec![Action::Report(Self::report(run))]
                    }
                    _ => Vec::new(),
                }
            }
            Event::Exited { instance } => self
                .runs
                .remove(&instance)
                .map(|r| vec![Action::End(r.id)])
                .unwrap_or_default(),
        }
    }

    /// One heartbeat per run in progress (none when the switch is off: those runs end).
    pub fn tick(&mut self, enabled: bool) -> Vec<Action> {
        if !enabled {
            return self.runs.drain().map(|(_, r)| Action::End(r.id)).collect();
        }
        self.runs
            .values()
            .map(|r| Action::Heartbeat(r.id))
            .collect()
    }
}

/// The reporter: events in, cloud calls out, on its own task.
pub struct PlayStats {
    tx: mpsc::UnboundedSender<Event>,
}

impl PlayStats {
    pub fn start(
        cloud: ApiCloud,
        rustic: Arc<rusticmc_auth::RusticAccount>,
        enabled: Arc<dyn Fn() -> bool + Send + Sync>,
    ) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
        tauri::async_runtime::spawn(async move {
            let mut tracker = Tracker::default();
            let mut beat = tokio::time::interval(HEARTBEAT);
            beat.tick().await;
            loop {
                let actions = tokio::select! {
                    e = rx.recv() => match e {
                        Some(e) => tracker.on(e, enabled()),
                        None => break,
                    },
                    _ = beat.tick() => tracker.tick(enabled()),
                };
                if actions.is_empty() {
                    continue;
                }
                // Signed out of Rustic: nothing to tell (the cloud would not know who plays).
                let Ok((token, _)) = rustic.access_token_with_expiry().await else {
                    continue;
                };
                for a in actions {
                    let r = match &a {
                        Action::Report(report) => {
                            cloud.play_session(&token, report).await.map(|_| ())
                        }
                        Action::Heartbeat(id) => {
                            cloud.play_session_heartbeat(&token, &id.to_string()).await
                        }
                        Action::End(id) => cloud.play_session_end(&token, &id.to_string()).await,
                    };
                    if let Err(e) = r {
                        tracing::debug!(error = %e, "play statistics not sent");
                    }
                }
            }
        });
        Self { tx }
    }

    pub fn send(&self, event: Event) {
        let _ = self.tx.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn started(i: &str) -> Event {
        Event::Started {
            instance: i.into(),
            edition: "rustic".into(),
            game_version: Some("26.3".into()),
        }
    }

    fn moved(i: &str, place: Place) -> Event {
        Event::Moved {
            instance: i.into(),
            place,
        }
    }

    fn report(a: &Action) -> &PlaySessionReport {
        match a {
            Action::Report(r) => r,
            other => panic!("not a report: {other:?}"),
        }
    }

    #[test]
    fn a_run_reports_each_move_once_and_ends() {
        let mut t = Tracker::default();
        let a = t.on(started("a"), true);
        assert_eq!(a.len(), 1);
        let r = report(&a[0]);
        let run = r.session_id;
        assert_eq!(
            (r.place, r.edition.as_str()),
            (PlayPlaceKind::Menu, "rustic")
        );
        assert_eq!(r.game_version.as_deref(), Some("26.3"));
        // The same place again, or the launcher itself: nothing.
        assert!(t.on(moved("a", Place::Menu), true).is_empty());
        assert!(t.on(moved("a", Place::Launcher), true).is_empty());
        let a = t.on(moved("a", Place::Server("play.example.com".into())), true);
        let r = report(&a[0]);
        assert_eq!(r.session_id, run, "one run, several places");
        assert_eq!(r.place, PlayPlaceKind::Server);
        assert_eq!(r.server_address.as_deref(), Some("play.example.com"));
        assert!(
            t.on(moved("a", Place::Server("play.example.com".into())), true)
                .is_empty()
        );
        let a = t.on(moved("a", Place::Singleplayer), true);
        assert_eq!(report(&a[0]).place, PlayPlaceKind::Singleplayer);
        assert!(
            report(&a[0]).server_address.is_none(),
            "worlds are never named"
        );
        assert_eq!(t.tick(true), vec![Action::Heartbeat(run)]);
        assert_eq!(
            t.on(
                Event::Exited {
                    instance: "a".into()
                },
                true
            ),
            vec![Action::End(run)]
        );
        assert!(t.tick(true).is_empty());
        // Unknown instances are ignored.
        assert!(t.on(moved("zz", Place::Menu), true).is_empty());
        assert!(
            t.on(
                Event::Exited {
                    instance: "zz".into()
                },
                true
            )
            .is_empty()
        );
    }

    #[test]
    fn opted_out_players_report_nothing() {
        let mut t = Tracker::default();
        assert!(t.on(started("a"), false).is_empty());
        assert!(t.on(moved("a", Place::Singleplayer), false).is_empty());
        assert!(t.tick(false).is_empty());
        assert!(
            t.on(
                Event::Exited {
                    instance: "a".into()
                },
                false
            )
            .is_empty()
        );
        // Turned off during a run: the run ends at once, nothing more is sent.
        let a = t.on(started("b"), true);
        let run = report(&a[0]).session_id;
        assert_eq!(
            t.on(moved("b", Place::Singleplayer), false),
            vec![Action::End(run)]
        );
        assert!(t.on(moved("b", Place::Menu), true).is_empty());
        let a = t.on(started("c"), true);
        let run = report(&a[0]).session_id;
        assert_eq!(t.tick(false), vec![Action::End(run)]);
    }

    #[test]
    fn several_instances_have_their_own_runs() {
        let mut t = Tracker::default();
        let a = report(&t.on(started("a"), true)[0]).session_id;
        let b = report(&t.on(started("b"), true)[0]).session_id;
        assert_ne!(a, b);
        assert_eq!(t.tick(true).len(), 2);
        // A restart of the same instance ends the old run first.
        let again = t.on(started("a"), true);
        assert_eq!(again[0], Action::End(a));
        assert_ne!(report(&again[1]).session_id, a);
    }

    #[test]
    fn editions() {
        assert_eq!(edition_of("rustic-edition"), "rustic");
        assert_eq!(edition_of("horizon"), "horizon");
        assert_eq!(edition_of("custom"), "horizon");
    }
}
