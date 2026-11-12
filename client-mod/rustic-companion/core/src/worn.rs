//! What players wear, as the cloud says: asked in batches for the players in view, kept for the
//! answer's TTL (1 minute to 1 hour), the last answer drawn while a new one is on its way, the
//! 4096 most recently seen players remembered. The game thread only reads this cache; the
//! asking happens on the companion's worker thread.
use crate::backoff::Backoff;
use crate::protocol::{MAX_BATCH, Playing, WornAnswer, WornItem};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub const MAX_PLAYERS: usize = 4096;
pub const MIN_TTL: Duration = Duration::from_secs(60);
pub const MAX_TTL: Duration = Duration::from_secs(3600);

#[derive(Clone, Debug)]
struct Known {
    items: Vec<WornItem>,
    playing: Option<(Playing, Instant)>,
    expires: Instant,
    seen: Instant,
}

#[derive(Debug, Default)]
pub struct WornCache {
    known: HashMap<u128, Known>,
    in_flight: Vec<u128>,
    backoff: Backoff,
}

impl WornCache {
    /// What `uuid` wears (empty when nothing or not known yet).
    pub fn worn(&self, uuid: u128) -> &[WornItem] {
        self.known.get(&uuid).map_or(&[], |k| k.items.as_slice())
    }

    /// The emote `uuid` plays and how long ago it started (ms), at `now`.
    pub fn playing(&self, uuid: u128, now: Instant) -> Option<(&WornItem, u64)> {
        let (p, at) = self.known.get(&uuid)?.playing.as_ref()?;
        let since = now.saturating_duration_since(*at).as_millis() as u64;
        Some((&p.item, p.ms + since))
    }

    /// The next batch to ask for among `in_view`: players never asked or past their TTL, not
    /// already being asked; none while backing off. Marks them in flight.
    pub fn next_batch(&mut self, in_view: &[u128], now: Instant) -> Vec<u128> {
        for u in in_view {
            if let Some(k) = self.known.get_mut(u) {
                k.seen = now;
            }
        }
        if !self.backoff.ready(now) || !self.in_flight.is_empty() {
            return Vec::new();
        }
        let mut batch: Vec<u128> = Vec::new();
        for u in in_view {
            let due = self.known.get(u).is_none_or(|k| now >= k.expires);
            if due && !batch.contains(u) {
                batch.push(*u);
                if batch.len() == MAX_BATCH {
                    break;
                }
            }
        }
        self.in_flight = batch.clone();
        batch
    }

    /// The cloud answered for the batch in flight.
    pub fn answered(&mut self, answer: WornAnswer, now: Instant) {
        self.backoff.succeeded();
        let ttl = Duration::from_secs(answer.ttl).clamp(MIN_TTL, MAX_TTL);
        for uuid in std::mem::take(&mut self.in_flight) {
            let hex = crate::uuid_hex(uuid);
            let items = answer.players.get(&hex).cloned().unwrap_or_default();
            let playing = answer.playing.get(&hex).cloned().map(|p| (p, now));
            self.known.insert(
                uuid,
                Known {
                    items,
                    playing,
                    expires: now + ttl,
                    seen: now,
                },
            );
        }
        self.evict();
    }

    /// The batch in flight failed: everything stays as it was; wait before asking again.
    pub fn failed(&mut self, now: Instant) {
        self.in_flight.clear();
        self.backoff.failed(now);
    }

    fn evict(&mut self) {
        if self.known.len() <= MAX_PLAYERS {
            return;
        }
        let mut by_age: Vec<(u128, Instant)> =
            self.known.iter().map(|(u, k)| (*u, k.seen)).collect();
        by_age.sort_by_key(|(_, seen)| *seen);
        for (u, _) in by_age.into_iter().take(self.known.len() - MAX_PLAYERS) {
            self.known.remove(&u);
        }
    }

    pub fn len(&self) -> usize {
        self.known.len()
    }

    pub fn is_empty(&self) -> bool {
        self.known.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str) -> WornItem {
        WornItem {
            id: id.into(),
            version: "1.0.0".into(),
            hash: "h".into(),
        }
    }

    #[test]
    fn batches_cache_and_backoff() {
        let t = Instant::now();
        let mut c = WornCache::default();
        let view: Vec<u128> = (1..=70).collect();
        let b = c.next_batch(&view, t);
        assert_eq!(b.len(), 64);
        // One batch in flight at a time.
        assert!(c.next_batch(&view, t).is_empty());
        let mut a = WornAnswer {
            ttl: 5, // clamped up to a minute
            ..WornAnswer::default()
        };
        a.players.insert(crate::uuid_hex(1), vec![item("hat")]);
        c.answered(a, t);
        assert_eq!(c.worn(1)[0].id, "hat");
        assert!(c.worn(2).is_empty());
        // The rest next, then nobody until the TTL.
        assert_eq!(c.next_batch(&view, t).len(), 6);
        c.answered(WornAnswer::default(), t);
        assert!(c.next_batch(&view, t + Duration::from_secs(30)).is_empty());
        assert_eq!(c.next_batch(&view, t + Duration::from_secs(61)).len(), 64);
        // A failure keeps the answers and backs off.
        c.failed(t + Duration::from_secs(61));
        assert_eq!(c.worn(1)[0].id, "hat");
        assert!(c.next_batch(&view, t + Duration::from_secs(62)).is_empty());
        assert!(!c.next_batch(&view, t + Duration::from_secs(80)).is_empty());
    }

    #[test]
    fn playing_emotes_advance_with_time() {
        let t = Instant::now();
        let mut c = WornCache::default();
        c.next_batch(&[7], t);
        let mut a = WornAnswer::default();
        a.playing.insert(
            crate::uuid_hex(7),
            Playing {
                item: item("wave"),
                ms: 500,
            },
        );
        c.answered(a, t);
        let (i, ms) = c.playing(7, t + Duration::from_millis(250)).unwrap();
        assert_eq!(i.id, "wave");
        assert_eq!(ms, 750);
    }
}
