//! Mikan public JSON API (`/api/v1`) data source.
//!
//! Home, detail, search and lazy subtitle-group episode loading all use the JSON API. The RSS
//! adapter remains available for compatibility with existing subscription URLs, but is no longer
//! used to populate application pages.

mod dto;

use std::collections::HashSet;

use chrono::{Datelike, Local};
use domain::{
    BangumiGroup, BangumiId, BangumiItem, BangumiKind, BangumiMeta, Episode, SearchEpisode,
    SearchResults, Season, SubgroupId, SubtitleGroup, Weekday,
};
use serde::de::DeserializeOwned;

use crate::{Network, SourceError, rss};
use dto::{BangumiDto, EpisodeDto, Page, SubtitleGroupDto, season_from};

/// The season query used by the home API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeasonQuery {
    pub year: i32,
    pub season: domain::SeasonName,
}

impl SeasonQuery {
    /// Calculates the current anime season from the local calendar date.
    pub fn current() -> Self {
        let now = Local::now();
        Self::from_year_month(now.year(), now.month())
    }

    /// Calculates an anime season from a calendar year and month.
    pub fn from_year_month(year: i32, month: u32) -> Self {
        let season = match month {
            1..=3 => domain::SeasonName::Winter,
            4..=6 => domain::SeasonName::Spring,
            7..=9 => domain::SeasonName::Summer,
            _ => domain::SeasonName::Autumn,
        };
        Self { year, season }
    }

    /// Returns the immediately preceding anime season, including the year rollover from winter
    /// to the previous autumn.
    pub fn previous(self) -> Self {
        match self.season {
            domain::SeasonName::Winter => Self {
                year: self.year - 1,
                season: domain::SeasonName::Autumn,
            },
            domain::SeasonName::Spring => Self {
                year: self.year,
                season: domain::SeasonName::Winter,
            },
            domain::SeasonName::Summer => Self {
                year: self.year,
                season: domain::SeasonName::Spring,
            },
            domain::SeasonName::Autumn => Self {
                year: self.year,
                season: domain::SeasonName::Summer,
            },
        }
    }

    fn api_name(self) -> &'static str {
        match self.season {
            domain::SeasonName::Spring => "Spring",
            domain::SeasonName::Summer => "Summer",
            domain::SeasonName::Autumn => "Autumn",
            domain::SeasonName::Winter => "Winter",
        }
    }
}

/// Home-list API address (used to reset rate-limit backoff by URL).
pub fn home_url(network: &Network) -> String {
    home_url_for(network, SeasonQuery::current())
}

/// Home-list API address for a specific year and season.
pub fn home_url_for(network: &Network, season: SeasonQuery) -> String {
    network.home_api_url(Some(season.year), Some(season.api_name()))
}

/// Anime-detail API address (used to reset rate-limit backoff by URL).
pub fn bangumi_url(network: &Network, id: BangumiId) -> String {
    format!("{}/api/v1/bangumi/{}", network.base_url(), id.get())
}

/// Fetches the current season home list, falling back to the previous season only when the current
/// request succeeds but contains no released content.
pub fn fetch_home(network: &Network) -> Result<Vec<BangumiGroup>, SourceError> {
    let current = SeasonQuery::current();
    let current_groups = fetch_home_season(network, current)?;
    if has_released_content(&current_groups) {
        return Ok(current_groups);
    }

    fetch_home_season(network, current.previous())
}

fn fetch_home_season(
    network: &Network,
    season: SeasonQuery,
) -> Result<Vec<BangumiGroup>, SourceError> {
    let page: Page<BangumiDto> = parse(&network.fetch_no_cache(&home_url_for(network, season))?)?;
    Ok(group_by_weekday(network, page.items))
}

fn has_released_content(groups: &[BangumiGroup]) -> bool {
    groups
        .iter()
        .flat_map(|group| &group.items)
        .any(|item| item.updated_at.is_some())
}

/// Fetches anime details (info + subtitle groups; episodes are empty and loaded on demand by
/// [`fetch_subgroup_episodes`]).
pub fn fetch_bangumi(network: &Network, id: BangumiId) -> Result<BangumiItem, SourceError> {
    let dto: BangumiDto = parse(&network.fetch_html(&bangumi_url(network, id))?)?;
    Ok(to_bangumi_item(network, dto))
}

/// Search releases and map them into the existing search page model. The API returns release rows
/// rather than complete bangumi records, so cards are de-duplicated by bangumi id and use the
/// detail page URL; covers are loaded when the user opens the detail page.
pub fn fetch_search(
    network: &Network,
    query: &str,
    page: usize,
    page_size: usize,
) -> Result<SearchResults, SourceError> {
    let url = network.search_api_url(query, page, page_size);
    let response: Page<EpisodeDto> = parse(&network.fetch_no_cache(&url)?)?;
    Ok(to_search_results(network, response.items))
}

/// Fetches all pages for one subtitle group. The API has a reliable `hasMore` flag for episode
/// queries, unlike the search endpoint.
pub fn fetch_subgroup_episodes(
    network: &Network,
    bangumi_id: BangumiId,
    subgroup_id: SubgroupId,
) -> Result<Vec<Episode>, SourceError> {
    const PAGE_SIZE: usize = 100;
    let mut page_number = 1;
    let mut episodes = Vec::new();
    loop {
        let url = network.episodes_api_url(
            bangumi_id.get(),
            page_number,
            PAGE_SIZE,
            Some(subgroup_id.get()),
        );
        let page: Page<EpisodeDto> = parse(&network.fetch_no_cache(&url)?)?;
        let count = page.items.len();
        episodes.extend(
            page.items
                .into_iter()
                .map(|episode| to_episode(network, episode)),
        );
        if !page.has_more.unwrap_or(count == PAGE_SIZE) || count == 0 {
            break;
        }
        page_number += 1;
    }
    Ok(episodes)
}

