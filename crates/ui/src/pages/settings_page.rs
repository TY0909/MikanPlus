//! Settings page: appearance (theme mode), download directory, network (backup domain), and about.
//!
//! Presentational component: the app pushes the current values in and supplies callbacks, and it
//! owns only its local edit state (the download-directory input). Composition: presentational
//! widgets in [`widgets`], data rows in [`rows`].

mod rows;
mod widgets;

use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::StyledExt;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::theme::ThemeMode;
use gpui_kit::{App, Context, Entity, ScrollHandle, Window, prelude::*, px};

use crate::actions::CloseFilterModal;
use crate::theme::icons::icon;
use crate::theme::layout::{MAX_SETTINGS_W, page_scroll};
use rows::{backup_row, cleanup_row, dir_row};
use widgets::{about_row, appearance_row, card};

/// Persists a new download directory (the app owns the state).
pub type SetDownloadDirCallback = Rc<dyn Fn(PathBuf, &mut App)>;
/// Toggles a boolean setting (backup domain / unsubscribe cleanup default).
pub type ToggleSettingCallback = Rc<dyn Fn(bool, &mut App)>;
/// Selects the theme mode.
pub type SelectThemeCallback = Rc<dyn Fn(ThemeMode, &mut Window, &mut App)>;

pub struct SettingsPage {
    /// Current download directory (pushed in by the app each render).
    pub download_dir: PathBuf,
    /// Whether the backup domain is enabled (pushed in by the app).
    pub use_backup_domain: bool,
    /// Whether the unsubscribe confirmation defaults to removing downloads (pushed in by the app).
    pub remove_downloads_on_unsubscribe: bool,
    /// Persist a new download directory.
    on_set_download_dir: SetDownloadDirCallback,
    /// Toggle the backup domain.
    on_toggle_backup: ToggleSettingCallback,
    /// Toggle the unsubscribe cleanup default.
    on_toggle_cleanup: ToggleSettingCallback,
    /// Select the theme mode.
    on_select_theme: SelectThemeCallback,
    /// Page scroll handle (held by the persistent entity; restores scroll position after switching pages).
    scroll_handle: ScrollHandle,
    /// Whether the download directory is currently being edited.
    editing_dir: bool,
    /// Download directory input field.
    dir_input: Entity<InputState>,
    /// Input event subscription (Enter saves / Change clears the error message).
    ///
    /// Its lifetime is owned by this entity (not leaked forever via `.detach()`), so it is automatically
    /// unsubscribed when the entity is dropped.
    _input_subscription: gpui_kit::Subscription,
    /// Inline error message for an empty directory input.
    dir_error: bool,
}

impl SettingsPage {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        download_dir: PathBuf,
        on_set_download_dir: SetDownloadDirCallback,
        on_toggle_backup: ToggleSettingCallback,
        on_toggle_cleanup: ToggleSettingCallback,
        on_select_theme: SelectThemeCallback,
    ) -> Self {
        let initial_dir = download_dir.to_string_lossy().into_owned();
        let dir_input = cx.new(|cx| {
            let mut input = InputState::new(window, cx);
            input.set_value(initial_dir, window, cx);
            input
        });
        // Subscribe to input events (same pattern as the filter input in main.rs):
        // Enter saves, and any content change clears the empty-directory error.
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
            download_dir,
            use_backup_domain: false,
            remove_downloads_on_unsubscribe: false,
            on_set_download_dir,
            on_toggle_backup,
            on_toggle_cleanup,
            on_select_theme,
            scroll_handle: ScrollHandle::default(),
            editing_dir: false,
            dir_input,
            _input_subscription: input_subscription,
            dir_error: false,
        }
    }

    /// Save the download directory (input contents); blank input is not saved and shows an inline error.
    fn save_dir(&mut self, cx: &mut Context<Self>) {
        let text = self.dir_input.read(cx).text().to_string();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            self.dir_error = true;
            cx.notify();
            return;
        }
        (self.on_set_download_dir)(PathBuf::from(trimmed), cx);
        self.editing_dir = false;
        self.dir_error = false;
        cx.notify();
    }

    /// Cancel editing: restore the input to the saved download directory and exit edit mode.
    fn cancel_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.dir_input.clone();
        let current = self.download_dir.to_string_lossy().into_owned();
        input.update(cx, |state, cx| {
            state.set_value(current, window, cx);
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
        let current_dir = self.download_dir.clone();
        let use_backup = self.use_backup_domain;
        let remove_downloads_on_unsubscribe = self.remove_downloads_on_unsubscribe;
        let on_select_theme = self.on_select_theme.clone();
        let on_toggle_backup = self.on_toggle_backup.clone();
        let on_toggle_cleanup = self.on_toggle_cleanup.clone();
        let entity = cx.entity();

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

        gpui_kit::div()
            .size_full()
            .bg(theme.background)
            .flex()
            .flex_col()
            // Esc: reuse the global CloseFilterModal keybinding (no new action). In edit mode, cancel editing and
            // restore the saved value; otherwise let it propagate to the root node (close the filter window / unsubscribe warning).
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
                        theme,
                        "外观",
                        "调整应用的显示效果",
                        vec![
                            appearance_row(theme, current_mode, on_select_theme).into_any_element(),
                        ],
                    ))
                    .child(card(
                        theme,
                        "下载",
                        "下载任务的保存位置与退订时的默认清理行为",
                        vec![
                            dir_row(
                                theme,
                                entity.clone(),
                                editing_dir,
                                dir_error,
                                dir_input,
                                current_dir,
                            )
                            .into_any_element(),
                            cleanup_row(theme, remove_downloads_on_unsubscribe, on_toggle_cleanup)
                                .into_any_element(),
                        ],
                    ))
                    .child(card(
                        theme,
                        "网络",
                        "数据源域名(备用域名适用于无法直连主站的网络)",
                        vec![backup_row(theme, use_backup, on_toggle_backup).into_any_element()],
                    ))
                    .child(card(
                        theme,
                        "关于",
                        "MikanPlus 的相关信息",
                        vec![
                            about_row(theme, "版本", env!("CARGO_PKG_VERSION"), "info", None, None)
                                .into_any_element(),
                            about_row(
                                theme,
                                "番剧数据",
                                "蜜柑计划 (mikanani.me)",
                                "globe",
                                Some("https://mikanani.me/"),
                                None,
                            )
                            .into_any_element(),
                            about_row(
                                theme,
                                "数据目录",
                                &storage::paths::app_data_dir().to_string_lossy(),
                                "layout-dashboard",
                                None,
                                Some(storage::paths::app_data_dir()),
                            )
                            .into_any_element(),
                            about_row(
                                theme,
                                "缓存目录",
                                &storage::paths::app_cache_dir().to_string_lossy(),
                                "clock",
                                None,
                                Some(storage::paths::app_cache_dir()),
                            )
                            .into_any_element(),
                            about_row(
                                theme,
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
