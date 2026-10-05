//! Page header: bangumi name / subtitle-group name / episode count.

use gpui_kit::component::StyledExt;
use gpui_kit::component::theme::Theme;
use gpui_kit::{prelude::*, px};

use crate::theme::icons::icon;

pub(super) fn render_header(
    bangumi_name: &str,
    group_name: &str,
    total: usize,
    visible_count: usize,
    filtering: bool,
    theme: &Theme,
) -> impl IntoElement {
    gpui_kit::div()
        .w_full()
        .flex()
        .flex_col()
        .child(
            gpui_kit::div()
                .text_2xl()
                .font_bold()
                .text_color(theme.foreground)
                .child(bangumi_name.to_string()),
        )
        .child(
            gpui_kit::div()
                .mt(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(icon("users", 15.).text_color(theme.primary))
                .child(
                    gpui_kit::div()
                        .text_sm()
                        .font_semibold()
                        .text_color(theme.primary)
                        .child(group_name.to_string()),
                ),
        )
        .child(
            gpui_kit::div()
                .mt(px(6.))
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(if filtering {
                    format!("共 {total} 集 · 显示 {visible_count} 集")
                } else {
                    format!("共 {total} 集")
                }),
        )
}
