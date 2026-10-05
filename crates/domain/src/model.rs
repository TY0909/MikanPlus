use serde::{Deserialize, Serialize};

use crate::id::{BangumiId, SubgroupId};

/// Kind of anime (finite state).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BangumiKind {
    /// Not provided or unrecognized (defensive fallback).
    #[default]
    Other,
    Tv,
    Web,
    Ova,
    Movie,
}

/// Update weekday (finite state).
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
    /// Stable key used for home-page grouping.
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

    /// User-facing display name.
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

/// Broadcast season name (finite state).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeasonName {
    Spring,
    Summer,
    Autumn,
    Winter,
}

/// Broadcast season (year plus season name).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Season {
    pub year: i32,
    pub season: SeasonName,
}

/// A single anime entry with its subtitle groups and schedule metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BangumiItem {
    pub name: String,
    /// Stable identity. Required: an entry without a bangumi id is not a Bangumi.
    pub bangumi_id: BangumiId,
    pub cover_url: Option<String>,
    pub detail_url: Option<String>,
    pub meta: Option<BangumiMeta>,
    #[serde(default)]
    pub subtitle_groups: Vec<SubtitleGroup>,
    #[serde(default)]
    pub update_date: Option<String>,
    /// Anime kind (TV / WEB / OVA / movie).
    #[serde(default)]
    pub kind: BangumiKind,
    /// Seasons this anime belongs to.
    #[serde(default)]
    pub seasons: Vec<Season>,
    /// Latest publish time across all subtitle groups (the maximum). `None`
    /// means the anime has no published episodes (the home page greys it out
    /// and disables it).
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Anime items grouped under one update day (a home-page section).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiGroup {
    pub day: String,
    pub title: String,
    pub items: Vec<BangumiItem>,
}

/// Extra metadata fetched from an anime's detail page.
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

/// A fansub group offering episodes for one anime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleGroup {
    pub name: String,
    /// Subtitle group id, or [`SubgroupId::UNKNOWN`] when the API hasn't assigned one.
    #[serde(default)]
    pub subgroup_id: SubgroupId,
    #[serde(default)]
    pub subscription_url: Option<String>,
    /// Episode list. The list/detail APIs omit episodes, so entries are filled
    /// in lazily from RSS (empty by default).
    #[serde(default)]
    pub episodes: Vec<Episode>,
    /// Total episode count for this subtitle group (from the API; `episodes`
    /// is lazily loaded, so this stays 0 until it has been loaded).
    #[serde(default)]
    pub episode_count: u32,
    /// Latest publish time for this group.
    #[serde(default)]
    pub latest_published_at: Option<String>,
}

/// A user subscription to one subtitle group of an anime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub bangumi_id: BangumiId,
    pub subgroup_id: SubgroupId,
    pub bangumi_name: String,
    pub group_name: String,
    pub cover_url: Option<String>,
}

/// A single downloadable episode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub title: String,
    #[serde(default)]
    pub magnet_link: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub publish_date: Option<String>,
    /// Info hash (from the API or RSS).
    #[serde(default)]
    pub hash: Option<String>,
    /// Direct `.torrent` link (from the API or RSS).
    #[serde(default)]
    pub torrent_url: Option<String>,
}

/// An episode result returned by a keyword search.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchEpisode {
    pub title: String,
    pub magnet: String,
    pub size: String,
    pub date: String,
}

/// Combined results of a keyword search (anime and episodes).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchResults {
    pub items: Vec<BangumiItem>,
    pub episodes: Vec<SearchEpisode>,
}
