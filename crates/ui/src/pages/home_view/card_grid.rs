//! Bangumi card grid: renders a batch of bangumi as a wrapping row of cards.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::{prelude::*, px};
use source::Network;

use crate::components::bangumi_card::{BangumiCard, BangumiFormat};
use domain::BangumiItem;

use super::CardClickCallback;

/// Render a batch of bangumi with the shared card layout (used by both weekday groups and movies).
/// `key_prefix` is appended with the index so card ids are unique within a grid.
pub(super) fn render_card_grid(
    items: &[BangumiItem],
    format: BangumiFormat,
    key_prefix: &str,
    on_card_click: &CardClickCallback,
    network: &Arc<Network>,
) -> impl IntoElement {
    let cards = items.iter().enumerate().map(move |(ix, item)| {
        let name = item.name.clone();
        let click_name = name.clone();
        let bid = item.bangumi_id;
        let on_click = on_card_click.clone();
        BangumiCard {
            name: name.clone(),
            bangumi_id: bid,
            poster_url: item.cover_url.clone().unwrap_or_default(),
            format,
            key: format!("{key_prefix}{ix}"),
            on_click: Some(Rc::new(move |window, app| {
                on_click(&click_name, bid, window, app);
            })),
            network: network.clone(),
        }
    });

    gpui_kit::div()
        .w_full()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(px(18.))
        .children(cards)
}
