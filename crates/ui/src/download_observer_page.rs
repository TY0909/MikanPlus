//! 正在下载任务观测页。
//!
//! 页面只展示尚未完成的任务。下载完成后任务仍保留在下载快照中，供详情页
//! 显示「打开」操作，但不会继续堆积在这里。

use downloader::{TaskState, TaskView, format_percent, format_rate};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::StyledExt;
use gpui_kit::{App, Window, prelude::*, px, relative};

use crate::icons::icon;
use crate::layout::MAX_SUBGROUP_W;

/// 展示正在获取信息或正在下载的任务。
#[derive(IntoElement)]
pub struct DownloadObserverPage {
    pub tasks: Vec<TaskView>,
}

impl RenderOnce for DownloadObserverPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let tasks: Vec<TaskView> = self
            .tasks
            .into_iter()
            .filter(|task| matches!(task.state, TaskState::Initializing | TaskState::Downloading))
            .collect();
        let count = tasks.len();

        let rows = tasks.into_iter().map(|task| {
            let progress = task.progress.clamp(0.0, 1.0);
            let status = if matches!(task.state, TaskState::Initializing) {
                "获取资源信息…".to_string()
            } else {
                format!(
                    "下载中 · {} · {} 个连接",
                    format_percent(progress),
                    task.peers
                )
            };
            let task_id = task.id;
            gpui_kit::div()
                .id(gpui_kit::SharedString::from(format!(
                    "download-observer-{task_id}"
                )))
                .w_full()
                .px(px(16.))
                .py(px(14.))
                .flex()
                .items_center()
                .gap(px(14.))
                .border_b_1()
                .border_color(theme.border)
                .child(
                    gpui_kit::div()
                        .flex_shrink_0()
                        .size(px(34.))
                        .rounded(px(8.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(theme.accent)
                        .child(icon("download", 16.).text_color(theme.primary)),
                )
                .child(
                    gpui_kit::div()
                        .flex_1()
                        .min_w(px(0.))
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            gpui_kit::div()
                                .text_sm()
                                .font_semibold()
                                .text_color(theme.foreground)
                                .truncate()
                                .child(task.title),
                        )
                        .child(
                            gpui_kit::div()
                                .w_full()
                                .h(px(6.))
                                .rounded_full()
                                .bg(theme.progress_bar)
                                .child(
                                    gpui_kit::div()
                                        .h_full()
                                        .w(relative(progress as f32))
                                        .rounded_full()
                                        .bg(theme.primary),
                                ),
                        ),
                )
                .child(
                    gpui_kit::div()
                        .flex_shrink_0()
                        .w(px(190.))
                        .flex()
                        .flex_col()
                        .items_end()
                        .gap(px(4.))
                        .child(
                            gpui_kit::div()
                                .text_xs()
                                .text_color(theme.foreground)
                                .child(status),
                        )
                        .child(
                            gpui_kit::div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(format!(
                                    "↓ {} · ↑ {}",
                                    format_rate(task.download_rate),
                                    format_rate(task.upload_rate)
                                )),
                        ),
                )
        });

        gpui_kit::div()
            .w_full()
            .max_w(px(MAX_SUBGROUP_W))
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                gpui_kit::div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        gpui_kit::div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                gpui_kit::div()
                                    .text_lg()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .child("下载观测"),
                            )
                            .child(
                                gpui_kit::div()
                                    .px(px(7.))
                                    .py(px(2.))
                                    .rounded_full()
                                    .bg(theme.accent)
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(count.to_string()),
                            ),
                    )
                    .child(
                        gpui_kit::div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("完成后会自动从此列表移除"),
                    ),
            )
            .child(if count == 0 {
                gpui_kit::div()
                    .w_full()
                    .py(px(64.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(10.))
                    .text_color(theme.muted_foreground)
                    .child(icon("download", 28.).text_color(theme.muted_foreground))
                    .child("当前没有正在下载的任务")
                    .into_any_element()
            } else {
                gpui_kit::div()
                    .w_full()
                    .rounded(px(10.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .overflow_hidden()
                    .children(rows)
                    .into_any_element()
            })
    }
}
