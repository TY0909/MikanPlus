//! Online search data loading.

use crate::data::model::{LoadUpdate, Loadable, Search, publish};
use crate::prelude::*;
use crate::shell::state::MikanPlus;

impl MikanPlus {
    /// Ensure an online search is loaded: a session hit in [`AppData`](crate::data::model::AppData) is used directly, otherwise request it once.
    pub(crate) fn load_search(&mut self, query: String, cx: &mut Context<Self>) {
        if query.trim().is_empty() {
            return;
        }
        match self.data.searches.get(&query) {
            Some(search) if search.results.is_loading() || search.results.ready().is_some() => {
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
            },
        );
        cx.notify();
        std::thread::spawn(move || {
            let url = network.search_url(&query);
            let results = network
                .fetch_html(&url)
                .map(|html| source::parser::parse_search_results(&network, &html));
            publish(LoadUpdate::Search { query, results });
        });
    }
}
