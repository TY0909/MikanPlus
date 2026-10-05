//! On-the-wire data shapes (DTOs) for Mikan's `/api/v1` and remote vocabulary parsing.
//!
//! This module only describes "what the wire JSON looks like" and the conversion from remote
//! vocabulary (TV/movie, weekday, season) into the `domain` finite states; [`super`] handles
//! fetching and mapping.

use domain::{BangumiKind, SeasonName, Weekday};
use serde::{Deserialize, Deserializer};

/// Generic envelope for list responses (only `items` is read; pagination fields are ignored).
#[derive(Deserialize)]
pub(super) struct Page<T> {
    pub(super) items: Vec<T>,
}

/// DTO shared by `/api/v1/bangumi` and `/api/v1/bangumi/{id}` (detail fields are absent from list
/// responses).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BangumiDto {
    pub(super) id: u32,
    pub(super) title: String,
    #[serde(rename = "type", default, deserialize_with = "de_kind")]
    pub(super) kind: BangumiKind,
    #[serde(default, deserialize_with = "de_weekday")]
    pub(super) day_of_week: Option<Weekday>,
    #[serde(default)]
    pub(super) cover_url: Option<String>,
    #[serde(default)]
    pub(super) start_date: Option<String>,
    #[serde(default)]
    pub(super) updated_at: Option<String>,
    #[serde(default)]
    pub(super) seasons: Vec<SeasonDto>,
    #[serde(default)]
    pub(super) description: Option<String>,
    #[serde(default)]
    pub(super) official_home_page: Option<String>,
    #[serde(default)]
    pub(super) bangumi_url: Option<String>,
    #[serde(default)]
    pub(super) subtitle_groups: Vec<SubtitleGroupDto>,
}

/// One season entry on the wire (`year` + Chinese season name).
#[derive(Deserialize)]
pub(super) struct SeasonDto {
    pub(super) year: i32,
    pub(super) season: String,
}

/// One subtitle-group summary on the wire.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SubtitleGroupDto {
    pub(super) id: u32,
    pub(super) name: String,
    #[serde(default)]
    pub(super) episode_count: u32,
    #[serde(default)]
    pub(super) latest_published_at: Option<String>,
}

/// Parses a remote Chinese season name (春/夏/秋/冬) into [`SeasonName`].
pub(super) fn season_from(raw: &str) -> Option<SeasonName> {
    match raw {
        "春" => Some(SeasonName::Spring),
        "夏" => Some(SeasonName::Summer),
        "秋" => Some(SeasonName::Autumn),
        "冬" => Some(SeasonName::Winter),
        _ => None,
    }
}

/// Parses a remote type name (TV/WEB/OVA/剧场版) into [`BangumiKind`].
fn kind_from(raw: &str) -> Option<BangumiKind> {
    match raw {
        "TV" => Some(BangumiKind::Tv),
        "WEB" => Some(BangumiKind::Web),
        "OVA" => Some(BangumiKind::Ova),
        "剧场版" => Some(BangumiKind::Movie),
        _ => None,
    }
}

/// Parses a remote Chinese weekday name (星期一…星期日) into [`Weekday`].
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

/// Deserializes the `type` field, defaulting to [`BangumiKind::default`] for unknown values.
fn de_kind<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BangumiKind, D::Error> {
    let raw = Option::<String>::deserialize(deserializer)?;
    Ok(raw.as_deref().and_then(kind_from).unwrap_or_default())
}

/// Deserializes the `dayOfWeek` field, yielding `None` for unknown values.
fn de_weekday<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Weekday>, D::Error> {
    let raw = Option::<String>::deserialize(deserializer)?;
    Ok(raw.as_deref().and_then(weekday_from))
}
