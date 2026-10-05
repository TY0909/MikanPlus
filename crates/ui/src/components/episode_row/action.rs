//! Right-hand action button for an episode row (three download states): not downloaded → Download; downloading → progress + speed + cancel; completed → Open.

use std::path::Path;
use std::sync::Arc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::StyledExt;
use gpui_kit::component::button::Button;
use gpui_kit::component::menu::{DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::theme::Theme;
use gpui_kit::{AnyElement, prelude::*, px};

use crate::components::episode_row::OpenCollectionCallback;
use crate::theme::icons::icon;
use downloader::{DownloadCmd, DownloadManager, TaskState, TaskView, magnet_info_hash};
use storage::paths;

/// Build the right-hand action area for an episode row.
#[allow(clippy::too_many_arguments)]
pub fn action_button(
    gix: usize,
    title: &str,
    magnet: &str,
    dl_dir: &Path,
    downloader: &Arc<DownloadManager>,
    snapshot: &[TaskView],
    on_open_collection: &OpenCollectionCallback,
    theme: &Theme,
) -> AnyElement {
    let has_magnet = !magnet.is_empty();
    let hash = magnet_info_hash(magnet);
    let task = hash
        .as_ref()
        .and_then(|h| snapshot.iter().find(|t| &t.id == h));

    if !has_magnet {
        return gpui_kit::div().into_any_element();
    }

    let downloader = downloader.clone();
    let cancel = |id: String| cancel_btn(gix, title, id, &downloader, theme);

    // "Download" button: shown when not downloaded, or when completed but the files were deleted externally.
    let dl_btn = || {
        // Clone on each call so this closure does not contend with the cancel closure for ownership.
        let downloader = downloader.clone();
        let magnet = magnet.to_string();
        let title = title.to_string();
        let dl_dir = dl_dir.to_path_buf();
        gpui_kit::div()
            .flex()
            .items_center()
            .gap(px(5.))
            .px(px(10.))
            .py(px(5.))
            .rounded(px(6.))
            .border_1()
            .border_color(theme.border)
            .text_xs()
            .text_color(theme.foreground)
            .cursor_pointer()
            .id(gpui_kit::SharedString::from(format!("ep-dl-{gix}-{title}")))
            .hover(|style| style.bg(theme.list_hover).border_color(theme.primary))
            .on_click(move |_, _, _| {
                if let Err(e) = downloader.send(DownloadCmd::Add {
                    magnet: magnet.clone(),
                    title: title.clone(),
                    output_dir: dl_dir.clone(),
                }) {
                    eprintln!("发送下载命令失败: {e}");
                }
            })
            .child(icon("download", 13.).text_color(theme.muted_foreground))
            .child("下载")
            .into_any_element()
    };

    match task.map(|t| &t.state) {
        Some(TaskState::Completed) => {
            let task = task.unwrap();
            let video_files = task.video_files.clone();
            let output_dir = task.output_dir.clone();
            let del_id = task.id.clone();
            let del_downloader = downloader.clone();
            let collection_id = task.id.clone();
            let open_id = gpui_kit::SharedString::from(format!("ep-open-{gix}-{title}"));
            let more_id = gpui_kit::SharedString::from(format!("ep-more-{gix}-{title}"));
            let open_action = match video_files.as_slice() {
                [path] => {
                    let path = path.clone();
                    gpui_kit::div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .px(px(10.))
                        .py(px(5.))
                        .rounded(px(6.))
                        .bg(theme.success)
                        .text_xs()
                        .font_semibold()
                        .text_color(theme.success_foreground)
                        .cursor_pointer()
                        .id(open_id)
                        .hover(|style| style.bg(theme.success_hover))
                        .on_click(move |_, _, _| {
                            let _ = paths::open_path(&path);
                        })
                        .child(icon("play", 13.).text_color(theme.success_foreground))
                        .child("打开")
                        .into_any_element()
                }
                [] => {
                    let output_dir = output_dir.clone();
                    gpui_kit::div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .px(px(10.))
                        .py(px(5.))
                        .rounded(px(6.))
                        .bg(theme.success)
                        .text_xs()
                        .font_semibold()
                        .text_color(theme.success_foreground)
                        .cursor_pointer()
                        .id(open_id)
                        .hover(|style| style.bg(theme.success_hover))
                        .on_click(move |_, _, _| {
                            if let Some(path) = &output_dir {
                                let _ = paths::open_path(path);
                            }
                        })
                        .child(icon("film", 13.).text_color(theme.success_foreground))
                        .child("打开目录")
                        .into_any_element()
                }
                _ => {
                    let on_open_collection = on_open_collection.clone();
                    gpui_kit::div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .px(px(10.))
                        .py(px(5.))
                        .rounded(px(6.))
                        .bg(theme.success)
                        .text_xs()
                        .font_semibold()
                        .text_color(theme.success_foreground)
                        .cursor_pointer()
                        .id(open_id)
                        .hover(|style| style.bg(theme.success_hover))
                        .on_click(move |_, window, app| {
                            on_open_collection(collection_id.clone(), window, app);
                        })
                        .child(icon("list", 13.).text_color(theme.success_foreground))
                        .child("查看合集")
                        .into_any_element()
                }
            };
            gpui_kit::div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(open_action)
                .child(
                    // More menu: a dropdown (extensible with more options) containing a red "Delete".
                    Button::new(more_id)
                        .dropdown_caret(true)
                        .compact()
                        .outline()
                        .dropdown_menu(move |menu: PopupMenu, _window, _cx| {
                            let del_downloader = del_downloader.clone();
                            let del_id = del_id.clone();
                            menu.item(
                                PopupMenuItem::element(move |_window, cx| {
                                    let theme = cx.theme();
                                    gpui_kit::div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .px(px(8.))
                                        .py(px(5.))
                                        .rounded(px(6.))
                                        .child(icon("delete", 14.).text_color(theme.danger))
                                        .child(
                                            gpui_kit::div()
                                                .text_sm()
                                                .font_semibold()
                                                .text_color(theme.danger)
                                                .child("删除"),
                                        )
                                })
                                .on_click(move |_, _, _| {
                                    // Delete the task + downloaded files (cancel semantics: remove every artifact of this download).
                                    if let Err(e) = del_downloader
                                        .send(DownloadCmd::Cancel { id: del_id.clone() })
                                    {
                                        eprintln!("发送取消命令失败: {e}");
                                    }
                                }),
                            )
                        }),
                )
                .into_any_element()
        }
        Some(TaskState::Error(err)) => {
            let task = task.unwrap();
            // Show only user-understandable errors; do not expose low-level details.
            let message = err.user_message().to_string();
            gpui_kit::div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    gpui_kit::div()
                        .text_xs()
                        .text_color(theme.danger)
                        .max_w(px(220.))
                        .truncate()
                        .child(message),
                )
                .child(cancel(task.id.clone()))
                .into_any_element()
        }
        Some(TaskState::Initializing) | Some(TaskState::Downloading) => {
            let task = task.unwrap();
            let state_txt = if matches!(task.state, TaskState::Initializing) {
                "获取信息…".to_string()
            } else {
                format!(
                    "{} · {}源",
                    downloader::format_percent(task.progress),
                    task.peers
                )
            };
            gpui_kit::div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(
                    gpui_kit::div()
                        .w(px(96.))
                        .text_xs()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .truncate()
                        .child(state_txt),
                )
                .child(speed_el("arrow-down", task.download_rate, theme))
                .child(speed_el("arrow-up", task.upload_rate, theme))
                .child(cancel(task.id.clone()))
                .into_any_element()
        }
        // Not downloaded, or completed but files deleted externally: show "Download".
        Some(TaskState::Missing) | None => dl_btn(),
    }
}

