//! A single subtitle-group card: title (expand/collapse) and subscribe button; renders episode rows when expanded.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::component::StyledExt;
use gpui_kit::component::theme::Theme;
use gpui_kit::{App, Window, prelude::*, px, relative};

use crate::components::load_state::GroupEpisodesState;
use crate::theme::app_theme;
use crate::theme::icons::icon;
use downloader::{DownloadManager, TaskView};

/// Rendering context for a subtitle-group card (download info + episode-row title column width).
pub(super) struct GroupCardContext {
    pub(super) dl_dir: PathBuf,
    pub(super) downloader: Arc<DownloadManager>,
    pub(super) snapshot: Vec<TaskView>,
    pub(super) on_open_collection: crate::components::episode_row::OpenCollectionCallback,
    /// Available width of the episode-row title column (used for data-layer truncation).
    pub(super) title_max_px: f32,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_group_card(
    gix: usize,
    group_name: String,
    declared_count: u32,
    expanded: bool,
    state: Option<GroupEpisodesState>,
    subscribed: bool,
    theme: &Theme,
    ctx: GroupCardContext,
    on_toggle_group: impl Fn(&mut Window, &mut App) + 'static,
    on_reload_group: impl Fn(&mut Window, &mut App) + 'static,
    on_toggle_subscribe: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let subscribe_btn = gpui_kit::div()
        .px(px(12.))
        .py(px(5.))
        .rounded_full()
        .text_sm()
        .font_semibold()
        .flex_shrink_0()
        .cursor_pointer()
        .id(gpui_kit::SharedString::from(format!("grp-sub-{gix}")))
        .when(subscribed, |this| {
            this.bg(theme.success)
                .text_color(theme.success_foreground)
                .hover(|style| style.bg(theme.success_hover))
        })
        .when(!subscribed, |this| {
            this.bg(theme.primary)
                .text_color(theme.primary_foreground)
                .hover(|style| style.bg(theme.primary_hover))
        })
        .on_click(move |_, window, app| on_toggle_subscribe(window, app))
        .child(if subscribed { "已订阅" } else { "订阅" });

    // Truncate over-long group names; show the full name on hover.
    let mut name_el = gpui_kit::div()
        .id(gpui_kit::SharedString::from(format!("grp-name-{gix}")))
        .text_base()
        .font_semibold()
        .text_color(theme.foreground)
        .truncate();
    if crate::components::episode_row::exceeds_lines(&group_name, 320.0, 14.0, 1) {
        name_el = name_el.tooltip(crate::components::episode_row::title_tooltip(
            group_name.clone(),
        ));
    }
    name_el = name_el.child(group_name);

    // Count badge: when loaded, show "downloadable/total"; when not loaded, show the declared episode count.
    let badge = match &state {
        Some(GroupEpisodesState::Ready(episodes)) => {
            let downloadable = episodes
                .iter()
                .filter(|ep| ep.magnet_link.is_some())
                .count();
            format!("{downloadable}/{} 可下载", episodes.len())
        }
        _ if declared_count > 0 => format!("{declared_count} 集"),
        _ => "点击加载".to_string(),
    };
    // Expanded but episodes not yet returned: show a spinner in the header.
    let loading = expanded && matches!(state.as_ref(), None | Some(GroupEpisodesState::Loading));

    // Title area: chevron + group name + count; click to expand/collapse.
    let toggle_area = gpui_kit::div()
        .id(gpui_kit::SharedString::from(format!("grp-toggle-{gix}")))
        .flex_1()
        .flex()
        .items_center()
        .gap(px(8.))
        .overflow_hidden()
        .cursor_pointer()
        .on_click(move |_, window, app| on_toggle_group(window, app))
        .child(
            icon(
                if expanded {
                    "chevron-down"
                } else {
                    "chevron-right"
                },
                15.,
            )
            .text_color(theme.muted_foreground),
        )
        .child(icon("users", 15.).text_color(theme.muted_foreground))
        .child(name_el)
        .child(
            gpui_kit::div()
                .flex_shrink_0()
                .px(px(7.))
                .py(px(2.))
                .rounded_full()
                .bg(theme.background)
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(badge),
        )
        .when(loading, |this| {
            this.child(crate::components::spinner::spinner(
                format!("grp-spin-hdr-{gix}"),
                13.,
                theme.primary,
            ))
        });

    // Expanded content (loading / episode rows / failure retry / no episodes).
    let body: Vec<gpui_kit::AnyElement> = if !expanded {
        Vec::new()
    } else {
        match state {
            None | Some(GroupEpisodesState::Loading) => vec![
                gpui_kit::div()
                    .w_full()
                    .px(px(14.))
                    .py(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(8.))
                    .border_t_1()
                    .border_color(theme.border)
                    .child(crate::components::spinner::spinner(
                        format!("grp-spin-{gix}"),
                        15.,
                        theme.muted_foreground,
                    ))
                    .child(
                        gpui_kit::div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("正在加载剧集…"),
                    )
                    .into_any_element(),
            ],
            Some(GroupEpisodesState::Failed(message)) => vec![
                gpui_kit::div()
                    .w_full()
                    .px(px(14.))
                    .py(px(20.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(10.))
                    .border_t_1()
                    .border_color(theme.border)
                    .child(
                        gpui_kit::div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(message),
                    )
                    .child(
                        gpui_kit::div()
                            .id(gpui_kit::SharedString::from(format!("grp-retry-{gix}")))
                            .px(px(12.))
                            .py(px(4.))
                            .rounded(px(6.))
                            .border_1()
                            .border_color(theme.border)
                            .text_xs()
                            .font_semibold()
                            .text_color(theme.foreground)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.list_hover).border_color(theme.primary))
                            .on_click(move |_, window, app| on_reload_group(window, app))
                            .child("重试"),
                    )
                    .into_any_element(),
            ],
            Some(GroupEpisodesState::Ready(episodes)) if episodes.is_empty() => vec![
                gpui_kit::div()
                    .w_full()
                    .px(px(14.))
                    .py(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .border_t_1()
                    .border_color(theme.border)
                    .child("该字幕组暂无剧集")
                    .into_any_element(),
            ],
            Some(GroupEpisodesState::Ready(episodes)) => episodes
                .into_iter()
                .enumerate()
                .map(|(ep_ix, ep)| {
                    let title = crate::components::episode_row::truncate_title(
                        &ep.title,
                        ctx.title_max_px,
                        14.0,
                    );
                    let magnet = ep.magnet_link.clone().unwrap_or_default();
                    let size = ep.size.clone().unwrap_or_default();
                    let date = ep.publish_date.clone().unwrap_or_default();

                    // Right-hand action area: not downloaded → Download; downloading → progress + speed + cancel; completed → Open.
                    // Per-row unique key: group index + episode index within the group (titles cannot be ids: identical or identically-truncated titles would cross-wire state).
                    let action_btn = crate::components::episode_row::action_button(
                        gix * 10_000 + ep_ix,
                        &title,
                        &magnet,
                        &ctx.dl_dir,
                        &ctx.downloader,
                        &ctx.snapshot,
                        &ctx.on_open_collection,
                        theme,
                    );

                    // Episode row: flex layout. Text column fixed at 60% (over-long text truncated),
                    // action column fixed at 40% (the download area is a separate child, right-aligned by default).
                    gpui_kit::div()
                        .id(gpui_kit::SharedString::from(format!(
                            "ep-row-{gix}-{ep_ix}"
                        )))
                        .w_full()
                        .px(px(14.))
                        .py(px(14.))
                        .flex()
                        .items_center()
                        .border_t_1()
                        .border_color(theme.border)
                        .hover(|style| style.bg(theme.list_hover))
                        .child(
                            // Text column: 60% of the space.
                            gpui_kit::div()
                                .w(relative(0.6))
                                .flex()
                                .flex_col()
                                .child(crate::components::episode_row::title_cell(
                                    gix * 10_000 + ep_ix,
                                    &ep.title,
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
                        .child(
                            // Action column: 40% of the space, download area right-aligned; in very narrow windows the overflowing
                            // left side is clipped while the key buttons (cancel/open) on the right stay intact.
                            gpui_kit::div()
                                .w(relative(0.4))
                                .overflow_hidden()
                                .flex()
                                .items_center()
                                .justify_end()
                                .child(action_btn),
                        )
                        .into_any_element()
                })
                .collect(),
        }
    };

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
                .py(px(12.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .bg(theme.muted)
                .child(toggle_area)
                .child(subscribe_btn),
        )
        .children(body)
}
