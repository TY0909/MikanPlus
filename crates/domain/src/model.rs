use serde::{Deserialize, Serialize};

/// 番剧类型（有限状态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BangumiKind {
    /// 未提供 / 无法识别（容错）
    #[default]
    Other,
    Tv,
    Web,
    Ova,
    Movie,
}

/// 更新星期（有限状态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    /// 首页分组使用的稳定 key。
    pub fn key(self) -> &'static str {
        match self {
            Weekday::Mon => "monday",
            Weekday::Tue => "tuesday",
            Weekday::Wed => "wednesday",
            Weekday::Thu => "thursday",
            Weekday::Fri => "friday",
            Weekday::Sat => "saturday",
            Weekday::Sun => "sunday",
        }
    }

    /// 展示名。
    pub fn label(self) -> &'static str {
        match self {
            Weekday::Mon => "星期一",
            Weekday::Tue => "星期二",
            Weekday::Wed => "星期三",
            Weekday::Thu => "星期四",
            Weekday::Fri => "星期五",
            Weekday::Sat => "星期六",
            Weekday::Sun => "星期日",
        }
    }
}

/// 放送季度名（有限状态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeasonName {
    Spring,
    Summer,
    Autumn,
    Winter,
}

/// 放送季度。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Season {
    pub year: i32,
    pub season: SeasonName,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BangumiItem {
    pub name: String,
    pub bangumi_id: Option<u32>,
    pub cover_url: Option<String>,
    pub detail_url: Option<String>,
    pub meta: Option<BangumiMeta>,
    #[serde(default)]
    pub subtitle_groups: Vec<SubtitleGroup>,
    #[serde(default)]
    pub update_date: Option<String>,
    /// 番剧类型（TV/WEB/OVA/剧场版）。
    #[serde(default)]
    pub kind: BangumiKind,
    /// 所属季度。
    #[serde(default)]
    pub seasons: Vec<Season>,
    /// 各字幕组最近发布时间(取最大值);为 `None` 表示该番剧下没有任何
    /// 已发布剧集(首页据此灰显不可点)。
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiGroup {
    pub day: String,
    pub title: String,
    pub items: Vec<BangumiItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BangumiMeta {
    #[serde(default)]
    pub broadcast_day: Option<String>,
    #[serde(default)]
    pub broadcast_start: Option<String>,
    #[serde(default)]
    pub official_site: Option<String>,
    #[serde(default)]
    pub bangumi_link: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleGroup {
    pub name: String,
    #[serde(default)]
    pub subgroup_id: Option<u32>,
    #[serde(default)]
    pub subscription_url: Option<String>,
    /// 剧集列表。列表/详情 API 不含剧集,由 RSS 按需加载后填充(默认留空)。
    #[serde(default)]
    pub episodes: Vec<Episode>,
    /// 该字幕组的剧集总数（来自 API；`episodes` 为懒加载，未加载时为 0）。
    #[serde(default)]
    pub episode_count: u32,
    /// 该组最近发布时间。
    #[serde(default)]
    pub latest_published_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub bangumi_id: u32,
    pub subgroup_id: u32,
    pub bangumi_name: String,
    pub group_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub title: String,
    #[serde(default)]
    pub magnet_link: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub publish_date: Option<String>,
    /// info hash（来自 API/RSS）。
    #[serde(default)]
    pub hash: Option<String>,
    /// `.torrent` 直链（来自 API/RSS）。
    #[serde(default)]
    pub torrent_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchEpisode {
    pub title: String,
    pub magnet: String,
    pub size: String,
    pub date: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchResults {
    pub items: Vec<BangumiItem>,
    pub episodes: Vec<SearchEpisode>,
}
