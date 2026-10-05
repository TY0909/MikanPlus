//! Icon helper functions.
//!
//! Prefer GPUI Kit's built-in icons; the project only provides the missing
//! Lucide-style SVGs. Icons use `currentColor`, colored through `text_color`.

use gpui_kit::{Svg, prelude::*, px};

/// Render an SVG icon at the given size (returns `Svg`, so further styling such
/// as color can be chained).
pub fn icon(name: &str, size: f32) -> Svg {
    gpui_kit::svg()
        .path(format!("icons/{name}.svg"))
        .size(px(size))
}

/// Render an SVG icon at the given size with the given color.
pub fn icon_colored(name: &str, size: f32, color: impl Into<gpui_kit::Hsla>) -> Svg {
    gpui_kit::svg()
        .path(format!("icons/{name}.svg"))
        .size(px(size))
        .text_color(color)
}

/// Build a menu item icon (for `PopupMenuItem::icon`).
pub fn menu_icon(name: &str) -> gpui_kit::component::Icon {
    gpui_kit::component::Icon::default().path(format!("icons/{name}.svg"))
}
