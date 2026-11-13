//! Long jobs the UI shows in the downloads panel (`downloads_list`, `download-progress`,
//! `downloads-changed`): build installs, preparations, server jar and content downloads. Raw
//! downloads of the core download manager appear as tasks of their own (`dl-<n>`).

use crate::dto::DownloadTask;
use rusticmc_launcher_core::{CancellationToken, DownloadEvent, DownloadManager};
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

struct Entry {
    task: DownloadTask,
    cancel: Option<CancellationToken>,
    last: (Instant, u64),
}

/// The task list.
pub struct Tasks {
    app: AppHandle,
    entries: Mutex<Vec<Entry>>,
}

impl Tasks {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            entries: Mutex::new(Vec::new()),
        }
    }

    fn changed(&self) {
        let _ = self.app.emit("downloads-changed", self.list());
    }

    pub fn list(&self) -> Vec<DownloadTask> {
        self.entries
            .lock()
            .expect("tasks")
            .iter()
            .map(|e| e.task.clone())
            .collect()
    }

    /// A new running task; returns its id and cancellation token.
    pub fn start(
        &self,
        kind: &'static str,
        label: impl Into<String>,
        instance_id: Option<String>,
        server_id: Option<String>,
    ) -> (String, CancellationToken) {
        let id = format!("task-{}", uuid::Uuid::new_v4().simple());
        let cancel = CancellationToken::new();
        self.insert(
            DownloadTask {
                id: id.clone(),
                kind,
                label: label.into(),
                instance_id,
                server_id,
                received_bytes: 0,
                total_bytes: None,
                speed_bps: 0,
                state: "running",
                error: None,
                detail: None,
            },
            Some(cancel.clone()),
        );
        (id, cancel)
    }

    fn insert(&self, task: DownloadTask, cancel: Option<CancellationToken>) {
        {
            let mut entries = self.entries.lock().expect("tasks");
            // Keep the list short: finished tasks beyond the last 50 go.
            if entries.len() > 50 {
                entries.retain(|e| matches!(e.task.state, "running" | "queued"));
            }
            entries.push(Entry {
                task,
                cancel,
                last: (Instant::now(), 0),
            });
        }
        self.changed();
    }

    fn update(&self, id: &str, f: impl FnOnce(&mut DownloadTask)) {
        let task = {
            let mut entries = self.entries.lock().expect("tasks");
            let Some(e) = entries.iter_mut().find(|e| e.task.id == id) else {
                return;
            };
            f(&mut e.task);
            let now = Instant::now();
            let dt = now.duration_since(e.last.0).as_secs_f64();
            if dt >= 0.5 {
                let delta = e.task.received_bytes.saturating_sub(e.last.1);
                e.task.speed_bps = (delta as f64 / dt) as u64;
                e.last = (now, e.task.received_bytes);
            }
            e.task.clone()
        };
        let _ = self.app.emit("download-progress", task);
    }

    /// Progress as a fraction (0..1) for tasks without bytes (preparations).
    pub fn fraction(&self, id: &str, fraction: f64) {
        self.update(id, |t| {
            t.total_bytes = Some(1000);
            t.received_bytes = (fraction.clamp(0.0, 1.0) * 1000.0) as u64;
        });
    }

    /// What the task is doing now (shown under its bar).
    pub fn detail(&self, id: &str, text: &str) {
        self.update(id, |t| t.detail = Some(text.to_owned()));
    }

    pub fn bytes(&self, id: &str, received: u64, total: Option<u64>) {
        self.update(id, |t| {
            t.received_bytes = received;
            t.total_bytes = total;
        });
    }

    pub fn finish(&self, id: &str) {
        self.update(id, |t| {
            t.state = "done";
            t.speed_bps = 0;
        });
        self.changed();
    }

    pub fn fail(&self, id: &str, error: impl Into<String>) {
        let error = error.into();
        self.update(id, |t| {
            t.state = if error.contains("cancel") {
                "cancelled"
            } else {
                "failed"
            };
            t.error = Some(error);
            t.speed_bps = 0;
        });
        self.changed();
    }

    /// Cancels a task (or a raw download `dl-<n>`).
    pub fn cancel(&self, id: &str, downloads: &DownloadManager) -> bool {
        if let Some(n) = id.strip_prefix("dl-").and_then(|n| n.parse().ok()) {
            return downloads.cancel(n);
        }
        let token = self
            .entries
            .lock()
            .expect("tasks")
            .iter()
            .find(|e| e.task.id == id)
            .and_then(|e| e.cancel.clone());
        match token {
            Some(t) => {
                t.cancel();
                true
            }
            None => false,
        }
    }

    /// Mirrors the download manager's events.
    pub fn on_download(&self, event: &DownloadEvent) {
        match event {
            DownloadEvent::Queued {
                id, label, size, ..
            } => {
                let label = label.clone().unwrap_or_else(|| "Download".into());
                let kind = if label.starts_with("Horizon") {
                    "game"
                } else if label.starts_with("Server files") {
                    "server"
                } else {
                    "content"
                };
                self.insert(
                    DownloadTask {
                        id: format!("dl-{id}"),
                        kind,
                        label,
                        instance_id: None,
                        server_id: None,
                        received_bytes: 0,
                        total_bytes: *size,
                        speed_bps: 0,
                        state: "queued",
                        error: None,
                        detail: None,
                    },
                    None,
                );
            }
            DownloadEvent::Started { id, .. } => {
                self.update(&format!("dl-{id}"), |t| t.state = "running");
            }
            DownloadEvent::Progress {
                id,
                downloaded,
                total,
                ..
            } => self.bytes(&format!("dl-{id}"), *downloaded, *total),
            DownloadEvent::Finished { id, .. } => self.finish(&format!("dl-{id}")),
            DownloadEvent::Failed { id, error, .. } => {
                self.fail(&format!("dl-{id}"), error.clone())
            }
            DownloadEvent::Cancelled { id, .. } => {
                self.update(&format!("dl-{id}"), |t| t.state = "cancelled");
                self.changed();
            }
        }
    }
}
