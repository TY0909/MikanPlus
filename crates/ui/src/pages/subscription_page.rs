//! My subscriptions page: shows subscription entries at the "subtitle group" granularity.
//!
//! - Each subtitle-group subscription shows one card (cover + bangumi name + subtitle-group name)
//! - Clicking a card → the episode detail for that subtitle group
//! - Hovering the poster area shows an "unsubscribe" button (unsubscribes that group)
//!
//! Composition: subscription card in [`card`], empty state in [`empty_state`].

mod card;
mod empty_state;

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::ActiveTheme;
use source::Network;

use gpui_kit::{App, ScrollHandle, Window, prelude::*, px};

use crate::theme::layout::{MAX_PAGE_W, page_scroll};
use card::{render_sub_card, responsive_card_size};
use domain::{BangumiId, SubgroupId, Subscription};

pub type GoBackCallback = Rc<dyn Fn(&mut Window, &mut App)>;
pub type SubCardClickCallback = Rc<dyn Fn(BangumiId, SubgroupId, &mut Window, &mut App)>;
/// Unsubscribe a single subtitle group.
pub type UnsubscribeCallback = Rc<dyn Fn(BangumiId, SubgroupId, &mut Window, &mut App)>;
pub type GoHomeCallback = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SubscriptionPage {
    pub subscriptions: Vec<Subscription>,
    /// Page scroll handle (owned by the app; restores scroll position after switching pages).
    pub scroll_handle: ScrollHandle,
    /// Network handle for lazy cover loading.
    pub network: Arc<Network>,
    pub on_sub_click: SubCardClickCallback,
    pub on_unsubscribe: UnsubscribeCallback,
    pub on_go_home: GoHomeCallback,
}

impl RenderOnce for SubscriptionPage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let is_dark = theme.mode.is_dark();
        let on_sub_click = self.on_sub_click;
        let on_unsubscribe = self.on_unsubscribe;
        let on_go_home = self.on_go_home;

        // Responsive column count: window width determines the number of cards per row and the card width.
        let win_w: f32 = window.bounds().size.width.into();
        let card_w = responsive_card_size(win_w).1;

        // Statistics: number of subtitle-group entries and number of bangumi involved.
        let group_count = self.subscriptions.len();
        let bangumi_count = {
            let mut ids: Vec<BangumiId> = self.subscriptions.iter().map(|s| s.bangumi_id).collect();
            ids.sort_unstable();
            ids.dedup();
            ids.len()
        };

        // Statistics row (supplementary info at the top of the page; the title is already in the toolbar segmented nav).
        let summary = || {
            gpui_kit::div()
                .w_full()
                .mb(px(24.))
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(format!(
                    "共 {group_count} 条字幕组订阅 · 涉及 {bangumi_count} 部番剧"
                ))
        };

        if self.subscriptions.is_empty() {
            // Empty state: centered in the whole viewport, with no page title or statistics.
            return gpui_kit::div()
                .size_full()
                .bg(theme.background)
                .flex()
                .items_center()
                .justify_center()
                .child(empty_state::render_empty_state(theme, on_go_home))
                .into_any_element();
        }

        // Non-empty: statistics row + subtitle-group card grid (scrollable).
        let grid = gpui_kit::div()
            .w_full()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(px(18.))
            .children(self.subscriptions.iter().map(|sub| {
                let on_sub_click = on_sub_click.clone();
                let on_unsub = on_unsubscribe.clone();
                let bid = sub.bangumi_id;
                let sid = sub.subgroup_id;
                let name = sub.bangumi_name.clone();
                let group_name = sub.group_name.clone();
                let cover = sub.cover_url.clone().unwrap_or_default();

                render_sub_card(
                    &self.network,
                    name.clone(),
                    group_name.clone(),
                    cover.clone(),
                    card_w,
                    is_dark,
                    theme,
                    move |window, app| {
                        on_sub_click(bid, sid, window, app);
                    },
                    move |window, app| {
                        on_unsub(bid, sid, window, app);
                    },
                )
            }));

        gpui_kit::div()
            .size_full()
            .bg(theme.background)
            .flex()
            .flex_col()
            .child(page_scroll(
                MAX_PAGE_W,
                &self.scroll_handle,
                gpui_kit::div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(summary())
                    .child(grid),
            ))
            .into_any_element()
    }
}
