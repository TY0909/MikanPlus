//! Episode list for search results: header, current-page episode rows, empty state, and pagination controls.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::component::StyledExt;
use gpui_kit::component::theme::Theme;
use gpui_kit::{prelude::*, px};

use crate::components::episode_row::OpenCollectionCallback;
use crate::theme::app_theme;
use crate::theme::icons::icon;
use domain::SearchEpisode;
use downloader::{DownloadManager, TaskView};

use super::SearchPageChangeCallback;

/// Rendering context for the episode results section: download info + episode-row title column width.
pub(super) struct EpisodeListContext {
    pub(super) download_dir: PathBuf,
    pub(super) downloader: Arc<DownloadManager>,
    pub(super) snapshot: Vec<TaskView>,
    pub(super) on_open_collection: OpenCollectionCallback,
    /// Available width of the episode-row title column (used for data-layer truncation).
    pub(super) title_max_px: f32,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_episode_list(
    episodes: &[SearchEpisode],
    start: usize,
    end: usize,
    total_episodes: usize,
    total_pages: usize,
    page: usize,
    prev_enabled: bool,
    next_enabled: bool,
    ctx: EpisodeListContext,
    on_page_change: &SearchPageChangeCallback,
    theme: &Theme,
) -> impl IntoElement {
    // Episode row: title + size/date + download action (only the current page is rendered, avoiding rendering everything at once).
    let episode_rows = episodes[start..end]
        .iter()
        .enumerate()
        .map(|(ix, ep)| {
            let ix = start + ix;
            let magnet = ep.magnet.clone();
            let full_title = ep.title.clone();
            let title =
                crate::components::episode_row::truncate_title(&full_title, ctx.title_max_px, 14.0);
            let size = ep.size.clone();
            let date = ep.date.clone();
            let download_button = crate::components::episode_row::action_button(
                ix,
                &full_title,
                &magnet,
                &ctx.download_dir,
                &ctx.downloader,
                &ctx.snapshot,
                &ctx.on_open_collection,
                theme,
            );

            gpui_kit::div()
                .id(gpui_kit::SharedString::from(format!("search-ep-{ix}")))
                .w_full()
                .px(px(14.))
                .py(px(12.))
                .flex()
                .items_center()
                .gap(px(12.))
                .border_t_1()
                .border_color(theme.border)
                .hover(|style| style.bg(theme.list_hover))
                .child(
                    // Title column: takes the remaining space; over-long text is truncated at the data layer.
                    gpui_kit::div()
                        .flex_1()
                        .min_w(px(0.))
                        .flex()
                        .flex_col()
                        .child(crate::components::episode_row::title_cell(
                            ix,
                            &full_title,
                            &title,
                            theme,
                        ))
                        .child(
                            gpui_kit::div()
                                .mt(px(3.))
                                .flex()
                                .gap(px(10.))
                                .child(
                                    gpui_kit::div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(size),
                                )
                                .child(
                                    gpui_kit::div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(date),
                                ),
                        ),
                )
                // Action column: download (the whole column is hidden when the magnet is empty).
                .when(!magnet.is_empty(), |this| {
                    this.child(
                        gpui_kit::div()
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_end()
                            .child(download_button),
                    )
                })
        })
        .collect::<Vec<_>>();

    gpui_kit::div()
        .w_full()
        .rounded(px(10.))
        .overflow_hidden()
        .border_1()
        .border_color(theme.border)
        .bg(app_theme::card(theme))
        .child(
            gpui_kit::div()
                .w_full()
                .px(px(14.))
                .py(px(8.))
                .bg(theme.muted)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    gpui_kit::div()
                        .text_sm()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .child("剧集结果"),
                )
                .child(
                    gpui_kit::div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(if total_pages > 1 {
                            format!(
                                "第 {} / {} 页 · 共 {total_episodes} 条",
                                page + 1,
                                total_pages
                            )
                        } else {
                            format!("共 {total_episodes} 条")
                        }),
                ),
        )
        .children(episode_rows)
        .when(total_episodes == 0, |this| {
            this.child(
                gpui_kit::div()
                    .w_full()
                    .py(px(32.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("没有匹配的剧集"),
            )
        })
        .when(total_pages > 1, |this| {
            let prev = on_page_change.clone();
            let next = on_page_change.clone();
            this.child(
                // Pagination controls: previous / next (disabled and greyed out at the boundary pages).
                gpui_kit::div()
                    .w_full()
                    .px(px(14.))
                    .py(px(10.))
                    .border_t_1()
                    .border_color(theme.border)
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(8.))
                    .child(
                        gpui_kit::div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .px(px(12.))
                            .py(px(5.))
                            .rounded(px(6.))
                            .border_1()
                            .border_color(theme.border)
                            .text_xs()
                            .text_color(if prev_enabled {
                                theme.foreground
                            } else {
                                theme.muted_foreground
                            })
                            .id("search-page-prev")
                            .when(prev_enabled, |this| {
                                this.cursor_pointer().hover(|style| {
                                    style.bg(theme.list_hover).border_color(theme.primary)
                                })
                            })
                            .on_click(move |_, window, app| {
                                if prev_enabled {
                                    prev(page - 1, window, app);
                                }
                            })
                            .child(icon("arrow-left", 13.).text_color(theme.muted_foreground))
                            .child("上一页"),
                    )
                    .child(
                        gpui_kit::div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .px(px(12.))
                            .py(px(5.))
                            .rounded(px(6.))
                            .border_1()
                            .border_color(theme.border)
                            .text_xs()
                            .text_color(if next_enabled {
                                theme.foreground
                            } else {
                                theme.muted_foreground
                            })
                            .id("search-page-next")
                            .when(next_enabled, |this| {
                                this.cursor_pointer().hover(|style| {
                                    style.bg(theme.list_hover).border_color(theme.primary)
                                })
                            })
                            .on_click(move |_, window, app| {
                                if next_enabled {
                                    next(page + 1, window, app);
                                }
                            })
                            .child("下一页")
                            .child(icon("arrow-right", 13.).text_color(theme.muted_foreground)),
                    ),
            )
        })
}
