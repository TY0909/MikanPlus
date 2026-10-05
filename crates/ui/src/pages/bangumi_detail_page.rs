//! Bangumi detail page: hero header (large cover + metadata + subscribe) and subtitle-group list.
//!
//! Detail data comes from a JSON API and contains no episodes; subtitle-group episodes are lazy-loaded
//! on demand (requested only when a group is expanded). See [`GroupEpisodesState`] for the state.
//!
//! Composition: hero header in [`hero`], single subtitle-group card in [`group_card`].

mod group_card;
mod hero;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::StyledExt;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{App, ScrollHandle, Window, prelude::*, px};

use crate::components::load_state::{GroupEpisodesState, ToggleGroupCallback};
use crate::theme::layout::MAX_DETAIL_W;
use domain::{BangumiId, BangumiItem, SubgroupId};
use downloader::DownloadManager;
use group_card::{GroupCardContext, render_group_card};
use source::Network;
use storage::paths;

pub type GoBackCallback = Rc<dyn Fn(&mut Window, &mut App)>;
pub type ToggleSubscribeCallback =
    Rc<dyn Fn(BangumiId, SubgroupId, Option<&str>, Option<&str>, &mut Window, &mut App)>;
pub type CheckSubscribedCallback = Rc<dyn Fn(BangumiId, SubgroupId) -> bool>;

#[derive(IntoElement)]
pub struct BangumiDetailPage {
    pub item: BangumiItem,
    pub is_subscribed: CheckSubscribedCallback,
    pub on_toggle_subscribe: ToggleSubscribeCallback,
    /// Page scroll handle (owned by the app; restores scroll position after switching pages).
    pub scroll_handle: ScrollHandle,
    /// Download manager (add/cancel tasks).
    pub downloader: Arc<DownloadManager>,
    /// Base download directory (read from the owned app state by the caller).
    pub download_base: PathBuf,
    /// Completed multi-file tasks open the collection page.
    pub on_open_collection: crate::components::episode_row::OpenCollectionCallback,
    /// Set of subtitle-group ids currently expanded for this bangumi.
    pub expanded_groups: HashSet<SubgroupId>,
    /// Subtitle-group episode load state (subgroup id → state).
    pub group_episodes: HashMap<SubgroupId, GroupEpisodesState>,
    /// Toggle a subtitle group on title click: `(bangumi id, subgroup id)`.
    pub on_toggle_group: ToggleGroupCallback,
    /// Retry after a load failure: `(bangumi id, subgroup id)`.
    pub on_reload_group: ToggleGroupCallback,
    /// Network client (for cover loading).
    pub network: Arc<Network>,
}

