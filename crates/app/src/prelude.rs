//! Shared imports used within the crate. Centralizes common types so each module
//! avoids repeating verbose `use` statements.

pub(crate) use std::collections::{HashMap, HashSet};
pub(crate) use std::rc::Rc;

pub(crate) use gpui_kit::prelude::*;

pub(crate) use gpui_kit::component::input::{Input, InputEvent, InputState};
pub(crate) use gpui_kit::component::switch::Switch;
pub(crate) use gpui_kit::component::{ActiveTheme, StyledExt, WindowExt};
pub(crate) use gpui_kit::{App, Context, Entity, ScrollHandle, Window, px};

pub(crate) use domain::navigation::{HomeFilter, Page, Section};
pub(crate) use domain::{SearchResults, Subscription};

pub(crate) use downloader::{DownloadCmd, DownloadManager};
pub(crate) use source::SourceError;

pub(crate) use ui::components::toolbar::{ActionCallback, NavigateCallback, Toolbar};
pub(crate) use ui::pages::home_view::{FilterChangeCallback, HomeView, today_weekday};
pub(crate) use ui::pages::subgroup_detail_page::{OpenFilterCallback, ReloadEpisodesCallback};
pub(crate) use ui::pages::subscription_page::{SubCardClickCallback, UnsubscribeCallback};
pub(crate) use ui::theme::app_theme;
pub(crate) use ui::{
    BangumiDetailPage, DownloadCollectionPage, DownloadObserverPage, GroupEpisodesState,
    SearchResultPage, SettingsPage, SubGroupDetailPage, SubscriptionPage,
};

pub(crate) use crate::shell::state::*;
