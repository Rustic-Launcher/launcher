//! The one error type of the hosting layer. Every variant says what happened in words the UI can
//! show, and carries a stable `code()` the frontend switches on (see launcher/docs/IPC.md).

use serde::Serialize;

/// Result alias for the crate.
pub type Result<T, E = HostingError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum HostingError {
    /// The provider exists in the UI but is not open yet (Rustic Cloud).
    #[error("{provider} is not available yet: {detail}")]
    NotAvailableYet {
        provider: &'static str,
        detail: String,
    },
    /// The provider can't do this (for example uploading an engine to a local server).
    #[error("not supported by this provider: {0}")]
    Unsupported(String),
    /// First connection to a host: the user must confirm its key's fingerprint, then call
    /// `hosting_ssh_trust_host`. Nothing was sent to the server (no credentials).
    #[error(
        "unknown host key for {host}: {algorithm} {fingerprint}. Check it against your provider's console, then trust it"
    )]
    HostKeyUnknown {
        host: String,
        algorithm: String,
        fingerprint: String,
    },
    /// The host presents another key than the pinned one: possibly a man-in-the-middle. Hard error;
    /// only the user removing the pin (after checking the server was reinstalled) clears it.
    #[error(
        "the host key of {host} changed: pinned {expected}, presented {actual}. Someone may be intercepting the connection; nothing was sent. If you reinstalled the server, forget the old key first"
    )]
    HostKeyMismatch {
        host: String,
        expected: String,
        actual: String,
    },
    #[error("authentication failed for {user}@{host}: {detail}")]
    AuthFailed {
        user: String,
        host: String,
        detail: String,
    },
    #[error("could not connect to {host}: {detail}")]
    Connect { host: String, detail: String },
    #[error("ssh: {0}")]
    Ssh(String),
    #[error("sftp: {0}")]
    Sftp(String),
    /// A remote command ended with a non-zero status. `stderr` is the end of its error output.
    #[error("`{command}` failed with status {status}: {stderr}")]
    RemoteCommand {
        command: String,
        status: u32,
        stderr: String,
    },
    /// The VPS runs a system Horizon has no release for, and no archive was supplied.
    #[error("unsupported system {os} {arch}: {detail}")]
    UnsupportedPlatform {
        os: String,
        arch: String,
        detail: String,
    },
    /// Not enough RAM or disk, or the game port is taken.
    #[error("the server is not ready for hosting: {0}")]
    Requirements(String),
    /// Something the user has to do or allow first (accept the EULA, consent to the firewall,
    /// give root/sudo for a system service...).
    #[error("{0}")]
    Precondition(String),
    /// The supervisor answered something we could not read.
    #[error("unexpected answer from the server: {0}")]
    Protocol(String),
    #[error("no server named {0}")]
    UnknownServer(String),
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("keychain: {0}")]
    Keychain(String),
    #[error("checksum mismatch for {what}: expected {expected}, got {actual}")]
    Checksum {
        what: String,
        expected: String,
        actual: String,
    },
    #[error("timed out: {0}")]
    Timeout(String),
    /// A step failed and the steps before it were rolled back (`rollback_errors` lists what could
    /// not be undone, for the troubleshooting page).
    #[error("{step} failed: {source}{}", rollback_note(.rollback_errors))]
    StepFailed {
        step: String,
        #[source]
        source: Box<HostingError>,
        rollback_errors: Vec<String>,
    },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

fn rollback_note(errors: &[String]) -> String {
    if errors.is_empty() {
        String::new()
    } else {
        format!(" (rollback incomplete: {})", errors.join("; "))
    }
}

