//! 字幕组详情页:单个字幕组的剧集列表,支持按标题关键词筛选。

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::StyledExt;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{App, ScrollHandle, Window, prelude::*, px, relative};

use crate::app_theme;
use crate::icons::icon;
use crate::layout::MAX_SUBGROUP_W;
use domain::SubtitleGroup;
use downloader::DownloadManager;
use storage::paths;

/// 打开筛选窗口的回调
pub type OpenFilterCallback = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SubGroupDetailPage {
    pub bangumi_name: String,
    pub group: SubtitleGroup,
    /// 页面滚动句柄(由应用持有,切换页面后恢复滚动位置)
    pub scroll_handle: ScrollHandle,
    /// 下载管理器(添加/取消任务)
    pub downloader: Arc<DownloadManager>,
    /// 当前筛选关键词(空 = 不过滤;剧集标题必须包含该关键词)
    pub keyword: String,
    /// 点击「筛选」按钮时打开筛选窗口
    pub on_open_filter: OpenFilterCallback,
}

impl RenderOnce for SubGroupDetailPage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let group = self.group;
        let bangumi_name = self.bangumi_name.clone();
        let group_name = group.name.clone();
        let episodes = group.episodes.clone();
        let total = episodes.len();
        let downloader = self.downloader.clone();
        let dl_snapshot = downloader.snapshot();
        let dl_dir =
            paths::subgroup_download_dir(&storage::load_download_dir(), &bangumi_name, &group_name);
        let on_open_filter = self.on_open_filter.clone();

        // 标题列可用宽度(窗口宽受页面最大宽度约束,减去页面与行的内边距后取 60%)。
        // gpui 0.2.2 的文本 ellipsis 在嵌套百分比布局中不可靠,这里在数据层截断。
        let win_w: f32 = window.bounds().size.width.into();
        let content_w = win_w.min(MAX_SUBGROUP_W) - 32.0 * 2.0 - 14.0 * 2.0;
        let title_max_px = (content_w * 0.6).max(60.0);

        // 关键词过滤:标题必须包含关键词(不区分大小写)
        let keyword = self.keyword.trim().to_string();
        let keyword_lower = keyword.to_lowercase();
        let visible: Vec<(usize, domain::Episode)> = episodes
            .into_iter()
            .enumerate()
            .filter(|(_, ep)| {
                keyword_lower.is_empty() || ep.title.to_lowercase().contains(&keyword_lower)
            })
            .collect();
        let visible_count = visible.len();
        let filtering = !keyword.is_empty();

        // 筛选按钮:无关键词时为普通按钮;有关键词时高亮并展示关键词
        let filter_btn = {
            let on_open_filter = on_open_filter.clone();
            gpui_kit::div()
                .id("sg-filter-btn")
                .flex()
                .items_center()
                .gap(px(5.))
                .px(px(10.))
                .py(px(4.))
                .rounded(px(6.))
                .border_1()
                .border_color(theme.border)
                .bg(if filtering {
                    theme.primary
                } else {
                    theme.background
                })
                .text_xs()
                .font_semibold()
                .text_color(if filtering {
                    theme.primary_foreground
                } else {
                    theme.foreground
                })
                .cursor_pointer()
                .hover(|style| {
                    if filtering {
                        style.bg(theme.primary_hover)
                    } else {
                        style.bg(theme.list_hover).border_color(theme.primary)
                    }
                })
                .on_click(move |_, window, app| on_open_filter(window, app))
                .child(icon("filter", 13.).text_color(if filtering {
                    theme.primary_foreground
                } else {
                    theme.muted_foreground
                }))
                .child(
                    // 关键词文本:超长时截断,避免把「剧集列表」挤出标题行
                    gpui_kit::div()
                        .max_w(px(180.))
                        .truncate()
                        .child(if filtering {
                            format!("筛选 · {keyword}")
                        } else {
                            "筛选".to_string()
                        }),
                )
        };

        let rows = visible.into_iter().map(move |(ix, ep)| {
            let magnet = ep.magnet_link.clone().unwrap_or_default();
            let size = ep.size.clone().unwrap_or_default();
            let date = ep.publish_date.clone().unwrap_or_default();
            let title = crate::episode_row::truncate_title(&ep.title, title_max_px, 14.0);

            let action_btn = crate::episode_row::action_button(
                ix,
                &title,
                &magnet,
                &dl_dir,
                &downloader,
                &dl_snapshot,
                theme,
            );

            // 剧集行:flex 布局。文本列固定 60%(超长截断),
            // 操作列固定 40%(下载区为独立子元素,默认右对齐)。
            gpui_kit::div()
                .id(gpui_kit::SharedString::from(format!("sg-row-{ix}")))
                .w_full()
                .px(px(14.))
                .py(px(14.))
                .flex()
                .items_center()
                .rounded(px(8.))
                .hover(|style| style.bg(theme.list_hover))
                .child(
                    // 文本列:60% 空间
                    gpui_kit::div()
                        .w(relative(0.6))
                        .flex()
                        .flex_col()
                        .child(crate::episode_row::title_cell(ix, &ep.title, &title, theme))
                        .child(
                            gpui_kit::div()
                                .mt(px(3.))
                                .flex()
                                .gap(px(10.))
                                .child(
                                    gpui_kit::div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(size),
                                )
                                .child(
                                    gpui_kit::div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(date),
                                ),
                        ),
                )
                .child(
                    // 操作列:40% 空间,下载区右对齐;极窄窗口下左侧溢出被裁剪,
                    // 右侧的取消/打开等关键按钮保持完整
                    gpui_kit::div()
                        .w(relative(0.4))
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .justify_end()
                        .child(action_btn),
                )
        });

        gpui_kit::div()
            .size_full()
            .bg(theme.background)
            .flex()
            .flex_col()
            .child(
                gpui_kit::div()
                    .id("sg-scroll")
                    .h_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .vertical_scrollbar(&self.scroll_handle)
                    .child(
                        gpui_kit::div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .items_center()
                            .child(
                            gpui_kit::div()
                                .w_full()
                                .max_w(px(MAX_SUBGROUP_W))
                                .px(px(32.))
                                .pt(px(28.))
                                .pb(px(24.))
                                .flex()
                                .flex_col()
                                .child(
                                    // 头部:番剧名 / 字幕组名 / 集数
                                    gpui_kit::div()
                                        .w_full()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            gpui_kit::div()
                                                .text_2xl()
                                                .font_bold()
                                                .text_color(theme.foreground)
                                                .child(bangumi_name),
                                        )
                                        .child(
                                            gpui_kit::div()
                                                .mt(px(8.))
                                                .flex()
                                                .items_center()
                                                .gap(px(8.))
                                                .child(icon("users", 15.).text_color(theme.primary))
                                                .child(
                                                    gpui_kit::div()
                                                        .text_sm()
                                                        .font_semibold()
                                                        .text_color(theme.primary)
                                                        .child(group_name),
                                                ),
                                        )
                                        .child(
                                            gpui_kit::div()
                                                .mt(px(6.))
                                                .text_sm()
                                                .text_color(theme.muted_foreground)
                                                .child(if filtering {
                                                    format!(
                                                        "共 {total} 集 · 显示 {visible_count} 集"
                                                    )
                                                } else {
                                                    format!("共 {total} 集")
                                                }),
                                        ),
                                )
                                .child(
                                    // 剧集列表(与头部之间显式留白)
                                    gpui_kit::div()
                                        .mt(px(20.))
                                        .w_full()
                                        .rounded(px(10.))
                                        .overflow_hidden()
                                        .border_1()
                                        .border_color(theme.border)
                                        .bg(app_theme::card(theme))
                                        .child(
                                            // 列表标题行:标题 + 筛选按钮(标题不压缩,按钮文本截断)
                                            gpui_kit::div()
                                                .w_full()
                                                .px(px(14.))
                                                .py(px(8.))
                                                .bg(theme.muted)
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .gap(px(12.))
                                                .child(
                                                    gpui_kit::div()
                                                        .flex_shrink_0()
                                                        .text_sm()
                                                        .font_semibold()
                                                        .text_color(theme.foreground)
                                                        .child("剧集列表"),
                                                )
                                                .child(filter_btn),
                                        )
                                        .children(rows)
                                        .when(visible_count == 0, |this| {
                                            this.child(
                                                // 筛选后无结果:友好空状态
                                                gpui_kit::div()
                                                    .w_full()
                                                    .py(px(40.))
                                                    .flex()
                                                    .flex_col()
                                                    .items_center()
                                                    .gap(px(8.))
                                                    .child(
                                                        icon("inbox", 24.)
                                                            .text_color(theme.muted_foreground),
                                                    )
                                                    .child(
                                                        gpui_kit::div()
                                                            .text_sm()
                                                            .text_color(theme.muted_foreground)
                                                            .child(format!(
                                                                "没有标题包含「{keyword}」的剧集"
                                                            )),
                                                    ),
                                            )
                                        }),
                                ),
                        ),
                    ),
            )
    }
}
