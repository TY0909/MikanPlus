//! Detail-page hero header: large cover + metadata (broadcast info / links / summary) + subscribe entry.

use std::sync::Arc;

use gpui_kit::component::StyledExt;
use gpui_kit::component::theme::Theme;
use gpui_kit::{prelude::*, px};
use source::Network;

use crate::components::poster::{CornerStyle, poster, poster_hues};
use crate::theme::app_theme;
use crate::theme::icons::icon;

/// Open an external link; only HTTP(S) is allowed.
fn open_url(url: &str) {
    let _ = storage::paths::open_url(url);
}

/// Convert a broadcast start date in "M/D/YYYY" or "M-D-YYYY" form to "YYYY年M月D日".
fn format_broadcast_start(raw: &str) -> String {
    let parts: Vec<&str> = raw.split(['/', '-']).collect();
    if parts.len() == 3
        && let (Ok(m), Ok(d), Ok(y)) = (
            parts[0].parse::<u32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        )
        && (1..=12).contains(&m)
        && (1..=31).contains(&d)
        && (1000..=9999).contains(&y)
    {
        return format!("{y}年{m}月{d}日");
    }
    raw.to_string()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_hero(
    name: &str,
    poster_url: &str,
    is_dark: bool,
    summary: Option<&str>,
    official_site: Option<&str>,
    bangumi_link: Option<&str>,
    broadcast_day: Option<&str>,
    broadcast_start: Option<&str>,
    group_count: usize,
    network: &Arc<Network>,
    theme: &Theme,
) -> impl IntoElement {
    // Link button.
    let link_btn = |label: &str, icon_name: &str, url: &str| {
        let url = url.to_string();
        gpui_kit::div()
            .flex()
            .items_center()
            .gap(px(4.))
            .text_sm()
            .text_color(theme.link)
            .cursor_pointer()
            .id(gpui_kit::SharedString::from(format!("link-{label}")))
            .hover(|style| style.text_color(theme.link_hover).underline())
            .on_click(move |_, _, _| open_url(&url))
            .child(icon(icon_name, 13.).text_color(theme.link))
            .child(label.to_string())
    };

    // Hero background color (derived from the poster palette).
    let (hue_a, hue_b) = poster_hues(name);
    let hero_bg = gpui_kit::hsla(hue_a / 360.0, 0.45, if is_dark { 0.16 } else { 0.94 }, 1.0);
    let hero_glow = gpui_kit::hsla(hue_b / 360.0, 0.5, if is_dark { 0.14 } else { 0.9 }, 1.0);

    // Link row: official website / Bangumi (compact links merged with the info above).
    let link_row = gpui_kit::div()
        .mt(px(10.))
        .flex()
        .items_center()
        .gap(px(18.))
        .when_some(official_site, |this, url| {
            this.child(link_btn("官方网站", "globe", url))
        })
        .when_some(bangumi_link, |this, url| {
            this.child(link_btn("番组计划", "info", url))
        });

    // Hero: large cover + metadata + subscribe.
    gpui_kit::div()
        .w_full()
        .rounded(px(14.))
        .overflow_hidden()
        .border_1()
        .border_color(theme.border)
        .relative()
        .child(gpui_kit::div().absolute().inset_0().bg(hero_bg))
        .child(
            gpui_kit::div()
                .absolute()
                .right(px(-60.))
                .top(px(-60.))
                .size(px(260.))
                .rounded_full()
                .bg(hero_glow)
                .opacity(0.7),
        )
        .child(
            gpui_kit::div()
                .relative()
                .p(px(28.))
                // Left: cover displayed at 1.5x (252×353), vertically centered with padding above and below.
                .child(
                    gpui_kit::div()
                        .absolute()
                        .top(px(28.))
                        .bottom(px(28.))
                        .left(px(28.))
                        .w(px(252.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .child(
                            gpui_kit::div()
                                .shadow(app_theme::card_shadow(theme))
                                .rounded(px(12.))
                                .overflow_hidden()
                                .child(poster(
                                    network,
                                    poster_url,
                                    name,
                                    is_dark,
                                    px(252.),
                                    px(353.),
                                    12.,
                                    CornerStyle::All,
                                )),
                        ),
                )
                // Right column: title / broadcast info / work info (2×2) / summary (the sole in-flow child, which determines the hero height).
                .child(
                    gpui_kit::div()
                        .ml(px(280.))
                        .flex()
                        .flex_col()
                        // Title.
                        .child(
                            gpui_kit::div()
                                .text_3xl()
                                .font_bold()
                                .text_color(theme.foreground)
                                .line_clamp(2)
                                .child(name.to_string()),
                        )
                        // Broadcast-info row: weekday badge + start time + subtitle-group count.
                        .child(
                            gpui_kit::div()
                                .mt(px(10.))
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .children(
                                    broadcast_day
                                        .map(|day| {
                                            gpui_kit::div()
                                                .px(px(10.))
                                                .py(px(3.))
                                                .rounded_full()
                                                .bg(theme.primary)
                                                .text_xs()
                                                .font_semibold()
                                                .text_color(theme.primary_foreground)
                                                .child(day.to_string())
                                                .into_any_element()
                                        })
                                        .into_iter()
                                        .chain(broadcast_start.map(|start| {
                                            gpui_kit::div()
                                                .text_sm()
                                                .text_color(theme.muted_foreground)
                                                .child(format!(
                                                    "{} 起放送",
                                                    format_broadcast_start(start)
                                                ))
                                                .into_any_element()
                                        }))
                                        .chain(std::iter::once(
                                            gpui_kit::div()
                                                .text_sm()
                                                .text_color(theme.muted_foreground)
                                                .child(format!("{group_count} 个字幕组"))
                                                .into_any_element(),
                                        )),
                                ),
                        )
                        // Link row: official website / Bangumi.
                        .child(link_row)
                        // Summary heading.
                        .child(
                            gpui_kit::div()
                                .mt(px(24.))
                                .text_base()
                                .font_semibold()
                                .text_color(theme.foreground)
                                .child("概况"),
                        )
                        // Summary.
                        .child(
                            gpui_kit::div()
                                .mt(px(8.))
                                .text_sm()
                                .text_color(theme.foreground)
                                .line_clamp(5)
                                .child(
                                    summary
                                        .map(|s| s.replace("\r\n", "\n"))
                                        .unwrap_or_else(|| "暂无概况".to_string()),
                                ),
                        ),
                ),
        )
}
