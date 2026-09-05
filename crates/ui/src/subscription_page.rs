//! 我的订阅页:以「字幕组」为最小单位展示订阅条目。
//!
//! - 每个字幕组订阅显示一张卡片(封面 + 番剧名 + 字幕组名)
//! - 点击卡片 → 该字幕组的剧集详情
//! - 悬停海报区显示「取消订阅」按钮(取消该字幕组)

use std::rc::Rc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::StyledExt;
use gpui_kit::{App, ScrollHandle, Window, prelude::*, px};

use crate::icons::icon;
use crate::layout::{MAX_PAGE_W, page_scroll};
use crate::poster::poster;
use domain::Subscription;

pub type GoBackCallback = Rc<dyn Fn(&mut Window, &mut App)>;
pub type SubCardClickCallback = Rc<dyn Fn(u32, u32, &mut Window, &mut App)>;
/// 取消单个字幕组的订阅
pub type UnsubscribeCallback = Rc<dyn Fn(u32, u32, &mut Window, &mut App)>;
pub type GoHomeCallback = Rc<dyn Fn(&mut Window, &mut App)>;

/// 按窗口宽度计算订阅卡片的列数与宽度(响应式)。
///
/// 内容容器左右各有 32px 内边距,宽度上限 `MAX_PAGE_W`;
/// 卡片间间距 18px,卡片最小宽度 240px。
fn responsive_card_size(win_w: f32) -> (usize, f32) {
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

#[cfg(test)]
mod tests {
    use super::responsive_card_size;

    #[test]
    fn wide_screen_uses_more_columns() {
        // 1920:5 列,每卡 ~357px
        let (cols, w) = responsive_card_size(1920.0);
        assert_eq!(cols, 5);
        assert!((w - 356.8).abs() < 0.1);
        // 1440:4 列
        assert_eq!(responsive_card_size(1440.0).0, 4);
        // 1100:3 列
        assert_eq!(responsive_card_size(1100.0).0, 3);
        // 800:2 列
        assert_eq!(responsive_card_size(800.0).0, 2);
    }

    #[test]
    fn card_width_fits_content() {
        // 超宽屏受 MAX_PAGE_W 上限约束
        let (cols, w) = responsive_card_size(3840.0);
        assert_eq!(cols, 5);
        assert!((w - 356.8).abs() < 0.1);
        // 卡片宽度不低于下限
        let (_, w) = responsive_card_size(700.0);
        assert!(w >= 240.0);
    }
}

#[derive(IntoElement)]
pub struct SubscriptionPage {
    pub subscriptions: Vec<Subscription>,
    /// 页面滚动句柄(由应用持有,切换页面后恢复滚动位置)
    pub scroll_handle: ScrollHandle,
    pub on_sub_click: SubCardClickCallback,
    pub on_unsubscribe: UnsubscribeCallback,
    pub on_go_home: GoHomeCallback,
}

impl RenderOnce for SubscriptionPage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let is_dark = theme.mode.is_dark();
        let on_sub_click = self.on_sub_click;
        let on_unsubscribe = self.on_unsubscribe;
        let on_go_home = self.on_go_home;

        // 响应式列数:按窗口宽度决定每行卡片数量与卡片宽度
        let win_w: f32 = window.bounds().size.width.into();
        let card_w = responsive_card_size(win_w).1;

        // 统计:字幕组条目数 与 涉及番剧数
        let group_count = self.subscriptions.len();
        let bangumi_count = {
            let mut ids: Vec<u32> = self.subscriptions.iter().map(|s| s.bangumi_id).collect();
            ids.sort_unstable();
            ids.dedup();
            ids.len()
        };

        let empty_state = gpui_kit::div()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(16.))
            .child(
                gpui_kit::div()
                    .size(px(72.))
                    .rounded_full()
                    .bg(theme.muted)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon("heart", 32.).text_color(theme.muted_foreground)),
            )
            .child(
                gpui_kit::div()
                    .text_lg()
                    .font_semibold()
                    .text_color(theme.foreground)
                    .child("还没有订阅任何字幕组"),
            )
            .child(
                gpui_kit::div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("在番剧详情页的字幕组卡片上点击「订阅」即可开始追番"),
            )
            .child(
                gpui_kit::div()
                    .mt(px(4.))
                    .px(px(18.))
                    .py(px(8.))
                    .rounded_full()
                    .bg(theme.primary)
                    .text_sm()
                    .font_semibold()
                    .text_color(theme.primary_foreground)
                    .cursor_pointer()
                    .id("sub-empty-cta")
                    .hover(|style| style.bg(theme.primary_hover))
                    .on_click(move |_, window, app| on_go_home(window, app))
                    .child("去发现番剧 →"),
            );

        // 统计信息行(页面顶部辅助信息,标题已在工具栏分段导航中)
        let summary = || {
            gpui_kit::div()
                .w_full()
                .mb(px(24.))
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(format!(
                    "共 {group_count} 条字幕组订阅 · 涉及 {bangumi_count} 部番剧"
                ))
        };

        if self.subscriptions.is_empty() {
            // 空状态:整个视口居中,无页面标题与统计
            return gpui_kit::div()
                .size_full()
                .bg(theme.background)
                .flex()
                .items_center()
                .justify_center()
                .child(empty_state)
                .into_any_element();
        }

        // 非空:统计行 + 字幕组卡片网格(可滚动)
        let grid = gpui_kit::div()
            .w_full()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(px(18.))
            .children(self.subscriptions.iter().map(|sub| {
                let on_sub_click = on_sub_click.clone();
                let on_unsub = on_unsubscribe.clone();
                let bid = sub.bangumi_id;
                let sid = sub.subgroup_id;
                let name = sub.bangumi_name.clone();
                let group_name = sub.group_name.clone();
                let cover = sub.cover_url.clone().unwrap_or_default();

                render_sub_card(
                    name.clone(),
                    group_name.clone(),
                    cover.clone(),
                    card_w,
                    is_dark,
                    theme,
                    move |window, app| {
                        on_sub_click(bid, sid, window, app);
                    },
                    move |window, app| {
                        on_unsub(bid, sid, window, app);
                    },
                )
            }));

        gpui_kit::div()
            .size_full()
            .bg(theme.background)
            .flex()
            .flex_col()
            .child(page_scroll(
                MAX_PAGE_W,
                &self.scroll_handle,
                gpui_kit::div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(summary())
                    .child(grid),
            ))
            .into_any_element()
    }
}

