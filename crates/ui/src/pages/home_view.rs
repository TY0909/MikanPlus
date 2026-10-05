//! Home page: today's updates / browse by weekday / movies.
//!
//! Composition: top filter bar in [`filter_bar`], page header in [`page_header`],
//! weekday groups in [`weekday`], movie group in [`movie`], shared card grid in [`card_grid`].

mod card_grid;
mod filter_bar;
mod movie;
mod page_header;
mod weekday;

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::{App, Window, prelude::*};
use source::Network;

use domain::BangumiGroup;
use domain::BangumiId;
use domain::navigation::HomeFilter;

pub type CardClickCallback = Rc<dyn Fn(&str, BangumiId, &mut Window, &mut App)>;
/// Home filter-bar change callback.
pub type FilterChangeCallback = Rc<dyn Fn(HomeFilter, &mut Window, &mut App)>;

/// Home view (stateless; constructed by the parent on each render).
#[derive(IntoElement)]
pub struct HomeView {
    pub groups: Vec<BangumiGroup>,
    pub filter: HomeFilter,
    pub today_weekday: usize,
    pub on_card_click: CardClickCallback,
    pub on_filter_change: FilterChangeCallback,
    /// Network client (for cover loading).
    pub network: Arc<Network>,
}

/// Compute today's day of the week (0=Mon … 6=Sun, in the local time zone).
pub fn today_weekday() -> usize {
    use chrono::Datelike;
    chrono::Local::now().weekday().num_days_from_monday() as usize
}

impl HomeView {
    /// Find a group by its `day` field (monday..sunday / movie).
    pub fn group_by_day<'a>(groups: &'a [BangumiGroup], day: &str) -> Option<&'a BangumiGroup> {
        groups.iter().find(|g| g.day == day)
    }
}

impl RenderOnce for HomeView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let on_card_click = self.on_card_click;
        let on_filter_change = self.on_filter_change;
        // The home page shows only bangumi that have a released work from some subtitle group (updated_at is the
        // most recent release time of each group); the rest (usually not yet aired) are hidden, skipping their cover loads.
        let groups: Vec<BangumiGroup> = self
            .groups
            .into_iter()
            .filter_map(|mut group| {
                group.items.retain(|item| item.updated_at.is_some());
                (!group.items.is_empty()).then_some(group)
            })
            .collect();
        let today = self.today_weekday;
        let filter = self.filter;
        let network = self.network;

        // Top filter bar: Today / Mon–Sun / Movies (replaces the former sidebar weekday navigation).
        let filter_bar = filter_bar::render_filter_bar(filter, &on_filter_change, theme);
        // Page header.
        let page_header = page_header::render_page_header(filter, today, &groups, theme);

        // Assemble the body: the filter bar is pinned at the top, with the page header and card groups below.
        let mut body = gpui_kit::div().w_full().flex().flex_col().child(filter_bar);

        match filter {
            HomeFilter::Today => {
                body = body
                    .child(page_header)
                    .child(weekday::render_weekday_section(
                        &groups,
                        today,
                        true,
                        &on_card_click,
                        &network,
                        theme,
                    ));
                for day in 0..7 {
                    if day != today {
                        body = body.child(weekday::render_weekday_section(
                            &groups,
                            day,
                            false,
                            &on_card_click,
                            &network,
                            theme,
                        ));
                    }
                }
                body = body.child(movie::render_movie_section(
                    &groups,
                    &on_card_click,
                    &network,
                    theme,
                ));
            }
            HomeFilter::Weekday(day) => {
                body = body
                    .child(page_header)
                    .child(weekday::render_weekday_section(
                        &groups,
                        day,
                        false,
                        &on_card_click,
                        &network,
                        theme,
                    ));
            }
            HomeFilter::Movies => {
                body = body.child(page_header).child(movie::render_movie_section(
                    &groups,
                    &on_card_click,
                    &network,
                    theme,
                ));
            }
        }

        body.into_any_element()
    }
}
