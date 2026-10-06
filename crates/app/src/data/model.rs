//! Application data model.
//!
//! This module only describes "what the data looks like": the shape of remote
//! data ([`Loadable`]), the bounded cache container ([`Cache`]), data shared
//! across scenes ([`AppData`]), overlay state ([`Overlays`]), and the single
//! channel through which background results flow back ([`LoadUpdate`]).
//!
//! How data moves between scenes is the responsibility of each scene module and
//! [`crate::shell::state::MikanPlus`].

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use domain::{
    BangumiGroup, BangumiId, BangumiItem, Episode, SearchResults, SubgroupId, SubgroupRef,
    Subscription,
};
use source::SourceError;

/// Capacity limit for the bounded cache.
pub(crate) const CACHE_LIMIT: usize = 64;

/// Shape of remote data: at any moment a piece of data is in exactly one of
/// four states.
///
/// All "data flowing between scenes" revolves around it: a request advances
/// `Idle`/`Failed` to `Loading`, and a background receipt then settles it into
/// `Ready` or `Failed`.
pub(crate) enum Loadable<T> {
    /// Not requested yet.
    Idle,
    /// Request in progress.
    Loading,
    /// Ready.
    Ready(T),
    /// Request failed.
    Failed(SourceError),
}

impl<T> Loadable<T> {
    /// Whether a request is in progress.
    pub(crate) fn is_loading(&self) -> bool {
        matches!(self, Loadable::Loading)
    }

    /// The ready value.
    pub(crate) fn ready(&self) -> Option<&T> {
        match self {
            Loadable::Ready(value) => Some(value),
            _ => None,
        }
    }

    /// Failure reason.
    pub(crate) fn failed(&self) -> Option<&SourceError> {
        match self {
            Loadable::Failed(error) => Some(error),
            _ => None,
        }
    }
}

impl<T> From<Result<T, SourceError>> for Loadable<T> {
    fn from(result: Result<T, SourceError>) -> Self {
        match result {
            Ok(value) => Loadable::Ready(value),
            Err(error) => Loadable::Failed(error),
        }
    }
}

/// Bounded cache: approximately LRU by insertion order; evicts the oldest entry
/// when the limit is exceeded.
pub(crate) struct Cache<K, V> {
    entries: HashMap<K, V>,
    order: VecDeque<K>,
    limit: usize,
}

impl<K: Clone + std::hash::Hash + Eq, V> Cache<K, V> {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            limit,
        }
    }

    pub(crate) fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    pub(crate) fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    pub(crate) fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.entries.get_mut(key)
    }

    /// Insert or update; new keys are recorded in eviction order, and the oldest is evicted when the limit is exceeded.
    pub(crate) fn insert(&mut self, key: K, value: V) {
        if !self.entries.contains_key(&key) {
            self.order.push_back(key.clone());
            while self.order.len() > self.limit {
                if let Some(oldest) = self.order.pop_front() {
                    self.entries.remove(&oldest);
                }
            }
        }
        self.entries.insert(key, value);
    }

    pub(crate) fn remove(&mut self, key: &K) -> Option<V> {
        let removed = self.entries.remove(key);
        if removed.is_some()
            && let Some(position) = self.order.iter().position(|k| k == key)
        {
            self.order.remove(position);
        }
        removed
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter()
    }
}

/// Data for one online search: results + pagination index.
pub(crate) struct Search {
    pub(crate) results: Loadable<SearchResults>,
    pub(crate) page: usize,
}

/// Data shared across scenes: home / detail / search / subscriptions / lazy episode loading.
pub(crate) struct AppData {
    /// Home bangumi list.
    pub(crate) home: Loadable<Vec<BangumiGroup>>,
    /// Bangumi details, keyed by the stable [`BangumiId`] rather than the display name, so two
    /// different bangumi that happen to share a title cannot overwrite each other.
    pub(crate) details: Cache<BangumiId, Loadable<BangumiItem>>,
    /// bid → display name while a detail is still loading or has failed; once it is ready the name
    /// is read from the cached item.
    pub(crate) detail_names: HashMap<BangumiId, String>,
    /// Lazy subtitle-group episode loading: subgroup reference → data.
    pub(crate) episodes: Cache<SubgroupRef, Loadable<Vec<Episode>>>,
    /// Subtitle groups expanded on the detail page.
    pub(crate) expanded: HashSet<SubgroupRef>,
    /// Online searches: keyword → data.
    pub(crate) searches: Cache<String, Search>,
    /// Subscription records.
    pub(crate) subscriptions: Vec<Subscription>,
    /// Episode filter keyword per subscription entry.
    pub(crate) keywords: HashMap<SubgroupRef, String>,
}

