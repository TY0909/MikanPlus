//! Search result page: search Mikan Project online and show bangumi cards and episode results.
//!
//! A pure content page (no scroll container): scrolling / centering / width cap are handled uniformly by the
//! outer `page_scroll`.
//!
//! Composition: bangumi card grid in [`card_grid`], episode result list in [`episode_list`].

mod card_grid;
mod episode_list;

use std::{path::PathBuf, rc::Rc, sync::Arc};

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::StyledExt;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::{App, Entity, Window, prelude::*, px};

use crate::pages::home_view::CardClickCallback;
use crate::theme::icons::icon;
use crate::theme::layout::MAX_PAGE_W;
use domain::SearchResults;
use downloader::DownloadManager;
use episode_list::EpisodeListContext;
use source::Network;

/// Number of episodes per page (matches Mikan's on-site pagination).
pub const SEARCH_PAGE_SIZE: usize = 50;

/// Search page keyword-submit callback.
pub type SearchCallback = Rc<dyn Fn(String, &mut Window, &mut App)>;
/// Pagination change callback.
pub type SearchPageChangeCallback = std::rc::Rc<dyn Fn(usize, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SearchResultPage {
    pub query: String,
    /// Persistent search-page input; its input state is preserved across page switches.
    pub input_state: Entity<InputState>,
    pub on_search: SearchCallback,
    pub results: SearchResults,
    pub on_card_click: CardClickCallback,
    /// Completed multi-file tasks open the collection page.
    pub on_open_collection: crate::components::episode_row::OpenCollectionCallback,
    /// Download manager (single-episode downloads from search results).
    pub downloader: Arc<DownloadManager>,
    /// Base download directory (read from the owned app state by the caller).
    pub download_dir: PathBuf,
    /// Current page number (0-based).
    pub page: usize,
    /// Pagination change callback.
    pub on_page_change: SearchPageChangeCallback,
    /// Network client (for cover loading).
    pub network: Arc<Network>,
}

impl RenderOnce for SearchResultPage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let query = self.query;
        let has_query = !query.trim().is_empty();
        let input_state = self.input_state.clone();
        let on_search = self.on_search.clone();
        let on_card_click = self.on_card_click;
        let on_open_collection = self.on_open_collection;
        let on_page_change = self.on_page_change;
        let downloader = self.downloader.clone();
        let download_snapshot = downloader.snapshot();
        // Search results have no bangumi/subtitle-group affiliation, so episodes go directly into the user's download directory.
        let download_dir = self.download_dir;
        let network = self.network;

        let total_episodes = self.results.episodes.len();
        let total_pages = total_episodes.div_ceil(SEARCH_PAGE_SIZE).max(1);
        // Clamp the page number (it may be out of range after data refreshes).
        let page = self.page.min(total_pages - 1);
        let start = page * SEARCH_PAGE_SIZE;
        let end = (start + SEARCH_PAGE_SIZE).min(total_episodes);
        // Pagination boundaries: first page disables "Previous", last page disables "Next".
        let prev_enabled = page > 0;
        let next_enabled = page + 1 < total_pages;
        // Whole-page empty state (both bangumi cards and episodes are empty).
        let whole_empty = has_query && self.results.items.is_empty() && total_episodes == 0;

        let search_input = Input::new(&input_state).w_full().h(px(36.)).rounded(px(8.));
        let search_submit_input = input_state.clone();
        let search_form = gpui_kit::div()
            .w_full()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(search_input)
            .child(
                Button::new("search-page-submit")
                    .label("搜索")
                    .small()
                    .primary()
                    .rounded(px(8.))
                    .on_click(move |_, window, app| {
                        let query = search_submit_input.read(app).text().to_string();
                        let query = query.trim().to_string();
                        if !query.is_empty() {
                            on_search(query, window, app);
                        }
                    }),
            );

        // Available title-column width (same estimation as the subgroup detail page): the window width is bounded
        // by the page max width, then 60% is taken after subtracting the page and row padding. gpui 0.2.2's CSS
        // ellipsis is unreliable in nested percentage layouts, so truncation happens at the data layer.
        let win_w: f32 = window.bounds().size.width.into();
        let content_w = win_w.min(MAX_PAGE_W) - 32.0 * 2.0 - 14.0 * 2.0;
        let title_max_px = (content_w * 0.6).max(60.0);

        // Bangumi card grid.
        let cards = card_grid::render_card_grid(&self.results.items, &on_card_click, &network);
        // Episode result list (header / current-page rows / empty state / pagination).
        let results_section = episode_list::render_episode_list(
            &self.results.episodes,
            start,
            end,
            total_episodes,
            total_pages,
            page,
            prev_enabled,
            next_enabled,
            EpisodeListContext {
                download_dir,
                downloader,
                snapshot: download_snapshot,
                on_open_collection,
                title_max_px,
            },
            &on_page_change,
            theme,
        );

        // Pure content (scrolling/centering/padding are handled by the outer page_scroll, avoiding a double scroll container).
        gpui_kit::div()
            .w_full()
            .flex()
            .flex_col()
            .child(
                // Header.
                gpui_kit::div()
                    .w_full()
                    .mb(px(24.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        gpui_kit::div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(icon("search", 22.).text_color(theme.primary))
                            .child(
                                gpui_kit::div()
                                    .text_2xl()
                                    .font_bold()
                                    .text_color(theme.foreground)
                                    .child(if has_query {
                                        format!("搜索「{query}」")
                                    } else {
                                        "搜索番剧".to_string()
                                    }),
                            ),
                    )
                    .child(search_form)
                    .when(has_query, |this| {
                        this.child(
                            gpui_kit::div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(format!(
                                    "找到 {} 部番剧 · {} 条剧集",
                                    self.results.items.len(),
                                    total_episodes
                                )),
                        )
                    }),
            )
            // Empty-query state: show only the input; do not issue a network request.
            .when(!has_query, |this| {
                this.child(
                    gpui_kit::div()
                        .w_full()
                        .py(px(64.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(10.))
                        .child(icon("search", 28.).text_color(theme.muted_foreground))
                        .child(
                            gpui_kit::div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("输入关键词开始搜索"),
                        ),
                )
            })
            // Whole-page empty state: centered message when both bangumi cards and episodes are empty.
            .when(whole_empty, |this| {
                this.child(
                    gpui_kit::div()
                        .w_full()
                        .py(px(64.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(10.))
                        .child(icon("inbox", 28.).text_color(theme.muted_foreground))
                        .child(
                            gpui_kit::div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("没有找到相关番剧或剧集"),
                        )
                        .child(
                            gpui_kit::div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("换个关键词再试试吧"),
                        ),
                )
            })
            // Bangumi card section + episode result section (not rendered when the whole page is empty).
            .when(has_query && !whole_empty, |this| {
                this.child(cards).child(results_section)
            })
    }
}
