//! Bangumi card grid for search results: one card per matching bangumi, wrapping automatically.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::{prelude::*, px};
use source::Network;

use crate::components::bangumi_card::{BangumiCard, BangumiFormat};
use crate::pages::home_view::CardClickCallback;
use domain::BangumiItem;

/// Bangumi card grid (wrapping).
pub(super) fn render_card_grid(
    items: &[BangumiItem],
    on_card_click: &CardClickCallback,
    network: &Arc<Network>,
) -> impl IntoElement {
    let cards = items
        .iter()
        .enumerate()
        .map(|(ix, item)| {
            let name = item.name.clone();
            let click_name = name.clone();
            let bid = item.bangumi_id;
            let on_click = on_card_click.clone();
            BangumiCard {
                name: name.clone(),
                bangumi_id: bid,
                poster_url: item.cover_url.clone().unwrap_or_default(),
                format: BangumiFormat::Tv,
                key: format!("search-card-{ix}"),
                on_click: Some(Rc::new(move |window, app| {
                    on_click(&click_name, bid, window, app);
                })),
                network: network.clone(),
            }
        })
        .collect::<Vec<_>>();

    gpui_kit::div()
        .w_full()
        .mb(px(20.))
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(px(18.))
        .children(cards)
}
