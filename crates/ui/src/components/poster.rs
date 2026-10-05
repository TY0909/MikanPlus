//! Poster cover component.
//!
//! `poster()` renders in three tiers depending on the data source:
//! - Local file path → show the image directly
//! - Remote URL (http/https) → **viewport lazy loading**: download only when it
//!   first enters the visible area, showing a gradient placeholder until then;
//!   a cached image is displayed directly (each image is requested only once
//!   over its lifetime)
//! - Anything else (empty/invalid) → a gradient placeholder cover generated from
//!   a hash of the name

use std::sync::Arc;

use gpui_kit::component::StyledExt;
use gpui_kit::{hsla, prelude::*, px};
use source::Network;

/// Image corner rounding style (the image rounds its own corners to match the
/// corresponding edges of its container).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CornerStyle {
    /// All four corners rounded.
    All,
    /// Only the top two corners rounded (square bottom, for joining with content below).
    Top,
    /// Only the bottom two corners rounded.
    Bottom,
    /// Only the left two corners rounded (square right, for joining with content to the right).
    Left,
    /// Only the right two corners rounded.
    Right,
}

/// Apply edge/corner rounding to an element according to the corner style.
fn with_corners<T: gpui_kit::Styled>(el: T, corners: CornerStyle, r: f32) -> T {
    match corners {
        CornerStyle::All => el.rounded(px(r)),
        CornerStyle::Top => el.rounded_t(px(r)),
        CornerStyle::Bottom => el.rounded_b(px(r)),
        CornerStyle::Left => el.rounded_l(px(r)),
        CornerStyle::Right => el.rounded_r(px(r)),
    }
}

/// A set of harmonious poster color pairs (hue pairs).
const PALETTE: &[(f32, f32)] = &[
    (340.0, 260.0), // rose → purple
    (265.0, 320.0), // purple → pink
    (210.0, 265.0), // blue → purple
    (200.0, 160.0), // cyan → teal
    (165.0, 120.0), // green → yellow-green
    (35.0, 10.0),   // amber → orange-red
    (0.0, 40.0),    // red → orange
    (250.0, 200.0), // indigo → cyan
    (330.0, 290.0), // magenta → indigo-purple
    (150.0, 190.0), // teal → blue
];

/// Compute a stable poster color pair (two hue values) from the name.
pub fn poster_hues(name: &str) -> (f32, f32) {
    let mut h: u64 = 5381;
    for b in name.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    let (a, b) = PALETTE[(h as usize) % PALETTE.len()];
    // Nudge the hue based on the hash to avoid exact color collisions within the same pair.
    let drift = ((h >> 8) % 24) as f32 - 12.0;
    (a + drift, b + drift)
}

/// Render a poster cover.
///
/// The image fills via Cover (when the `rounded` radius matches the container,
/// the drawn area equals the displayed area and the corners are exact);
/// `corners` controls the rounding direction so the poster can join with other
/// parts of a card.
#[allow(clippy::too_many_arguments)]
pub fn poster(
    network: &Arc<Network>,
    poster_path: &str,
    name: &str,
    is_dark: bool,
    width: gpui_kit::Pixels,
    height: gpui_kit::Pixels,
    rounded: f32,
    corners: CornerStyle,
) -> gpui_kit::AnyElement {
    let path = std::path::PathBuf::from(poster_path);
    if path.exists() {
        // Local asset (cached file / bundled resource)
        with_corners(gpui_kit::img(path), corners, rounded)
            .w(width)
            .h(height)
            .object_fit(gpui_kit::ObjectFit::Cover)
            .into_any_element()
    } else {
        // Normalize the remote URL to the current data-source host first: historic
        // caches / subscriptions may hold absolute addresses with the old domain,
        // and requests to the old address would fail after switching to a backup
        // domain.
        let url = if poster_path.starts_with("http://") || poster_path.starts_with("https://") {
            network.normalize_url(poster_path)
        } else {
            poster_path.to_string()
        };
        if url.starts_with("http://") || url.starts_with("https://") {
            lazy_cover(
                network, &url, name, is_dark, width, height, rounded, corners,
            )
        } else {
            placeholder(name, is_dark, width, height, rounded, corners).into_any_element()
        }
    }
}

/// Uniformly request a portrait WebP crop (400×560).
///
/// Matches the old HTML cover (`?format=webp&width=400&height=560`): it both makes
/// the server return a smaller WebP and keeps the URL query string identical to the
/// old version. The image cache key is "path + query string", so covers cached by
/// the old version are reused directly and are not all re-downloaded when the data
/// source changes.
fn cover_url(url: &str) -> String {
    let base = url.split_once('?').map(|(base, _)| base).unwrap_or(url);
    format!("{base}?format=webp&width=400&height=560")
}