impl RenderOnce for BangumiDetailPage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let is_dark = theme.mode.is_dark();
        // Available width of the episode-row title column (used for data-layer truncation; see episode_row::truncate_title).
        let win_w: f32 = window.bounds().size.width.into();
        let content_w = win_w.min(MAX_DETAIL_W) - 32.0 * 2.0 - 14.0 * 2.0;
        let title_max_px = (content_w * 0.6).max(60.0);
        let item = self.item;
        let bangumi_id = item.bangumi_id;
        let name = item.name.clone();
        let poster_url = item.cover_url.clone().unwrap_or_default();
        let meta = item.meta.as_ref();
        let summary = meta.and_then(|m| m.summary.as_deref());
        let official_site = meta.and_then(|m| m.official_site.as_deref());
        let bangumi_link = meta.and_then(|m| m.bangumi_link.as_deref());
        let broadcast_day = meta.and_then(|m| m.broadcast_day.as_deref());
        let broadcast_start = meta.and_then(|m| m.broadcast_start.as_deref());
        let groups = item.subtitle_groups.clone();
        let on_toggle_subscribe = self.on_toggle_subscribe.clone();
        let on_toggle_group = self.on_toggle_group.clone();
        let on_reload_group = self.on_reload_group.clone();
        let expanded_groups = self.expanded_groups;
        let group_episodes = self.group_episodes;
        let is_subscribed = self.is_subscribed;
        let downloader = self.downloader.clone();
        let on_open_collection = self.on_open_collection.clone();
        let dl_snapshot = downloader.snapshot();
        let dl_base = self.download_base;
        let network = self.network;

        // ---- Assembly ----
        gpui_kit::div()
            .size_full()
            .bg(theme.background)
            .flex()
            .flex_col()
            .child(
                gpui_kit::div()
                    .id("page-scroll")
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
                                    .max_w(px(MAX_DETAIL_W))
                                    .px(px(32.))
                                    .pt(px(28.))
                                    .pb(px(24.))
                                    .flex()
                                    .flex_col()
                                    // Hero: large cover + metadata + subscribe.
                                    .child(hero::render_hero(
                                        &name,
                                        &poster_url,
                                        is_dark,
                                        summary,
                                        official_site,
                                        bangumi_link,
                                        broadcast_day,
                                        broadcast_start,
                                        groups.len(),
                                        &network,
                                        theme,
                                    ))
                                    .child(
                                        // Subtitle-group list (explicit spacing between it and the hero).
                                        gpui_kit::div()
                                            .mt(px(28.))
                                            .w_full()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                gpui_kit::div()
                                                    .text_lg()
                                                    .font_bold()
                                                    .text_color(theme.foreground)
                                                    .child(format!("字幕组 ({})", groups.len())),
                                            )
                                            .children(groups.iter().enumerate().map(|(gix, g)| {
                                                let sid = g.subgroup_id;
                                                // An unknown group has no id and is rendered but inert.
                                                let known = sid.is_known();
                                                let check = is_subscribed.clone();
                                                let subscribed = known && check(bangumi_id, sid);
                                                let on_toggle = on_toggle_subscribe.clone();
                                                let bn = name.clone();
                                                let gn = g.name.clone();
                                                let expanded = expanded_groups.contains(&sid);
                                                let state = group_episodes.get(&sid).cloned();
                                                let bid = bangumi_id;
                                                let toggle_group = on_toggle_group.clone();
                                                let reload_group = on_reload_group.clone();
                                                // Explicit spacing between subtitle-group cards.
                                                gpui_kit::div().mt(px(14.)).child(
                                                    render_group_card(
                                                        gix,
                                                        g.name.clone(),
                                                        g.episode_count,
                                                        expanded,
                                                        state,
                                                        subscribed,
                                                        theme,
                                                        GroupCardContext {
                                                            dl_dir: paths::subgroup_download_dir(
                                                                &dl_base, &name, &g.name,
                                                            ),
                                                            downloader: downloader.clone(),
                                                            snapshot: dl_snapshot.clone(),
                                                            on_open_collection: on_open_collection
                                                                .clone(),
                                                            title_max_px,
                                                        },
                                                        move |window, app| {
                                                            if known {
                                                                toggle_group(bid, sid, window, app);
                                                            }
                                                        },
                                                        move |window, app| {
                                                            if known {
                                                                reload_group(bid, sid, window, app);
                                                            }
                                                        },
                                                        move |window, app| {
                                                            if known {
                                                                on_toggle(
                                                                    bid,
                                                                    sid,
                                                                    Some(&bn),
                                                                    Some(&gn),
                                                                    window,
                                                                    app,
                                                                );
                                                            }
                                                        },
                                                    ),
                                                )
                                            }))
                                            .when(groups.is_empty(), |this| {
                                                this.child(
                                                    gpui_kit::div()
                                                        .mt(px(14.))
                                                        .w_full()
                                                        .py(px(40.))
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .rounded(px(10.))
                                                        .border_1()
                                                        .border_color(theme.border)
                                                        .text_sm()
                                                        .text_color(theme.muted_foreground)
                                                        .child(
                                                            "该番剧暂未开播，或暂无字幕组发布作品",
                                                        ),
                                                )
                                            }),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }
}
