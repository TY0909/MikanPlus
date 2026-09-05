//! Domain models and navigation state shared by MikanPlus features.

mod model;
pub mod navigation;

pub use model::{
    BangumiGroup, BangumiItem, BangumiMeta, Episode, SearchEpisode, SearchResults, Subscription,
    SubtitleGroup,
};
