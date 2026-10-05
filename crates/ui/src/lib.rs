//! MikanPlus application presentation built on GPUI Kit.

pub mod actions;
pub mod components;
pub mod pages;
pub mod theme;

pub use components::bangumi_card::{BangumiCard, BangumiFormat};
pub use components::load_state::{GroupEpisodesState, ToggleGroupCallback};
pub use components::toolbar::Toolbar;
pub use pages::bangumi_detail_page::BangumiDetailPage;
pub use pages::download_collection_page::DownloadCollectionPage;
pub use pages::download_observer_page::DownloadObserverPage;
pub use pages::home_view::HomeView;
pub use pages::search_result_page::SearchResultPage;
pub use pages::settings_page::SettingsPage;
pub use pages::subgroup_detail_page::SubGroupDetailPage;
pub use pages::subscription_page::SubscriptionPage;
