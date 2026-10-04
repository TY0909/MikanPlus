//! 蜜柑公开 JSON API(`/api/v1`)数据源。
//!
//! 该 API **忽略一切 query 参数**（仅路径参数生效），且端点有限：
//! - `GET /api/v1/bangumi`          当前季在播番剧列表（服务端已按星期排好）
//! - `GET /api/v1/bangumi/{id}`     番剧信息 + 字幕组摘要（**不含剧集**）
//!
//! 字幕组剧集按需获取，见 [`crate::rss`]（详情页懒加载）。
//!
//! 这里只做 DTO 反序列化 + 映射/归一化，对外统一产出 `domain` 类型。

use domain::{
    BangumiGroup, BangumiItem, BangumiKind, BangumiMeta, Season, SeasonName, SubtitleGroup, Weekday,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

use crate::{SourceError, network, rss};

/// 列表类响应的通用信封（只取 `items`，忽略分页字段）。
#[derive(Deserialize)]
struct Page<T> {
    items: Vec<T>,
}

/// `/api/v1/bangumi` 与 `/api/v1/bangumi/{id}` 共用的 DTO（详情字段在列表响应中缺省）。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BangumiDto {
    id: u32,
    title: String,
    #[serde(rename = "type", default, deserialize_with = "de_kind")]
    kind: BangumiKind,
    #[serde(default, deserialize_with = "de_weekday")]
    day_of_week: Option<Weekday>,
    #[serde(default)]
    cover_url: Option<String>,
    #[serde(default)]
    start_date: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    seasons: Vec<SeasonDto>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    official_home_page: Option<String>,
    #[serde(default)]
    bangumi_url: Option<String>,
    #[serde(default)]
    subtitle_groups: Vec<SubtitleGroupDto>,
}

#[derive(Deserialize)]
struct SeasonDto {
    year: i32,
    season: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubtitleGroupDto {
    id: u32,
    name: String,
    #[serde(default)]
    episode_count: u32,
    #[serde(default)]
    latest_published_at: Option<String>,
}

/// 首页列表 API 地址（供限速退避按 URL 复位）。
pub fn home_url() -> String {
    format!("{}/api/v1/bangumi", network::base_url())
}

/// 番剧详情 API 地址（供限速退避按 URL 复位）。
pub fn bangumi_url(id: u32) -> String {
    format!("{}/api/v1/bangumi/{id}", network::base_url())
}

/// 拉取首页列表（当前季在播），按星期/剧场版分组。
pub fn fetch_home() -> Result<Vec<BangumiGroup>, SourceError> {
    let page: Page<BangumiDto> = parse(&network::fetch_html(&home_url())?)?;
    Ok(group_by_weekday(page.items))
}

/// 拉取番剧详情（信息 + 字幕组；剧集为空，由 [`crate::rss::fetch_subgroup_episodes`] 按需加载）。
pub fn fetch_bangumi(id: u32) -> Result<BangumiItem, SourceError> {
    let dto: BangumiDto = parse(&network::fetch_html(&bangumi_url(id))?)?;
    Ok(to_bangumi_item(dto))
}

fn parse<T: DeserializeOwned>(json: &str) -> Result<T, SourceError> {
    serde_json::from_str(json).map_err(|_| SourceError::Decode)
}

/// 按星期分组；`剧场版` 单独成一组（与现有首页 UI 的 `movie` key 对齐）。
fn group_by_weekday(items: Vec<BangumiDto>) -> Vec<BangumiGroup> {
    const ORDER: [Weekday; 7] = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];
    let mut groups: Vec<BangumiGroup> = ORDER
        .into_iter()
        .map(|day| BangumiGroup {
            day: day.key().to_string(),
            title: day.label().to_string(),
            items: Vec::new(),
        })
        .collect();
    let mut movies: Vec<BangumiItem> = Vec::new();
    for dto in items {
        let weekday = dto.day_of_week;
        let is_movie = dto.kind == BangumiKind::Movie;
        let item = to_bangumi_item(dto);
        if is_movie {
            movies.push(item);
        } else if let Some(day) = weekday
            && let Some(group) = groups.iter_mut().find(|group| group.day == day.key())
        {
            group.items.push(item);
        }
    }
    groups.retain(|group| !group.items.is_empty());
    if !movies.is_empty() {
        groups.push(BangumiGroup {
            day: "movie".to_string(),
            title: "剧场版".to_string(),
            items: movies,
        });
    }
    groups
}

