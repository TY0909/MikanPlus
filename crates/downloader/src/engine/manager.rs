//! The download manager handle and the state shared between the UI and the background thread.
//!
//! Data flow:
//! - UI → background: commands are sent through [`DownloadManager`]'s channel.
//! - Background → UI: the background writes into [`EngineState`] (task snapshot, connection
//!   status, events, version) and the UI polls it through [`DownloadManager`]'s methods.
//!
//! There is exactly one [`EngineState`] per engine (created in [`DownloadManager::start`] and
//! `Arc`-shared with the background), so all shared state has a single owner instead of scattered
//! module globals.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use storage::paths;

use crate::data::model::{
    DownloadCmd, DownloadError, DownloadEvent, DownloadStatus, PendingAdd, TaskView,
};

/// Download manager (a thread-safe handle held by the UI).
///
/// Owns the engine's shared state and the command channel; all UI access to the background goes
/// through this handle.
pub struct DownloadManager {
    cmd_tx: UnboundedSender<DownloadCmd>,
    state: Arc<EngineState>,
}

impl DownloadManager {
    /// Start the download background (dedicated thread + tokio runtime). Call once per application lifetime.
    pub fn start() -> Arc<Self> {
        Self::start_with_base(paths::app_data_dir())
    }

    /// Start the download background with a configurable data root (for test isolation).
    pub fn start_with_base(base: PathBuf) -> Arc<Self> {
        let (cmd_tx, cmd_rx) = unbounded_channel::<DownloadCmd>();
        let state = Arc::new(EngineState::default());
        let background_state = state.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("构建下载 tokio runtime 失败");
            rt.block_on(crate::engine::runtime::run_loop(
                cmd_rx,
                background_state,
                base,
            ));
        });
        Arc::new(Self { cmd_tx, state })
    }

    /// Send a command (thread-safe, non-blocking). Returns `Err` when the receiver has exited.
    pub fn send(&self, cmd: DownloadCmd) -> Result<(), DownloadError> {
        self.cmd_tx
            .send(cmd)
            .map_err(|_| DownloadError::EngineUnavailable)
    }

    /// Read the task snapshot (UI polling).
    pub fn snapshot(&self) -> Vec<TaskView> {
        self.state.snapshot()
    }

    /// Read connection status (UI polling; diagnoses NAT / inbound reachability).
    pub fn status(&self) -> DownloadStatus {
        self.state.status()
    }

    /// Read by UI polling: the snapshot version (a change means a redraw is needed).
    pub fn version(&self) -> u64 {
        self.state.version()
    }

    /// Drain the backlog of background events (consumed by UI polling, used for in-app notifications).
    pub fn take_events(&self) -> Vec<DownloadEvent> {
        self.state.take_events()
    }
}

/// The single state object shared between the UI thread and the background thread.
///
/// The background produces (writes) it; the UI polls (reads) it through [`DownloadManager`].
/// Interior mutability keeps it shareable across the thread boundary while still being one named
/// owner of the download data.
#[derive(Default)]
pub(crate) struct EngineState {
    snapshot: Mutex<Vec<TaskView>>,
    status: Mutex<DownloadStatus>,
    events: Mutex<Vec<DownloadEvent>>,
    /// Incremented on every snapshot/status change so UI polling can trigger a redraw.
    version: AtomicU64,
    /// In-progress add requests (during metadata fetching, the task is not yet registered in librqbit).
    pending: Mutex<HashMap<String, PendingAdd>>,
    /// Cancelled info_hashes (cancelled while fetching metadata; cleaned up right after completion).
    cancelled: Mutex<HashSet<String>>,
}

impl EngineState {
    pub(crate) fn snapshot(&self) -> Vec<TaskView> {
        self.snapshot.lock().unwrap().clone()
    }

    /// Replace the snapshot; bump the version only when the content actually changed, so idle
    /// polling does not trigger pointless redraws.
    pub(crate) fn set_snapshot(&self, views: Vec<TaskView>) {
        let mut guard = self.snapshot.lock().unwrap();
        if *guard != views {
            *guard = views;
            self.bump_version();
        }
    }

    pub(crate) fn status(&self) -> DownloadStatus {
        self.status.lock().unwrap().clone()
    }

    /// Replace the connection status; bump the version only when it actually changed.
    pub(crate) fn set_status(&self, status: DownloadStatus) {
        let mut guard = self.status.lock().unwrap();
        if *guard != status {
            *guard = status;
            self.bump_version();
        }
    }

    pub(crate) fn version(&self) -> u64 {
        self.version.load(Ordering::Relaxed)
    }

    fn bump_version(&self) {
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn push_event(&self, event: DownloadEvent) {
        self.events.lock().unwrap().push(event);
    }

    pub(crate) fn take_events(&self) -> Vec<DownloadEvent> {
        std::mem::take(&mut *self.events.lock().unwrap())
    }

    /// Register an in-progress add request.
    pub(crate) fn pending_insert(&self, hash: String, title: String, output_dir: PathBuf) {
        self.pending
            .lock()
            .unwrap()
            .insert(hash, PendingAdd { title, output_dir });
    }

    /// Remove an in-progress add request, returning whether it existed before.
    pub(crate) fn pending_remove(&self, hash: &str) -> bool {
        self.pending.lock().unwrap().remove(hash).is_some()
    }

    /// All current in-progress add requests (hash → request).
    pub(crate) fn pending_snapshot(&self) -> HashMap<String, PendingAdd> {
        self.pending.lock().unwrap().clone()
    }

    /// Mark a task as cancelled during metadata fetching.
    pub(crate) fn cancelled_insert(&self, hash: &str) {
        self.cancelled.lock().unwrap().insert(hash.to_string());
    }

    /// Take and clear the cancellation mark, returning whether the task was cancelled while
    /// fetching metadata.
    pub(crate) fn cancelled_take(&self, hash: &str) -> bool {
        self.cancelled.lock().unwrap().remove(hash)
    }
}