impl AppData {
    pub(crate) fn new(state: &storage::State) -> Self {
        Self {
            home: Loadable::Idle,
            details: Cache::new(CACHE_LIMIT),
            detail_names: HashMap::new(),
            episodes: Cache::new(CACHE_LIMIT),
            expanded: HashSet::new(),
            searches: Cache::new(CACHE_LIMIT),
            subscriptions: state.subscriptions(),
            keywords: state.subgroup_keywords(),
        }
    }
}

/// A single unsubscribe awaiting a receipt from the download thread.
pub(crate) struct PendingUnsub {
    pub(crate) bangumi_id: BangumiId,
    pub(crate) subgroup_id: SubgroupId,
    pub(crate) bangumi_name: String,
    pub(crate) group_name: String,
}

/// Warning content when an unsubscribe is blocked (the subscription has episodes downloading).
#[derive(Clone)]
pub(crate) struct UnsubscribeWarning {
    pub(crate) bangumi_name: String,
    pub(crate) group_name: String,
    /// Titles of episodes currently downloading.
    pub(crate) active_titles: Vec<String>,
}

/// Content for the second confirmation before unsubscribing.
#[derive(Clone)]
pub(crate) struct UnsubscribeConfirmation {
    pub(crate) bangumi_id: BangumiId,
    pub(crate) subgroup_id: SubgroupId,
    pub(crate) bangumi_name: String,
    pub(crate) group_name: String,
    /// Whether the confirmation dialog has "remove download directory and files" selected.
    pub(crate) remove_downloads: bool,
}

/// Overlay (modal window) state.
#[derive(Default)]
pub(crate) struct Overlays {
    /// Subscription entry whose filter window is currently open.
    pub(crate) filter: Option<SubgroupRef>,
    /// Unsubscribe-blocked warning.
    pub(crate) warning: Option<UnsubscribeWarning>,
    /// Unsubscribe second confirmation.
    pub(crate) confirmation: Option<UnsubscribeConfirmation>,
    /// Unsubscribe request currently being checked by the download thread.
    pub(crate) checking: Option<UnsubscribeConfirmation>,
    /// Unsubscribe awaiting a receipt from the download thread.
    pub(crate) pending_unsub: Option<PendingUnsub>,
}

/// Background load receipt: remote results from every scene flow back into the
/// data model through this single channel.
///
/// Variant sizes differ considerably, but this is only a one-shot message from
/// a background thread (the queue usually holds just a few items), so we don't
/// introduce Box indirection to save those few hundred bytes.
#[allow(clippy::large_enum_variant)]
pub(crate) enum LoadUpdate {
    Home(Result<Vec<BangumiGroup>, SourceError>),
    Detail {
        bid: BangumiId,
        item: Result<BangumiItem, SourceError>,
    },
    Episodes {
        key: SubgroupRef,
        episodes: Result<Vec<Episode>, SourceError>,
    },
    Search {
        query: String,
        results: Result<SearchResults, SourceError>,
    },
}

/// Posted load receipts awaiting settlement.
static UPDATES: Mutex<Vec<LoadUpdate>> = Mutex::new(Vec::new());
/// Load-completion signal: incremented whenever any receipt is posted, for the polling loop to detect changes.
static LOAD_VERSION: AtomicU64 = AtomicU64::new(0);

/// Post a load receipt (called from a background thread).
pub(crate) fn publish(update: LoadUpdate) {
    UPDATES.lock().unwrap().push(update);
    LOAD_VERSION.fetch_add(1, Ordering::Relaxed);
}

/// Take all currently pending receipts.
pub(crate) fn take_updates() -> Vec<LoadUpdate> {
    UPDATES.lock().unwrap().drain(..).collect()
}

/// Current load-completion signal.
pub(crate) fn load_version() -> u64 {
    LOAD_VERSION.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(bangumi_id: u32, name: &str) -> BangumiItem {
        BangumiItem {
            name: name.to_string(),
            bangumi_id: BangumiId::from(bangumi_id),
            ..Default::default()
        }
    }

    /// Regression: details are keyed by the stable bangumi id. When the key was the display name,
    /// two different bangumi sharing a title collapsed into one entry, so opening the second one
    /// showed (and acted on) the first.
    #[test]
    fn details_are_keyed_by_id_not_by_name() {
        let mut details: Cache<BangumiId, Loadable<BangumiItem>> = Cache::new(CACHE_LIMIT);
        let first = item(1, "同名番剧");
        let second = item(2, "同名番剧");
        details.insert(first.bangumi_id, Loadable::Ready(first));
        details.insert(second.bangumi_id, Loadable::Ready(second));

        assert_eq!(details.iter().count(), 2, "同名不应合并为一条");
        for id in [1, 2] {
            let entry = details
                .get(&BangumiId::from(id))
                .and_then(Loadable::ready)
                .expect("两个 id 都应各自可查");
            assert_eq!(entry.bangumi_id, BangumiId::from(id));
        }
    }
}
