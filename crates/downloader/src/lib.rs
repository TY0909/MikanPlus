//! Download management module (embedded librqbit).
//!
//! Threading model:
//! - A background thread runs the tokio runtime and owns the [`librqbit::Session`].
//! - The UI communicates with the background through a command channel
//!   (`Add` / `Cancel`) without blocking.
//! - The background produces a task snapshot every second; the UI polls it
//!   (reusing the existing 500ms polling mechanism).
//!
//! Persistence:
//! - librqbit's `SessionPersistenceConfig::Json` automatically saves/restores tasks
//!   (torrent state, downloaded piece bitmap, output directory), so downloads resume
//!   after a restart.
//! - Business metadata (episode title, etc.) is stored in the same directory as
//!   `<info_hash>.json`.
//!
//! Tracker policy: when adding a task, the trackers carried by the mikan magnet are
//! merged with the public list (see [`TRACKERS`]) into one unified, deduplicated list;
//! their origin is not distinguished.
//!
//! The module split is organized around "data" and "the flow of data":
//! - `data`: what the data looks like (in-memory model / on-disk metadata)
//! - `engine`: data shared between the UI and the background, plus the loop that drives it
//! - `support`: pure logic for magnets, file paths, and formatting

mod data;
mod engine;
mod support;

pub use data::model::{
    DownloadCmd, DownloadError, DownloadEvent, DownloadStatus, TRACKERS, TaskState, TaskView,
};
pub use engine::manager::DownloadManager;
pub use support::files::ensure_output_dir;
pub use support::format::{format_percent, format_rate};
pub use support::magnet::magnet_info_hash;
