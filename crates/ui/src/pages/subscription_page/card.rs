//! Rendering of a single subscription card, plus responsive card sizing by window width.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::StyledExt;
use gpui_kit::component::theme::Theme;
use gpui_kit::{App, Window, prelude::*, px};
use source::Network;

use crate::components::poster::poster;
use crate::theme::layout::MAX_PAGE_W;

/// Compute the subscription card column count and width from the window width (responsive).
///
/// The content container has 32px padding on each side and a width cap of `MAX_PAGE_W`;
/// the gap between cards is 18px and the minimum card width is 240px.
pub(super) fn responsive_card_size(win_w: f32) -> (usize, f32) {
    let cols = if win_w >= 1700. {
        5
    } else if win_w >= 1300. {
        4
    } else if win_w >= 900. {
        3
    } else {
        2
    };
    let content_w = (win_w.min(MAX_PAGE_W)) - 64.0;
    let card_w = ((content_w - 18.0 * (cols as f32 - 1.0)) / cols as f32).max(240.0);
    (cols, card_w)
}

/// Subscription card: poster on the left + text on the right; width changes responsively with the window.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_sub_card(
    network: &Arc<Network>,
    name: String,
    group_name: String,
    poster_url: String,
    card_w: f32,
    is_dark: bool,
    theme: &Theme,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
    on_unsubscribe: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let name_owned = name.clone();
    let group_name_owned = group_name.clone();
    let on_unsubscribe = Rc::new(on_unsubscribe);

    // Hover overlay: "取消订阅" (unsubscribe) button (clicking unsubscribes that group without bubbling to the card).
    // Rounding matches the poster area (left corners only) so right angles do not break the single-side rounded design.
    let overlay = gpui_kit::div()
        .absolute()
        .inset_0()
        .rounded_l(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui_kit::hsla(0., 0., 0., 0.55))
        .opacity(0.)
        .hover(|style| style.opacity(1.))
        .child(
            gpui_kit::div()
                .px(px(12.))
                .py(px(6.))
                .rounded_full()
                .bg(theme.danger)
                .text_sm()
                .font_semibold()
                .text_color(theme.danger_foreground)
                .cursor_pointer()
                .id(gpui_kit::SharedString::from(format!(
                    "unsub-{}-{}",
                    name, group_name
                )))
                .hover(|style| style.bg(theme.danger_hover))
                .on_click({
                    let on_unsubscribe = on_unsubscribe.clone();
                    move |_ev, window, cx: &mut App| {
                        cx.stop_propagation();
                        on_unsubscribe(window, cx);
                    }
                })
                .child("取消订阅"),
        );

    gpui_kit::div()
        .w(px(card_w))
        .h(px(140.))
        .bg(theme.background)
        .rounded(px(10.))
        .overflow_hidden()
        .border_1()
        .border_color(theme.border)
        .relative()
        .cursor_pointer()
        .id(gpui_kit::SharedString::from(format!(
            "sub-card-{}-{}",
            name, group_name
        )))
        .on_click(move |_, window, app| on_click(window, app))
        .hover(|style| style.border_color(theme.primary))
        .child(
            // Poster area: absolutely positioned on the left with a 5:7 ratio matching the cover; only the two left
            // corners are rounded (the right corners stay square to join the text area).
            gpui_kit::div()
                .absolute()
                .left_0()
                .top_0()
                .w(px(100.))
                .h_full()
                .rounded_l(px(10.))
                .overflow_hidden()
                .child(poster(
                    network,
                    &poster_url,
                    &name_owned,
                    is_dark,
                    px(100.),
                    px(140.),
                    10.,
                    crate::components::poster::CornerStyle::Left,
                ))
                .child(overlay),
        )
        .child(
            // Text area: margin avoids the poster, width automatically = remaining space.
            // Line spacing uses explicit mt (vertical gap is unreliable in gpui 0.2.2).
            gpui_kit::div()
                .ml(px(112.))
                .h_full()
                .flex()
                .flex_col()
                .p(px(12.))
                .child({
                    // Bangumi name: at most two lines + ellipsis; shows the full name on hover when it may be truncated.
                    let name_w = (card_w - 136.0).max(60.0);
                    let mut name_el = gpui_kit::div()
                        .id(gpui_kit::SharedString::from(format!(
                            "sub-card-name-{name}"
                        )))
                        .text_sm()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .line_clamp(2)
                        .text_ellipsis();
                    if crate::components::episode_row::exceeds_lines(&name, name_w, 14.0, 2) {
                        name_el = name_el
                            .tooltip(crate::components::episode_row::title_tooltip(name.clone()));
                    }
                    name_el.child(name_owned)
                })
                .child(
                    gpui_kit::div()
                        .mt(px(6.))
                        .text_xs()
                        .font_semibold()
                        .text_color(theme.primary)
                        .truncate()
                        .child(group_name_owned),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::responsive_card_size;

    #[test]
    fn wide_screen_uses_more_columns() {
        // 1920: 5 columns, ~357px per card.
        let (cols, w) = responsive_card_size(1920.0);
        assert_eq!(cols, 5);
        assert!((w - 356.8).abs() < 0.1);
        // 1440: 4 columns.
        assert_eq!(responsive_card_size(1440.0).0, 4);
        // 1100: 3 columns.
        assert_eq!(responsive_card_size(1100.0).0, 3);
        // 800: 2 columns.
        assert_eq!(responsive_card_size(800.0).0, 2);
    }

    #[test]
    fn card_width_fits_content() {
        // Ultra-wide screens are bounded by the MAX_PAGE_W cap.
        let (cols, w) = responsive_card_size(3840.0);
        assert_eq!(cols, 5);
        assert!((w - 356.8).abs() < 0.1);
        // Card width does not fall below the minimum.
        let (_, w) = responsive_card_size(700.0);
        assert!(w >= 240.0);
    }
}
