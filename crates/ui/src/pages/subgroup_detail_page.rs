//! Subgroup detail page: the episode list of a single subtitle group, filterable by title keyword.
//!
//! Composition: page header in [`header`], filter button in [`filter_button`],
//! episode list in [`episode_list`], single episode row in [`row`].

mod episode_list;
mod filter_button;
mod header;
mod row;

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::ActiveTheme;

use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{App, ScrollHandle, Window, prelude::*, px};

use crate::components::load_state::GroupEpisodesState;
use crate::theme::layout::MAX_SUBGROUP_W;
use domain::SubtitleGroup;
use downloader::DownloadManager;
use storage::paths;

use episode_list::EpisodeListContext;

/// Callback that opens the filter window.
pub type OpenFilterCallback = Rc<dyn Fn(&mut Window, &mut App)>;
/// Callback that reloads episodes (retry after failure).
pub type ReloadEpisodesCallback = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SubGroupDetailPage {
    pub bangumi_name: String,
    pub group: SubtitleGroup,
    /// Page scroll handle (owned by the app; restores scroll position after switching pages).
    pub scroll_handle: ScrollHandle,
    /// Download manager (add/cancel tasks).
    pub downloader: Arc<DownloadManager>,
    /// Base download directory (read from the owned app state by the caller).
    pub download_base: PathBuf,
    /// Completed multi-file tasks open the collection page.
    pub on_open_collection: crate::components::episode_row::OpenCollectionCallback,
    /// Current filter keyword (empty = no filtering; the episode title must contain this keyword).
    pub keyword: String,
    /// Opens the filter window when the "filter" button is clicked.
    pub on_open_filter: OpenFilterCallback,
    /// Episode lazy-load state (detail data contains no episodes; they are fetched on demand when entering the page).
    pub episodes_state: GroupEpisodesState,
    /// Retries episodes after a load failure.
    pub on_reload_episodes: ReloadEpisodesCallback,
}

impl RenderOnce for SubGroupDetailPage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let group = self.group;
        let bangumi_name = self.bangumi_name.clone();
        let group_name = group.name.clone();
        let episodes_state = self.episodes_state;
        let on_reload_episodes = self.on_reload_episodes.clone();
        // Use the actual episodes when loaded; fall back to the count declared by the API when not loaded/failed.
        let (episodes, total) = match &episodes_state {
            GroupEpisodesState::Ready(episodes) => (episodes.clone(), episodes.len()),
            _ => (Vec::new(), group.episode_count as usize),
        };
        let downloader = self.downloader.clone();
        let on_open_collection = self.on_open_collection.clone();
        let dl_snapshot = downloader.snapshot();
        let dl_dir = paths::subgroup_download_dir(&self.download_base, &bangumi_name, &group_name);
        let on_open_filter = self.on_open_filter.clone();

        // Available title-column width (the window width is bounded by the page max width; 60% is taken after
        // subtracting the page and row padding). gpui 0.2.2's text ellipsis is unreliable in nested percentage
        // layouts, so truncation happens at the data layer.
        let win_w: f32 = window.bounds().size.width.into();
        let content_w = win_w.min(MAX_SUBGROUP_W) - 32.0 * 2.0 - 14.0 * 2.0;
        let title_max_px = (content_w * 0.6).max(60.0);

        // Keyword filtering: the title must contain the keyword (case-insensitive).
        let keyword = self.keyword.trim().to_string();
        let keyword_lower = keyword.to_lowercase();
        let visible: Vec<(usize, domain::Episode)> = episodes
            .into_iter()
            .enumerate()
            .filter(|(_, ep)| {
                keyword_lower.is_empty() || ep.title.to_lowercase().contains(&keyword_lower)
            })
            .collect();
        let visible_count = visible.len();
        let filtering = !keyword.is_empty();

        gpui_kit::div()
            .size_full()
            .bg(theme.background)
            .flex()
            .flex_col()
            .child(
                gpui_kit::div()
                    .id("sg-scroll")
                    .h_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .vertical_scrollbar(&self.scroll_handle)
                    .child(
                        gpui_kit::div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .items_center()
                            .child(
                                gpui_kit::div()
                                    .w_full()
                                    .max_w(px(MAX_SUBGROUP_W))
                                    .px(px(32.))
                                    .pt(px(28.))
                                    .pb(px(24.))
                                    .flex()
                                    .flex_col()
                                    .child(header::render_header(
                                        &bangumi_name,
                                        &group_name,
                                        total,
                                        visible_count,
                                        filtering,
                                        theme,
                                    ))
                                    .child(episode_list::render_episode_list(EpisodeListContext {
                                        visible: visible.as_slice(),
                                        visible_count,
                                        filtering,
                                        keyword: keyword.as_str(),
                                        episodes_state: &episodes_state,
                                        dl_dir: dl_dir.as_path(),
                                        downloader: &downloader,
                                        snapshot: dl_snapshot.as_slice(),
                                        on_open_collection: &on_open_collection,
                                        on_open_filter: &on_open_filter,
                                        on_reload_episodes: &on_reload_episodes,
                                        title_max_px,
                                        theme,
                                    })),
                            ),
                    ),
            )
    }
}
