//! Home top filter bar: Today / Mon–Sun / Movies.

use gpui_kit::component::StyledExt;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::theme::Theme;
use gpui_kit::{prelude::*, px};

use domain::navigation::{HomeFilter, WEEKDAY_SHORT};

use super::FilterChangeCallback;

/// Horizontally scrollable segmented filter bar; each segment carries its own click callback.
pub(super) fn render_filter_bar(
    filter: HomeFilter,
    on_filter_change: &FilterChangeCallback,
    theme: &Theme,
) -> impl IntoElement {
    let filter_items: Vec<(HomeFilter, &str, bool)> =
        vec![(HomeFilter::Today, "今日", filter == HomeFilter::Today)]
            .into_iter()
            .chain((0..7).map(|day| {
                let active = matches!(filter, HomeFilter::Weekday(d) if d == day);
                (HomeFilter::Weekday(day), WEEKDAY_SHORT[day], active)
            }))
            .chain(std::iter::once((
                HomeFilter::Movies,
                "剧场版",
                filter == HomeFilter::Movies,
            )))
            .collect();

    gpui_kit::div().w_full().mb(px(20.)).child(
        gpui_kit::div().w_full().overflow_x_scrollbar().child(
            gpui_kit::div()
                .flex_shrink_0()
                .flex()
                .gap(px(2.))
                .bg(theme.muted)
                .rounded(px(8.))
                .p(px(2.))
                .children(filter_items.into_iter().map(|(f, label, active)| {
                    let on_filter_change = on_filter_change.clone();
                    let (bg, fg) = if active {
                        (theme.background, theme.foreground)
                    } else {
                        (theme.transparent, theme.muted_foreground)
                    };
                    gpui_kit::div()
                        .px(px(14.))
                        .py(px(4.))
                        .rounded(px(6.))
                        .bg(bg)
                        .text_color(fg)
                        .text_sm()
                        .font_semibold()
                        .cursor_pointer()
                        .id(gpui_kit::SharedString::from(format!("home-filter-{f:?}")))
                        .when(active, |this| this.shadow_xs())
                        .hover(move |style| {
                            if active {
                                style
                            } else {
                                style.bg(theme.accent).text_color(theme.foreground)
                            }
                        })
                        .on_click(move |_, window, app| on_filter_change(f, window, app))
                        .child(label.to_string())
                })),
        ),
    )
}
