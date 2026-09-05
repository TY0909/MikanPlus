//! 番剧详情页:Hero 头部(大封面 + 元信息 + 订阅)与字幕组剧集列表。

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::StyledExt;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{App, ScrollHandle, Window, prelude::*, px, relative};

use crate::app_theme;
use crate::icons::icon;
use crate::layout::MAX_DETAIL_W;
use crate::poster::poster;
use domain::BangumiItem;
use downloader::{DownloadManager, TaskView};
use storage::paths;

pub type GoBackCallback = Rc<dyn Fn(&mut Window, &mut App)>;
pub type ToggleSubscribeCallback =
    Rc<dyn Fn(u32, u32, Option<&str>, Option<&str>, &mut Window, &mut App)>;
pub type CheckSubscribedCallback = Rc<dyn Fn(u32, u32) -> bool>;

#[derive(IntoElement)]
pub struct BangumiDetailPage {
    pub item: Option<BangumiItem>,
    pub is_subscribed: CheckSubscribedCallback,
    pub on_toggle_subscribe: ToggleSubscribeCallback,
    /// 页面滚动句柄(由应用持有,切换页面后恢复滚动位置)
    pub scroll_handle: ScrollHandle,
    /// 下载管理器(添加/取消任务)
    pub downloader: Arc<DownloadManager>,
}

/// 打开外部链接，仅允许 HTTP(S)。
fn open_url(url: &str) {
    let _ = storage::paths::open_url(url);
}

/// 将 "M/D/YYYY" 或 "M-D-YYYY" 格式的放送开始日期转为 "YYYY年M月D日"
fn format_broadcast_start(raw: &str) -> String {
    let parts: Vec<&str> = raw.split(['/', '-']).collect();
    if parts.len() == 3
        && let (Ok(m), Ok(d), Ok(y)) = (
            parts[0].parse::<u32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        )
        && (1..=12).contains(&m)
        && (1..=31).contains(&d)
        && (1000..=9999).contains(&y)
    {
        return format!("{y}年{m}月{d}日");
    }
    raw.to_string()
}