/// 订阅卡片:左侧海报 + 右侧文本,宽度随窗口响应式变化。
#[allow(clippy::too_many_arguments)]
fn render_sub_card(
    name: String,
    group_name: String,
    poster_url: String,
    card_w: f32,
    is_dark: bool,
    theme: &gpui_kit::component::theme::Theme,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
    on_unsubscribe: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let name_owned = name.clone();
    let group_name_owned = group_name.clone();
    let on_unsubscribe = Rc::new(on_unsubscribe);

    // 悬停遮罩:「取消订阅」按钮(点击取消该字幕组,不冒泡到卡片)
    // 圆角与海报区一致(只圆左侧),避免直角破坏单边圆角设计
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
            // 海报区:绝对定位在左侧,5:7 与封面比例一致;只圆左侧两角
            // (右侧直角,与文字区拼接)
            gpui_kit::div()
                .absolute()
                .left_0()
                .top_0()
                .w(px(100.))
                .h_full()
                .rounded_l(px(10.))
                .overflow_hidden()
                .child(poster(
                    &poster_url,
                    &name_owned,
                    is_dark,
                    px(100.),
                    px(140.),
                    10.,
                    crate::poster::CornerStyle::Left,
                ))
                .child(overlay),
        )
        .child(
            // 文本区:margin 避开海报,宽度自动 = 剩余空间
            // 行间距用显式 mt(垂直 gap 在 gpui 0.2.2 不可靠)
            gpui_kit::div()
                .ml(px(112.))
                .h_full()
                .flex()
                .flex_col()
                .p(px(12.))
                .child({
                    // 番剧名:最多两行 + 省略号;可能被截断时悬停显示完整名称
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
                    if crate::episode_row::exceeds_lines(&name, name_w, 14.0, 2) {
                        name_el = name_el.tooltip(crate::episode_row::title_tooltip(name.clone()));
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
