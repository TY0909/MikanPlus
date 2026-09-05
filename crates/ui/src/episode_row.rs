//! 剧集行操作按钮(下载三态):未下载 → 下载;下载中 → 进度+速度+取消;完成 → 打开。
//!
//! 番剧详情页与字幕组详情页共用。

use std::path::Path;
use std::sync::Arc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::StyledExt;
use gpui_kit::component::button::Button;
use gpui_kit::component::menu::{DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::theme::Theme;
use gpui_kit::{AnyElement, AnyView, App, Context, Render, Window, prelude::*, px};

use crate::icons::icon;
use downloader::{DownloadCmd, DownloadManager, TaskState, TaskView, magnet_info_hash};
use storage::paths;

/// 悬停提示内容视图:完整剧集标题,限宽自动换行。
///
/// 不用 gpui-component 的 `Tooltip`(其内部是 flex 行布局,长文本
/// 会按固有宽度测量而溢出容器被裁剪),这里用普通块级 div,文本
/// 在 max_w 约束下可靠换行。
struct TitleTooltip {
    text: String,
}

impl Render for TitleTooltip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        gpui_kit::div()
            .id("ep-title-tooltip")
            .max_w(px(480.))
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.popover)
            .shadow_md()
            .px(px(12.))
            .py(px(8.))
            .child(
                gpui_kit::div()
                    .max_w(px(456.))
                    .text_sm()
                    .text_color(theme.popover_foreground)
                    .child(self.text.clone()),
            )
    }
}

/// 构建标题 tooltip 视图(限宽自动换行),供剧集行与番剧卡片复用。
pub fn title_tooltip(text: String) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |_window, cx| cx.new(|_| TitleTooltip { text: text.clone() }).into()
}

/// 估算文本是否超出给定行宽/行数的容量(数据层保守估计)。
///
/// 用于决定是否给被 `line_clamp` 截断的文本挂 tooltip:按字符宽度
/// (全角 ≈ 字号、半角 ≈ 0.55×字号)贪心折行,行宽按 95% 计,
/// 为拉丁文本按词换行造成的行尾浪费留出余量。
pub fn exceeds_lines(text: &str, line_width_px: f32, font_size: f32, max_lines: usize) -> bool {
    if line_width_px <= 0.0 || max_lines == 0 {
        return false;
    }
    let effective = line_width_px * 0.95;
    let mut lines = 1usize;
    let mut used = 0.0f32;
    for ch in text.chars() {
        let w = if ch.is_ascii() {
            font_size * 0.55
        } else {
            font_size
        };
        if used + w > effective {
            lines += 1;
            used = 0.0;
            if lines > max_lines {
                return true;
            }
        }
        used += w;
    }
    false
}

/// 剧集标题单元格:显示截断后的标题;若发生截断,悬停时显示完整标题。
///
/// 注意:`.tooltip()` 属于 `StatefulInteractiveElement`,只对 `Stateful<Div>`
/// 可用,因此这里必须先 `.id()`(与 `.on_click()` 需要 `.id()` 同理)。
pub fn title_cell(
    gix: usize,
    full_title: &str,
    truncated: &str,
    theme: &Theme,
) -> impl IntoElement {
    let truncated_owned = truncated.to_string();
    let is_truncated = truncated != full_title;
    let full = full_title.to_string();
    let mut el = gpui_kit::div()
        .id(gpui_kit::SharedString::from(format!(
            "ep-title-{gix}-{full}"
        )))
        .w_full()
        .text_sm()
        .text_color(theme.foreground)
        .truncate();
    if is_truncated {
        el = el.tooltip(title_tooltip(full.clone()));
    }
    el.child(truncated_owned)
}

