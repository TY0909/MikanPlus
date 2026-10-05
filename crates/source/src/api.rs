//! Mikan public JSON API (`/api/v1`) data source.
//!
//! This API **ignores all query parameters** (only path parameters take effect), and its endpoints
//! are limited:
//! - `GET /api/v1/bangumi`          currently airing anime for the season (already ordered by weekday server-side)
//! - `GET /api/v1/bangumi/{id}`     anime info + subtitle-group summary (**no episodes**)
//!
//! Subtitle-group episodes are fetched on demand, see [`crate::rss`] (lazy-loaded on the detail
//! page).
//!
//! This module handles fetching and mapping: the on-the-wire data shape is in [`dto`], and it
//! uniformly produces `domain` types outward.

mod dto;

use domain::{
    BangumiGroup, BangumiId, BangumiItem, BangumiKind, BangumiMeta, Season, SubgroupId,
    SubtitleGroup, Weekday,
};
use serde::de::DeserializeOwned;

use crate::{Network, SourceError, rss};
use dto::{BangumiDto, Page, SubtitleGroupDto, season_from};

/// Home-list API address (used to reset rate-limit backoff by URL).
pub fn home_url(network: &Network) -> String {
    format!("{}/api/v1/bangumi", network.base_url())
}

/// Anime-detail API address (used to reset rate-limit backoff by URL).
pub fn bangumi_url(network: &Network, id: BangumiId) -> String {
    format!("{}/api/v1/bangumi/{}", network.base_url(), id.get())
}

/// Fetches the home list (currently airing this season), grouped by weekday / movie.
pub fn fetch_home(network: &Network) -> Result<Vec<BangumiGroup>, SourceError> {
    let page: Page<BangumiDto> = parse(&network.fetch_html(&home_url(network))?)?;
    Ok(group_by_weekday(network, page.items))
}

/// Fetches anime details (info + subtitle groups; episodes are empty and loaded on demand by
/// [`crate::rss::fetch_subgroup_episodes`]).
pub fn fetch_bangumi(network: &Network, id: BangumiId) -> Result<BangumiItem, SourceError> {
    let dto: BangumiDto = parse(&network.fetch_html(&bangumi_url(network, id))?)?;
    Ok(to_bangumi_item(network, dto))
}

/// Parses JSON, mapping any deserialization error to [`SourceError::Decode`].
fn parse<T: DeserializeOwned>(json: &str) -> Result<T, SourceError> {
    serde_json::from_str(json).map_err(|_| SourceError::Decode)
}

/// Groups by weekday; movies (`剧场版`) form a separate group (aligned with the existing home UI's
/// `movie` key).
fn group_by_weekday(network: &Network, items: Vec<BangumiDto>) -> Vec<BangumiGroup> {
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
        let item = to_bangumi_item(network, dto);
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

/// Maps a wire DTO into the domain [`BangumiItem`].
fn to_bangumi_item(network: &Network, dto: BangumiDto) -> BangumiItem {
    let id = BangumiId::from(dto.id);
    let cover_url = dto.cover_url.as_deref().map(|u| network.site_url(u));
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
        .map(|group| to_subtitle_group(network, id, group))
        .collect();
    BangumiItem {
        name: dto.title,
        bangumi_id: id,
        cover_url,
        detail_url: Some(format!("{}/Home/Bangumi/{}", network.base_url(), id.get())),
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

/// Maps a wire subtitle-group DTO into the domain [`SubtitleGroup`] (with no episodes attached).
fn to_subtitle_group(
    network: &Network,
    bangumi_id: BangumiId,
    dto: SubtitleGroupDto,
) -> SubtitleGroup {
    let subgroup_id = SubgroupId::from(dto.id);
    SubtitleGroup {
        name: dto.name,
        subgroup_id,
        subscription_url: Some(rss::subgroup_rss_url(network, bangumi_id, subgroup_id)),
        episodes: Vec::new(),
        episode_count: dto.episode_count,
        latest_published_at: dto.latest_published_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::SeasonName;

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
        let network = Network::new();
        let groups = group_by_weekday(&network, list_items(LIST));
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
        let network = Network::new();
        let item = to_bangumi_item(&network, parse::<BangumiDto>(DETAIL).unwrap());
        assert_eq!(item.bangumi_id, BangumiId::from(3560));
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
        assert_eq!(group.subgroup_id, SubgroupId::from(370));
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
        // Unknown kind → Other; unknown weekday → discarded; must not panic
        assert!(group_by_weekday(&Network::new(), list_items(json)).is_empty());
    }

    #[test]
    fn invalid_json_is_decode_error() {
        assert!(matches!(
            parse::<Page<BangumiDto>>("not json"),
            Err(SourceError::Decode)
        ));
    }
}
