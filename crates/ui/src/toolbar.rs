//! 顶部工具栏:返回/前进 + 分段导航(首页/订阅/设置)+ 下载观测 + 搜索 + 主题切换。
//!
//! 与 macOS 统一工具栏风格保持一致,内容区顶部常驻。
//! 分段导航替代了原侧边栏,承担顶层视图切换;子页面(详情/搜索)
//! 时左侧显示页面标题,分段控件不再高亮任何一项。

use std::rc::Rc;

use gpui_kit::component::Sizable;
use gpui_kit::component::StyledExt;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::theme::Theme;
use gpui_kit::{App, Context, Window, prelude::*, px};

use crate::icons::icon;
use domain::navigation::TopSection;

pub type ActionCallback = Rc<dyn Fn(&mut Window, &mut App)>;
/// 顶层分区导航回调(点击分段控件)
pub type NavigateCallback = Rc<dyn Fn(TopSection, &mut Window, &mut App)>;

pub struct Toolbar {
    on_open_search: ActionCallback,
    on_go_back: ActionCallback,
    on_toggle_theme: ActionCallback,
    on_navigate: NavigateCallback,
    on_open_downloads: ActionCallback,
    pub can_go_back: bool,
    pub title: String,
    /// 当前顶层分区(子页面时为 None)
    pub current_section: Option<TopSection>,
    /// 当前正在下载的任务数,用于工具栏提示和未读感知。
    pub download_count: usize,
}

impl Toolbar {
    pub fn new(
        on_open_search: ActionCallback,
        on_go_back: ActionCallback,
        on_toggle_theme: ActionCallback,
        on_navigate: NavigateCallback,
        on_open_downloads: ActionCallback,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            on_open_search,
            on_go_back,
            on_toggle_theme,
            on_navigate,
            on_open_downloads,
            can_go_back: false,
            title: String::new(),
            current_section: Some(TopSection::Home),
            download_count: 0,
        }
    }
}

