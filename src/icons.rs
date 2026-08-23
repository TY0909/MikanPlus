//! 图标辅助函数。
//!
//! 使用 lucide 风格 SVG(lucide-static,ISC 许可),存放在 `assets/icons/`。
//! 图标使用 `currentColor`,通过 `text_color` 着色。

use gpui::{Svg, prelude::*, px};

/// 渲染一个指定尺寸的 SVG 图标(返回 `Svg`,可继续链式设置颜色等样式)。
pub fn icon(name: &str, size: f32) -> Svg {
    gpui::svg().path(format!("icons/{name}.svg")).size(px(size))
}

/// 渲染一个指定尺寸、指定颜色的 SVG 图标。
pub fn icon_colored(name: &str, size: f32, color: impl Into<gpui::Hsla>) -> Svg {
    gpui::svg()
        .path(format!("icons/{name}.svg"))
        .size(px(size))
        .text_color(color)
}

/// 构造一个菜单项图标(用于 `PopupMenuItem::icon`)。
pub fn menu_icon(name: &str) -> gpui_component::Icon {
    gpui_component::Icon::default().path(format!("icons/{name}.svg"))
}
