//! Mikan network client.
//!
//! Access policy (core goal: hit the servers as little as possible):
//! - **Throttling**: a global minimum interval between requests, to avoid bursts
//! - **Backoff**: after a failure, stop requesting the same resource for a while (exponential)
//! - **Restrained concurrency**: at most 2 requests in flight
//! - **Deduplication**: only one download task per URL at a time (the image task table)
//! - On a cache hit (see `storage::cache`) execution never reaches this layer
//!
//! All of the layer's state lives in one [`Network`]: the shared connection pool, the request
//! rhythm (throttle / backoff / concurrency), the image task table, and the selected data source.
//! The submodules hold pure helpers and the state types only.

mod endpoints;
mod http;
mod image;
mod throttle;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use crate::SourceError;
use throttle::Throttle;

pub use image::ImgStatus;

/// The Mikan network client: owns the connection pool and all request / image / data-source state.
pub struct Network {
    /// Shared `ureq` agent (connection pool), built lazily on the first request.
    agent: OnceLock<ureq::Agent>,
    /// Request rhythm (throttle / backoff / concurrency).
    throttle: Mutex<Throttle>,
    /// Image task table: url → status
    images: Mutex<HashMap<String, ImgStatus>>,
    /// Image failure-time table: url → failure instant (retry allowed after the cooldown)
    image_errors: Mutex<HashMap<String, Instant>>,
    /// Image download task version: incremented when the image cache changes, for UI polling.
    image_version: AtomicU64,
    /// Whether the host of the last successful image fetch was the "alternate" side.
    image_use_alternate: AtomicBool,
    /// Whether the backup data-source domain is enabled.
    use_backup_domain: AtomicBool,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            agent: OnceLock::new(),
            throttle: Mutex::new(Throttle::new()),
            images: Mutex::new(HashMap::new()),
            image_errors: Mutex::new(HashMap::new()),
            image_version: AtomicU64::new(0),
            image_use_alternate: AtomicBool::new(false),
            use_backup_domain: AtomicBool::new(false),
        }
    }
}

impl Network {
    /// Creates a network client. The connection pool is built lazily on the first request.
    pub fn new() -> Self {
        Self::default()
    }

    /// The shared connection pool (built on first use).
    fn agent(&self) -> &ureq::Agent {
        self.agent.get_or_init(http::build_agent)
    }

    // ---- transport ----

    /// Fetches an HTML page (with throttling / backoff / concurrency control). Returns
    /// [`SourceError::Throttled`] while in backoff.
    pub fn fetch_html(&self, url: &str) -> Result<String, SourceError> {
        if self.in_backoff(url) {
            return Err(SourceError::Throttled);
        }
        self.acquire_slot();
        let result = http::get_html(self.agent(), url);
        self.release_slot();
        self.record(url, &result);
        result
    }

    /// Downloads binary content (images). With throttling / backoff / concurrency control; returns
    /// [`SourceError::Throttled`] while in backoff.
    pub fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>, SourceError> {
        if self.in_backoff(url) {
            return Err(SourceError::Throttled);
        }
        self.acquire_slot();
        let result = http::get_bytes(self.agent(), url);
        self.release_slot();
        self.record(url, &result);
        result
    }

    /// Clears the backoff state for the given URL (user-initiated retry).
    pub fn reset_backoff(&self, url: &str) {
        self.throttle.lock().unwrap().reset_backoff(url);
    }

    fn in_backoff(&self, url: &str) -> bool {
        self.throttle.lock().unwrap().in_backoff(url)
    }

