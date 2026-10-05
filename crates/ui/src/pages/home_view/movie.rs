//! Movie group: heading and movie card grid.

use std::sync::Arc;

use gpui_kit::component::StyledExt;
use gpui_kit::component::theme::Theme;
use gpui_kit::{AnyElement, prelude::*, px};
use source::Network;

use domain::BangumiGroup;

use super::CardClickCallback;
use super::HomeView;
use super::card_grid::render_card_grid;
use crate::components::bangumi_card::BangumiFormat;

/// Movie group; renders nothing when there is no data.
pub(super) fn render_movie_section(
    groups: &[BangumiGroup],
    on_card_click: &CardClickCallback,
    network: &Arc<Network>,
    theme: &Theme,
) -> AnyElement {
    let Some(group) = HomeView::group_by_day(groups, "movie") else {
        return gpui_kit::div().into_any_element();
    };

    let header = gpui_kit::div()
        .w_full()
        .mb(px(14.))
        .flex()
        .items_center()
        .gap(px(8.))
        .child(
            gpui_kit::div()
                .w(px(4.))
                .h(px(18.))
                .rounded(px(2.))
                .bg(theme.info),
        )
        .child(
            gpui_kit::div()
                .text_base()
                .font_semibold()
                .text_color(theme.foreground)
                .child("剧场版"),
        )
        .child(
            gpui_kit::div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(format!("{} 部", group.items.len())),
        );

    gpui_kit::div()
        .w_full()
        .mb(px(32.))
        .child(header)
        .child(render_card_grid(
            &group.items,
            BangumiFormat::Movie,
            "movie-card-",
            on_card_click,
            network,
        ))
        .into_any_element()
}
