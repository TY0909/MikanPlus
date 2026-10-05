//! Request throttling, backoff, and concurrency control.
//!
//! This is "the rhythm of requests over time": a global minimum interval, exponential backoff
//! after failures, and at most N requests in flight. On a cache hit, execution never reaches this
//! layer.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Minimum interval between requests.
const MIN_INTERVAL: Duration = Duration::from_millis(300);
/// Base backoff duration after a failure.
const BACKOFF_BASE: Duration = Duration::from_secs(30);
/// Maximum number of concurrent requests.
const MAX_CONCURRENCY: usize = 2;

/// The request rhythm: interval, backoff table, and concurrency counter.
pub(crate) struct Throttle {
    /// Time of the last request (throttling)
    last_request: Option<Instant>,
    /// Number of in-flight requests
    inflight: usize,
    /// Backoff table: url → earliest time a request is allowed again
    backoff: HashMap<String, Instant>,
    /// Consecutive failure count (amplifies the global backoff)
    fail_streak: u32,
}

impl Throttle {
    pub(crate) fn new() -> Self {
        Self {
            last_request: None,
            inflight: 0,
            backoff: HashMap::new(),
            fail_streak: 0,
        }
    }

    /// Whether the URL is within its backoff period (expired entries are lazily cleaned up to keep
    /// the backoff table bounded).
    pub(crate) fn in_backoff(&mut self, url: &str) -> bool {
        match self.backoff.get(url) {
            Some(until) if Instant::now() >= *until => {
                self.backoff.remove(url);
                false
            }
            Some(_) => true,
            None => false,
        }
    }

    /// Clears the backoff state for the given URL (user-initiated retry: backoff only suppresses
    /// automatic program retries, explicit user actions are let through immediately).
    pub(crate) fn reset_backoff(&mut self, url: &str) {
        self.backoff.remove(url);
    }

    /// Tries to acquire a concurrency slot after the global interval; returns `false` when the
    /// caller must wait and retry. The concurrency and throttle checks share one critical section,
    /// so the "at most N in flight" invariant holds strictly.
    pub(crate) fn try_acquire(&mut self) -> bool {
        let now = Instant::now();
        if self.inflight >= MAX_CONCURRENCY {
            return false;
        }
        if let Some(last) = self.last_request
            && now.duration_since(last) < MIN_INTERVAL
        {
            return false;
        }
        self.last_request = Some(now);
        self.inflight += 1;
        true
    }

    /// Releases a concurrency slot.
    pub(crate) fn release_slot(&mut self) {
        self.inflight -= 1;
    }

    /// Records a failure: backs off the URL (duration amplified by the global consecutive failure
    /// count) and updates that count.
    pub(crate) fn note_failure(&mut self, url: &str) {
        self.fail_streak += 1;
        let factor = 1u32 << self.fail_streak.min(4); // 30s → 60s → 120s → 240s → 480s
        let until = Instant::now() + BACKOFF_BASE * factor;
        self.backoff.insert(url.to_string(), until);
    }

    /// Records a success: resets the consecutive failure count to zero.
    pub(crate) fn note_success(&mut self) {
        self.fail_streak = 0;
    }
}
