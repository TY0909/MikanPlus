//! Domain models and navigation state shared by MikanPlus features.

mod model;
pub mod navigation;

pub use model::{
    BangumiGroup, BangumiItem, BangumiKind, BangumiMeta, Episode, SearchEpisode, SearchResults,
    Season, SeasonName, Subscription, SubtitleGroup, Weekday,
};
