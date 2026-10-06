//! Bangumi detail data loading.

use crate::data::model::{LoadUpdate, Loadable, publish};
use crate::prelude::*;
use crate::shell::state::MikanPlus;
use domain::BangumiId;

impl MikanPlus {
    /// Ensure the detail is loaded (lazy): a session hit in [`AppData`](crate::data::model::AppData) is used directly, otherwise request it once.
    ///
    /// The caller already holds the anime's identity and display name (from the card that was clicked), so no
    /// name-based reverse lookup is needed here.
    pub(crate) fn load_detail(
        &mut self,
        bangumi_id: BangumiId,
        name: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(Loadable::Ready(_) | Loadable::Loading) = self.data.details.get(&bangumi_id) {
            return;
        }
        // Record bid → name so lookups still resolve while this detail is loading
        self.data.detail_names.insert(bangumi_id, name.clone());
        // User-initiated entry / retry: clear this detail page's backoff state and proceed immediately
        self.network
            .reset_backoff(&source::api::bangumi_url(&self.network, bangumi_id));
        // Keep the list / search name as the fetched item's display name
        let display_name = name.clone();
        let network = self.network.clone();
        self.data.details.insert(bangumi_id, Loadable::Loading);
        cx.notify();
        std::thread::spawn(move || {
            let item = source::api::fetch_bangumi(&network, bangumi_id).map(|mut item| {
                item.name = display_name;
                item
            });
            publish(LoadUpdate::Detail {
                bid: bangumi_id,
                item,
            });
        });
    }

    /// Retry after a detail load failure: clear the failed state and load again.
    pub(crate) fn retry_detail(
        &mut self,
        bangumi_id: BangumiId,
        name: String,
        cx: &mut Context<Self>,
    ) {
        self.data.details.remove(&bangumi_id);
        self.load_detail(bangumi_id, name, cx);
    }
}
