//! 首页:今日更新 / 按星期浏览 / 剧场版。

use std::rc::Rc;

use gpui::{App, Window, prelude::*, px};
use gpui_component::ActiveTheme;
use gpui_component::StyledExt;
use gpui_component::scroll::ScrollableElement;

use crate::bangumi_card::{BangumiCard, BangumiFormat};
use crate::data::BangumiGroup;
use crate::icons::icon;
use crate::navigation::{HomeFilter, WEEKDAY_NAMES, WEEKDAY_SHORT, weekday_color};

pub type CardClickCallback = Rc<dyn Fn(&str, Option<u32>, &mut Window, &mut App)>;
/// 首页过滤条切换回调
pub type FilterChangeCallback = Rc<dyn Fn(HomeFilter, &mut Window, &mut App)>;

/// 首页视图(无状态,父级每次渲染时构建)。
#[derive(IntoElement)]
pub struct HomeView {
    pub groups: Vec<BangumiGroup>,
    pub filter: HomeFilter,
    pub today_weekday: usize,
    pub on_card_click: CardClickCallback,
    pub on_filter_change: FilterChangeCallback,
}

/// 计算今天是一周中的第几天(0=周一 … 6=周日,按本地时区)
pub fn today_weekday() -> usize {
    use chrono::Datelike;
    chrono::Local::now().weekday().num_days_from_monday() as usize
}

impl HomeView {
    /// 按 day 字段找分组(monday..sunday / movie)
    pub fn group_by_day<'a>(groups: &'a [BangumiGroup], day: &str) -> Option<&'a BangumiGroup> {
        groups.iter().find(|g| g.day == day)
    }
}

