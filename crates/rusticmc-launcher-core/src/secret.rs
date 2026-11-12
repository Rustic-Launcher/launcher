//! A string that must never be printed: tokens, API keys.

use serde::{Deserialize, Serialize};

/// A secret string. `Debug` and `Display` print `[redacted]`; only [`Secret::expose`] gives the
/// value, so a token can't reach a log line, an error or an IPC reply by accident.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The value. Call it only where the secret leaves for its one legitimate destination (an
    /// `Authorization` header, the keychain, the game's stdin).
    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}

impl std::fmt::Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}

impl From<String> for Secret {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Secret {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_prints() {
        let s = Secret::new("eyJhbGciOi.secret");
        assert_eq!(format!("{s:?}"), "[redacted]");
        assert_eq!(format!("{s}"), "[redacted]");
        assert_eq!(s.expose(), "eyJhbGciOi.secret");
        // Serialization is for the keychain only and keeps the value.
        assert_eq!(serde_json::to_string(&s).unwrap(), "\"eyJhbGciOi.secret\"");
    }
}
