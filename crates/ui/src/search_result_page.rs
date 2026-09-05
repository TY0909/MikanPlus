//! 搜索结果页:在线搜索蜜柑计划,展示番剧卡片与剧集结果。
//!
//! 纯内容页(无滚动容器):滚动/居中/宽度上限由外层 `page_scroll` 统一管理。

use std::sync::Arc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::StyledExt;
use gpui_kit::{App, Window, prelude::*, px};

use crate::bangumi_card::{BangumiCard, BangumiFormat};
use crate::home_view::CardClickCallback;
use crate::icons::icon;
use crate::layout::MAX_PAGE_W;
use domain::SearchResults;
use downloader::DownloadManager;

/// 每页剧集条数(与蜜柑站内分页一致)
pub const SEARCH_PAGE_SIZE: usize = 50;

/// 分页切换回调
pub type SearchPageChangeCallback = std::rc::Rc<dyn Fn(usize, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SearchResultPage {
    pub query: String,
    pub results: SearchResults,
    pub on_card_click: CardClickCallback,
    /// 下载管理器(预留:搜索结果的剧集行提供下载)
    pub downloader: Arc<DownloadManager>,
    /// 当前页码(0 起)
    pub page: usize,
    /// 分页切换回调
    pub on_page_change: SearchPageChangeCallback,
}

impl RenderOnce for SearchResultPage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let on_card_click = self.on_card_click;
        let on_page_change = self.on_page_change;

        let total_episodes = self.results.episodes.len();
        let total_pages = total_episodes.div_ceil(SEARCH_PAGE_SIZE).max(1);
        // 页码钳制(数据刷新后页码可能越界)
        let page = self.page.min(total_pages - 1);
        let start = page * SEARCH_PAGE_SIZE;
        let end = (start + SEARCH_PAGE_SIZE).min(total_episodes);
        // 分页边界:第一页禁用「上一页」,最后一页禁用「下一页」
        let prev_enabled = page > 0;
        let next_enabled = page + 1 < total_pages;
        // 整页空状态(番剧卡片与剧集均为空)
        let whole_empty = self.results.items.is_empty() && total_episodes == 0;

        // 标题列可用宽度(与字幕组详情页同款估算):窗口宽受页面最大宽度约束,
        // 减去页面与行的内边距后取 60%。gpui 0.2.2 的 CSS ellipsis 在嵌套
        // 百分比布局中不可靠,这里在数据层截断。
        let win_w: f32 = window.bounds().size.width.into();
        let content_w = win_w.min(MAX_PAGE_W) - 32.0 * 2.0 - 14.0 * 2.0;
        let title_max_px = (content_w * 0.6).max(60.0);

        // 剧集行:标题 + 大小/时间 + 复制磁力(仅渲染当前页,避免一次渲染全部)
        let episode_rows = self.results.episodes[start..end]
            .iter()
            .enumerate()
            .map(|(ix, ep)| {
                let ix = start + ix;
                let magnet = ep.magnet.clone();
                let full_title = ep.title.clone();
                let title = crate::episode_row::truncate_title(&full_title, title_max_px, 14.0);
                let size = ep.size.clone();
                let date = ep.date.clone();

                gpui_kit::div()
                    .id(gpui_kit::SharedString::from(format!("search-ep-{ix}")))
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
                        // 标题列:占剩余空间,超长在数据层截断
                        gpui_kit::div()
                            .flex_1()
                            .min_w(px(0.))
                            .flex()
                            .flex_col()
                            .child(crate::episode_row::title_cell(
                                ix,
                                &full_title,
                                &title,
                                theme,
                            ))
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
                    // 操作列:复制磁力(磁力为空时整列隐藏)
                    .when(!magnet.is_empty(), move |this| {
                        let copy_magnet = magnet.clone();
                        this.child(
                            gpui_kit::div()
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .justify_end()
                                .child(
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
                                        .id(gpui_kit::SharedString::from(format!(
                                            "search-copy-{ix}"
                                        )))
                                        .hover(|style| {
                                            style.bg(theme.list_hover).border_color(theme.primary)
                                        })
                                        .on_click(move |_, _, app| {
                                            app.write_to_clipboard(
                                                gpui_kit::ClipboardItem::new_string(
                                                    copy_magnet.clone(),
                                                ),
                                            );
                                        })
                                        .child(icon("copy", 13.).text_color(theme.muted_foreground))
                                        .child("复制磁力"),
                                ),
                        )
                    })
            });

        // 番剧卡片网格
        let cards = self
            .results
            .items
            .iter()
            .enumerate()
            .map(move |(ix, item)| {
                let name = item.name.clone();
                let click_name = name.clone();
                let bid = item.bangumi_id;
                let on_click = on_card_click.clone();
                BangumiCard {
                    name: name.clone(),
                    bangumi_id: bid,
                    poster_url: item.cover_url.clone().unwrap_or_default(),
                    format: BangumiFormat::Tv,
                    key: format!("search-card-{ix}"),
                    on_click: Some(std::rc::Rc::new(move |window, app| {
                        on_click(&click_name, bid, window, app);
                    })),
                }
            });

        // 纯内容(滚动/居中/padding 由外层 page_scroll 统一管理,避免双层滚动容器)
        gpui_kit::div()
            .w_full()
            .flex()
            .flex_col()
            .child(
                // 头部
                gpui_kit::div()
                    .w_full()
                    .mb(px(24.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        gpui_kit::div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(icon("search", 22.).text_color(theme.primary))
                            .child(
                                gpui_kit::div()
                                    .text_2xl()
                                    .font_bold()
                                    .text_color(theme.foreground)
                                    .child(format!("搜索「{}」", self.query)),
                            ),
                    )
                    .child(
                        gpui_kit::div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!(
                                "找到 {} 部番剧 · {} 条剧集",
                                self.results.items.len(),
                                total_episodes
                            )),
                    ),
            )
            // 整页空状态:番剧卡片与剧集均为空时居中提示
            .when(whole_empty, |this| {
                this.child(
                    gpui_kit::div()
                        .w_full()
                        .py(px(64.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(10.))
                        .child(icon("inbox", 28.).text_color(theme.muted_foreground))
                        .child(
                            gpui_kit::div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("没有找到相关番剧或剧集"),
                        )
                        .child(
                            gpui_kit::div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("换个关键词再试试吧"),
                        ),
                )
            })
            // 番剧卡片区 + 剧集结果区(整页为空时不再渲染)
            .when(!whole_empty, |this| {
                this.child(
                    // 番剧卡片区
                    gpui_kit::div()
                        .w_full()
                        .mb(px(20.))
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(18.))
                        .children(cards),
                )
                .child(
                    // 剧集结果区
                    gpui_kit::div()
                        .w_full()
                        .rounded(px(10.))
                        .overflow_hidden()
                        .border_1()
                        .border_color(theme.border)
                        .bg(crate::app_theme::card(theme))
                        .child(
                            gpui_kit::div()
                                .w_full()
                                .px(px(14.))
                                .py(px(8.))
                                .bg(theme.muted)
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    gpui_kit::div()
                                        .text_sm()
                                        .font_semibold()
                                        .text_color(theme.foreground)
                                        .child("剧集结果"),
                                )
                                .child(
                                    gpui_kit::div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(if total_pages > 1 {
                                            format!(
                                                "第 {} / {} 页 · 共 {total_episodes} 条",
                                                page + 1,
                                                total_pages
                                            )
                                        } else {
                                            format!("共 {total_episodes} 条")
                                        }),
                                ),
                        )
                        .children(episode_rows)
                        .when(total_episodes == 0, |this| {
                            this.child(
                                gpui_kit::div()
                                    .w_full()
                                    .py(px(32.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("没有匹配的剧集"),
                            )
                        })
                        .when(total_pages > 1, |this| {
                            let prev = on_page_change.clone();
                            let next = on_page_change.clone();
                            this.child(
                                // 分页控件:上一页 / 下一页(边界页禁用并置灰)
                                gpui_kit::div()
                                    .w_full()
                                    .px(px(14.))
                                    .py(px(10.))
                                    .border_t_1()
                                    .border_color(theme.border)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .gap(px(8.))
                                    .child(
                                        gpui_kit::div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .px(px(12.))
                                            .py(px(5.))
                                            .rounded(px(6.))
                                            .border_1()
                                            .border_color(theme.border)
                                            .text_xs()
                                            .text_color(if prev_enabled {
                                                theme.foreground
                                            } else {
                                                theme.muted_foreground
                                            })
                                            .id("search-page-prev")
                                            .when(prev_enabled, |this| {
                                                this.cursor_pointer().hover(|style| {
                                                    style
                                                        .bg(theme.list_hover)
                                                        .border_color(theme.primary)
                                                })
                                            })
                                            .on_click(move |_, window, app| {
                                                if prev_enabled {
                                                    prev(page - 1, window, app);
                                                }
                                            })
                                            .child(
                                                icon("arrow-left", 13.)
                                                    .text_color(theme.muted_foreground),
                                            )
                                            .child("上一页"),
                                    )
                                    .child(
                                        gpui_kit::div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .px(px(12.))
                                            .py(px(5.))
                                            .rounded(px(6.))
                                            .border_1()
                                            .border_color(theme.border)
                                            .text_xs()
                                            .text_color(if next_enabled {
                                                theme.foreground
                                            } else {
                                                theme.muted_foreground
                                            })
                                            .id("search-page-next")
                                            .when(next_enabled, |this| {
                                                this.cursor_pointer().hover(|style| {
                                                    style
                                                        .bg(theme.list_hover)
                                                        .border_color(theme.primary)
                                                })
                                            })
                                            .on_click(move |_, window, app| {
                                                if next_enabled {
                                                    next(page + 1, window, app);
                                                }
                                            })
                                            .child("下一页")
                                            .child(
                                                icon("arrow-right", 13.)
                                                    .text_color(theme.muted_foreground),
                                            ),
                                    ),
                            )
                        }),
                )
            })
    }
}