impl HostingError {
    /// A stable machine-readable code for the UI (`HostingErrorCode` in IPC.md).
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotAvailableYet { .. } => "not_available_yet",
            Self::Unsupported(_) => "unsupported",
            Self::HostKeyUnknown { .. } => "host_key_unknown",
            Self::HostKeyMismatch { .. } => "host_key_mismatch",
            Self::AuthFailed { .. } => "auth_failed",
            Self::Connect { .. } => "connect",
            Self::Ssh(_) => "ssh",
            Self::Sftp(_) => "sftp",
            Self::RemoteCommand { .. } => "remote_command",
            Self::UnsupportedPlatform { .. } => "unsupported_platform",
            Self::Requirements(_) => "requirements",
            Self::Precondition(_) => "precondition",
            Self::Protocol(_) => "protocol",
            Self::UnknownServer(_) => "unknown_server",
            Self::Invalid(_) => "invalid",
            Self::Keychain(_) => "keychain",
            Self::Checksum { .. } => "checksum",
            Self::Timeout(_) => "timeout",
            Self::StepFailed { .. } => "step_failed",
            Self::Io(_) => "io",
        }
    }

    /// The shared `IpcError.code` (launcher/docs/IPC.md "Conventions"); the precise hosting code goes in
    /// `details.hosting_code`.
    pub fn ipc_code(&self) -> &'static str {
        match self {
            Self::NotAvailableYet { .. } | Self::Unsupported(_) => "unavailable",
            Self::HostKeyUnknown { .. } | Self::HostKeyMismatch { .. } => "conflict",
            Self::AuthFailed { .. } => "unauthorized",
            Self::Connect { .. } | Self::Ssh(_) | Self::Sftp(_) | Self::Timeout(_) => "network",
            Self::UnknownServer(_) => "not_found",
            Self::Invalid(_)
            | Self::Precondition(_)
            | Self::Requirements(_)
            | Self::UnsupportedPlatform { .. } => "invalid_argument",
            Self::RemoteCommand { .. }
            | Self::Protocol(_)
            | Self::Keychain(_)
            | Self::Checksum { .. }
            | Self::StepFailed { .. }
            | Self::Io(_) => "internal",
        }
    }

    /// What the IPC layer sends to the UI.
    pub fn to_payload(&self) -> ErrorPayload {
        let fingerprint = match self {
            Self::HostKeyUnknown { fingerprint, .. } => Some(fingerprint.clone()),
            Self::HostKeyMismatch { actual, .. } => Some(actual.clone()),
            _ => None,
        };
        ErrorPayload {
            code: self.code(),
            message: self.to_string(),
            fingerprint,
        }
    }
}

/// The serialised error (`HostingError` in IPC.md's TS types).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ErrorPayload {
    pub code: &'static str,
    pub message: String,
    /// For host key errors: the SHA256 fingerprint to show (and, when unknown, to confirm).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}

impl From<russh::Error> for HostingError {
    fn from(e: russh::Error) -> Self {
        Self::Ssh(e.to_string())
    }
}

impl From<russh_sftp::client::error::Error> for HostingError {
    fn from(e: russh_sftp::client::error::Error) -> Self {
        Self::Sftp(e.to_string())
    }
}

impl From<serde_json::Error> for HostingError {
    fn from(e: serde_json::Error) -> Self {
        Self::Protocol(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_has_code_and_fingerprint() {
        let e = HostingError::HostKeyUnknown {
            host: "vps:22".into(),
            algorithm: "ssh-ed25519".into(),
            fingerprint: "SHA256:abc".into(),
        };
        let p = e.to_payload();
        assert_eq!(p.code, "host_key_unknown");
        assert_eq!(p.fingerprint.as_deref(), Some("SHA256:abc"));
        assert!(p.message.contains("SHA256:abc"));
    }

    #[test]
    fn step_failed_mentions_rollback_problems() {
        let e = HostingError::StepFailed {
            step: "start".into(),
            source: Box::new(HostingError::Timeout("game".into())),
            rollback_errors: vec!["could not remove unit".into()],
        };
        let text = e.to_string();
        assert!(text.contains("start failed"));
        assert!(text.contains("rollback incomplete: could not remove unit"));
    }
}
