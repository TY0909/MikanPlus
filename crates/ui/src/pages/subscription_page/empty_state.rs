//! Subscription page empty state: a hint and a "discover bangumi" entry when there are no subscriptions.

use gpui_kit::component::StyledExt;
use gpui_kit::component::theme::Theme;
use gpui_kit::{prelude::*, px};

use super::GoHomeCallback;
use crate::theme::icons::icon;

pub(super) fn render_empty_state(theme: &Theme, on_go_home: GoHomeCallback) -> impl IntoElement {
    gpui_kit::div()
        .w_full()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(16.))
        .child(
            gpui_kit::div()
                .size(px(72.))
                .rounded_full()
                .bg(theme.muted)
                .flex()
                .items_center()
                .justify_center()
                .child(icon("heart", 32.).text_color(theme.muted_foreground)),
        )
        .child(
            gpui_kit::div()
                .text_lg()
                .font_semibold()
                .text_color(theme.foreground)
                .child("还没有订阅任何字幕组"),
        )
        .child(
            gpui_kit::div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child("在番剧详情页的字幕组卡片上点击「订阅」即可开始追番"),
        )
        .child(
            gpui_kit::div()
                .mt(px(4.))
                .px(px(18.))
                .py(px(8.))
                .rounded_full()
                .bg(theme.primary)
                .text_sm()
                .font_semibold()
                .text_color(theme.primary_foreground)
                .cursor_pointer()
                .id("sub-empty-cta")
                .hover(|style| style.bg(theme.primary_hover))
                .on_click(move |_, window, app| on_go_home(window, app))
                .child("去发现番剧 →"),
        )
}