    /// Waits for the global throttle interval and a concurrency slot (blocks the current thread;
    /// call only on background threads).
    fn acquire_slot(&self) {
        loop {
            if self.throttle.lock().unwrap().try_acquire() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
    }

    fn release_slot(&self) {
        self.throttle.lock().unwrap().release_slot();
    }

    fn record<T>(&self, url: &str, result: &Result<T, SourceError>) {
        let mut throttle = self.throttle.lock().unwrap();
        match result {
            Ok(_) => throttle.note_success(),
            Err(_) => throttle.note_failure(url),
        }
    }

    // ---- images ----

    /// The current image download task version (for UI polling).
    pub fn image_version(&self) -> u64 {
        self.image_version.load(Ordering::Relaxed)
    }

    /// Queries the image status.
    pub fn image_status(&self, url: &str) -> ImgStatus {
        self.images
            .lock()
            .unwrap()
            .get(url)
            .copied()
            .unwrap_or(ImgStatus::Pending)
    }

    /// Tries to claim the download task: only the first call for a given URL returns true; in the
    /// failure state, re-claiming is allowed once the cooldown has elapsed (automatic retry).
    pub fn claim_image(&self, url: &str) -> bool {
        let mut images = self.images.lock().unwrap();
        match images.get(url) {
            Some(ImgStatus::Pending) | None => {
                images.insert(url.to_string(), ImgStatus::Loading);
                true
            }
            Some(ImgStatus::Error) => {
                let cooldown_ok = self.image_errors.lock().unwrap().get(url).is_none_or(|at| {
                    Instant::now().duration_since(*at) >= image::IMAGE_RETRY_COOLDOWN
                });
                if cooldown_ok {
                    images.insert(url.to_string(), ImgStatus::Loading);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Records the image download result.
    pub fn finish_image(&self, url: &str, ok: bool) {
        self.images.lock().unwrap().insert(
            url.to_string(),
            if ok {
                ImgStatus::Loaded
            } else {
                ImgStatus::Error
            },
        );
        if !ok {
            self.image_errors
                .lock()
                .unwrap()
                .insert(url.to_string(), Instant::now());
        }
        self.image_version.fetch_add(1, Ordering::Relaxed);
    }

    /// Fetches an image: tries the current data-source host first, falls back to the other host on
    /// failure, and remembers which side last succeeded so later covers need not repeat a failing
    /// request. The cache key is host-independent, so both attempts share one cache entry.
    pub fn fetch_image_bytes(&self, url: &str) -> Result<Vec<u8>, SourceError> {
        let alternate = endpoints::alternate_url(url);
        let (first, second, first_is_alternate) = match alternate {
            Some(alt) if self.image_use_alternate.load(Ordering::Relaxed) => {
                (alt, url.to_string(), true)
            }
            Some(alt) => (url.to_string(), alt, false),
            None => (url.to_string(), String::new(), false),
        };
        match self.fetch_bytes(&first) {
            Ok(bytes) => {
                self.image_use_alternate
                    .store(first_is_alternate, Ordering::Relaxed);
                Ok(bytes)
            }
            Err(first_err) => {
                if second.is_empty() {
                    return Err(first_err);
                }
                match self.fetch_bytes(&second) {
                    Ok(bytes) => {
                        self.image_use_alternate
                            .store(!first_is_alternate, Ordering::Relaxed);
                        Ok(bytes)
                    }
                    Err(_) => Err(first_err),
                }
            }
        }
    }

    // ---- data source ----

    /// Whether the backup data-source domain is currently in use.
    pub fn use_backup_domain(&self) -> bool {
        self.use_backup_domain.load(Ordering::Relaxed)
    }

    /// Sets whether to use the backup domain.
    ///
    /// The switch also resets image host probing and clears failed covers, so covers that
    /// failed on the previous domain retry immediately instead of waiting out the cooldown.
    pub fn set_backup_domain(&self, enabled: bool) {
        self.use_backup_domain.store(enabled, Ordering::Relaxed);
        self.image_use_alternate.store(false, Ordering::Relaxed);
        self.clear_failed_images();
    }

    /// Clears failed image tasks and their retry cooldown so covers retry from scratch; the
    /// version bump makes the UI re-attempt the placeholder downloads.
    fn clear_failed_images(&self) {
        self.images
            .lock()
            .unwrap()
            .retain(|_, status| !matches!(status, ImgStatus::Error));
        self.image_errors.lock().unwrap().clear();
        self.image_version.fetch_add(1, Ordering::Relaxed);
    }

    /// Root address of the currently selected data source.
    pub fn base_url(&self) -> &'static str {
        endpoints::base_url(self.use_backup_domain())
    }

    /// Joins a full site URL (see [`endpoints::site_url`]).
    pub fn site_url(&self, path: &str) -> String {
        endpoints::site_url(self.base_url(), path)
    }

    /// Search-page URL for the current data source.
    pub fn search_url(&self, query: &str) -> String {
        endpoints::search_url(self.base_url(), query)
    }

    /// Rewrites a URL on a known data-source host to the currently selected host.
    pub fn normalize_url(&self, url: &str) -> String {
        endpoints::rewrite_host(url, self.base_url())
    }
}

#[cfg(test)]
mod tests {
    use super::{ImgStatus, Network};

    #[test]
    fn image_claim_dedup() {
        let network = Network::new();
        let url = "https://example.com/a.jpg";
        assert!(network.claim_image(url), "首次认领成功");
        assert!(!network.claim_image(url), "第二次认领被拒绝(去重)");
        network.finish_image(url, true);
        assert_eq!(network.image_status(url), ImgStatus::Loaded);
        assert!(!network.claim_image(url), "已加载不再重复下载");
    }
}
