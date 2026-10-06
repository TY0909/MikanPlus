//! Application shell state: navigation, cross-scene data, overlays, plus the
//! resident components, services, and callbacks.

use std::collections::HashMap;
use std::rc::Rc;

use gpui_kit::component::input::InputState;
use gpui_kit::{App, Entity, ScrollHandle, Window};

use domain::navigation::{Page, Section};
use domain::{BangumiId, SubgroupId};
use downloader::DownloadManager;
use storage::State;
use ui::{SettingsPage, Toolbar};

use crate::data::model::{AppData, Cache, LoadUpdate, Overlays, Search, take_updates};

/// Maximum navigation history length (back stack).
pub(crate) const HISTORY_LIMIT: usize = 100;

/// Close / go-back style callback.
pub type GoBackCallback = Rc<dyn Fn(&mut Window, &mut App)>;
/// Home card click callback.
pub type CardClickCallback = Rc<dyn Fn(&str, BangumiId, &mut Window, &mut App)>;
/// Callback that toggles a subtitle-group subscription.
pub type ToggleSubscribeCallback =
    Rc<dyn Fn(BangumiId, SubgroupId, Option<&str>, Option<&str>, &mut Window, &mut App)>;
/// Unsubscribe confirmation callback (the argument indicates whether to remove downloads).
pub type UnsubscribeConfirmCallback = Rc<dyn Fn(bool, &mut Window, &mut App)>;
/// Checks whether a given (bangumi, subtitle group) is already subscribed.
pub type SubscribedChecker = Rc<dyn Fn(BangumiId, SubgroupId) -> bool>;

/// Application shell: navigation state + data + overlays + resident components / services / callbacks.
pub(crate) struct MikanPlus {
    /// Current page.
    pub(crate) page: Page,
    /// Active top-level section.
    pub(crate) section: Section,
    /// Per-section navigation stacks; the last element of a section's stack is its current page.
    pub(crate) stacks: HashMap<Section, Vec<Page>>,
    pub(crate) window_handle: gpui_kit::AnyWindowHandle,
    /// Cross-scene data model.
    pub(crate) data: AppData,
    /// Persisted application state (settings, subscriptions), owned here and shared with the UI.
    pub(crate) state: Rc<State>,
    /// Overlay state.
    pub(crate) overlays: Overlays,
    /// Scroll position per page: page key → handle (held by the app, restored after a page switch).
    pub(crate) scroll: Cache<String, ScrollHandle>,
    pub(crate) toolbar: Entity<Toolbar>,
    pub(crate) settings: Entity<SettingsPage>,
    pub(crate) filter_input: Entity<InputState>,
    pub(crate) search_input: Entity<InputState>,
    pub(crate) downloader: std::sync::Arc<DownloadManager>,
    /// Network client (connection pool, request rhythm, image tasks, data source).
    pub(crate) network: std::sync::Arc<source::Network>,
    pub(crate) on_card_click: CardClickCallback,
    pub(crate) on_open_collection: ui::components::episode_row::OpenCollectionCallback,
    pub(crate) on_toggle_subscribe: ToggleSubscribeCallback,
    /// Fullscreen state currently reflected in the macOS menu bar (drives the
    /// "Enter Full Screen" / "Exit Full Screen" label).
    #[cfg(target_os = "macos")]
    pub(crate) fullscreen: bool,
}

impl MikanPlus {
    /// Settle load receipts posted by background threads into the data model (called during render, idempotent).
    pub(crate) fn settle_updates(&mut self) {
        for update in take_updates() {
            match update {
                LoadUpdate::Home(result) => self.data.home = result.into(),
                LoadUpdate::Detail { bid, item } => {
                    self.data.detail_names.remove(&bid);
                    self.data.details.insert(bid, item.into());
                }
                LoadUpdate::Episodes { key, episodes } => {
                    self.data.episodes.insert(key, episodes.into());
                }
                LoadUpdate::Search { query, results } => {
                    let settled = results.into();
                    match self.data.searches.get_mut(&query) {
                        Some(entry) => entry.results = settled,
                        None => {
                            self.data.searches.insert(
                                query,
                                Search {
                                    results: settled,
                                    page: 0,
                                },
                            );
                        }
                    }
                }
            }
        }
    }
}
