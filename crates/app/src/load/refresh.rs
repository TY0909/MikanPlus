//! Re-requesting data that previously failed, e.g. after the data source changes.

use domain::{BangumiId, SubgroupRef};

use crate::prelude::*;
use crate::shell::state::MikanPlus;

impl MikanPlus {
    /// Re-fetch every entry currently in a failed state against the selected data source.
    ///
    /// Used after switching the data-source domain: the previous domain's backoff and
    /// failures no longer apply. Already-loaded data keeps working (its cover URLs are
    /// rewritten to the current host on render), so only failures need a fresh request.
    /// Each load call clears the (new) request URL's backoff first.
    pub(crate) fn reload_failed(&mut self, cx: &mut Context<Self>) {
        if self.data.home.failed().is_some() {
            self.load_home(cx);
        }

        // Collect before reloading: the caches are borrowed while iterating, and the
        // retry calls below need `&mut self`.
        let details: Vec<(BangumiId, String)> = self
            .data
            .detail_names
            .iter()
            .filter(|(bangumi_id, _)| {
                self.data
                    .details
                    .get(*bangumi_id)
                    .is_some_and(|state| state.failed().is_some())
            })
            .map(|(bangumi_id, name)| (*bangumi_id, name.clone()))
            .collect();
        for (bangumi_id, name) in details {
            self.retry_detail(bangumi_id, name, cx);
        }

        let searches: Vec<String> = self
            .data
            .searches
            .iter()
            .filter(|(_, search)| search.results.failed().is_some())
            .map(|(query, _)| query.clone())
            .collect();
        for query in searches {
            self.load_search(query, cx);
        }

        let episodes: Vec<SubgroupRef> = self
            .data
            .episodes
            .iter()
            .filter(|(_, state)| state.failed().is_some())
            .map(|(key, _)| *key)
            .collect();
        for key in episodes {
            self.reload_episodes(key, cx);
        }
    }
}
