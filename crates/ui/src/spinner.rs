//! 旋转加载指示器:与首页加载态同款(旋转的 Loader 图标),可内联/缩小复用。

use gpui_kit::component::{Icon, IconName, Sizable};
use gpui_kit::{Animation, AnimationExt, Hsla, Transformation, percentage, prelude::*, px};

/// 旋转加载图标。
///
/// - `id`:动画唯一键(同一视图内多个 spinner 必须各不相同,否则动画状态会串)
/// - `size`:图标尺寸(px);首页加载态用 30,内联场景用 14~16
/// - `color`:图标颜色(通常用 `theme.primary` 或 `theme.muted_foreground`)
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
