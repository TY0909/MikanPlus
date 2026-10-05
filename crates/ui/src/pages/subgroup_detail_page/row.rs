//! Single episode row: text column (title + size/date) and action column (download button).

use std::path::Path;
use std::sync::Arc;

use gpui_kit::component::theme::Theme;
use gpui_kit::{prelude::*, px, relative};

use crate::components::episode_row::OpenCollectionCallback;
use downloader::{DownloadManager, TaskView};

#[allow(clippy::too_many_arguments)]
pub(super) fn render_episode_row(
    ix: usize,
    ep: &domain::Episode,
    title_max_px: f32,
    dl_dir: &Path,
    downloader: &Arc<DownloadManager>,
    snapshot: &[TaskView],
    on_open_collection: &OpenCollectionCallback,
    theme: &Theme,
) -> gpui_kit::AnyElement {
    let magnet = ep.magnet_link.clone().unwrap_or_default();
    let size = ep.size.clone().unwrap_or_default();
    let date = ep.publish_date.clone().unwrap_or_default();
    let title = crate::components::episode_row::truncate_title(&ep.title, title_max_px, 14.0);

    let action_btn = crate::components::episode_row::action_button(
        ix,
        &title,
        &magnet,
        dl_dir,
        downloader,
        snapshot,
        on_open_collection,
        theme,
    );

    // Episode row: flex layout. Text column fixed at 60% (over-long text truncated),
    // action column fixed at 40% (the download area is a separate child, right-aligned by default).
    gpui_kit::div()
        .id(gpui_kit::SharedString::from(format!("sg-row-{ix}")))
        .w_full()
        .px(px(14.))
        .py(px(14.))
        .flex()
        .items_center()
        .rounded(px(8.))
        .hover(|style| style.bg(theme.list_hover))
        .child(
            // Text column: 60% of the space.
            gpui_kit::div()
                .w(relative(0.6))
                .flex()
                .flex_col()
                .child(crate::components::episode_row::title_cell(
                    ix, &ep.title, &title, theme,
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
}
