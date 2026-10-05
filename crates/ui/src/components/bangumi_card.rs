use std::sync::Arc;
use std::{rc::Rc, time::Duration};

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::StyledExt;
use gpui_kit::{App, Window, prelude::*, px};
use source::Network;

use crate::components::poster::poster;
use crate::theme::app_theme;
use domain::BangumiId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BangumiFormat {
    Tv,
    Movie,
}

impl BangumiFormat {
    pub fn label(&self) -> &'static str {
        match self {
            BangumiFormat::Tv => "TV",
            BangumiFormat::Movie => "剧场版",
        }
    }
}

/// Callback invoked when a card is clicked.
pub type CardAction = Rc<dyn Fn(&mut Window, &mut App)>;

struct BangumiCardHoverState {
    hovered: bool,
}

/// An anime card: poster + name + subscription state.
/// On hover the whole card shifts up, with a border and shadow providing lightweight feedback.
#[derive(IntoElement)]
pub struct BangumiCard {
    pub name: String,
    /// Stable identity (required).
    pub bangumi_id: BangumiId,
    pub poster_url: String,
    pub format: BangumiFormat,
    /// Unique card key (must not repeat within the same grid — the anime name cannot be used as the id, or same-named entries would share interaction state).
    pub key: String,
    pub on_click: Option<CardAction>,
    /// Network client (for cover loading).
    pub network: Arc<Network>,
}

impl RenderOnce for BangumiCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let name = self.name;
        let on_click = self.on_click;
        let is_dark = cx.theme().mode.is_dark();

        // The caller guarantees `key` is unique (falls back to the name when unset, so it at least runs).
        let card_id: gpui_kit::SharedString = if self.key.is_empty() {
            name.clone().into()
        } else {
            self.key.into()
        };

        let hover_state = window.use_keyed_state(card_id.clone(), cx, |_, _| {
            BangumiCardHoverState { hovered: false }
        });
        let hover_target = if hover_state.read(cx).hovered {
            px(-4.)
        } else {
            px(0.)
        };
        let hover_offset = gpui_kit::base::transition(
            card_id.clone(),
            hover_target,
            gpui_kit::base::Transition::new(Duration::from_millis(800)).ease(
                gpui_kit::base::animation::cubic_bezier(0.22, 1.0, 0.36, 1.0),
            ),
            window,
            cx,
        );
        let hover_state_for_listener = hover_state.clone();
        let theme = cx.theme();

        let mut card = gpui_kit::div()
            .w(px(180.))
            .bg(app_theme::card(theme))
            .rounded(px(10.))
            .overflow_hidden()
            .border_1()
            .border_color(theme.border)
            .relative()
            .top(hover_offset)
            .id(card_id)
            .flex()
            .flex_col()
            .hover(|style| {
                style
                    .bg(app_theme::card_hover(theme))
                    .border_color(theme.primary)
                    .shadow(app_theme::card_shadow_hover(theme))
            })
            .on_hover(move |hovered, _, cx| {
                hover_state_for_listener.update(cx, |state, cx| {
                    if state.hovered != *hovered {
                        state.hovered = *hovered;
                        cx.notify();
                    }
                });
            });

        if let Some(cb) = on_click {
            card = card.cursor_pointer().on_click(move |_, window, app| {
                cb(window, app);
            });
        }

        // Poster area: only the top two corners are rounded (the bottom stays square so it joins the text area below).
        let poster_area = gpui_kit::div()
            .h(px(252.))
            .w_full()
            .relative()
            .rounded_t(px(10.))
            .overflow_hidden()
            .child(poster(
                &self.network,
                &self.poster_url,
                &name,
                is_dark,
                px(180.),
                px(252.),
                10.,
                crate::components::poster::CornerStyle::Top,
            ));

        // Name area: shows only the anime name; at most two lines, with an ellipsis on overflow.
        // (gpui 0.2.2's line_clamp omits the ellipsis, so text_ellipsis must be paired with it.)
        let mut name_el = gpui_kit::div()
            .id(gpui_kit::SharedString::from(format!(
                "bangumi-card-name-{name}"
            )))
            .text_sm()
            .font_semibold()
            .text_color(theme.foreground)
            .w_full()
            .line_clamp(2)
            .text_ellipsis();
        // The name may be truncated to two lines: show the full name on hover.
        if crate::components::episode_row::exceeds_lines(&name, 160.0, 14.0, 2) {
            name_el = name_el.tooltip(crate::components::episode_row::title_tooltip(name.clone()));
        }
        name_el = name_el.child(name.clone());

        let label = gpui_kit::div()
            .px(px(10.))
            .py(px(10.))
            .w_full()
            .flex()
            .flex_col()
            .child(name_el);

        card.child(poster_area).child(label)
    }
}