/// 按显示宽度截断剧集标题(超出部分替换为省略号)。
///
/// gpui 0.2.2 的文本 ellipsis 在多层级百分比宽度的 flex 布局中不可靠
/// (首次测量时可用空间非 Definite 则不截断),因此在数据层做一次
/// 保守截断,保证超长标题不会溢出到操作区。
///
/// 宽度估算:全角字符(中文等)≈ 1 个字号宽,半角(ASCII)≈ 0.55 个字号宽。
pub fn truncate_title(text: &str, max_px: f32, font_size: f32) -> String {
    if max_px <= 0.0 {
        return text.to_string();
    }
    const ELLIPSIS: char = '…';
    let ellipsis_w = font_size; // 省略号按全角计
    let mut used = 0.0f32;
    for (i, ch) in text.chars().enumerate() {
        let w = if ch.is_ascii() {
            font_size * 0.55
        } else {
            font_size
        };
        // 当前字符 + 省略号放不下时截断
        if used + w + ellipsis_w > max_px {
            let mut out: String = text.chars().take(i).collect();
            out.push(ELLIPSIS);
            return out;
        }
        used += w;
    }
    text.to_string()
}

/// 构建剧集行的右侧操作按钮区
pub fn action_button(
    gix: usize,
    title: &str,
    magnet: &str,
    dl_dir: &Path,
    downloader: &Arc<DownloadManager>,
    snapshot: &[TaskView],
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

    // 「下载」按钮:未下载、或已完成但文件被外部删除时显示
    let dl_btn = || {
        // 每次调用时克隆,避免闭包与 cancel 闭包争夺所有权
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
            let path = task.output_file.clone();
            let del_id = task.id.clone();
            let del_downloader = downloader.clone();
            let open_id = gpui_kit::SharedString::from(format!("ep-open-{gix}-{title}"));
            let more_id = gpui_kit::SharedString::from(format!("ep-more-{gix}-{title}"));
            gpui_kit::div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(
                    // 打开按钮:系统默认播放器播放
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
                            if let Some(p) = &path {
                                let _ = paths::open_path(p);
                            }
                        })
                        .child(icon("play", 13.).text_color(theme.success_foreground))
                        .child("打开"),
                )
                .child(
                    // 更多菜单:下拉列表(可扩展更多选项),含标红的「删除」
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
                                    // 删除任务 + 已下载文件(取消语义:移除本次下载所有产物)
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
            // 只展示用户能理解的错误,不暴露底层细节
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
        // 未下载,或已完成但文件被外部删除:显示「下载」
        Some(TaskState::Missing) | None => dl_btn(),
    }
}

/// 速度指示:箭头 + 数值(下载 ↓ / 上传 ↑)
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

/// 取消按钮
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

#[cfg(test)]
mod truncate_tests {
    use super::truncate_title;

    fn estimated_width(text: &str, font_size: f32) -> f32 {
        text.chars()
            .map(|c| {
                if c.is_ascii() {
                    font_size * 0.55
                } else {
                    font_size
                }
            })
            .sum()
    }

    #[test]
    fn short_text_untouched() {
        assert_eq!(truncate_title("第 1 集", 200.0, 14.0), "第 1 集");
        assert_eq!(truncate_title("S01E01", 200.0, 14.0), "S01E01");
    }

    #[test]
    fn long_text_truncated_within_limit() {
        let long = "【某某字幕组】这个番剧的名字真的非常非常长以至于肯定装不下一行";
        let out = truncate_title(long, 150.0, 14.0);
        assert!(out.ends_with('…'), "应以省略号结尾: {out}");
        assert!(out.len() < long.len(), "应发生截断: {out}");
        assert!(
            estimated_width(&out, 14.0) <= 150.0 + 0.01,
            "截断结果宽度应不超过限制: {:?} vs 150",
            estimated_width(&out, 14.0)
        );
    }

    #[test]
    fn ascii_long_truncated() {
        let long = "[SomeFanSub] Some.Anime.Title.S2.E01.1080p.WEB-DL.AAC.x264-SomeFanSub";
        let out = truncate_title(long, 120.0, 14.0);
        assert!(out.ends_with('…'));
        assert!(estimated_width(&out, 14.0) <= 120.0 + 0.01);
    }

    #[test]
    fn single_char_tight_space() {
        // 空间只够 1 个全角 + 省略号
        let out = truncate_title("一二三", 14.0 * 2.0, 14.0);
        assert_eq!(out, "一…");
    }
}
