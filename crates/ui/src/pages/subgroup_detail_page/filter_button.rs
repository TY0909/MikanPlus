//! The "filter" button in the episode-list header row.

use gpui_kit::component::StyledExt;
use gpui_kit::component::theme::Theme;
use gpui_kit::{prelude::*, px};

use crate::theme::icons::icon;

use super::OpenFilterCallback;

pub(super) fn render_filter_button(
    filtering: bool,
    keyword: &str,
    on_open_filter: &OpenFilterCallback,
    theme: &Theme,
) -> impl IntoElement {
    let on_open_filter = on_open_filter.clone();
    gpui_kit::div()
        .id("sg-filter-btn")
        .flex()
        .items_center()
        .gap(px(5.))
        .px(px(10.))
        .py(px(4.))
        .rounded(px(6.))
        .border_1()
        .border_color(theme.border)
        .bg(if filtering {
            theme.primary
        } else {
            theme.background
        })
        .text_xs()
        .font_semibold()
        .text_color(if filtering {
            theme.primary_foreground
        } else {
            theme.foreground
        })
        .cursor_pointer()
        .hover(|style| {
            if filtering {
                style.bg(theme.primary_hover)
            } else {
                style.bg(theme.list_hover).border_color(theme.primary)
            }
        })
        .on_click(move |_, window, app| on_open_filter(window, app))
        .child(icon("filter", 13.).text_color(if filtering {
            theme.primary_foreground
        } else {
            theme.muted_foreground
        }))
        .child(
            // Keyword text: truncated when too long so it does not push "剧集列表" (episode list) out of the header row.
            gpui_kit::div()
                .max_w(px(180.))
                .truncate()
                .child(if filtering {
                    format!("筛选 · {keyword}")
                } else {
                    "筛选".to_string()
                }),
        )
}
