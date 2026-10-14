//! Home data loading.

use crate::data::model::{HOME_CACHE_TTL, LoadUpdate, Loadable, publish};
use crate::prelude::*;
use crate::shell::state::MikanPlus;

impl MikanPlus {
    /// Load the home page once; a session hit lives in [`AppData`](crate::data::model::AppData), so this only runs on startup and retry.
    pub(crate) fn load_home(&mut self, cx: &mut Context<Self>) {
        if self.data.home.is_loading()
            || self
                .data
                .home_cached_at
                .is_some_and(|loaded| loaded.elapsed() < HOME_CACHE_TTL)
        {
            return;
        }
        // User-initiated load / retry: clear this page's backoff state and proceed immediately
        self.network
            .reset_backoff(&source::api::home_url(&self.network));
        let network = self.network.clone();
        let url = source::api::home_url(&network);
        self.data.home = Loadable::Loading;
        cx.notify();
        std::thread::spawn(move || {
            let result = storage::cache::cached_api(&url, HOME_CACHE_TTL).map_or_else(
                || {
                    let result = source::api::fetch_home(&network);
                    if let Ok(groups) = &result {
                        let _ = storage::cache::store_api(&url, groups);
                    }
                    result
                },
                Ok,
            );
            publish(LoadUpdate::Home(result));
        });
    }
}
