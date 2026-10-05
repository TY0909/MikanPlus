use crate::id::{BangumiId, SubgroupRef};

/// A top-level destination. Each owns its own navigation stack, so switching between
/// sections preserves where you were in each one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    Home,
    Subscription,
    Download,
    Settings,
}

impl Section {
    /// Sections shown as toolbar tabs, in order. `Download` is opened by its own toolbar
    /// button rather than a tab.
    pub const TABS: [Section; 3] = [Section::Home, Section::Subscription, Section::Settings];

    /// The page shown when the section is entered for the first time.
    pub fn root_page(&self) -> Page {
        match self {
            Section::Home => Page::Home(HomeFilter::Today),
            Section::Subscription => Page::Subscription,
            Section::Download => Page::DownloadObserver,
            Section::Settings => Page::Settings,
        }
    }

    /// The section a page is the root of; sub-pages (detail, search, …) return `None`.
    pub fn from_page(page: &Page) -> Option<Section> {
        match page {
            Page::Home(_) => Some(Section::Home),
            Page::Subscription => Some(Section::Subscription),
            Page::DownloadObserver => Some(Section::Download),
            Page::Settings => Some(Section::Settings),
            _ => None,
        }
    }

    /// Display name shown in the toolbar.
    pub fn label(&self) -> &'static str {
        match self {
            Section::Home => "首页",
            Section::Subscription => "我的订阅",
            Section::Download => "下载",
            Section::Settings => "设置",
        }
    }
}

/// Home-page view filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeFilter {
    /// Today: highlight today's weekday group, then show all anime.
    Today,
    /// A single weekday group, 0 = Monday … 6 = Sunday.
    Weekday(usize),
    /// Movie (theatrical) releases.
    Movies,
}

impl HomeFilter {
    /// Display name used in menus and page titles.
    pub fn label(&self) -> &'static str {
        match self {
            HomeFilter::Today => "今日更新",
            // Defensive fallback: Weekday can be built by arbitrary code, so keep
            // out-of-range values from panicking.
            HomeFilter::Weekday(day) => WEEKDAY_NAMES.get(*day).copied().unwrap_or("星期"),
            HomeFilter::Movies => "剧场版",
        }
    }
}

/// Weekday names (0 = Monday … 6 = Sunday).
pub const WEEKDAY_NAMES: [&str; 7] = [
    "星期一",
    "星期二",
    "星期三",
    "星期四",
    "星期五",
    "星期六",
    "星期日",
];

/// Short weekday names (for compact places such as card badges).
pub const WEEKDAY_SHORT: [&str; 7] = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];

/// The page currently being shown (navigation state).
#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    Home(HomeFilter),
    Subscription,
    Settings,
    /// Anime detail page: the anime's identity plus its display name.
    BangumiDetail {
        bangumi_id: BangumiId,
        name: String,
    },
    /// Subtitle-group detail page (which subgroup offering of which bangumi).
    SubGroupDetail(SubgroupRef),
    /// Keyword search results (the query; empty opens the search page).
    SearchResult(String),
    /// Observer page for in-progress download tasks.
    DownloadObserver,
    /// Detail page for a completed multi-file download (task info hash).
    DownloadCollection(String),
}
