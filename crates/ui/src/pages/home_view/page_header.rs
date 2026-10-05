//! Home page header: shows the title and statistics for the current filter.

use gpui_kit::component::StyledExt;
use gpui_kit::component::theme::Theme;
use gpui_kit::{prelude::*, px};

use crate::theme::icons::icon;
use domain::BangumiGroup;
use domain::navigation::{HomeFilter, WEEKDAY_NAMES};

use super::HomeView;
use super::weekday::{day_key, weekday_color};

/// Page header below the filter bar (three variants: Today / Weekday / Movies).
pub(super) fn render_page_header(
    filter: HomeFilter,
    today: usize,
    groups: &[BangumiGroup],
    theme: &Theme,
) -> impl IntoElement {
    match filter {
        HomeFilter::Today => {
            let day_color = weekday_color(today);
            // Build the local date string, e.g. 「8月3日 · 星期一」 (August 3 · Monday).
            let (y, m, d) = {
                use chrono::Datelike;
                let now = chrono::Local::now();
                (now.year(), now.month(), now.day())
            };
            gpui_kit::div()
                .w_full()
                .mb(px(24.))
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    gpui_kit::div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(icon("sparkles", 22.).text_color(theme.primary))
                        .child(
                            gpui_kit::div()
                                .text_2xl()
                                .font_bold()
                                .text_color(theme.foreground)
                                .child("今日更新"),
                        ),
                )
                .child(
                    gpui_kit::div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(format!("{y} 年 {m} 月 {d} 日 · {}", WEEKDAY_NAMES[today])),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(4.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(gpui_kit::div().size(px(8.)).rounded_full().bg(day_color))
                        .child(
                            gpui_kit::div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(format!(
                                    "今天是 {},共 {} 部新番放送",
                                    WEEKDAY_NAMES[today],
                                    HomeView::group_by_day(groups, day_key(today))
                                        .map(|g| g.items.len())
                                        .unwrap_or(0)
                                )),
                        ),
                )
        }
        HomeFilter::Weekday(day) => gpui_kit::div()
            .w_full()
            .mb(px(24.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                gpui_kit::div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        gpui_kit::div()
                            .size(px(14.))
                            .rounded_full()
                            .bg(weekday_color(day)),
                    )
                    .child(
                        gpui_kit::div()
                            .text_2xl()
                            .font_bold()
                            .text_color(theme.foreground)
                            .child(WEEKDAY_NAMES[day].to_string()),
                    ),
            )
            .child(
                gpui_kit::div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(format!(
                        "共 {} 部番剧 · 每周{}更新",
                        HomeView::group_by_day(groups, day_key(day))
                            .map(|g| g.items.len())
                            .unwrap_or(0),
                        WEEKDAY_NAMES[day]
                    )),
            ),
        HomeFilter::Movies => gpui_kit::div()
            .w_full()
            .mb(px(24.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                gpui_kit::div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(icon("film", 22.).text_color(theme.info))
                    .child(
                        gpui_kit::div()
                            .text_2xl()
                            .font_bold()
                            .text_color(theme.foreground)
                            .child("剧场版"),
                    ),
            )
            .child(
                gpui_kit::div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("电影与特别篇"),
            ),
    }
}