impl RenderOnce for BangumiDetailPage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let is_dark = theme.mode.is_dark();
        // 剧集行标题列可用宽度(数据层截断用,见 episode_row::truncate_title)
        let win_w: f32 = window.bounds().size.width.into();
        let content_w = win_w.min(MAX_DETAIL_W) - 32.0 * 2.0 - 14.0 * 2.0;
        let title_max_px = (content_w * 0.6).max(60.0);
        let item = self.item;
        let bangumi_id = item.as_ref().and_then(|i| i.bangumi_id);
        let name = item.as_ref().map(|i| i.name.clone()).unwrap_or_default();
        let poster_url = item
            .as_ref()
            .and_then(|i| i.cover_url.clone())
            .unwrap_or_default();
        let meta = item.as_ref().and_then(|i| i.meta.as_ref());
        let summary = meta.and_then(|m| m.summary.as_deref());
        let official_site = meta.and_then(|m| m.official_site.as_deref());
        let bangumi_link = meta.and_then(|m| m.bangumi_link.as_deref());
        let broadcast_day = meta.and_then(|m| m.broadcast_day.as_deref());
        let broadcast_start = meta.and_then(|m| m.broadcast_start.as_deref());
        let groups = item
            .as_ref()
            .map(|i| i.subtitle_groups.clone())
            .unwrap_or_default();
        let on_toggle_subscribe = self.on_toggle_subscribe.clone();
        let is_subscribed = self.is_subscribed;
        let downloader = self.downloader.clone();
        let dl_snapshot = downloader.snapshot();
        let dl_base = storage::load_download_dir();

        if item.is_none() {
            return gpui_kit::div()
                .size_full()
                .bg(theme.background)
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child("未找到该番剧")
                .into_any_element();
        }

        // 链接按钮
        let link_btn = |label: &str, icon_name: &str, url: &str| {
            let url = url.to_string();
            gpui_kit::div()
                .flex()
                .items_center()
                .gap(px(4.))
                .text_sm()
                .text_color(theme.link)
                .cursor_pointer()
                .id(gpui_kit::SharedString::from(format!("link-{label}")))
                .hover(|style| style.text_color(theme.link_hover).underline())
                .on_click(move |_, _, _| open_url(&url))
                .child(icon(icon_name, 13.).text_color(theme.link))
                .child(label.to_string())
        };

        // Hero 头部的背景色(取自海报配色)
        let (hue_a, hue_b) = crate::poster::poster_hues(&name);
        let hero_bg = gpui_kit::hsla(hue_a / 360.0, 0.45, if is_dark { 0.16 } else { 0.94 }, 1.0);
        let hero_glow = gpui_kit::hsla(hue_b / 360.0, 0.5, if is_dark { 0.14 } else { 0.9 }, 1.0);

        // 链接行:官方网站 / 番组计划(简洁链接,与上方信息合并)
        let link_row = gpui_kit::div()
            .mt(px(10.))
            .flex()
            .items_center()
            .gap(px(18.))
            .when_some(official_site, |this, url| {
                this.child(link_btn("官方网站", "globe", url))
            })
            .when_some(bangumi_link, |this, url| {
                this.child(link_btn("番组计划", "info", url))
            });

        // ---- 组装 ----
        gpui_kit::div()
            .size_full()
            .bg(theme.background)
            .flex().flex_col()
            .child(
                gpui_kit::div()
                    .id("page-scroll")
                    .h_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .vertical_scrollbar(&self.scroll_handle)
                    .child(
                    gpui_kit::div().w_full().flex().flex_col().items_center().child(
                        gpui_kit::div()
                            .w_full()
                            .max_w(px(MAX_DETAIL_W))
                            .px(px(32.))
                            .pt(px(28.))
                            .pb(px(24.))
                            .flex().flex_col()
                            .child(
                                // Hero:大封面 + 元信息 + 订阅
                                gpui_kit::div()
                                    .w_full()
                                    .rounded(px(14.))
                                    .overflow_hidden()
                                    .border_1()
                                    .border_color(theme.border)
                                    .relative()
                                    .child(gpui_kit::div().absolute().inset_0().bg(hero_bg))
                                    .child(
                                        gpui_kit::div()
                                            .absolute()
                                            .right(px(-60.))
                                            .top(px(-60.))
                                            .size(px(260.))
                                            .rounded_full()
                                            .bg(hero_glow)
                                            .opacity(0.7),
                                    )
                                    .child(
                                        gpui_kit::div()
                                            .relative()
                                            .p(px(28.))
                                            // 左:封面 1.5 倍展示(252×353),垂直居中,上下留白
                                            .child(
                                                gpui_kit::div()
                                                    .absolute()
                                                    .top(px(28.))
                                                    .bottom(px(28.))
                                                    .left(px(28.))
                                                    .w(px(252.))
                                                    .flex().flex_col()
                                                    .items_center()
                                                    .justify_center()
                                                    .child(
                                                        gpui_kit::div()
                                                            .shadow(app_theme::card_shadow(theme))
                                                            .rounded(px(12.))
                                                            .overflow_hidden()
                                                            .child(poster(
                                                                &poster_url,
                                                                &name,
                                                                is_dark,
                                                                px(252.),
                                                                px(353.),
                                                                12.,
                                                                crate::poster::CornerStyle::All,
                                                            )),
                                                    ),
                                            )
                                            // 右列:标题 / 放送信息 / 作品信息 2×2 / 概况(唯一流式子元素,决定 Hero 高度)
                                            .child(
                                                gpui_kit::div()
                                                    .ml(px(280.))
                                                    .flex().flex_col()
                                                    // 标题
                                                    .child(
                                                        gpui_kit::div()
                                                            .text_3xl()
                                                            .font_bold()
                                                            .text_color(theme.foreground)
                                                            .line_clamp(2)
                                                            .child(name.clone()),
                                                    )
                                                    // 放送信息行:星期徽标 + 起播时间 + 字幕组数
                                                    .child(
                                                        gpui_kit::div()
                                                            .mt(px(10.))
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(10.))
                                                            .children(
                                                                broadcast_day
                                                                    .map(|day| {
                                                                        gpui_kit::div()
                                                                            .px(px(10.))
                                                                            .py(px(3.))
                                                                            .rounded_full()
                                                                            .bg(theme.primary)
                                                                            .text_xs()
                                                                            .font_semibold()
                                                                            .text_color(
                                                                                theme
                                                                                    .primary_foreground,
                                                                            )
                                                                            .child(day.to_string())
                                                                            .into_any_element()
                                                                    })
                                                                    .into_iter()
                                                                    .chain(
                                                                        broadcast_start.map(|start| {
                                                                            gpui_kit::div()
                                                                                .text_sm()
                                                                                .text_color(
                                                                                    theme
                                                                                        .muted_foreground,
                                                                                )
                                                                                .child(format!(
                                                                                    "{} 起放送",
                                                                                    format_broadcast_start(
                                                                                        start
                                                                                    )
                                                                                ))
                                                                                .into_any_element()
                                                                        }),
                                                                    )
                                                                    .chain(std::iter::once(
                                                                        gpui_kit::div()
                                                                            .text_sm()
                                                                            .text_color(
                                                                                theme
                                                                                    .muted_foreground,
                                                                            )
                                                                            .child(format!(
                                                                                "{} 个字幕组",
                                                                                groups.len()
                                                                            ))
                                                                            .into_any_element(),
                                                                    )),
                                                            ),
                                                    )
                                                    // 链接行:官方网站 / 番组计划
                                                    .child(link_row)
                                                    // 概况标题
                                                    .child(
                                                        gpui_kit::div()
                                                            .mt(px(24.))
                                                            .text_base()
                                                            .font_semibold()
                                                            .text_color(theme.foreground)
                                                            .child("概况"),
                                                    )
                                                    // 简介
                                                    .child(
                                                        gpui_kit::div()
                                                            .mt(px(8.))
                                                            .text_sm()
                                                            .text_color(theme.foreground)
                                                            .line_clamp(5)
                                                            .child(
                                                                summary
                                                                    .map(|s| {
                                                                        s.replace("\r\n", "\n")
                                                                    })
                                                                    .unwrap_or_else(|| {
                                                                        "暂无概况".to_string()
                                                                    }),
                                                            ),
                                                    ),
                                            ),
                                    )
                            )
                            .child(
                                // 字幕组列表(与 Hero 之间显式留白)
                                gpui_kit::div()
                                    .mt(px(28.))
                                    .w_full()
                                    .flex().flex_col()
                                    .child(
                                        gpui_kit::div()
                                            .text_lg()
                                            .font_bold()
                                            .text_color(theme.foreground)
                                            .child(format!("字幕组 ({})", groups.len())),
                                    )
                                    .children(groups.iter().enumerate().map(|(gix, g)| {
                                        let gid = g.subgroup_id.unwrap_or(0);
                                        let check = is_subscribed.clone();
                                        let subscribed =
                                            bangumi_id.map(|bid| check(bid, gid)).unwrap_or(false);
                                        let on_toggle = on_toggle_subscribe.clone();
                                        let bn = name.clone();
                                        let gn = g.name.clone();
                                        // 字幕组卡片之间显式留白
                                        gpui_kit::div()
                                            .mt(px(14.))
                                            .child(render_group_card(
                                                gix,
                                                g.name.clone(),
                                                g.episodes.clone(),
                                                subscribed,
                                                theme,
                                                GroupCardContext {
                                                    dl_dir: paths::subgroup_download_dir(
                                                        &dl_base,
                                                        &name,
                                                        &g.name,
                                                    ),
                                                    downloader: downloader.clone(),
                                                    snapshot: dl_snapshot.clone(),
                                                    title_max_px,
                                                },
                                                move |window, app| {
                                                    on_toggle(
                                                        bangumi_id.unwrap_or(0),
                                                        gid,
                                                        Some(&bn),
                                                        Some(&gn),
                                                        window,
                                                        app,
                                                    );
                                                },
                                            ))
                                    })),
                            ),
                    ),
                ),
            )
            .into_any_element()
    }
}