/// Remote cover: show directly if cached; otherwise show a placeholder and only
/// download once it first enters the viewport. Downloads/caching use the portrait
/// poster URL (400×560), matching the card aspect ratio with exact corners.
#[allow(clippy::too_many_arguments)]
fn lazy_cover(
    network: &Arc<Network>,
    url: &str,
    name: &str,
    is_dark: bool,
    width: gpui_kit::Pixels,
    height: gpui_kit::Pixels,
    rounded: f32,
    corners: CornerStyle,
) -> gpui_kit::AnyElement {
    let fetch_url = cover_url(url);
    // Already cached: show directly.
    if let Some(cache_path) = storage::cache::cached_image(&fetch_url) {
        return with_corners(gpui_kit::img(cache_path), corners, rounded)
            .w(width)
            .h(height)
            .object_fit(gpui_kit::ObjectFit::Cover)
            .into_any_element();
    }

    let url = fetch_url;
    let name = name.to_string();
    // The canvas callback must be `'static`, so hold an owned handle instead of the borrow.
    let network = network.clone();

    gpui_kit::div()
        .w(width)
        .h(height)
        .relative()
        .overflow_hidden()
        .when(corners == CornerStyle::Top, |this| {
            this.rounded_t(px(rounded))
        })
        .when(corners == CornerStyle::All, |this| {
            this.rounded(px(rounded))
        })
        .when(corners == CornerStyle::Left, |this| {
            this.rounded_l(px(rounded))
        })
        .when(corners == CornerStyle::Right, |this| {
            this.rounded_r(px(rounded))
        })
        .when(corners == CornerStyle::Bottom, |this| {
            this.rounded_b(px(rounded))
        })
        // Placeholder
        .child(placeholder_inner(
            &name,
            is_dark,
            rounded,
            corners,
            Some((width, height)),
        ))
        // Viewport detection: on every frame paint, check whether the element has
        // entered the visible area; on first visibility with no completed or
        // in-flight download → start a background download (deduplication is
        // guaranteed by claim_image)
        .child(
            gpui_kit::canvas(
                move |_bounds, _window, _app| {},
                move |bounds, _prepaint, window, _app| {
                    // Trigger the download only when it first enters the window's visible area.
                    let visible = window.bounds();
                    if bounds.intersects(&visible) && network.claim_image(&url) {
                        let url = url.clone();
                        let network = network.clone();
                        std::thread::spawn(move || {
                            // Prefer an available host and fall back on failure (see
                            // fetch_image_bytes); the cache key is host-independent, so it
                            // is always stored under the original URL.
                            let ok = network
                                .fetch_image_bytes(&url)
                                .and_then(|bytes| {
                                    storage::cache::store_image(&url, &bytes)
                                        .map(|_| ())
                                        .ok_or(source::SourceError::Cache)
                                })
                                .is_ok();
                            network.finish_image(&url, ok);
                        });
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .into_any_element()
}

/// Gradient placeholder cover generated from a hash of the name (fixed size).
fn placeholder(
    name: &str,
    is_dark: bool,
    width: gpui_kit::Pixels,
    height: gpui_kit::Pixels,
    rounded: f32,
    corners: CornerStyle,
) -> impl IntoElement {
    placeholder_inner(name, is_dark, rounded, corners, Some((width, height)))
}

/// Gradient placeholder cover (fixed size or filling the parent).
fn placeholder_inner(
    name: &str,
    is_dark: bool,
    rounded: f32,
    corners: CornerStyle,
    fixed: Option<(gpui_kit::Pixels, gpui_kit::Pixels)>,
) -> impl IntoElement {
    let (hue_a, hue_b) = poster_hues(name);
    // In dark mode, lower the saturation and lightness slightly so it harmonizes with the dark background.
    let (s, l) = if is_dark { (0.50, 0.28) } else { (0.55, 0.42) };
    let base = hsla(hue_a / 360.0, s, l, 1.0);
    let glow = hsla(hue_b / 360.0, s, l + 0.12, 0.9);
    let shade = hsla(hue_a / 360.0, s, l - 0.25, 0.6);

    let first_char: String = name.chars().next().unwrap_or('?').to_string();

    let mut el = gpui_kit::div()
        .when(corners == CornerStyle::Top, |this| {
            this.rounded_t(px(rounded))
        })
        .when(corners == CornerStyle::All, |this| {
            this.rounded(px(rounded))
        })
        .when(corners == CornerStyle::Left, |this| {
            this.rounded_l(px(rounded))
        })
        .when(corners == CornerStyle::Right, |this| {
            this.rounded_r(px(rounded))
        })
        .when(corners == CornerStyle::Bottom, |this| {
            this.rounded_b(px(rounded))
        })
        .overflow_hidden()
        .relative()
        .bg(base);
    el = match fixed {
        Some((w, h)) => el.w(w).h(h),
        None => el.size_full(),
    };

    el.child(
        // Decorative circle in the top-right corner
        gpui_kit::div()
            .absolute()
            .top_0()
            .right_0()
            .w_3_4()
            .h_3_4()
            .rounded_full()
            .bg(glow)
            .opacity(0.45),
    )
    .child(
        // Darken the bottom to provide contrast for the text
        gpui_kit::div()
            .absolute()
            .bottom_0()
            .left_0()
            .w_full()
            .h_1_2()
            .bg(shade),
    )
    .child(
        gpui_kit::div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .child(
                gpui_kit::div()
                    .text_color(hsla(0., 0., 1.0, 0.9))
                    .text_3xl()
                    .font_semibold()
                    .child(first_char),
            ),
    )
    .into_any_element()
}
