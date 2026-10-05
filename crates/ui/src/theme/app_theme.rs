//! Application-wide color scheme.
//!
//! The design language references shadcn/ui (the same lineage as gpui-component); the primary
//! color uses Mikan Project's brand orange. Both light and dark themes are defined together
//! here, and are applied automatically when `Theme::change` switches modes.

use std::rc::Rc;

use gpui_kit::component::theme::{Theme, ThemeConfig, ThemeConfigColors, ThemeMode};
use gpui_kit::{App, Hsla, SharedString, Window, hsla};

/// Brand orange (Mikan Project's primary color)
const PRIMARY: &str = "#F97316";
const PRIMARY_HOVER: &str = "#EA580C";
const PRIMARY_ACTIVE: &str = "#C2410C";

/// Builds a [`ThemeConfigColors`] from `field: value` pairs, leaving all other fields at their defaults.
macro_rules! colors {
    ($($field:ident: $value:expr),* $(,)?) => {{
        let mut colors = ThemeConfigColors::default();
        $(colors.$field = Some(SharedString::from($value));)*
        colors
    }};
}

/// Light theme configuration.
fn light_config() -> ThemeConfig {
    ThemeConfig {
        name: "Mikan Light".into(),
        mode: ThemeMode::Light,
        radius: Some(6),
        radius_lg: Some(8),
        shadow: Some(true),
        colors: colors! {
            // Base
            background: "#F8FAFC",        // slate-50
            foreground: "#0F172A",        // slate-900
            border: "#E2E8F0",            // slate-200
            input: "#CBD5E1",             // slate-300
            ring: "#F9731666",            // primary @ 40%
            caret: "#F97316",
            selection: "#F9731640",       // primary @ 25%
            overlay: "#0F172A80",

            // Primary
            primary: PRIMARY,
            primary_hover: PRIMARY_HOVER,
            primary_active: PRIMARY_ACTIVE,
            primary_foreground: "#FFFFFF",

            // Secondary
            secondary: "#F1F5F9",
            secondary_hover: "#E2E8F0",
            secondary_active: "#CBD5E1",
            secondary_foreground: "#0F172A",

            // Muted
            muted: "#F1F5F9",
            muted_foreground: "#64748B",

            // Accent (menu/list hover)
            accent: "#F1F5F9",
            accent_foreground: "#0F172A",

            // Surfaces / popovers
            popover: "#FFFFFF",
            popover_foreground: "#0F172A",

            // Semantic colors
            danger: "#DC2626",
            danger_hover: "#B91C1C",
            danger_active: "#991B1B",
            danger_foreground: "#FFFFFF",
            success: "#16A34A",
            success_hover: "#15803D",
            success_active: "#166534",
            success_foreground: "#FFFFFF",
            info: "#2563EB",
            info_hover: "#1D4ED8",
            info_active: "#1E40AF",
            info_foreground: "#FFFFFF",
            warning: "#D97706",
            warning_hover: "#B45309",
            warning_active: "#92400E",
            warning_foreground: "#451A03",

            // Links
            link: "#2563EB",
            link_hover: "#1D4ED8",
            link_active: "#1E40AF",

            // Lists
            list: "#FFFFFF",
            list_hover: "#F1F5F9",
            list_active: "#F9731626",
            list_active_border: "#F9731680",
            list_even: "#F8FAFC",
            list_head: "#F8FAFC",

            // Tables
            table: "#FFFFFF",
            table_hover: "#F1F5F9",
            table_active: "#F9731626",
            table_active_border: "#F9731680",
            table_even: "#F8FAFC",
            table_head: "#F8FAFC",
            table_head_foreground: "#64748B",
            table_row_border: "#E2E8F0",

            // Scrollbars
            scrollbar: "#F1F5F9",
            scrollbar_thumb: "#CBD5E1",
            scrollbar_thumb_hover: "#94A3B8",

            // Other components
            group_box: "#FFFFFF",
            group_box_foreground: "#0F172A",
            skeleton: "#E2E8F0",
            progress_bar: "#E2E8F0",
            slider_bar: "#E2E8F0",
            slider_thumb: "#FFFFFF",
            switch: "#CBD5E1",
            switch_thumb: "#FFFFFF",
            tab: "#FFFFFF",
            tab_active: "#F1F5F9",
            tab_active_foreground: "#0F172A",
            tab_bar: "#F8FAFC",
            tab_bar_segmented: "#E2E8F0",
            tab_foreground: "#0F172A",
            title_bar: "#FFFFFF",
            title_bar_border: "#E2E8F0",
            accordion: "#F8FAFC",

            window_border: "#E2E8F0",
            drag_border: "#F9731680",
            drop_target: "#F9731633",
        },
        ..Default::default()
    }
}