fn to_bangumi_item(dto: BangumiDto) -> BangumiItem {
    let id = dto.id;
    let cover_url = dto.cover_url.as_deref().map(network::site_url);
    let meta = BangumiMeta {
        broadcast_day: dto.day_of_week.map(|day| day.label().to_string()),
        broadcast_start: dto.start_date,
        official_site: dto.official_home_page,
        bangumi_link: dto.bangumi_url,
        cover_url: cover_url.clone(),
        summary: dto.description,
    };
    let subtitle_groups = dto
        .subtitle_groups
        .into_iter()
        .map(|group| to_subtitle_group(id, group))
        .collect();
    BangumiItem {
        name: dto.title,
        bangumi_id: Some(id),
        cover_url,
        detail_url: Some(format!("{}/Home/Bangumi/{id}", network::base_url())),
        meta: Some(meta),
        kind: dto.kind,
        seasons: dto
            .seasons
            .into_iter()
            .filter_map(|season| {
                season_from(&season.season).map(|name| Season {
                    year: season.year,
                    season: name,
                })
            })
            .collect(),
        updated_at: dto.updated_at,
        subtitle_groups,
        ..Default::default()
    }
}

fn to_subtitle_group(bangumi_id: u32, dto: SubtitleGroupDto) -> SubtitleGroup {
    SubtitleGroup {
        name: dto.name,
        subgroup_id: Some(dto.id),
        subscription_url: Some(rss::subgroup_rss_url(bangumi_id, dto.id)),
        episodes: Vec::new(),
        episode_count: dto.episode_count,
        latest_published_at: dto.latest_published_at,
    }
}

fn kind_from(raw: &str) -> Option<BangumiKind> {
    match raw {
        "TV" => Some(BangumiKind::Tv),
        "WEB" => Some(BangumiKind::Web),
        "OVA" => Some(BangumiKind::Ova),
        "剧场版" => Some(BangumiKind::Movie),
        _ => None,
    }
}

fn weekday_from(raw: &str) -> Option<Weekday> {
    match raw {
        "星期一" => Some(Weekday::Mon),
        "星期二" => Some(Weekday::Tue),
        "星期三" => Some(Weekday::Wed),
        "星期四" => Some(Weekday::Thu),
        "星期五" => Some(Weekday::Fri),
        "星期六" => Some(Weekday::Sat),
        "星期日" => Some(Weekday::Sun),
        _ => None,
    }
}

fn season_from(raw: &str) -> Option<SeasonName> {
    match raw {
        "春" => Some(SeasonName::Spring),
        "夏" => Some(SeasonName::Summer),
        "秋" => Some(SeasonName::Autumn),
        "冬" => Some(SeasonName::Winter),
        _ => None,
    }
}

fn de_kind<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BangumiKind, D::Error> {
    let raw = Option::<String>::deserialize(deserializer)?;
    Ok(raw.as_deref().and_then(kind_from).unwrap_or_default())
}

