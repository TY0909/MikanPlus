//! 顶部工具栏:返回/前进 + 分段导航(首页/订阅/设置)+ 搜索 + 主题切换。
//!
//! 与 macOS 统一工具栏风格保持一致,内容区顶部常驻。
//! 分段导航替代了原侧边栏,承担顶层视图切换;子页面(详情/搜索)
//! 时左侧显示页面标题,分段控件不再高亮任何一项。

use std::rc::Rc;

use gpui_kit::component::Sizable;
use gpui_kit::component::StyledExt;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::theme::Theme;
use gpui_kit::{App, Context, Entity, Window, prelude::*, px};

use crate::icons::icon;
use domain::navigation::TopSection;

pub type SearchCallback = Rc<dyn Fn(String, &mut Window, &mut App)>;
pub type ActionCallback = Rc<dyn Fn(&mut Window, &mut App)>;
/// 顶层分区导航回调(点击分段控件)
pub type NavigateCallback = Rc<dyn Fn(TopSection, &mut Window, &mut App)>;

pub struct Toolbar {
    input_state: Entity<InputState>,
    window_handle: gpui_kit::AnyWindowHandle,
    on_search: SearchCallback,
    on_go_back: ActionCallback,
    on_toggle_theme: ActionCallback,
    on_navigate: NavigateCallback,
    /// 回车触发搜索的订阅(持有它,随实体销毁自动退订)
    _enter_subscription: gpui_kit::Subscription,
    pub can_go_back: bool,
    pub title: String,
    /// 当前顶层分区(子页面时为 None)
    pub current_section: Option<TopSection>,
}

impl Toolbar {
    pub fn new(
        window: &mut Window,
        on_search: SearchCallback,
        on_go_back: ActionCallback,
        on_toggle_theme: ActionCallback,
        on_navigate: NavigateCallback,
        cx: &mut Context<Self>,
    ) -> Self {
        let window_handle = window.window_handle();
        let input_state = cx.new(|cx| InputState::new(window, cx).placeholder("搜索番剧名称…"));
        window.blur(cx);

        // 回车搜索:只订阅一次(订阅随 Toolbar 实体生命周期存续,
        // 绝不能在 render 中重复订阅——会无限累积回调)
        let _enter_subscription = cx.subscribe(
            &input_state,
            move |this: &mut Toolbar,
                  _input_state: Entity<InputState>,
                  event: &InputEvent,
                  cx: &mut Context<Toolbar>| {
                if let InputEvent::PressEnter { .. } = event {
                    this.do_search(cx);
                }
            },
        );

        Self {
            input_state,
            window_handle,
            on_search,
            on_go_back,
            on_toggle_theme,
            on_navigate,
            _enter_subscription,
            can_go_back: false,
            title: String::new(),
            current_section: Some(TopSection::Home),
        }
    }

    /// ⌘F 聚焦搜索输入框
    pub fn focus_search(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.input_state
            .update(cx, |state, cx| state.focus(window, cx));
    }

    /// 触发搜索(读取输入框内容并回调)
    fn do_search(&self, cx: &mut Context<Self>) {
        let text = self.input_state.read(cx).text().to_string();
        let query = if text.trim().is_empty() {
            return;
        } else {
            text.trim().to_string()
        };
        let on_search = self.on_search.clone();
        let handle = self.window_handle;
        let _ = handle.update(cx, move |_, window, app| {
            on_search(query, window, app);
        });
    }
}

impl Render for Toolbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let on_go_back = self.on_go_back.clone();
        let on_toggle_theme = self.on_toggle_theme.clone();
        let on_navigate = self.on_navigate.clone();
        let can_go_back = self.can_go_back;
        let current_section = self.current_section;
        let input_state = self.input_state.clone();
        let on_search_btn = self.on_search.clone();
        let window_handle = self.window_handle;

        let theme = Theme::global(cx);
        let is_dark = theme.mode.is_dark();

        let search_input = Input::new(&self.input_state)
            .w(px(220.))
            .h(px(30.))
            .rounded(px(15.));

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

        // 页面标题:仅在子页面(详情/搜索等)时显示;
        // 固定宽度避免与居中的分段控件重叠(分段控件在 960px 窗口下从 ~372px 开始)
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
            // 搜索:右侧操作区(搜索框 + 搜索按钮)
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
                        gpui_kit::div()
                            .relative()
                            .flex()
                            .items_center()
                            .child(
                                gpui_kit::div()
                                    .absolute()
                                    .left(px(9.))
                                    .text_color(theme.muted_foreground)
                                    .child(icon("search", 14.).text_color(theme.muted_foreground)),
                            )
                            .child(search_input.pl(px(28.))),
                    )
                    .child(
                        Button::new("tb-search-btn")
                            .label("搜索")
                            .small()
                            .primary()
                            .rounded(px(15.))
                            .on_click({
                                let on_search = on_search_btn.clone();
                                let input_state = input_state.clone();
                                move |_ev, _window: &mut Window, app: &mut App| {
                                    let text = input_state.read(app).text().to_string();
                                    let query = text.trim().to_string();
                                    if !query.is_empty() {
                                        let on_search = on_search.clone();
                                        let _ = window_handle.update(app, move |_, window, app| {
                                            on_search(query, window, app);
                                        });
                                    }
                                }
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
