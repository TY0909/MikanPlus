//! Online search data loading.

use crate::data::model::{LoadUpdate, Loadable, SEARCH_CACHE_TTL, Search, publish};
use crate::prelude::*;
use crate::shell::state::MikanPlus;

impl MikanPlus {
    /// Ensure an online search is loaded. Search results are intentionally memory-only because
    /// they are temporary and depend on the exact query string.
    pub(crate) fn load_search(&mut self, query: String, cx: &mut Context<Self>) {
        if query.trim().is_empty() {
            return;
        }
        match self.data.searches.get(&query) {
            Some(search)
                if search.results.is_loading()
                    || (search.results.ready().is_some()
                        && search.cached_at.elapsed() < SEARCH_CACHE_TTL) =>
            {
                return;
            }
            _ => {}
        }
        // User-initiated search / retry: clear backoff and proceed immediately
        self.network.reset_backoff(&self.network.search_url(&query));
        let network = self.network.clone();
        self.data.searches.insert(
            query.clone(),
            Search {
                results: Loadable::Loading,
                page: 0,
                cached_at: std::time::Instant::now(),
            },
        );
        cx.notify();
        std::thread::spawn(move || {
            let results = source::api::fetch_search(&network, &query, 1, 100);
            publish(LoadUpdate::Search { query, results });
        });
    }
}
