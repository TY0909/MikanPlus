use gpui::{Hsla, hsla};

/// 顶层分区(工具栏分段导航):首页 / 订阅 / 设置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopSection {
    Home,
    Subscription,
    Settings,
}

impl TopSection {
    pub const ALL: [TopSection; 3] = [
        TopSection::Home,
        TopSection::Subscription,
        TopSection::Settings,
    ];

    /// 页面 → 对应的顶层分区;详情/搜索等子页面返回 None。
    pub fn from_page(page: &Page) -> Option<TopSection> {
        match page {
            Page::Home(_) => Some(TopSection::Home),
            Page::Subscription => Some(TopSection::Subscription),
            Page::Settings => Some(TopSection::Settings),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            TopSection::Home => "首页",
            TopSection::Subscription => "我的订阅",
            TopSection::Settings => "设置",
        }
    }
}

/// 首页的视图筛选。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeFilter {
    /// 今日更新:高亮今天的星期分组,随后展示全部番剧
    Today,
    /// 某个星期分组,0=周一 … 6=周日
    Weekday(usize),
    /// 剧场版
    Movies,
}

impl HomeFilter {
    /// 菜单与页面标题使用的显示名称
    pub fn label(&self) -> &'static str {
        match self {
            HomeFilter::Today => "今日更新",
            // 防御:Weekday 可由任意代码构造,越界时兜底(不 panic)
            HomeFilter::Weekday(day) => WEEKDAY_NAMES.get(*day).copied().unwrap_or("星期"),
            HomeFilter::Movies => "剧场版",
        }
    }
}

/// 星期名称(0=周一 … 6=周日)
pub const WEEKDAY_NAMES: [&str; 7] = [
    "星期一",
    "星期二",
    "星期三",
    "星期四",
    "星期五",
    "星期六",
    "星期日",
];

/// 星期短名(用于卡片角标等紧凑场景)
pub const WEEKDAY_SHORT: [&str; 7] = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];

/// 星期颜色(日本传统星期配色):月赤/火橙/水黄/木绿/金青/土蓝/日紫
pub fn weekday_color(day: usize) -> Hsla {
    match day {
        0 => hsla(0.0 / 360.0, 0.72, 0.55, 1.0),   // 月 红
        1 => hsla(24.0 / 360.0, 0.90, 0.55, 1.0),  // 火 橙
        2 => hsla(46.0 / 360.0, 0.95, 0.55, 1.0),  // 水 黄
        3 => hsla(145.0 / 360.0, 0.55, 0.48, 1.0), // 木 绿
        4 => hsla(210.0 / 360.0, 0.95, 0.55, 1.0), // 金 青
        5 => hsla(250.0 / 360.0, 0.55, 0.60, 1.0), // 土 蓝
        _ => hsla(280.0 / 360.0, 0.55, 0.58, 1.0), // 日 紫
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    Home(HomeFilter),
    Subscription,
    Settings,
    BangumiDetail(String),
    SubGroupDetail(u32, u32),
    SearchResult(String),
}