/// Dark theme configuration.
fn dark_config() -> ThemeConfig {
    ThemeConfig {
        name: "Mikan Dark".into(),
        mode: ThemeMode::Dark,
        radius: Some(6),
        radius_lg: Some(8),
        shadow: Some(true),
        colors: colors! {
            // Base
            background: "#0F1114",
            foreground: "#E5E7EB",
            border: "#262A30",
            input: "#343A43",
            ring: "#F9731673",            // primary @ 45%
            caret: "#FB923C",
            selection: "#F973164D",       // primary @ 30%
            overlay: "#00000099",

            // Primary (hover brightens in dark mode)
            primary: PRIMARY,
            primary_hover: "#FB923C",
            primary_active: PRIMARY_HOVER,
            primary_foreground: "#FFFFFF",

            // Secondary
            secondary: "#202329",
            secondary_hover: "#2A2E36",
            secondary_active: "#343943",
            secondary_foreground: "#E5E7EB",

            // Muted
            muted: "#1A1D23",
            muted_foreground: "#9AA2AD",

            // Accent
            accent: "#202329",
            accent_foreground: "#E5E7EB",

            // Surfaces / popovers
            popover: "#171A20",
            popover_foreground: "#E5E7EB",

            // Semantic colors
            danger: "#EF4444",
            danger_hover: "#DC2626",
            danger_active: "#B91C1B",
            danger_foreground: "#FFFFFF",
            success: "#16A34A",
            success_hover: "#15803D",
            success_active: "#166534",
            success_foreground: "#FFFFFF",
            info: "#2563EB",
            info_hover: "#1D4ED8",
            info_active: "#1E40AF",
            info_foreground: "#FFFFFF",
            warning: "#D97706",
            warning_hover: "#B45309",
            warning_active: "#92400E",
            warning_foreground: "#451A03",

            // Links (brighter blue in dark mode)
            link: "#60A5FA",
            link_hover: "#93C5FD",
            link_active: "#3B82F6",

            // Lists
            list: "#14161B",
            list_hover: "#1F232B",
            list_active: "#F9731626",
            list_active_border: "#F9731680",
            list_even: "#0F1114",
            list_head: "#0F1114",

            // Tables
            table: "#14161B",
            table_hover: "#1F232B",
            table_active: "#F9731626",
            table_active_border: "#F9731680",
            table_even: "#0F1114",
            table_head: "#0F1114",
            table_head_foreground: "#9AA2AD",
            table_row_border: "#262A30",

            // Scrollbars
            scrollbar: "#1A1D23",
            scrollbar_thumb: "#3A404A",
            scrollbar_thumb_hover: "#4A515C",

            // Other components
            group_box: "#14161B",
            group_box_foreground: "#E5E7EB",
            skeleton: "#262A30",
            progress_bar: "#262A30",
            slider_bar: "#343A43",
            slider_thumb: "#E5E7EB",
            switch: "#343A43",
            switch_thumb: "#E5E7EB",
            tab: "#14161B",
            tab_active: "#1F232B",
            tab_active_foreground: "#E5E7EB",
            tab_bar: "#0F1114",
            tab_bar_segmented: "#262A30",
            tab_foreground: "#E5E7EB",
            title_bar: "#0F1114",
            title_bar_border: "#262A30",
            accordion: "#14161B",

            window_border: "#262A30",
            drag_border: "#F9731680",
            drop_target: "#F9731633",
        },
        ..Default::default()
    }
}

/// Installs the custom theme (light and dark variants) and applies a default mode. Call after
/// `gpui_kit::component::init`; the persisted mode is applied next by the caller that owns the state.
pub fn install(cx: &mut App) {
    {
        let theme = Theme::global_mut(cx);
        theme.light_theme = Rc::new(light_config());
        theme.dark_theme = Rc::new(dark_config());
    }
    // Default mode until the persisted one is applied.
    Theme::change(ThemeMode::Light, None, cx);
}

/// Applies a theme mode. Callers persist it through the owned application state.
pub fn apply_mode(mode: ThemeMode, window: Option<&mut Window>, cx: &mut App) {
    Theme::change(mode, window, cx);
}

/// Card surface color (one step above the page background to create depth).
pub fn card(theme: &Theme) -> Hsla {
    if theme.mode.is_dark() {
        hsla(220. / 360., 0.16, 0.11, 1.0) // #171A20
    } else {
        hsla(0., 0., 1.0, 1.0) // #FFFFFF
    }
}

/// Card hover surface color.
pub fn card_hover(theme: &Theme) -> Hsla {
    if theme.mode.is_dark() {
        hsla(220. / 360., 0.16, 0.14, 1.0)
    } else {
        hsla(210. / 360., 0.4, 0.98, 1.0) // #F8FAFC
    }
}

/// Card shadow (stronger in light mode, weaker in dark mode).
pub fn card_shadow(theme: &Theme) -> Vec<gpui_kit::BoxShadow> {
    if theme.mode.is_dark() {
        vec![gpui_kit::component::box_shadow(
            0.,
            2.,
            8.,
            0.,
            hsla(0., 0., 0., 0.35),
        )]
    } else {
        vec![gpui_kit::component::box_shadow(
            0.,
            2.,
            8.,
            0.,
            hsla(222. / 360., 0.47, 0.11, 0.08),
        )]
    }
}

/// Card hover shadow (larger and more pronounced).
pub fn card_shadow_hover(theme: &Theme) -> Vec<gpui_kit::BoxShadow> {
    if theme.mode.is_dark() {
        vec![gpui_kit::component::box_shadow(
            0.,
            8.,
            24.,
            0.,
            hsla(0., 0., 0., 0.5),
        )]
    } else {
        vec![gpui_kit::component::box_shadow(
            0.,
            10.,
            28.,
            0.,
            hsla(222. / 360., 0.47, 0.11, 0.16),
        )]
    }
}
