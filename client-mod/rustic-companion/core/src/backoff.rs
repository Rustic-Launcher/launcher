//! Retry delays after failures: 15 s, doubling, at most 10 minutes (as Horizon's own cosmetics
//! service client waits).
use std::time::{Duration, Instant};

pub const FIRST: Duration = Duration::from_secs(15);
pub const MAX: Duration = Duration::from_secs(600);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Backoff {
    failures: u32,
    until: Option<Instant>,
}

impl Backoff {
    /// Whether a request may be sent at `now`.
    pub fn ready(&self, now: Instant) -> bool {
        self.until.is_none_or(|u| now >= u)
    }

    pub fn failed(&mut self, now: Instant) {
        let wait = FIRST.saturating_mul(1 << self.failures.min(6)).min(MAX);
        self.failures = self.failures.saturating_add(1);
        self.until = Some(now + wait);
    }

    pub fn succeeded(&mut self) {
        *self = Backoff::default();
    }

    pub fn failures(&self) -> u32 {
        self.failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_up_to_ten_minutes() {
        let t = Instant::now();
        let mut b = Backoff::default();
        assert!(b.ready(t));
        b.failed(t);
        assert!(!b.ready(t + Duration::from_secs(14)));
        assert!(b.ready(t + Duration::from_secs(15)));
        for _ in 0..10 {
            b.failed(t);
        }
        assert!(b.ready(t + MAX));
        assert!(!b.ready(t + MAX - Duration::from_secs(1)));
        b.succeeded();
        assert!(b.ready(t));
    }
}
