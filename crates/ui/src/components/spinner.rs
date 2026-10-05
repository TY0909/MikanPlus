//! Spinning loading indicator: the same style as the home loading state (a
//! rotating Loader icon), reusable inline and at smaller sizes.

use gpui_kit::component::{Icon, IconName, Sizable};
use gpui_kit::{Animation, AnimationExt, Hsla, Transformation, percentage, prelude::*, px};

/// Spinning loading icon.
///
/// - `id`: unique key for the animation (multiple spinners in the same view must
///   all differ, otherwise their animation state collides)
/// - `size`: icon size in px; the home loading state uses 30, inline uses 14-16
/// - `color`: icon color (usually `theme.primary` or `theme.muted_foreground`)
pub fn spinner(id: impl Into<gpui_kit::SharedString>, size: f32, color: Hsla) -> impl IntoElement {
    Icon::new(IconName::Loader)
        .with_size(px(size))
        .text_color(color)
        .with_animation(
            id.into(),
            Animation::new(std::time::Duration::from_secs_f64(1.6)).repeat(),
            |this, delta| this.transform(Transformation::rotate(percentage(delta))),
        )
}
