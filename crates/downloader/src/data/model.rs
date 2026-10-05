//! Download data model: tasks, states, commands, events, and errors.
//!
//! This module only describes "what the data looks like". How the background produces
//! data and how it flows back to the UI is the responsibility of
//! [`crate::engine::manager`] / [`crate::engine::snapshot`] / [`crate::engine::commands`].

use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;

/// Tracker list (unified; protocol and priority are not distinguished).
/// Sources: qBittorrent's default list + live entries from ngosang/trackerslist + the ACG ecosystem.
/// Note: when adding a task, the trackers carried by the mikan magnet are merged with
/// this list (deduplicated) and take effect uniformly; their origin is not distinguished.
pub const TRACKERS: &[&str] = &[
    "http://tracker.opentrackr.org:1337/announce",
    "https://tracker.gbitt.info:443/announce",
    "udp://tracker.opentrackr.org:1337/announce",
    "udp://tracker.openbittorrent.com:6969/announce",
    "http://tracker.openbittorrent.com:80/announce",
    "https://open.acgnxtracker.com/announce",
    "udp://open.stealth.si:80/announce",
    "https://tr.bangumi.moe:9696/announce",
    "udp://tracker.torrent.eu.org:451/announce",
    "http://tracker.bt4g.com:2095/announce",
    "https://tracker.zhuqiy.com:443/announce",
    "http://share.camoe.cn:8080/announce",
    "http://t.nyaatracker.com/announce",
    "https://tracker.pmman.tech:443/announce",
    "https://tracker.nekomi.cn:443/announce",
    "http://opentracker.acgnx.se/announce",
    "udp://exodus.desync.com:6969/announce",
    "udp://open.demonii.com:1337/announce",
    "udp://explodie.org:6969/announce",
    "udp://zer0day.ch:1337/announce",
    "udp://tracker-udp.gbitt.info:80/announce",
    "udp://tracker.tiny-vps.com:6969/announce",
    "udp://opentracker.i2p.rocks:6969/announce",
    "udp://tracker.moeking.me:6969/announce",
    "udp://tracker.cyberia.is:6969/announce",
    "udp://tracker.leechers-paradise.org:6969/announce",
    "udp://tracker.internetwarriors.net:1337/announce",
];

/// Timeout for adding a magnet (fetching metadata).
pub(crate) const METADATA_TIMEOUT: Duration = Duration::from_secs(60);

/// Download-related errors. Variants are distinguished so the UI can give targeted
/// hints without exposing low-level details.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DownloadError {
    /// The download background has exited (command channel disconnected).
    #[error("下载引擎不可用")]
    EngineUnavailable,
    /// The download engine failed to start.
    #[error("下载引擎启动失败")]
    EngineInit,
    /// Failed to parse the magnet link.
    #[error("磁力链接解析失败")]
    MagnetParse,
    /// Failed to add the download task.
    #[error("添加下载任务失败")]
    AddFailed,
    /// Timed out fetching resource metadata.
    #[error("获取资源信息超时")]
    MetadataTimeout,
    /// An error occurred while downloading (reported by librqbit).
    #[error("下载出错")]
    Torrent,
}

impl DownloadError {
    /// A brief user-facing description (without technical details).
    pub fn user_message(&self) -> &'static str {
        match self {
            DownloadError::EngineUnavailable => "下载功能不可用",
            DownloadError::EngineInit => "下载引擎启动失败",
            DownloadError::MagnetParse => "磁力链接无效",
            DownloadError::AddFailed => "添加下载任务失败",
            DownloadError::MetadataTimeout => "获取资源信息超时",
            DownloadError::Torrent => "下载出错",
        }
    }

    /// The next step the user should check or take.
    pub fn user_hint(&self) -> &'static str {
        match self {
            DownloadError::EngineUnavailable => "请重启应用",
            DownloadError::EngineInit => "请重启应用",
            DownloadError::MagnetParse => "该资源的磁力链接可能无效",
            DownloadError::AddFailed => "该资源可能暂无可用来源，请稍后重试",
            DownloadError::MetadataTimeout => "该资源可能暂无可用来源，请稍后重试",
            DownloadError::Torrent => "请取消后重新下载",
        }
    }
}

/// Task state visible to the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    /// Fetching metadata / initial verification.
    Initializing,
    /// Downloading.
    Downloading,
    /// Download complete.
    Completed,
    /// The task is complete but its files were deleted externally (the UI returns to the "download" state).
    Missing,
    /// Errored (no automatic retry; waits for user action).
    Error(DownloadError),
}

