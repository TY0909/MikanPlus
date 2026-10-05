//! Domain models and navigation state shared by MikanPlus features.

mod id;
mod model;
pub mod navigation;

pub use id::{BangumiId, SubgroupId, SubgroupRef};
pub use model::{
    BangumiGroup, BangumiItem, BangumiKind, BangumiMeta, Episode, SearchEpisode, SearchResults,
    Season, SeasonName, Subscription, SubtitleGroup, Weekday,
};
