//! Episode title cell: shows the truncated title, and the full title on hover when truncated.

use gpui_kit::component::ActiveTheme;

use gpui_kit::component::theme::Theme;
use gpui_kit::{AnyView, App, Context, Render, Window, prelude::*, px};

/// Hover tooltip content view: the full episode title, width-limited and word-wrapped.
///
/// Does not use gpui-component's `Tooltip` (internally a flex row layout, where long text is measured at
/// its intrinsic width and overflows/clips the container). A plain block-level div is used instead so the
/// text wraps reliably under the max_w constraint.
struct TitleTooltip {
    text: String,
}

impl Render for TitleTooltip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        gpui_kit::div()
            .id("ep-title-tooltip")
            .max_w(px(480.))
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.popover)
            .shadow_md()
            .px(px(12.))
            .py(px(8.))
            .child(
                gpui_kit::div()
                    .max_w(px(456.))
                    .text_sm()
                    .text_color(theme.popover_foreground)
                    .child(self.text.clone()),
            )
    }
}

/// Build a title tooltip view (width-limited, word-wrapped), reused by episode rows and bangumi cards.
pub fn title_tooltip(text: String) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |_window, cx| cx.new(|_| TitleTooltip { text: text.clone() }).into()
}

/// Episode title cell: shows the truncated title; if truncation occurred, shows the full title on hover.
///
/// Note: `.tooltip()` belongs to `StatefulInteractiveElement` and is only available on `Stateful<Div>`,
/// so `.id()` must be called first here (just as `.on_click()` requires `.id()`).
pub fn title_cell(
    gix: usize,
    full_title: &str,
    truncated: &str,
    theme: &Theme,
) -> impl IntoElement {
    let truncated_owned = truncated.to_string();
    let is_truncated = truncated != full_title;
    let full = full_title.to_string();
    let mut el = gpui_kit::div()
        .id(gpui_kit::SharedString::from(format!(
            "ep-title-{gix}-{full}"
        )))
        .w_full()
        .text_sm()
        .text_color(theme.foreground)
        .truncate();
    if is_truncated {
        el = el.tooltip(title_tooltip(full.clone()));
    }
    el.child(truncated_owned)
}
