//! Weekday groups: weekday colors, `day` field names, and a single-day card section.

use std::sync::Arc;

use gpui_kit::component::StyledExt;
use gpui_kit::component::theme::Theme;
use gpui_kit::{AnyElement, Hsla, prelude::*, px};
use source::Network;

use domain::BangumiGroup;
use domain::navigation::WEEKDAY_NAMES;

use super::CardClickCallback;
use super::HomeView;
use super::card_grid::render_card_grid;
use crate::components::bangumi_card::BangumiFormat;

/// The `day` field name for a weekday index (0=Mon … 6=Sun).
pub(super) fn day_key(i: usize) -> &'static str {
    [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ][i]
}

/// Accent color for each weekday.
pub(super) fn weekday_color(day: usize) -> Hsla {
    match day {
        0 => gpui_kit::hsla(0.0, 0.72, 0.55, 1.0),
        1 => gpui_kit::hsla(24.0 / 360.0, 0.90, 0.55, 1.0),
        2 => gpui_kit::hsla(46.0 / 360.0, 0.95, 0.55, 1.0),
        3 => gpui_kit::hsla(145.0 / 360.0, 0.55, 0.48, 1.0),
        4 => gpui_kit::hsla(210.0 / 360.0, 0.95, 0.55, 1.0),
        5 => gpui_kit::hsla(250.0 / 360.0, 0.55, 0.60, 1.0),
        _ => gpui_kit::hsla(280.0 / 360.0, 0.55, 0.58, 1.0),
    }
}

/// Build a card grid for one weekday; when `featured` is true the heading gets a "今天" (today) badge.
pub(super) fn render_weekday_section(
    groups: &[BangumiGroup],
    day_idx: usize,
    featured: bool,
    on_card_click: &CardClickCallback,
    network: &Arc<Network>,
    theme: &Theme,
) -> AnyElement {
    let Some(group) = HomeView::group_by_day(groups, day_key(day_idx)) else {
        return gpui_kit::div().into_any_element();
    };
    let color = weekday_color(day_idx);

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
                .bg(color),
        )
        .child(
            gpui_kit::div()
                .text_base()
                .font_semibold()
                .text_color(theme.foreground)
                .child(WEEKDAY_NAMES[day_idx].to_string()),
        )
        .child(
            gpui_kit::div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(format!("{} 部", group.items.len())),
        )
        .when(featured, |this| {
            this.child(
                gpui_kit::div()
                    .ml(px(4.))
                    .px(px(8.))
                    .py(px(2.))
                    .rounded_full()
                    .bg(theme.primary)
                    .text_xs()
                    .text_color(theme.primary_foreground)
                    .child("今天"),
            )
        });

    gpui_kit::div()
        .w_full()
        .mb(px(32.))
        .child(header)
        .child(render_card_grid(
            &group.items,
            BangumiFormat::Tv,
            &format!("home-card-{day_idx}-"),
            on_card_click,
            network,
        ))
        .into_any_element()
}
