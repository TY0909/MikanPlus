//! Episode list card: header row (title + filter button), episode rows, and loading/failure/empty states.

use std::path::Path;
use std::sync::Arc;

use gpui_kit::component::StyledExt;
use gpui_kit::component::theme::Theme;
use gpui_kit::{prelude::*, px};

use crate::components::episode_row::OpenCollectionCallback;
use crate::components::load_state::GroupEpisodesState;
use crate::theme::app_theme;
use crate::theme::icons::icon;
use downloader::{DownloadManager, TaskView};

use super::filter_button::render_filter_button;
use super::row::render_episode_row;
use super::{OpenFilterCallback, ReloadEpisodesCallback};

/// Rendering context for the episode list card (filtered results + download info + callbacks).
pub(super) struct EpisodeListContext<'a> {
    pub(super) visible: &'a [(usize, domain::Episode)],
    pub(super) visible_count: usize,
    pub(super) filtering: bool,
    pub(super) keyword: &'a str,
    pub(super) episodes_state: &'a GroupEpisodesState,
    pub(super) dl_dir: &'a Path,
    pub(super) downloader: &'a Arc<DownloadManager>,
    pub(super) snapshot: &'a [TaskView],
    pub(super) on_open_collection: &'a OpenCollectionCallback,
    pub(super) on_open_filter: &'a OpenFilterCallback,
    pub(super) on_reload_episodes: &'a ReloadEpisodesCallback,
    pub(super) title_max_px: f32,
    pub(super) theme: &'a Theme,
}

pub(super) fn render_episode_list(ctx: EpisodeListContext<'_>) -> impl IntoElement {
    let rows: Vec<gpui_kit::AnyElement> = ctx
        .visible
        .iter()
        .map(|(ix, ep)| {
            render_episode_row(
                *ix,
                ep,
                ctx.title_max_px,
                ctx.dl_dir,
                ctx.downloader,
                ctx.snapshot,
                ctx.on_open_collection,
                ctx.theme,
            )
        })
        .collect();

    let filter_btn =
        render_filter_button(ctx.filtering, ctx.keyword, ctx.on_open_filter, ctx.theme);

    // List body: loading / failure retry / episode rows (including the filtered empty state).
    let list_body: Vec<gpui_kit::AnyElement> = match ctx.episodes_state {
        GroupEpisodesState::Loading => vec![
            gpui_kit::div()
                .w_full()
                .py(px(40.))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(8.))
                .child(crate::components::spinner::spinner(
                    "sg-spin",
                    15.,
                    ctx.theme.muted_foreground,
                ))
                .child(
                    gpui_kit::div()
                        .text_sm()
                        .text_color(ctx.theme.muted_foreground)
                        .child("正在加载剧集…"),
                )
                .into_any_element(),
        ],
        GroupEpisodesState::Failed(message) => {
            let on_reload_episodes = ctx.on_reload_episodes.clone();
            vec![
                gpui_kit::div()
                    .w_full()
                    .py(px(40.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        gpui_kit::div()
                            .text_sm()
                            .text_color(ctx.theme.muted_foreground)
                            .child(message.clone()),
                    )
                    .child(
                        gpui_kit::div()
                            .id("sg-retry")
                            .px(px(12.))
                            .py(px(4.))
                            .rounded(px(6.))
                            .border_1()
                            .border_color(ctx.theme.border)
                            .text_xs()
                            .font_semibold()
                            .text_color(ctx.theme.foreground)
                            .cursor_pointer()
                            .hover(|style| {
                                style
                                    .bg(ctx.theme.list_hover)
                                    .border_color(ctx.theme.primary)
                            })
                            .on_click(move |_, window, app| on_reload_episodes(window, app))
                            .child("重试"),
                    )
                    .into_any_element(),
            ]
        }
        GroupEpisodesState::Ready(_) if ctx.visible_count == 0 => vec![
            // No results after filtering: a friendly empty state.
            gpui_kit::div()
                .w_full()
                .py(px(40.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.))
                .child(icon("inbox", 24.).text_color(ctx.theme.muted_foreground))
                .child(
                    gpui_kit::div()
                        .text_sm()
                        .text_color(ctx.theme.muted_foreground)
                        .child(if ctx.filtering {
                            format!("没有标题包含「{}」的剧集", ctx.keyword)
                        } else {
                            "该字幕组暂无剧集".to_string()
                        }),
                )
                .into_any_element(),
        ],
        GroupEpisodesState::Ready(_) => rows,
    };

    gpui_kit::div()
        .mt(px(20.))
        .w_full()
        .rounded(px(10.))
        .overflow_hidden()
        .border_1()
        .border_color(ctx.theme.border)
        .bg(app_theme::card(ctx.theme))
        .child(
            // List header row: title + filter button (the title does not shrink; the button text truncates).
            gpui_kit::div()
                .w_full()
                .px(px(14.))
                .py(px(8.))
                .bg(ctx.theme.muted)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .child(
                    gpui_kit::div()
                        .flex_shrink_0()
                        .text_sm()
                        .font_semibold()
                        .text_color(ctx.theme.foreground)
                        .child("剧集列表"),
                )
                .child(filter_btn),
        )
        .children(list_body)
}
