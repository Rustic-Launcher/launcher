//! Tracing: human-readable lines on the console and in daily rotated files
//! (`logs/launcher.YYYY-MM-DD.log`, the newest `keep_files` kept). `log` records (from Tauri and
//! other crates) are forwarded into tracing. Tokens never reach a log line: they are
//! [`crate::Secret`]s, which print `[redacted]`.

use crate::error::{CoreError, Result};
use std::path::Path;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Keeps the file writer flushing; drop it at exit.
pub struct LogGuard {
    _file: tracing_appender::non_blocking::WorkerGuard,
}

/// Sets up the global subscriber. `level` is an `EnvFilter` directive (`RUST_LOG` overrides it).
/// Fails if a global subscriber is already set.
pub fn init(log_dir: &Path, level: &str, keep_files: usize) -> Result<LogGuard> {
    std::fs::create_dir_all(log_dir).map_err(|e| CoreError::io(log_dir, e))?;
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("launcher")
        .filename_suffix("log")
        .max_log_files(keep_files.max(1))
        .build(log_dir)
        .map_err(|e| CoreError::Logging(e.to_string()))?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true))
        .with(fmt::layer().with_ansi(false).with_writer(writer))
        .try_init()
        .map_err(|e| CoreError::Logging(e.to_string()))?;
    Ok(LogGuard { _file: guard })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_log_file() {
        let dir = tempfile::tempdir().unwrap();
        let guard = init(dir.path(), "info", 3).unwrap();
        tracing::info!(token = %crate::Secret::new("abc"), "hello from the test");
        drop(guard);
        let files: Vec<_> = std::fs::read_dir(dir.path()).unwrap().flatten().collect();
        assert_eq!(files.len(), 1);
        let text = std::fs::read_to_string(files[0].path()).unwrap();
        assert!(text.contains("hello from the test"));
        assert!(text.contains("[redacted]") && !text.contains("abc\n"));
    }
}
