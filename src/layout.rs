//! 页面级布局:全宽滚动 + 内容自适应居中。
//!
//! 滚动容器占满整个视口宽度(滚动条始终贴窗口右缘),
//! 内容 `w_full` 自适应窗口宽度;仅在超宽屏幕上受上限约束居中,
//! 避免网格/文本行无限延伸。所有页面的宽度上限集中在本模块管理。
//!
//! 滚动位置由调用方持有的 [`ScrollHandle`] 维持(而非元素内部 keyed state):
//! 页面切换时 handle 不销毁,返回后滚动位置得以保留。

use gpui::{ScrollHandle, prelude::*, px};
use gpui_component::scroll::ScrollableElement;

/// 网格类页面(首页 / 订阅 / 搜索)内容宽度上限
pub const MAX_PAGE_W: f32 = 1920.0;
/// 番剧详情页内容宽度上限
pub const MAX_DETAIL_W: f32 = 1600.0;
/// 字幕组剧集列表页内容宽度上限
pub const MAX_SUBGROUP_W: f32 = 1280.0;
/// 设置页内容宽度上限(表单行不需要网格那么宽)
pub const MAX_SETTINGS_W: f32 = 960.0;

/// 通用页面滚动容器:滚动条贴窗口右缘,内容自适应宽度并居中。
///
/// 用法:页面根节点为 `size_full`,本函数返回的滚动容器 `w_full + h_full`
/// 撑满父级,内容再按 `max_w` 上限居中。
pub fn page_scroll(
    max_w: f32,
    handle: &ScrollHandle,
    content: impl IntoElement,
) -> impl IntoElement {
    gpui::div()
        .id("page-scroll")
        .w_full()
        .h_full()
        .overflow_y_scroll()
        .track_scroll(handle)
        .vertical_scrollbar(handle)
        .child(
            gpui::div().w_full().flex().justify_center().child(
                gpui::div()
                    .w_full()
                    .max_w(px(max_w))
                    .px(px(32.))
                    .pt(px(28.))
                    .pb(px(24.))
                    .child(content),
            ),
        )
}
