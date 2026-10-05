//! Top toolbar: back/forward + segmented navigation (home/subscriptions/settings)
//! + download observer + search + theme toggle.
//!
//! Consistent with the macOS unified toolbar style, pinned to the top of the
//! content area. The segmented navigation replaces the former sidebar and handles
//! top-level view switching; on subpages (detail/search) the page title is shown
//! on the left and the segmented control highlights no item.

use std::rc::Rc;

use gpui_kit::component::Sizable;
use gpui_kit::component::StyledExt;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::theme::Theme;
use gpui_kit::{App, Context, Window, prelude::*, px};

use crate::theme::icons::icon;
use domain::navigation::Section;

/// Generic toolbar action callback.
pub type ActionCallback = Rc<dyn Fn(&mut Window, &mut App)>;
/// Top-level section navigation callback (triggered by clicking a segmented control).
pub type NavigateCallback = Rc<dyn Fn(Section, &mut Window, &mut App)>;

pub struct Toolbar {
    on_open_search: ActionCallback,
    on_go_back: ActionCallback,
    on_toggle_theme: ActionCallback,
    on_navigate: NavigateCallback,
    on_open_downloads: ActionCallback,
    pub can_go_back: bool,
    pub title: String,
    /// Current top-level section (`None` on subpages).
    pub current_section: Option<Section>,
    /// Number of tasks currently downloading, used for the toolbar hint and unread awareness.
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
            current_section: Some(Section::Home),
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

        // Left: top-level pages show the MikanPlus logo (click to return home); subpages show a back button.
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
                    move |_, window, app| on_navigate(Section::Home, window, app)
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

        // Segmented navigation: Home / My Subscriptions / Settings.
        let section_btn = |section: Section, active: bool| {
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
            .children(Section::TABS.map(|s| section_btn(s, current_section == Some(s))));

        // Page title: shown only on subpages (detail/search, etc.).
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
            // Left: logo (top-level pages) / back button (subpages)
            .child(left_area)
            // Segmented navigation: horizontally centered
            .child(
                gpui_kit::div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(segmented),
            )
            // Page title (subpages)
            .child(title_el)
            // Search and download observer: left of the theme button on the right
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
            // Theme toggle
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
