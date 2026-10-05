//! Purely presentational settings widgets: card container, segmented selector, appearance row, and about row.

use std::path::PathBuf;

use gpui_kit::component::StyledExt;
use gpui_kit::component::theme::{Theme, ThemeMode};
use gpui_kit::{prelude::*, px};

use super::SelectThemeCallback;
use crate::theme::icons::icon;

/// Group card: title + description + several rows.
pub(super) fn card(
    theme: &Theme,
    title: &str,
    desc: &str,
    body: Vec<gpui_kit::AnyElement>,
) -> impl IntoElement {
    gpui_kit::div()
        .w_full()
        .rounded(px(10.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .overflow_hidden()
        .child(
            gpui_kit::div()
                .w_full()
                .px(px(16.))
                .py(px(12.))
                .border_b_1()
                .border_color(theme.border)
                .bg(theme.muted)
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    gpui_kit::div()
                        .text_base()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .child(title.to_string()),
                )
                .child(
                    gpui_kit::div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(desc.to_string()),
                ),
        )
        .children(body)
}

/// iOS-style segmented control: grey track + thumb.
fn segment_btn(label: &str, active: bool, theme: &Theme) -> gpui_kit::Div {
    let (bg, fg) = if active {
        (theme.background, theme.foreground)
    } else {
        (theme.transparent, theme.muted_foreground)
    };

    gpui_kit::div()
        .px(px(14.))
        .py(px(5.))
        .bg(bg)
        .text_color(fg)
        .text_sm()
        .rounded(px(6.))
        .cursor_pointer()
        .when(active, |this| this.shadow_xs())
        .hover(move |style| {
            if active {
                style
            } else {
                style.bg(theme.accent).text_color(theme.foreground)
            }
        })
        .child(label.to_string())
}

/// Appearance row: theme-mode segmented selector.
pub(super) fn appearance_row(
    theme: &Theme,
    mode: ThemeMode,
    on_select: SelectThemeCallback,
) -> impl IntoElement {
    let segment = gpui_kit::div()
        .flex()
        .gap(px(2.))
        .bg(theme.muted)
        .rounded(px(8.))
        .p(px(2.))
        .child(
            segment_btn("浅色", matches!(mode, ThemeMode::Light), theme)
                .id("theme-light")
                .on_click({
                    let on_select = on_select.clone();
                    move |_, window, app| on_select(ThemeMode::Light, window, app)
                }),
        )
        .child(
            segment_btn("深色", matches!(mode, ThemeMode::Dark), theme)
                .id("theme-dark")
                .on_click(move |_, window, app| on_select(ThemeMode::Dark, window, app)),
        );

    gpui_kit::div()
        .w_full()
        .px(px(16.))
        .py(px(14.))
        .flex()
        .items_center()
        .justify_between()
        .child(
            gpui_kit::div()
                .flex_1()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(icon("moon", 16.).text_color(theme.muted_foreground))
                .child(
                    gpui_kit::div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            gpui_kit::div()
                                .text_sm()
                                .text_color(theme.foreground)
                                .child("主题模式"),
                        )
                        .child(
                            gpui_kit::div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("浅色 / 深色 · 也可使用 ⌘⇧L 快速切换"),
                        ),
                ),
        )
        .child(segment)
}

/// About row (link: open a URL; open_dir: open a local directory).
pub(super) fn about_row(
    theme: &Theme,
    label: &str,
    value: &str,
    icon_name: &str,
    link: Option<&str>,
    open_dir: Option<PathBuf>,
) -> impl IntoElement {
    let value = value.to_string();
    gpui_kit::div()
        .w_full()
        .px(px(16.))
        .py(px(14.))
        .border_t_1()
        .border_color(theme.border)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .child(
            gpui_kit::div()
                .flex_1()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(icon(icon_name, 15.).text_color(theme.muted_foreground))
                .child(
                    gpui_kit::div()
                        .text_sm()
                        .text_color(theme.foreground)
                        .child(label.to_string()),
                ),
        )
        .child(if let Some(url) = link {
            let url = url.to_string();
            gpui_kit::div()
                .text_sm()
                .text_color(theme.link)
                .cursor_pointer()
                .id(gpui_kit::SharedString::from(format!("about-{label}")))
                .hover(|style| style.text_color(theme.link_hover).underline())
                .on_click(move |_, _, _| {
                    let _ = storage::paths::open_url(&url);
                })
                .child(value)
                .into_any_element()
        } else {
            gpui_kit::div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .max_w(px(300.))
                .truncate()
                .child(value)
                .into_any_element()
        })
        .child(if let Some(dir) = open_dir {
            gpui_kit::div()
                .px(px(10.))
                .py(px(4.))
                .rounded(px(6.))
                .border_1()
                .border_color(theme.border)
                .text_xs()
                .text_color(theme.foreground)
                .cursor_pointer()
                .id(gpui_kit::SharedString::from(format!("open-dir-{label}")))
                .hover(|style| style.bg(theme.list_hover).border_color(theme.primary))
                .on_click(move |_, _, _| {
                    let _ = storage::paths::open_path(&dir);
                })
                .child("打开")
                .into_any_element()
        } else {
            gpui_kit::div().into_any_element()
        })
}