fn de_weekday<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Weekday>, D::Error> {
    let raw = Option::<String>::deserialize(deserializer)?;
    Ok(raw.as_deref().and_then(weekday_from))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = r#"{"items":[
        {"id":4111,"title":"A","type":"TV","dayOfWeek":"星期日","coverUrl":"/images/a.jpg","startDate":"2026-10-04T00:00:00","updatedAt":null,"seasons":[{"year":2026,"season":"秋"}],"links":{"site":"x","rss":"y"}},
        {"id":9,"title":"B","type":"剧场版","dayOfWeek":"星期日","coverUrl":"/images/b.jpg"}
    ],"page":1,"pageSize":100,"total":2,"hasMore":false}"#;

    const DETAIL: &str = r#"{"id":3560,"title":"T","type":"TV","dayOfWeek":"星期六","coverUrl":"/images/c.jpg","startDate":"2025-01-04T16:00:00","description":"简介","officialHomePage":"https://o/","bangumiUrl":"https://bgm.tv/subject/1","seasons":[{"year":2025,"season":"冬"}],"subtitleGroups":[{"id":370,"name":"LoliHouse","episodeCount":13,"latestPublishedAt":"2025-03-30T01:38:59.154"}]}"#;

    fn list_items(json: &str) -> Vec<BangumiDto> {
        parse::<Page<BangumiDto>>(json).unwrap().items
    }

    #[test]
    fn groups_home_by_weekday_and_movie() {
        let groups = group_by_weekday(list_items(LIST));
        let sunday = groups.iter().find(|g| g.day == "sunday").unwrap();
        assert_eq!(sunday.items.len(), 1);
        assert_eq!(sunday.title, "星期日");
        let item = &sunday.items[0];
        assert_eq!(item.name, "A");
        assert_eq!(item.kind, BangumiKind::Tv);
        assert_eq!(item.seasons.len(), 1);
        assert_eq!(item.seasons[0].season, SeasonName::Autumn);
        assert!(
            item.cover_url
                .as_deref()
                .unwrap()
                .starts_with("https://mikanani.me/")
        );

        let movie = groups.iter().find(|g| g.day == "movie").unwrap();
        assert_eq!(movie.items.len(), 1);
        assert_eq!(movie.items[0].kind, BangumiKind::Movie);
    }

    #[test]
    fn detail_maps_meta_and_groups_without_episodes() {
        let item = to_bangumi_item(parse::<BangumiDto>(DETAIL).unwrap());
        assert_eq!(item.bangumi_id, Some(3560));
        assert_eq!(item.kind, BangumiKind::Tv);
        let meta = item.meta.as_ref().unwrap();
        assert_eq!(meta.summary.as_deref(), Some("简介"));
        assert_eq!(meta.official_site.as_deref(), Some("https://o/"));
        assert_eq!(
            meta.bangumi_link.as_deref(),
            Some("https://bgm.tv/subject/1")
        );
        assert_eq!(meta.broadcast_day.as_deref(), Some("星期六"));
        assert_eq!(meta.broadcast_start.as_deref(), Some("2025-01-04T16:00:00"));

        assert_eq!(item.subtitle_groups.len(), 1);
        let group = &item.subtitle_groups[0];
        assert_eq!(group.subgroup_id, Some(370));
        assert_eq!(group.name, "LoliHouse");
        assert_eq!(group.episode_count, 13);
        assert_eq!(
            group.latest_published_at.as_deref(),
            Some("2025-03-30T01:38:59.154")
        );
        assert!(group.episodes.is_empty());
        assert!(
            group
                .subscription_url
                .as_deref()
                .unwrap()
                .ends_with("/RSS/Bangumi?bangumiId=3560&subgroupid=370")
        );
    }

    #[test]
    fn unknown_kind_and_weekday_are_tolerated() {
        let json = r#"{"items":[{"id":1,"title":"X","type":"UNKNOWN","dayOfWeek":"星期八"}]}"#;
        // 未知类型 → Other；未知星期 → 丢弃；不应 panic
        assert!(group_by_weekday(list_items(json)).is_empty());
    }

    #[test]
    fn invalid_json_is_decode_error() {
        assert!(matches!(
            parse::<Page<BangumiDto>>("not json"),
            Err(SourceError::Decode)
        ));
    }
}
