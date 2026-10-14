//! Online search data loading.

use crate::data::model::{
    DETAIL_CACHE_TTL, LoadUpdate, Loadable, SEARCH_CACHE_TTL, Search, publish,
};
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
            let Ok(results) = results else {
                publish(LoadUpdate::Search { query, results });
                return;
            };

            // Search rows do not carry cover URLs. Publish the text results immediately, then
            // hydrate covers independently so a slow detail request never blocks the search UI.
            publish(LoadUpdate::Search {
                query: query.clone(),
                results: Ok(results.clone()),
            });

            let mut covers = HashMap::new();
            for item in &results.items {
                let url = source::api::bangumi_url(&network, item.bangumi_id);
                let detail =
                    storage::cache::cached_api::<domain::BangumiItem>(&url, DETAIL_CACHE_TTL)
                        .map(Ok)
                        .unwrap_or_else(|| {
                            let result = source::api::fetch_bangumi(&network, item.bangumi_id);
                            if let Ok(detail) = &result {
                                let _ = storage::cache::store_api(&url, detail);
                            }
                            result
                        });
                if let Ok(detail) = detail
                    && let Some(cover) = detail.cover_url
                {
                    covers.insert(item.bangumi_id, cover);
                }
            }
            if !covers.is_empty() {
                publish(LoadUpdate::SearchCovers { query, covers });
            }
        });
    }
}
