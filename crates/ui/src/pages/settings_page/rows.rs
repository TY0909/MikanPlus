//! Settings data rows: download directory (with edit mode), backup domain switch, and unsubscribe cleanup default.
//!
//! These rows need to write back to [`SettingsPage`] state, so they take the entity and `update` it in callbacks.

use std::path::PathBuf;

use gpui_kit::component::StyledExt;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::theme::Theme;
use gpui_kit::{Entity, prelude::*, px};

use super::{SettingsPage, ToggleSettingCallback};
use crate::theme::icons::icon;

/// Download directory row: shows the current path; in edit mode shows an input + save/cancel.
pub(super) fn dir_row(
    theme: &Theme,
    entity: Entity<SettingsPage>,
    editing_dir: bool,
    dir_error: bool,
    dir_input: Entity<InputState>,
    current_dir: PathBuf,
) -> impl IntoElement {
    gpui_kit::div()
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
            let this = entity;
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
            let this = entity;
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
        })
}

/// Backup domain switch: when on, the data source uses mikanime.tv (directly reachable in China, no proxy needed).
pub(super) fn backup_row(
    theme: &Theme,
    enabled: bool,
    on_toggle: ToggleSettingCallback,
) -> impl IntoElement {
    gpui_kit::div()
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
            Switch::new("switch-backup-domain")
                .checked(enabled)
                .on_click(move |checked, _window, app| {
                    on_toggle(*checked, app);
                })
        })
}

/// Default cleanup option for the unsubscribe confirmation window (does not skip the confirmation window).
pub(super) fn cleanup_row(
    theme: &Theme,
    enabled: bool,
    on_toggle: ToggleSettingCallback,
) -> impl IntoElement {
    gpui_kit::div()
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
                .child(icon("delete", 15.).text_color(theme.muted_foreground))
                .child(
                    gpui_kit::div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            gpui_kit::div()
                                .text_sm()
                                .text_color(theme.foreground)
                                .child("退订时默认移除下载目录和文件"),
                        )
                        .child(
                            gpui_kit::div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("控制退订确认窗口的默认选项,仍需手动确认"),
                        ),
                ),
        )
        .child({
            Switch::new("switch-remove-downloads-on-unsubscribe")
                .checked(enabled)
                .on_click(move |checked, _window, app| {
                    on_toggle(*checked, app);
                })
        })
}
