//! Page-level layout: full-width scrolling with content centered under a limit.
//!
//! The scroll container spans the full viewport width (so the scrollbar always
//! hugs the window's right edge), while the content is `w_full` and adapts to the
//! window width; it is centered under a limit only on ultra-wide screens to keep
//! grids and text lines from stretching endlessly. All page width limits are
//! managed centrally in this module.
//!
//! Scroll position is maintained by a caller-held [`ScrollHandle`] (rather than
//! element-internal keyed state): the handle is not dropped on page switches, so
//! the scroll position is preserved when returning.

use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{ScrollHandle, prelude::*, px};

/// Content width limit for grid pages (home / subscriptions / search).
pub const MAX_PAGE_W: f32 = 1920.0;
/// Content width limit for the anime detail page.
pub const MAX_DETAIL_W: f32 = 1600.0;
/// Content width limit for the subtitle-group episode list page.
pub const MAX_SUBGROUP_W: f32 = 1280.0;
/// Content width limit for the settings page (form rows don't need to be as wide as a grid).
pub const MAX_SETTINGS_W: f32 = 960.0;

/// Generic page scroll container: the scrollbar hugs the window's right edge, and
/// the content adapts to the width and is centered.
///
/// Usage: the page root is `size_full`; the scroll container returned by this
/// function is `w_full + h_full` to fill the parent, and the content is then
/// centered under the `max_w` limit.
pub fn page_scroll(
    max_w: f32,
    handle: &ScrollHandle,
    content: impl IntoElement,
) -> impl IntoElement {
    gpui_kit::div()
        .id("page-scroll")
        .w_full()
        .h_full()
        .overflow_y_scroll()
        .track_scroll(handle)
        .vertical_scrollbar(handle)
        .child(
            gpui_kit::div().w_full().flex().justify_center().child(
                gpui_kit::div()
                    .w_full()
                    .max_w(px(max_w))
                    .px(px(32.))
                    .pt(px(28.))
                    .pb(px(24.))
                    .child(content),
            ),
        )
}
