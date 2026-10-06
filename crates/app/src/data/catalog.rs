//! Cross-scene data queries.
//!
//! The same bangumi data flows between the home list, search results, details,
//! and subscriptions; this module centralizes the "locate it by name / by id"
//! lookups so each scene doesn't repeat the search.

use domain::{BangumiId, BangumiItem, SubgroupId, Subscription};

use crate::data::model::AppData;

impl AppData {
    /// Look up a basic entry by id in the home list.
    pub(crate) fn item_with_id(&self, bangumi_id: BangumiId) -> Option<BangumiItem> {
        self.home_items()
            .find(|item| item.bangumi_id == bangumi_id)
            .cloned()
    }

    /// Look up an entry by id among loaded details (the cache is keyed by id).
    pub(crate) fn detail_with_id(&self, bangumi_id: BangumiId) -> Option<BangumiItem> {
        self.details
            .get(&bangumi_id)
            .and_then(|state| state.ready())
            .cloned()
    }

    /// Locate a detail name by id (detail cache → load mapping → home → subscription records).
    pub(crate) fn name_for(&self, bangumi_id: BangumiId) -> Option<String> {
        self.detail_with_id(bangumi_id)
            .map(|item| item.name)
            .or_else(|| self.detail_names.get(&bangumi_id).cloned())
            .or_else(|| self.item_with_id(bangumi_id).map(|item| item.name))
            .or_else(|| {
                self.subscriptions
                    .iter()
                    .find(|s| s.bangumi_id == bangumi_id)
                    .map(|s| s.bangumi_name.clone())
            })
    }

    /// Subscription record for a given (bangumi, subtitle group).
    pub(crate) fn subscription(
        &self,
        bangumi_id: BangumiId,
        subgroup_id: SubgroupId,
    ) -> Option<&Subscription> {
        self.subscriptions
            .iter()
            .find(|s| s.bangumi_id == bangumi_id && s.subgroup_id == subgroup_id)
    }

    /// Whether a given (bangumi, subtitle group) is already subscribed.
    pub(crate) fn is_subscribed(&self, bangumi_id: BangumiId, subgroup_id: SubgroupId) -> bool {
        self.subscription(bangumi_id, subgroup_id).is_some()
    }

    /// Subscription cover: home entry → loaded detail → search result, by id.
    pub(crate) fn cover_of(&self, bangumi_id: BangumiId) -> Option<String> {
        self.item_with_id(bangumi_id)
            .and_then(|item| item.cover_url)
            .or_else(|| {
                self.detail_with_id(bangumi_id)
                    .and_then(|item| item.cover_url)
            })
            .or_else(|| {
                self.search_items()
                    .find(|item| item.bangumi_id == bangumi_id)
                    .and_then(|item| item.cover_url.clone())
            })
    }

    /// Subscriptions resolved against live data: the stored cover wins, otherwise
    /// it is derived by bangumi id from the loaded home / detail / search data.
    pub(crate) fn resolved_subscriptions(&self) -> Vec<Subscription> {
        self.subscriptions
            .iter()
            .map(|sub| {
                let mut sub = sub.clone();
                if sub.cover_url.is_none() {
                    sub.cover_url = self.cover_of(sub.bangumi_id);
                }
                sub
            })
            .collect()
    }

    /// Iterates every entry across all home groups.
    fn home_items(&self) -> impl Iterator<Item = &BangumiItem> {
        self.home
            .ready()
            .into_iter()
            .flatten()
            .flat_map(|group| group.items.iter())
    }

    /// Iterates every entry across the results of all loaded searches.
    fn search_items(&self) -> impl Iterator<Item = &BangumiItem> {
        self.searches
            .iter()
            .filter_map(|(_, search)| search.results.ready())
            .flat_map(|results| results.items.iter())
    }
}
