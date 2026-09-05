//! 设置页:外观(主题模式)、下载目录、网络(备用域名)与关于。

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::StyledExt;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::theme::{Theme, ThemeMode};
use gpui_kit::{Context, Entity, ScrollHandle, Window, prelude::*, px};

use crate::actions::CloseFilterModal;
use crate::icons::icon;
use crate::layout::{MAX_SETTINGS_W, page_scroll};

pub struct SettingsPage {
    /// 页面滚动句柄(常驻实体持有,切换页面后恢复滚动位置)
    scroll_handle: ScrollHandle,
    /// 是否正在编辑下载目录
    editing_dir: bool,
    /// 下载目录输入框
    dir_input: Entity<InputState>,
    /// 输入框事件订阅(Enter 保存 / Change 清除错误提示)。
    ///
    /// 由本实体持有生命周期(不用 `.detach()` 永久泄漏),实体释放时自动退订。
    _input_subscription: gpui_kit::Subscription,
    /// 空目录输入的内联错误提示
    dir_error: bool,
    /// 是否启用备用域名(与网络层开关、state.json 同步)
    use_backup_domain: bool,
}

impl SettingsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let dir_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(
                storage::load_download_dir().to_string_lossy().into_owned(),
                window,
                cx,
            );
            state
        });
        // 订阅输入框事件(与 main.rs 中筛选输入框同款模式):
        // Enter 保存、内容变化时清除空目录错误提示。
        let input_subscription = cx.subscribe(
            &dir_input,
            move |this: &mut SettingsPage,
                  _input: Entity<InputState>,
                  event: &InputEvent,
                  cx: &mut Context<SettingsPage>| {
                match event {
                    InputEvent::PressEnter { .. } => {
                        if this.editing_dir {
                            this.save_dir(cx);
                        }
                    }
                    InputEvent::Change if this.dir_error => {
                        this.dir_error = false;
                        cx.notify();
                    }
                    _ => {}
                }
            },
        );
        Self {
            scroll_handle: ScrollHandle::default(),
            editing_dir: false,
            dir_input,
            _input_subscription: input_subscription,
            dir_error: false,
            use_backup_domain: storage::load_use_backup_domain(),
        }
    }

    /// 保存下载目录(输入框内容);空白输入不保存,并显示内联错误提示。
    fn save_dir(&mut self, cx: &mut Context<Self>) {
        let text = self.dir_input.read(cx).text().to_string();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            self.dir_error = true;
            cx.notify();
            return;
        }
        storage::save_download_dir(std::path::Path::new(trimmed));
        self.editing_dir = false;
        self.dir_error = false;
        cx.notify();
    }

    /// 取消编辑:输入框恢复为已保存的下载目录,退出编辑态。
    fn cancel_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.dir_input.clone();
        input.update(cx, |state, cx| {
            state.set_value(
                storage::load_download_dir().to_string_lossy().into_owned(),
                window,
                cx,
            );
        });
        self.editing_dir = false;
        self.dir_error = false;
        cx.notify();
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let current_mode = theme.mode;
        let editing_dir = self.editing_dir;
        let dir_error = self.dir_error;
        let dir_input = self.dir_input.clone();
        let current_dir = storage::load_download_dir();
        let use_backup = self.use_backup_domain;

        let card = |title: &str, desc: &str, body: Vec<gpui_kit::AnyElement>| {
            gpui_kit::div()
                .w_full()
                .rounded(px(10.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.background)
                .overflow_hidden()
                .child(
                    gpui_kit::div()
                        .w_full()
                        .px(px(16.))
                        .py(px(12.))
                        .border_b_1()
                        .border_color(theme.border)
                        .bg(theme.muted)
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            gpui_kit::div()
                                .text_base()
                                .font_semibold()
                                .text_color(theme.foreground)
                                .child(title.to_string()),
                        )
                        .child(
                            gpui_kit::div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(desc.to_string()),
                        ),
                )
                .children(body)
        };

        let header = gpui_kit::div()
            .w_full()
            .mb(px(24.))
            .flex()
            .items_center()
            .gap(px(10.))
            .child(icon("settings", 22.).text_color(theme.primary))
            .child(
                gpui_kit::div()
                    .text_2xl()
                    .font_bold()
                    .text_color(theme.foreground)
                    .child("设置"),
            );

        // 主题分段选择
        let segment = gpui_kit::div()
            .flex()
            .gap(px(2.))
            .bg(theme.muted)
            .rounded(px(8.))
            .p(px(2.))
            .child(
                segment_btn("浅色", matches!(current_mode, ThemeMode::Light), theme)
                    .id("theme-light")
                    .on_click(|_, window, app| {
                        crate::app_theme::set_mode(ThemeMode::Light, Some(window), app);
                    }),
            )
            .child(
                segment_btn("深色", matches!(current_mode, ThemeMode::Dark), theme)
                    .id("theme-dark")
                    .on_click(|_, window, app| {
                        crate::app_theme::set_mode(ThemeMode::Dark, Some(window), app);
                    }),
            );

        let appearance_row = gpui_kit::div()
            .w_full()
            .px(px(16.))
            .py(px(14.))
            .flex()
            .items_center()
            .justify_between()
            .child(
                gpui_kit::div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(icon("moon", 16.).text_color(theme.muted_foreground))
                    .child(
                        gpui_kit::div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                gpui_kit::div()
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .child("主题模式"),
                            )
                            .child(
                                gpui_kit::div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("浅色 / 深色 · 也可使用 ⌘⇧L 快速切换"),
                            ),
                    ),
            )
            .child(segment);

        // 关于行(link:跳转网址;open_dir:打开本地目录)
        let about_row = |label: &str,
                         value: &str,
                         icon_name: &str,
                         link: Option<&str>,
                         open_dir: Option<std::path::PathBuf>| {
            let value = value.to_string();
            gpui_kit::div()
                .w_full()
                .px(px(16.))
                .py(px(14.))
                .border_t_1()
                .border_color(theme.border)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .child(
                    gpui_kit::div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(icon(icon_name, 15.).text_color(theme.muted_foreground))
                        .child(
                            gpui_kit::div()
                                .text_sm()
                                .text_color(theme.foreground)
                                .child(label.to_string()),
                        ),
                )
                .child(if let Some(url) = link {
                    let url = url.to_string();
                    gpui_kit::div()
                        .text_sm()
                        .text_color(theme.link)
                        .cursor_pointer()
                        .id(gpui_kit::SharedString::from(format!("about-{label}")))
                        .hover(|style| style.text_color(theme.link_hover).underline())
                        .on_click(move |_, _, _| {
                            let _ = storage::paths::open_url(&url);
                        })
                        .child(value)
                        .into_any_element()
                } else {
                    gpui_kit::div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .max_w(px(300.))
                        .truncate()
                        .child(value)
                        .into_any_element()
                })
                .child(if let Some(dir) = open_dir {
                    gpui_kit::div()
                        .px(px(10.))
                        .py(px(4.))
                        .rounded(px(6.))
                        .border_1()
                        .border_color(theme.border)
                        .text_xs()
                        .text_color(theme.foreground)
                        .cursor_pointer()
                        .id(gpui_kit::SharedString::from(format!("open-dir-{label}")))
                        .hover(|style| style.bg(theme.list_hover).border_color(theme.primary))
                        .on_click(move |_, _, _| {
                            let _ = storage::paths::open_path(&dir);
                        })
                        .child("打开")
                        .into_any_element()
                } else {
                    gpui_kit::div().into_any_element()
                })
        };

        // 下载目录行:展示当前路径;编辑态显示输入框 + 保存/取消
        let dir_row = gpui_kit::div()
            .w_full()
            .px(px(16.))
            .py(px(14.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(
                gpui_kit::div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(icon("download", 15.).text_color(theme.muted_foreground))
                    .child(
                        gpui_kit::div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                gpui_kit::div()
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .child("下载目录"),
                            )
                            .child(
                                gpui_kit::div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("剧集会下载到该目录下以番剧名命名的文件夹"),
                            ),
                    ),
            )
            .child(if editing_dir {
                let this = cx.entity();
                let this_cancel = this.clone();
                gpui_kit::div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        gpui_kit::div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(Input::new(&dir_input).w_full().h(px(30.)).rounded(px(6.)))
                            .child(
                                gpui_kit::div()
                                    .px(px(12.))
                                    .py(px(5.))
                                    .rounded(px(6.))
                                    .bg(theme.primary)
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(theme.primary_foreground)
                                    .cursor_pointer()
                                    .id("dir-save")
                                    .hover(|style| style.bg(theme.primary_hover))
                                    .on_click(move |_, _, app| {
                                        this.update(app, |s, cx| s.save_dir(cx));
                                    })
                                    .child("保存"),
                            )
                            .child(
                                gpui_kit::div()
                                    .px(px(12.))
                                    .py(px(5.))
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .cursor_pointer()
                                    .id("dir-cancel")
                                    .hover(|style| style.bg(theme.list_hover))
                                    .on_click(move |_, window, app| {
                                        this_cancel.update(app, |s, cx| {
                                            s.cancel_edit(window, cx);
                                        });
                                    })
                                    .child("取消"),
                            ),
                    )
                    .when(dir_error, |this| {
                        this.child(
                            gpui_kit::div()
                                .text_xs()
                                .text_color(theme.danger)
                                .child("目录不能为空,请输入有效的下载目录"),
                        )
                    })
                    .into_any_element()
            } else {
                let this = cx.entity();
                let this_edit = this.clone();
                gpui_kit::div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        gpui_kit::div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .max_w(px(320.))
                            .truncate()
                            .child(current_dir.to_string_lossy().into_owned()),
                    )
                    .child(
                        gpui_kit::div()
                            .px(px(10.))
                            .py(px(4.))
                            .rounded(px(6.))
                            .border_1()
                            .border_color(theme.border)
                            .text_xs()
                            .text_color(theme.foreground)
                            .cursor_pointer()
                            .id("dir-edit")
                            .hover(|style| style.bg(theme.list_hover).border_color(theme.primary))
                            .on_click(move |_, _, app| {
                                this_edit.update(app, |s, cx| {
                                    s.editing_dir = true;
                                    s.dir_error = false;
                                    cx.notify();
                                });
                            })
                            .child("更改"),
                    )
                    .into_any_element()
            });

        // 备用域名开关:开启后数据源改用 mikanime.tv(国内可直连,无需代理)
        let backup_row = gpui_kit::div()
            .w_full()
            .px(px(16.))
            .py(px(14.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(
                gpui_kit::div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(icon("globe", 15.).text_color(theme.muted_foreground))
                    .child(
                        gpui_kit::div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                gpui_kit::div()
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .child("备用域名"),
                            )
                            .child(
                                gpui_kit::div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("无法访问 mikanani.me 时,改用 mikanime.tv 获取数据"),
                            ),
                    ),
            )
            .child({
                let this = cx.entity();
                Switch::new("switch-backup-domain")
                    .checked(use_backup)
                    .on_click(move |checked, _window, app| {
                        let enabled = *checked;
                        storage::save_use_backup_domain(enabled);
                        source::network::set_backup_domain(enabled);
                        this.update(app, |s, cx| {
                            s.use_backup_domain = enabled;
                            cx.notify();
                        });
                    })
            });

        gpui_kit::div()
            .size_full()
            .bg(theme.background)
            .flex()
            .flex_col()
            // Esc:复用全局 CloseFilterModal 键位(不新增动作)。编辑态时取消编辑并
            // 恢复已保存值;非编辑态放行,交回根节点处理(关闭筛选窗口/退订警告)。
            .on_action(cx.listener(|this, _: &CloseFilterModal, window, cx| {
                if this.editing_dir {
                    this.cancel_edit(window, cx);
                } else {
                    cx.propagate();
                }
            }))
            .child(page_scroll(
                MAX_SETTINGS_W,
                &self.scroll_handle,
                gpui_kit::div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(24.))
                    .child(header)
                    .child(card(
                        "外观",
                        "调整应用的显示效果",
                        vec![appearance_row.into_any_element()],
                    ))
                    .child(card(
                        "下载",
                        "下载任务的保存位置",
                        vec![dir_row.into_any_element()],
                    ))
                    .child(card(
                        "网络",
                        "数据源域名(备用域名适用于无法直连主站的网络)",
                        vec![backup_row.into_any_element()],
                    ))
                    .child(card(
                        "关于",
                        "MikanPlus 的相关信息",
                        vec![
                            about_row("版本", "0.1.0", "info", None, None).into_any_element(),
                            about_row(
                                "番剧数据",
                                "蜜柑计划 (mikanani.me)",
                                "globe",
                                Some("https://mikanani.me/"),
                                None,
                            )
                            .into_any_element(),
                            about_row(
                                "数据目录",
                                &storage::paths::app_data_dir().to_string_lossy(),
                                "layout-dashboard",
                                None,
                                Some(storage::paths::app_data_dir()),
                            )
                            .into_any_element(),
                            about_row(
                                "缓存目录",
                                &storage::paths::app_cache_dir().to_string_lossy(),
                                "clock",
                                None,
                                Some(storage::paths::app_cache_dir()),
                            )
                            .into_any_element(),
                            about_row(
                                "图标",
                                "Lucide (ISC License)",
                                "palette",
                                Some("https://lucide.dev/"),
                                None,
                            )
                            .into_any_element(),
                        ],
                    )),
            ))
    }
}

/// iOS 风格分段选择器:灰色轨道 + 滑块
fn segment_btn(label: &str, active: bool, theme: &Theme) -> gpui_kit::Div {
    let (bg, fg) = if active {
        (theme.background, theme.foreground)
    } else {
        (theme.transparent, theme.muted_foreground)
    };

    gpui_kit::div()
        .px(px(14.))
        .py(px(5.))
        .bg(bg)
        .text_color(fg)
        .text_sm()
        .rounded(px(6.))
        .cursor_pointer()
        .when(active, |this| this.shadow_xs())
        .hover(move |style| {
            if active {
                style
            } else {
                style.bg(theme.accent).text_color(theme.foreground)
            }
        })
        .child(label.to_string())
}