/// Stable cache key for the complete episode collection represented by
/// [`fetch_subgroup_episodes`].
pub fn subgroup_episodes_url(
    network: &Network,
    bangumi_id: BangumiId,
    subgroup_id: SubgroupId,
) -> String {
    network.episodes_api_url(bangumi_id.get(), 1, 100, Some(subgroup_id.get()))
}

fn to_search_results(network: &Network, releases: Vec<EpisodeDto>) -> SearchResults {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    let mut episodes = Vec::new();
    for release in releases {
        if release.bangumi_id != 0 && seen.insert(release.bangumi_id) {
            let id = BangumiId::from(release.bangumi_id);
            items.push(BangumiItem {
                name: release.bangumi_title.clone(),
                bangumi_id: id,
                detail_url: Some(format!("{}/Home/Bangumi/{}", network.base_url(), id.get())),
                ..Default::default()
            });
        }
        episodes.push(SearchEpisode {
            title: release.title,
            magnet: release
                .magnet_url
                .or_else(|| {
                    release
                        .hash
                        .map(|hash| format!("magnet:?xt=urn:btih:{hash}"))
                })
                .unwrap_or_default(),
            size: release.size.unwrap_or_default(),
            date: release
                .published_at
                .as_deref()
                .and_then(format_publish_date)
                .unwrap_or_default(),
        });
    }
    SearchResults { items, episodes }
}

fn to_episode(network: &Network, dto: EpisodeDto) -> Episode {
    let magnet_link = dto.magnet_url.or_else(|| {
        dto.hash
            .as_ref()
            .map(|hash| format!("magnet:?xt=urn:btih:{hash}"))
    });
    Episode {
        title: dto.title,
        magnet_link,
        size: dto.size,
        publish_date: dto.published_at.as_deref().and_then(format_publish_date),
        hash: dto.hash,
        torrent_url: dto.torrent_url.map(|url| network.normalize_url(&url)),
    }
}

fn format_publish_date(iso: &str) -> Option<String> {
    let (date, time) = iso.split_once('T')?;
    let date = date.replace('-', "/");
    let hhmm: String = time.chars().take(5).collect();
    (date.len() == 10 && hhmm.len() == 5).then(|| format!("{date} {hhmm}"))
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

    #[test]
    fn season_query_uses_calendar_quarters() {
        assert_eq!(
            SeasonQuery::from_year_month(2026, 1),
            SeasonQuery {
                year: 2026,
                season: SeasonName::Winter
            }
        );
        assert_eq!(
            SeasonQuery::from_year_month(2026, 4).season,
            SeasonName::Spring
        );
        assert_eq!(
            SeasonQuery::from_year_month(2026, 7).season,
            SeasonName::Summer
        );
        assert_eq!(
            SeasonQuery::from_year_month(2026, 10).season,
            SeasonName::Autumn
        );
    }

    #[test]
    fn previous_season_rolls_winter_into_previous_year() {
        assert_eq!(
            SeasonQuery {
                year: 2026,
                season: SeasonName::Winter
            }
            .previous(),
            SeasonQuery {
                year: 2025,
                season: SeasonName::Autumn
            }
        );
        assert_eq!(
            SeasonQuery {
                year: 2026,
                season: SeasonName::Autumn
            }
            .previous(),
            SeasonQuery {
                year: 2026,
                season: SeasonName::Summer
            }
        );
    }

    #[test]
    fn released_content_requires_updated_at() {
        let mut groups = group_by_weekday(
            &Network::new(),
            vec![
                parse::<BangumiDto>(
                    r#"{"id":1,"title":"未开播","type":"TV","dayOfWeek":"星期日"}"#,
                )
                .unwrap(),
            ],
        );
        assert!(!has_released_content(&groups));
        groups[0].items[0].updated_at = Some("2026-10-14T00:00:00".to_string());
        assert!(has_released_content(&groups));
    }

    #[test]
    fn search_release_rows_map_to_cards_and_episodes() {
        let json = r#"{"items":[
            {"bangumiId":3503,"bangumiTitle":"测试番剧","title":"[G] 测试 - 01","hash":"ABC","size":"1 GB","publishedAt":"2026-10-14T12:34:56.000","magnetUrl":"magnet:?xt=urn:btih:ABC"},
            {"bangumiId":3503,"bangumiTitle":"测试番剧","title":"[G] 测试 - 02","hash":"DEF","publishedAt":null}
        ],"page":1,"pageSize":100,"total":2,"hasMore":false}"#;
        let releases = parse::<Page<EpisodeDto>>(json).unwrap().items;
        let results = to_search_results(&Network::new(), releases);
        assert_eq!(results.items.len(), 1);
        assert_eq!(results.items[0].name, "测试番剧");
        assert_eq!(results.episodes.len(), 2);
        assert_eq!(results.episodes[0].magnet, "magnet:?xt=urn:btih:ABC");
        assert_eq!(results.episodes[0].date, "2026/10/14 12:34");
        assert_eq!(results.episodes[1].date, "");
    }
}