impl TaskState {
    /// The task is still in the first two phases covered by download observation.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Initializing | Self::Downloading)
    }
}

/// Task snapshot (read by UI polling).
#[derive(Debug, Clone, PartialEq)]
pub struct TaskView {
    /// info_hash (hexadecimal; unique task identifier).
    pub id: String,
    /// Display name (episode title).
    pub title: String,
    /// 0.0 ~ 1.0
    pub progress: f64,
    /// Download rate in B/s.
    pub download_rate: u64,
    /// Upload rate in B/s.
    pub upload_rate: u64,
    pub downloaded: u64,
    pub total: u64,
    pub state: TaskState,
    /// Number of currently active (connected) peers.
    pub peers: usize,
    /// Paths of all video files once complete. A single video opens directly; multiple
    /// videos open a collection page.
    pub video_files: Vec<PathBuf>,
    /// Task output directory (used to locate tasks by directory when unsubscribing/cleaning up).
    pub output_dir: Option<PathBuf>,
}

/// Connection status of the download engine (diagnoses NAT / inbound reachability).
///
/// A `listen_port == 0`, or a `peers_live` that stays at 0 for a long time, usually
/// means the app is behind NAT/CGNAT with no usable IPv6 inbound path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadStatus {
    /// Inbound listening port (0 means not listening).
    pub listen_port: u16,
    /// Whether listening is dual-stack (IPv4 + IPv6).
    pub ipv6: bool,
    /// Number of DHT routing-table nodes (IPv4 / IPv6).
    pub dht_nodes: usize,
    pub dht_nodes_v6: usize,
    /// Number of connected peers.
    pub peers_live: u32,
    /// Number of discovered peers.
    pub peers_seen: u32,
    /// Number of peers currently connecting.
    pub peers_connecting: u32,
    /// Of those, the number of TCP / uTP connections.
    pub peers_tcp: u32,
    pub peers_utp: u32,
}

impl DownloadStatus {
    /// A one-line, UI-facing summary.
    pub fn summary(&self) -> String {
        if self.listen_port == 0 {
            return "入站未监听（NAT 下只能主动出站）".to_string();
        }
        let stack = if self.ipv6 { "IPv4+IPv6" } else { "IPv4" };
        format!(
            "入站 {} · {} · DHT {} 节点 · {} 连接（TCP {} / uTP {}）",
            self.listen_port,
            stack,
            self.dht_nodes + self.dht_nodes_v6,
            self.peers_live,
            self.peers_tcp,
            self.peers_utp
        )
    }
}

/// UI → background command.
pub enum DownloadCmd {
    /// Add a download task.
    Add {
        magnet: String,
        title: String,
        output_dir: PathBuf,
    },
    /// Cancel a task (deletes downloaded files).
    Cancel { id: String },
    /// Before unsubscribing, check whether the directory has any in-progress tasks; does not modify any data.
    CheckUnsubscribeDir {
        dir: PathBuf,
        bangumi_name: String,
        group_name: String,
    },
    /// Complete the unsubscribe: atomically check for in-progress tasks and, per the option,
    /// decide whether to clean up the directory and files.
    Unsubscribe {
        dir: PathBuf,
        bangumi_name: String,
        group_name: String,
        remove_downloads: bool,
    },
}

/// Background events (for UI notifications): add failure, completion, unsubscribe acknowledgements, etc.
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    /// Adding a task failed (metadata fetch failed/timed out, etc.).
    AddFailed { title: String, error: DownloadError },
    /// The download engine failed to start (background thread exited; all download features unavailable).
    EngineFailed { error: DownloadError },
    /// A download task completed (sent only on the first transition from a non-completed
    /// state to completed during this run).
    DownloadCompleted { title: String },
    /// Unsubscribe blocked: the directory still has in-progress tasks.
    UnsubscribeBlocked { dir: PathBuf, titles: Vec<String> },
    /// The pre-unsubscribe check passed; the UI can show the final confirmation dialog.
    UnsubscribeCheckReady { dir: PathBuf },
    /// Unsubscribe directory cleanup is complete (the directory has been deleted; the UI
    /// can remove the subscription record).
    UnsubscribeDone { dir: PathBuf },
}

/// An in-progress add request (during metadata fetching, the task is not yet registered
/// in librqbit). Lets the UI give immediate "fetching info…" feedback after the user
/// clicks download.
#[derive(Clone)]
pub(crate) struct PendingAdd {
    pub(crate) title: String,
    pub(crate) output_dir: PathBuf,
}
