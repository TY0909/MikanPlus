//! Image download task status.

use std::time::Duration;

/// Image status: only one download task is allowed per URL at a time (deduplication).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImgStatus {
    /// Not downloaded yet
    Pending,
    /// Downloading
    Loading,
    /// Downloaded (the cache file is ready)
    Loaded,
    /// Download failed (retry allowed after the cooldown)
    Error,
}

/// Retry cooldown after an image failure.
pub(crate) const IMAGE_RETRY_COOLDOWN: Duration = Duration::from_secs(60);