impl RenderOnce for HomeView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let on_card_click = self.on_card_click;
        let on_filter_change = self.on_filter_change;
        let groups = self.groups;
        let today = self.today_weekday;
        let filter = self.filter;

        // 顶部过滤条:今日 / 周一~周日 / 剧场版(替代原侧边栏星期导航)
        let filter_items: Vec<(HomeFilter, &str, bool)> =
            vec![(HomeFilter::Today, "今日", filter == HomeFilter::Today)]
                .into_iter()
                .chain((0..7).map(|day| {
                    let active = matches!(filter, HomeFilter::Weekday(d) if d == day);
                    (HomeFilter::Weekday(day), WEEKDAY_SHORT[day], active)
                }))
                .chain(std::iter::once((
                    HomeFilter::Movies,
                    "剧场版",
                    filter == HomeFilter::Movies,
                )))
                .collect();

        let filter_bar = gpui::div().w_full().mb(px(20.)).child(
            gpui::div().w_full().overflow_x_scrollbar().child(
                gpui::div()
                    .flex_shrink_0()
                    .flex()
                    .gap(px(2.))
                    .bg(theme.muted)
                    .rounded(px(8.))
                    .p(px(2.))
                    .children(filter_items.into_iter().map(|(f, label, active)| {
                        let on_filter_change = on_filter_change.clone();
                        let (bg, fg) = if active {
                            (theme.background, theme.foreground)
                        } else {
                            (theme.transparent, theme.muted_foreground)
                        };
                        gpui::div()
                            .px(px(14.))
                            .py(px(4.))
                            .rounded(px(6.))
                            .bg(bg)
                            .text_color(fg)
                            .text_sm()
                            .font_semibold()
                            .cursor_pointer()
                            .id(gpui::SharedString::from(format!("home-filter-{f:?}")))
                            .when(active, |this| this.shadow_xs())
                            .hover(move |style| {
                                if active {
                                    style
                                } else {
                                    style.bg(theme.accent).text_color(theme.foreground)
                                }
                            })
                            .on_click(move |_, window, app| on_filter_change(f, window, app))
                            .child(label.to_string())
                    })),
            ),
        );

        let day_key = |i: usize| {
            [
                "monday",
                "tuesday",
                "wednesday",
                "thursday",
                "friday",
                "saturday",
                "sunday",
            ][i]
        };

        // 计算本地日期字符串,例如「8月3日 · 星期一」
        let now = chrono::Local::now();
        let (y, m, d) = {
            use chrono::Datelike;
            (now.year(), now.month(), now.day())
        };

        // 构建一个星期的卡片网格
        let section = |day_idx: usize, featured: bool| {
            let Some(group) = Self::group_by_day(&groups, day_key(day_idx)) else {
                return gpui::div().into_any_element();
            };
            let color = weekday_color(day_idx);
            let on_card_click = on_card_click.clone();

            let cards = group.items.iter().enumerate().map(move |(ix, item)| {
                let name = item.name.clone();
                let click_name = name.clone();
                let bid = item.bangumi_id;
                let on_click = on_card_click.clone();
                BangumiCard {
                    name: name.clone(),
                    bangumi_id: bid,
                    poster_url: item.cover_url.clone().unwrap_or_default(),
                    format: BangumiFormat::Tv,
                    key: format!("home-card-{day_idx}-{ix}"),
                    on_click: Some(Rc::new(move |window, app| {
                        on_click(&click_name, bid, window, app);
                    })),
                }
            });

            let header = gpui::div()
                .w_full()
                .mb(px(14.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(gpui::div().w(px(4.)).h(px(18.)).rounded(px(2.)).bg(color))
                .child(
                    gpui::div()
                        .text_base()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .child(WEEKDAY_NAMES[day_idx].to_string()),
                )
                .child(
                    gpui::div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(format!("{} 部", group.items.len())),
                )
                .when(featured, |this| {
                    this.child(
                        gpui::div()
                            .ml(px(4.))
                            .px(px(8.))
                            .py(px(2.))
                            .rounded_full()
                            .bg(theme.primary)
                            .text_xs()
                            .text_color(theme.primary_foreground)
                            .child("今天"),
                    )
                });

            gpui::div()
                .w_full()
                .mb(px(32.))
                .child(header)
                .child(
                    gpui::div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(18.))
                        .children(cards),
                )
                .into_any_element()
        };

        // 剧场版分组
        let movie_section = match Self::group_by_day(&groups, "movie") {
            Some(group) => {
                let on_card_click = on_card_click.clone();

                let cards = group.items.iter().enumerate().map(move |(ix, item)| {
                    let name = item.name.clone();
                    let click_name = name.clone();
                    let bid = item.bangumi_id;
                    let on_click = on_card_click.clone();
                    BangumiCard {
                        name: name.clone(),
                        bangumi_id: bid,
                        poster_url: item.cover_url.clone().unwrap_or_default(),
                        format: BangumiFormat::Movie,
                        key: format!("movie-card-{ix}"),
                        on_click: Some(Rc::new(move |window, app| {
                            on_click(&click_name, bid, window, app);
                        })),
                    }
                });

                let header = gpui::div()
                    .w_full()
                    .mb(px(14.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        gpui::div()
                            .w(px(4.))
                            .h(px(18.))
                            .rounded(px(2.))
                            .bg(theme.info),
                    )
                    .child(
                        gpui::div()
                            .text_base()
                            .font_semibold()
                            .text_color(theme.foreground)
                            .child("剧场版"),
                    )
                    .child(
                        gpui::div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!("{} 部", group.items.len())),
                    );

                gpui::div()
                    .w_full()
                    .mb(px(32.))
                    .child(header)
                    .child(
                        gpui::div()
                            .w_full()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap(px(18.))
                            .children(cards),
                    )
                    .into_any_element()
            }
            None => gpui::div().into_any_element(),
        };

        // 页面头部
        let page_header = match filter {
            HomeFilter::Today => {
                let day_color = weekday_color(today);
                gpui::div()
                    .w_full()
                    .mb(px(24.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(icon("sparkles", 22.).text_color(theme.primary))
                            .child(
                                gpui::div()
                                    .text_2xl()
                                    .font_bold()
                                    .text_color(theme.foreground)
                                    .child("今日更新"),
                            ),
                    )
                    .child(
                        gpui::div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!("{y} 年 {m} 月 {d} 日 · {}", WEEKDAY_NAMES[today])),
                    )
                    .child(
                        gpui::div()
                            .mt(px(4.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(gpui::div().size(px(8.)).rounded_full().bg(day_color))
                            .child(
                                gpui::div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(format!(
                                        "今天是 {},共 {} 部新番放送",
                                        WEEKDAY_NAMES[today],
                                        Self::group_by_day(&groups, day_key(today))
                                            .map(|g| g.items.len())
                                            .unwrap_or(0)
                                    )),
                            ),
                    )
            }
            HomeFilter::Weekday(day) => gpui::div()
                .w_full()
                .mb(px(24.))
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    gpui::div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(
                            gpui::div()
                                .size(px(14.))
                                .rounded_full()
                                .bg(weekday_color(day)),
                        )
                        .child(
                            gpui::div()
                                .text_2xl()
                                .font_bold()
                                .text_color(theme.foreground)
                                .child(WEEKDAY_NAMES[day].to_string()),
                        ),
                )
                .child(
                    gpui::div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(format!(
                            "共 {} 部番剧 · 每周{}更新",
                            Self::group_by_day(&groups, day_key(day))
                                .map(|g| g.items.len())
                                .unwrap_or(0),
                            WEEKDAY_NAMES[day]
                        )),
                ),
            HomeFilter::Movies => gpui::div()
                .w_full()
                .mb(px(24.))
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    gpui::div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(icon("film", 22.).text_color(theme.info))
                        .child(
                            gpui::div()
                                .text_2xl()
                                .font_bold()
                                .text_color(theme.foreground)
                                .child("剧场版"),
                        ),
                )
                .child(
                    gpui::div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("电影与特别篇"),
                ),
        };

        // 组装主体:过滤条固定在顶部,下方是页面头部与卡片分组
        let mut body = gpui::div().w_full().flex().flex_col().child(filter_bar);

        match filter {
            HomeFilter::Today => {
                body = body.child(page_header).child(section(today, true));
                for day in 0..7 {
                    if day != today {
                        body = body.child(section(day, false));
                    }
                }
                body = body.child(movie_section);
            }
            HomeFilter::Weekday(day) => {
                body = body.child(page_header).child(section(day, false));
            }
            HomeFilter::Movies => {
                body = body.child(page_header).child(movie_section);
            }
        }

        body.into_any_element()
    }
}