impl Render for Toolbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let on_go_back = self.on_go_back.clone();
        let on_toggle_theme = self.on_toggle_theme.clone();
        let on_navigate = self.on_navigate.clone();
        let on_open_search = self.on_open_search.clone();
        let on_open_downloads = self.on_open_downloads.clone();
        let can_go_back = self.can_go_back;
        let current_section = self.current_section;
        let download_count = self.download_count;

        let theme = Theme::global(cx);
        let is_dark = theme.mode.is_dark();

        let nav_btn = |icon_name: &str, enabled: bool, cb: ActionCallback| {
            let id: gpui_kit::SharedString = format!("tb-{icon_name}").into();
            let mut el = gpui_kit::div()
                .id(id.clone())
                .size(px(28.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .when(enabled, |this| {
                    this.cursor_pointer()
                        .hover(|style| style.bg(theme.list_hover).text_color(theme.foreground))
                })
                .when(!enabled, |this| this.opacity(0.35))
                .child(icon(icon_name, 16.).text_color(theme.muted_foreground));
            if enabled {
                el = el.on_click(move |_, window, app| cb(window, app));
            }
            el
        };

        // 左侧:顶层页面显示 MikanPlus Logo(点击回到首页);子页面显示返回按钮
        let left_area = if current_section.is_some() {
            gpui_kit::div()
                .id("tb-logo")
                .absolute()
                .left(px(16.))
                .top(px(8.))
                .size(px(32.))
                .rounded(px(8.))
                .overflow_hidden()
                .cursor_pointer()
                .on_click({
                    let on_navigate = on_navigate.clone();
                    move |_, window, app| on_navigate(TopSection::Home, window, app)
                })
                .child(gpui_kit::img("mikan-pic.png").size(px(32.)))
                .into_any_element()
        } else {
            gpui_kit::div()
                .absolute()
                .left(px(16.))
                .top(px(10.))
                .child(nav_btn("arrow-left", can_go_back, on_go_back.clone()))
                .into_any_element()
        };

        // 分段导航:首页 / 我的订阅 / 设置
        let section_btn = |section: TopSection, active: bool| {
            let (bg, fg) = if active {
                (theme.background, theme.foreground)
            } else {
                (theme.transparent, theme.muted_foreground)
            };
            let on_navigate = on_navigate.clone();
            gpui_kit::div()
                .px(px(16.))
                .py(px(4.))
                .rounded(px(6.))
                .bg(bg)
                .text_color(fg)
                .text_sm()
                .font_semibold()
                .cursor_pointer()
                .id(gpui_kit::SharedString::from(format!(
                    "tb-section-{:?}",
                    section
                )))
                .when(active, |this| this.shadow_xs())
                .hover(move |style| {
                    if active {
                        style
                    } else {
                        style.bg(theme.accent).text_color(theme.foreground)
                    }
                })
                .on_click(move |_, window, app| on_navigate(section, window, app))
                .child(section.label().to_string())
        };

        let segmented = gpui_kit::div()
            .flex()
            .gap(px(2.))
            .bg(theme.muted)
            .rounded(px(8.))
            .p(px(2.))
            .children(TopSection::ALL.map(|s| section_btn(s, current_section == Some(s))));

        // 页面标题:仅在子页面(详情/搜索等)时显示。
        let title_el = if current_section.is_none() {
            gpui_kit::div()
                .absolute()
                .left(px(120.))
                .top_0()
                .bottom_0()
                .w(px(240.))
                .flex()
                .items_center()
                .overflow_hidden()
                .child(
                    gpui_kit::div()
                        .text_sm()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .truncate()
                        .child(self.title.clone()),
                )
                .into_any_element()
        } else {
            gpui_kit::div().into_any_element()
        };

        gpui_kit::div()
            .w_full()
            .h(px(48.))
            .relative()
            .bg(theme.background)
            .border_b_1()
            .border_color(theme.border)
            // 左侧:Logo(顶层页面)/ 返回按钮(子页面)
            .child(left_area)
            // 分段导航:水平居中
            .child(
                gpui_kit::div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(segmented),
            )
            // 页面标题(子页面)
            .child(title_el)
            // 搜索与下载观测:右侧主题按钮左边
            .child(
                gpui_kit::div()
                    .absolute()
                    .right(px(48.))
                    .top(px(9.))
                    .h(px(30.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        Button::new("tb-search-btn")
                            .label("搜索")
                            .small()
                            .primary()
                            .rounded(px(15.))
                            .on_click(move |_ev, window, app| on_open_search(window, app)),
                    )
                    .child(
                        gpui_kit::div()
                            .id("tb-download-observer")
                            .h(px(30.))
                            .px(px(9.))
                            .rounded(px(15.))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .cursor_pointer()
                            .text_color(theme.muted_foreground)
                            .hover(|style| style.bg(theme.list_hover).text_color(theme.foreground))
                            .on_click(move |_, window, app| on_open_downloads(window, app))
                            .child(icon("download", 14.).text_color(theme.muted_foreground))
                            .child("下载")
                            .when(download_count > 0, |this| {
                                this.child(
                                    gpui_kit::div()
                                        .min_w(px(16.))
                                        .h(px(16.))
                                        .px(px(4.))
                                        .rounded_full()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .bg(theme.primary)
                                        .text_xs()
                                        .font_semibold()
                                        .text_color(theme.primary_foreground)
                                        .child(download_count.to_string()),
                                )
                            }),
                    ),
            )
            // 主题切换
            .child(
                gpui_kit::div()
                    .id("tb-theme")
                    .absolute()
                    .right(px(8.))
                    .top(px(10.))
                    .size(px(28.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .text_color(theme.muted_foreground)
                    .hover(|style| style.bg(theme.list_hover).text_color(theme.foreground))
                    .on_click(move |_, window, app| on_toggle_theme(window, app))
                    .child(
                        icon(if is_dark { "sun" } else { "moon" }, 16.)
                            .text_color(theme.muted_foreground),
                    ),
            )
    }
}