/// 字幕组卡片的渲染上下文(下载信息 + 剧集行标题列宽度)
struct GroupCardContext {
    dl_dir: PathBuf,
    downloader: Arc<DownloadManager>,
    snapshot: Vec<TaskView>,
    /// 剧集行标题列可用宽度(数据层截断用)
    title_max_px: f32,
}

fn render_group_card(
    gix: usize,
    group_name: String,
    episodes: Vec<domain::Episode>,
    subscribed: bool,
    theme: &gpui_kit::component::theme::Theme,
    ctx: GroupCardContext,
    on_toggle: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // 总集数(与下方渲染行数一致)与可下载集数
    let total_count = episodes.len();
    let episode_count: usize = episodes
        .iter()
        .filter(|ep| ep.magnet_link.is_some())
        .count();
    let subscribe_btn = gpui_kit::div()
        .px(px(12.))
        .py(px(5.))
        .rounded_full()
        .text_sm()
        .font_semibold()
        .flex_shrink_0()
        .cursor_pointer()
        .id(gpui_kit::SharedString::from(format!("grp-sub-{gix}")))
        .when(subscribed, |this| {
            this.bg(theme.success)
                .text_color(theme.success_foreground)
                .hover(|style| style.bg(theme.success_hover))
        })
        .when(!subscribed, |this| {
            this.bg(theme.primary)
                .text_color(theme.primary_foreground)
                .hover(|style| style.bg(theme.primary_hover))
        })
        .on_click(move |_, window, app| on_toggle(window, app))
        .child(if subscribed { "已订阅" } else { "订阅" });

    // 组名超长时截断,悬停显示完整名称
    let mut name_el = gpui_kit::div()
        .id(gpui_kit::SharedString::from(format!("grp-name-{gix}")))
        .text_base()
        .font_semibold()
        .text_color(theme.foreground)
        .truncate();
    if crate::episode_row::exceeds_lines(&group_name, 320.0, 14.0, 1) {
        name_el = name_el.tooltip(crate::episode_row::title_tooltip(group_name.clone()));
    }
    name_el = name_el.child(group_name);

    let rows = episodes.into_iter().enumerate().map(move |(ep_ix, ep)| {
        let title = crate::episode_row::truncate_title(&ep.title, ctx.title_max_px, 14.0);
        let magnet = ep.magnet_link.clone().unwrap_or_default();
        let size = ep.size.clone().unwrap_or_default();
        let date = ep.publish_date.clone().unwrap_or_default();

        // 右侧按钮区:未下载 → 下载;下载中 → 进度+速度+取消;完成 → 打开
        // 行内唯一键:组索引 + 组内剧集索引(标题不能作 id,同名/截断同名会串状态)
        let action_btn = crate::episode_row::action_button(
            gix * 10_000 + ep_ix,
            &title,
            &magnet,
            &ctx.dl_dir,
            &ctx.downloader,
            &ctx.snapshot,
            theme,
        );

        // 剧集行:flex 布局。文本列固定 60%(超长截断),
        // 操作列固定 40%(下载区为独立子元素,默认右对齐)。
        gpui_kit::div()
            .id(gpui_kit::SharedString::from(format!(
                "ep-row-{gix}-{ep_ix}"
            )))
            .w_full()
            .px(px(14.))
            .py(px(14.))
            .flex()
            .items_center()
            .border_t_1()
            .border_color(theme.border)
            .hover(|style| style.bg(theme.list_hover))
            .child(
                // 文本列:60% 空间
                gpui_kit::div()
                    .w(relative(0.6))
                    .flex()
                    .flex_col()
                    .child(crate::episode_row::title_cell(
                        gix * 10_000 + ep_ix,
                        &ep.title,
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
        .w_full()
        .rounded(px(10.))
        .overflow_hidden()
        .border_1()
        .border_color(theme.border)
        .bg(app_theme::card(theme))
        .child(
            gpui_kit::div()
                .w_full()
                .px(px(14.))
                .py(px(12.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .bg(theme.muted)
                .child(
                    gpui_kit::div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .overflow_hidden()
                        .child(icon("users", 15.).text_color(theme.muted_foreground))
                        .child(name_el)
                        .child(
                            gpui_kit::div()
                                .flex_shrink_0()
                                .px(px(7.))
                                .py(px(2.))
                                .rounded_full()
                                .bg(theme.background)
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(format!("{episode_count}/{total_count} 可下载")),
                        ),
                )
                .child(subscribe_btn),
        )
        .when(total_count == 0, |this| {
            this.child(
                gpui_kit::div()
                    .w_full()
                    .px(px(14.))
                    .py(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("该字幕组暂无剧集"),
            )
        })
        .children(rows)
}
