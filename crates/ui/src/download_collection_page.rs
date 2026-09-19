//! 多视频下载任务的文件列表页。

use std::path::PathBuf;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::StyledExt;
use gpui_kit::{App, Window, prelude::*, px};

use crate::icons::icon;
use crate::layout::MAX_PAGE_W;
use storage::paths;

/// 展示一个已完成下载任务中的所有视频文件,每个文件都可以独立打开。
#[derive(IntoElement)]
pub struct DownloadCollectionPage {
    pub title: String,
    pub files: Vec<PathBuf>,
    pub output_dir: Option<PathBuf>,
}

impl RenderOnce for DownloadCollectionPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let output_dir = self.output_dir;
        let rows = self
            .files
            .into_iter()
            .enumerate()
            .map(move |(index, path)| {
                let display_path = output_dir
                    .as_deref()
                    .and_then(|dir| path.strip_prefix(dir).ok())
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                let open_path = path.clone();
                gpui_kit::div()
                    .id(gpui_kit::SharedString::from(format!(
                        "download-video-{index}"
                    )))
                    .w_full()
                    .px(px(14.))
                    .py(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .border_t_1()
                    .border_color(theme.border)
                    .hover(|style| style.bg(theme.list_hover))
                    .child(
                        gpui_kit::div()
                            .flex_1()
                            .min_w(px(0.))
                            .text_sm()
                            .text_color(theme.foreground)
                            .truncate()
                            .child(display_path),
                    )
                    .child(
                        gpui_kit::div()
                            .flex_shrink_0()
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
                            .id(gpui_kit::SharedString::from(format!(
                                "download-video-open-{index}"
                            )))
                            .hover(|style| style.bg(theme.success_hover))
                            .on_click(move |_, _, _| {
                                let _ = paths::open_path(&open_path);
                            })
                            .child(icon("play", 13.).text_color(theme.success_foreground))
                            .child("打开"),
                    )
            });

        gpui_kit::div()
            .w_full()
            .max_w(px(MAX_PAGE_W))
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                gpui_kit::div()
                    .flex()
                    .flex_col()
                    .gap(px(5.))
                    .child(
                        gpui_kit::div()
                            .text_lg()
                            .font_semibold()
                            .text_color(theme.foreground)
                            .child("查看合集"),
                    )
                    .child(
                        gpui_kit::div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(self.title),
                    ),
            )
            .child(
                gpui_kit::div()
                    .w_full()
                    .rounded(px(10.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .overflow_hidden()
                    .children(rows),
            )
    }
}
