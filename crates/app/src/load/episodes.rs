//! Lazy loading and expand state for subtitle-group episode data.

use ui::GroupEpisodesState;

use domain::{Episode, SubgroupRef};

use crate::data::model::{EPISODE_CACHE_TTL, LoadUpdate, Loadable, publish};
use crate::prelude::*;
use crate::shell::state::MikanPlus;

impl MikanPlus {
    /// Ensure a subtitle group's episodes are loaded (lazy): reuse an in-memory hit directly,
    /// otherwise load the API-backed collection.
    pub(crate) fn load_episodes(&mut self, key: SubgroupRef, cx: &mut Context<Self>) {
        if self
            .data
            .episodes
            .get(&key)
            .is_some_and(Loadable::is_loading)
            || self
                .data
                .episode_cached_at
                .get(&key)
                .is_some_and(|loaded| loaded.elapsed() < EPISODE_CACHE_TTL)
        {
            return;
        }
        // User-initiated expand / retry: clear this API's backoff state and proceed immediately.
        let cache_key =
            source::api::subgroup_episodes_url(&self.network, key.bangumi, key.subgroup);
        self.network.reset_backoff(&cache_key);
        let network = self.network.clone();
        self.data.episodes.insert(key, Loadable::Loading);
        cx.notify();
        std::thread::spawn(move || {
            let episodes = storage::cache::cached_api(&cache_key, EPISODE_CACHE_TTL).map_or_else(
                || {
                    let result =
                        source::api::fetch_subgroup_episodes(&network, key.bangumi, key.subgroup);
                    if let Ok(episodes) = &result {
                        let _ = storage::cache::store_api(&cache_key, episodes);
                    }
                    result
                },
                Ok,
            );
            publish(LoadUpdate::Episodes { key, episodes });
        });
    }

    /// Reload a subtitle group's episodes (retry after failure).
    pub(crate) fn reload_episodes(&mut self, key: SubgroupRef, cx: &mut Context<Self>) {
        self.data.episodes.remove(&key);
        self.data.episode_cached_at.remove(&key);
        self.load_episodes(key, cx);
    }

    /// Expand / collapse a subtitle group on the detail page; loads episodes on demand when expanding.
    pub(crate) fn toggle_group(&mut self, key: SubgroupRef, cx: &mut Context<Self>) {
        if self.data.expanded.remove(&key) {
            cx.notify();
            return;
        }
        self.data.expanded.insert(key);
        self.load_episodes(key, cx);
    }
}

/// Internal episode state → UI view state (errors expose only user-facing information).
pub(crate) fn episodes_view(state: &Loadable<Vec<Episode>>) -> GroupEpisodesState {
    match state {
        Loadable::Idle | Loadable::Loading => GroupEpisodesState::Loading,
        Loadable::Ready(episodes) => GroupEpisodesState::Ready(episodes.clone()),
        Loadable::Failed(error) => GroupEpisodesState::Failed(error.user_message().to_string()),
    }
}
