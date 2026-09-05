//! MikanPlus application presentation built on GPUI Kit.

pub mod actions;
pub mod app_theme;
pub mod bangumi_card;
pub mod bangumi_detail_page;
pub mod episode_row;
pub mod home_view;
pub mod icons;
pub mod layout;
pub mod poster;
pub mod search_result_page;
pub mod settings_page;
pub mod subgroup_detail_page;
pub mod subscription_page;
pub mod toolbar;

pub use bangumi_card::{BangumiCard, BangumiFormat};
pub use bangumi_detail_page::BangumiDetailPage;
pub use home_view::HomeView;
pub use search_result_page::SearchResultPage;
pub use settings_page::SettingsPage;
pub use subgroup_detail_page::SubGroupDetailPage;
pub use subscription_page::SubscriptionPage;
pub use toolbar::Toolbar;