/// Speed indicator: arrow + value (download ↓ / upload ↑).
fn speed_el(icon_name: &str, rate: u64, theme: &Theme) -> impl IntoElement {
    gpui_kit::div()
        .flex()
        .items_center()
        .gap(px(3.))
        .child(icon(icon_name, 12.).text_color(theme.muted_foreground))
        .child(
            gpui_kit::div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(downloader::format_rate(rate)),
        )
}

/// Cancel button.
fn cancel_btn(
    gix: usize,
    title: &str,
    id: String,
    downloader: &Arc<DownloadManager>,
    theme: &Theme,
) -> impl IntoElement {
    let downloader = downloader.clone();
    gpui_kit::div()
        .flex()
        .items_center()
        .gap(px(4.))
        .px(px(8.))
        .py(px(4.))
        .rounded(px(6.))
        .border_1()
        .border_color(theme.border)
        .text_xs()
        .text_color(theme.muted_foreground)
        .cursor_pointer()
        .id(gpui_kit::SharedString::from(format!(
            "ep-cancel-{gix}-{title}"
        )))
        .hover(|style| style.bg(theme.list_hover))
        .on_click(move |_, _, _| {
            if let Err(e) = downloader.send(DownloadCmd::Cancel { id: id.clone() }) {
                eprintln!("发送取消命令失败: {e}");
            }
        })
        .child(icon("x", 12.).text_color(theme.muted_foreground))
        .child("取消")
}
