//! Local persistence, cache, migration, and platform path capabilities.

pub mod cache;
pub mod migrate;
pub mod paths;
mod state;

pub use state::{
    load_download_dir, load_json_field, load_subgroup_keywords, load_subscriptions,
    load_theme_mode, save_download_dir, save_json_field, save_subgroup_keywords,
    save_subscriptions, save_theme_mode,
};
