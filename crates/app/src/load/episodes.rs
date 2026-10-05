//! Lazy loading and expand state for subtitle-group episode data.

use ui::GroupEpisodesState;

use domain::{Episode, SubgroupRef};

use crate::data::model::{LoadUpdate, Loadable, publish};
use crate::prelude::*;
use crate::shell::state::MikanPlus;

impl MikanPlus {
    /// Ensure a subtitle group's episodes are loaded (lazy): reuse an in-memory hit directly, otherwise fetch the RSS.
    pub(crate) fn load_episodes(&mut self, key: SubgroupRef, cx: &mut Context<Self>) {
        if self.data.episodes.contains(&key) {
            return;
        }
        // User-initiated expand / retry: clear this RSS's backoff state and proceed immediately
        self.network.reset_backoff(&source::rss::subgroup_rss_url(
            &self.network,
            key.bangumi,
            key.subgroup,
        ));
        let network = self.network.clone();
        self.data.episodes.insert(key, Loadable::Loading);
        cx.notify();
        std::thread::spawn(move || {
            let episodes =
                source::rss::fetch_subgroup_episodes(&network, key.bangumi, key.subgroup);
            publish(LoadUpdate::Episodes { key, episodes });
        });
    }

    /// Reload a subtitle group's episodes (retry after failure).
    pub(crate) fn reload_episodes(&mut self, key: SubgroupRef, cx: &mut Context<Self>) {
        self.data.episodes.remove(&key);
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
