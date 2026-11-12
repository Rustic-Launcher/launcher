//! The owner gate (launcher/AGENTS.md: "Owning the game is mandatory").
//!
//! - Play with a Microsoft account needs that account to own Minecraft: Java Edition, asked again
//!   at every launch (horizon/docs/microsoft-account.md, "The owner gate").
//! - **Offline profiles** (LAN, no network) are allowed only while some linked Microsoft account
//!   passed an ownership check within the revalidation window (30 days by default). The answers
//!   are kept with the tokens in the OS keychain, so `accounts.json` can't say yes.

use serde::{Deserialize, Serialize};

/// The last ownership answer for one account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnershipRecord {
    pub owns: bool,
    /// When the Minecraft services last answered (unix seconds).
    pub checked_at: u64,
}

/// Whether offline play is open now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum OfflineAccess {
    /// Open until `expires_at`.
    Allowed { verified_at: u64, expires_at: u64 },
    /// No linked account ever passed the check.
    NeverVerified,
    /// The last successful check is older than the window: sign in (online) once.
    Expired { last_verified_at: u64 },
}

impl OfflineAccess {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed { .. })
    }

    /// What the player reads when offline play is refused.
    pub fn refusal(&self) -> Option<String> {
        match self {
            Self::Allowed { .. } => None,
            Self::NeverVerified => Some(
                "Offline play needs a Microsoft account that owns Minecraft: Java Edition: sign in once first."
                    .into(),
            ),
            Self::Expired { .. } => Some(
                "Offline play has expired: connect to the internet and sign in so the launcher can check your game licence again."
                    .into(),
            ),
        }
    }
}

/// Clock skew tolerated on a stored timestamp from the future.
const FUTURE_SKEW: u64 = 300;

/// The rule, with its window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnershipPolicy {
    pub window_seconds: u64,
}

impl OwnershipPolicy {
    pub fn new(window: std::time::Duration) -> Self {
        Self {
            window_seconds: window.as_secs(),
        }
    }

    /// Offline access from every linked account's last answer, at `now`. Only answers of "owns"
    /// count; a timestamp further than five minutes in the future is ignored (a clock set back,
    /// or an edited record, can't stretch the window).
    pub fn evaluate<'a>(
        &self,
        records: impl IntoIterator<Item = &'a OwnershipRecord>,
        now: u64,
    ) -> OfflineAccess {
        let last = records
            .into_iter()
            .filter(|r| r.owns && r.checked_at <= now + FUTURE_SKEW)
            .map(|r| r.checked_at)
            .max();
        match last {
            None => OfflineAccess::NeverVerified,
            Some(at) if now.saturating_sub(at) <= self.window_seconds => OfflineAccess::Allowed {
                verified_at: at,
                expires_at: at + self.window_seconds,
            },
            Some(at) => OfflineAccess::Expired {
                last_verified_at: at,
            },
        }
    }

    /// Whether one account's own last answer lets it start offline (no network at Play).
    pub fn account_may_start_offline(&self, record: &OwnershipRecord, now: u64) -> bool {
        self.evaluate([record], now).is_allowed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = 86_400;
    fn policy() -> OwnershipPolicy {
        OwnershipPolicy {
            window_seconds: 30 * DAY,
        }
    }
    fn owns(at: u64) -> OwnershipRecord {
        OwnershipRecord {
            owns: true,
            checked_at: at,
        }
    }

    #[test]
    fn no_account_means_no_offline_play() {
        assert_eq!(policy().evaluate([], 1000), OfflineAccess::NeverVerified);
    }

    #[test]
    fn a_non_owner_never_opens_offline_play() {
        let r = OwnershipRecord {
            owns: false,
            checked_at: 1000,
        };
        assert_eq!(policy().evaluate([&r], 1000), OfflineAccess::NeverVerified);
    }

    #[test]
    fn within_the_window_it_is_open() {
        let now = 100 * DAY;
        let r = owns(now - 29 * DAY);
        assert_eq!(
            policy().evaluate([&r], now),
            OfflineAccess::Allowed {
                verified_at: now - 29 * DAY,
                expires_at: now + DAY
            }
        );
    }

    #[test]
    fn past_the_window_it_closes() {
        let now = 100 * DAY;
        let r = owns(now - 31 * DAY);
        assert_eq!(
            policy().evaluate([&r], now),
            OfflineAccess::Expired {
                last_verified_at: now - 31 * DAY
            }
        );
        assert!(policy().evaluate([&r], now).refusal().is_some());
    }

    #[test]
    fn the_newest_owner_answer_counts() {
        let now = 100 * DAY;
        let old = owns(now - 60 * DAY);
        let fresh = owns(now - DAY);
        assert!(policy().evaluate([&old, &fresh], now).is_allowed());
    }

    #[test]
    fn a_future_timestamp_is_ignored() {
        let now = 100 * DAY;
        let forged = owns(now + 365 * DAY);
        assert_eq!(
            policy().evaluate([&forged], now),
            OfflineAccess::NeverVerified
        );
        // Small skew is fine.
        assert!(policy().evaluate([&owns(now + 60)], now).is_allowed());
    }
}
