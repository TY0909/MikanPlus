use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub episodes: Vec<Episode>,
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
