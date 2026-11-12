//! The error every Tauri command rejects with (launcher/docs/IPC.md, TS `IpcError`):
//! `{code, message, details?}`. Each launcher crate converts its own errors into it.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcErrorCode {
    NotFound,
    InvalidArgument,
    Conflict,
    Cancelled,
    Network,
    Unauthorized,
    OwnershipRequired,
    ParentalBlock,
    Unavailable,
    Internal,
}

/// A command's failure, for the UI. `message` is for the player; `details.reason` carries the
/// crate's own stable code (`auth.not_owned`, `build.signature`...).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IpcError {
    pub code: IpcErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl IpcError {
    pub fn new(code: IpcErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    /// With `details.reason`.
    pub fn with_reason(code: IpcErrorCode, message: impl Into<String>, reason: &str) -> Self {
        Self {
            code,
            message: message.into(),
            details: Some(serde_json::json!({ "reason": reason })),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(IpcErrorCode::NotFound, message)
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(IpcErrorCode::InvalidArgument, message)
    }
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(IpcErrorCode::Unavailable, message)
    }
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(IpcErrorCode::Internal, message)
    }
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(IpcErrorCode::Conflict, message)
    }
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for IpcError {}

impl From<crate::CoreError> for IpcError {
    fn from(e: crate::CoreError) -> Self {
        Self::with_reason(IpcErrorCode::Internal, e.to_string(), "io")
    }
}

impl From<crate::DownloadError> for IpcError {
    fn from(e: crate::DownloadError) -> Self {
        let code = match &e {
            crate::DownloadError::Cancelled => IpcErrorCode::Cancelled,
            crate::DownloadError::Network { .. } | crate::DownloadError::Http { .. } => {
                IpcErrorCode::Network
            }
            _ => IpcErrorCode::Internal,
        };
        Self::with_reason(code, e.to_string(), "download")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialises_like_the_ts_type() {
        let e = IpcError::with_reason(IpcErrorCode::OwnershipRequired, "Sign in", "auth.not_owned");
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            serde_json::json!({"code": "ownership_required", "message": "Sign in", "details": {"reason": "auth.not_owned"}})
        );
        assert_eq!(
            serde_json::to_value(IpcError::not_found("x")).unwrap(),
            serde_json::json!({"code": "not_found", "message": "x"})
        );
    }
}
