//! 应用整体配色方案。
//!
//! 设计语言参考 shadcn/ui（与 gpui-component 同源），主色采用蜜柑计划的品牌橙。
//! 浅色/深色两套主题在此处统一定义，`Theme::change` 切换模式时自动应用。

use std::rc::Rc;

use gpui::{App, Hsla, SharedString, Window, hsla};
use gpui_component::theme::{Theme, ThemeConfig, ThemeConfigColors, ThemeMode};

/// 品牌橙（蜜柑计划主色）
const PRIMARY: &str = "#F97316";
const PRIMARY_HOVER: &str = "#EA580C";
const PRIMARY_ACTIVE: &str = "#C2410C";

macro_rules! colors {
    ($($field:ident: $value:expr),* $(,)?) => {{
        let mut colors = ThemeConfigColors::default();
        $(colors.$field = Some(SharedString::from($value));)*
        colors
    }};
}

/// 浅色主题配置
fn light_config() -> ThemeConfig {
    ThemeConfig {
        name: "Mikan Light".into(),
        mode: ThemeMode::Light,
        radius: Some(6),
        radius_lg: Some(8),
        shadow: Some(true),
        colors: colors! {
            // 基础
            background: "#F8FAFC",        // slate-50
            foreground: "#0F172A",        // slate-900
            border: "#E2E8F0",            // slate-200
            input: "#CBD5E1",             // slate-300
            ring: "#F9731666",            // primary @ 40%
            caret: "#F97316",
            selection: "#F9731640",       // primary @ 25%
            overlay: "#0F172A80",

            // 主色
            primary: PRIMARY,
            primary_hover: PRIMARY_HOVER,
            primary_active: PRIMARY_ACTIVE,
            primary_foreground: "#FFFFFF",

            // 次级
            secondary: "#F1F5F9",
            secondary_hover: "#E2E8F0",
            secondary_active: "#CBD5E1",
            secondary_foreground: "#0F172A",

            // 弱化
            muted: "#F1F5F9",
            muted_foreground: "#64748B",

            // 强调（菜单/列表 hover）
            accent: "#F1F5F9",
            accent_foreground: "#0F172A",

            // 浮层
            popover: "#FFFFFF",
            popover_foreground: "#0F172A",

            // 语义色
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

            // 链接
            link: "#2563EB",
            link_hover: "#1D4ED8",
            link_active: "#1E40AF",

            // 列表
            list: "#FFFFFF",
            list_hover: "#F1F5F9",
            list_active: "#F9731626",
            list_active_border: "#F9731680",
            list_even: "#F8FAFC",
            list_head: "#F8FAFC",

            // 表格
            table: "#FFFFFF",
            table_hover: "#F1F5F9",
            table_active: "#F9731626",
            table_active_border: "#F9731680",
            table_even: "#F8FAFC",
            table_head: "#F8FAFC",
            table_head_foreground: "#64748B",
            table_row_border: "#E2E8F0",

            // 滚动条
            scrollbar: "#F1F5F9",
            scrollbar_thumb: "#CBD5E1",
            scrollbar_thumb_hover: "#94A3B8",

            // 其余组件
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
            accordion_hover: "#F1F5F9",
            window_border: "#E2E8F0",
            drag_border: "#F9731680",
            drop_target: "#F9731633",
        },
        ..Default::default()
    }
}

/// 深色主题配置
fn dark_config() -> ThemeConfig {
    ThemeConfig {
        name: "Mikan Dark".into(),
        mode: ThemeMode::Dark,
        radius: Some(6),
        radius_lg: Some(8),
        shadow: Some(true),
        colors: colors! {
            // 基础
            background: "#0F1114",
            foreground: "#E5E7EB",
            border: "#262A30",
            input: "#343A43",
            ring: "#F9731673",            // primary @ 45%
            caret: "#FB923C",
            selection: "#F973164D",       // primary @ 30%
            overlay: "#00000099",

            // 主色（深色下 hover 变亮）
            primary: PRIMARY,
            primary_hover: "#FB923C",
            primary_active: PRIMARY_HOVER,
            primary_foreground: "#FFFFFF",

            // 次级
            secondary: "#202329",
            secondary_hover: "#2A2E36",
            secondary_active: "#343943",
            secondary_foreground: "#E5E7EB",

            // 弱化
            muted: "#1A1D23",
            muted_foreground: "#9AA2AD",

            // 强调
            accent: "#202329",
            accent_foreground: "#E5E7EB",

            // 浮层
            popover: "#171A20",
            popover_foreground: "#E5E7EB",

            // 语义色
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

            // 链接（深色下用更亮的蓝）
            link: "#60A5FA",
            link_hover: "#93C5FD",
            link_active: "#3B82F6",

            // 列表
            list: "#14161B",
            list_hover: "#1F232B",
            list_active: "#F9731626",
            list_active_border: "#F9731680",
            list_even: "#0F1114",
            list_head: "#0F1114",

            // 表格
            table: "#14161B",
            table_hover: "#1F232B",
            table_active: "#F9731626",
            table_active_border: "#F9731680",
            table_even: "#0F1114",
            table_head: "#0F1114",
            table_head_foreground: "#9AA2AD",
            table_row_border: "#262A30",

            // 滚动条
            scrollbar: "#1A1D23",
            scrollbar_thumb: "#3A404A",
            scrollbar_thumb_hover: "#4A515C",

            // 其余组件
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
            accordion_hover: "#1F232B",
            window_border: "#262A30",
            drag_border: "#F9731680",
            drop_target: "#F9731633",
        },
        ..Default::default()
    }
}

/// 安装自定义主题并应用。在 `gpui_component::init` 之后调用。
pub fn init(cx: &mut App) {
    {
        let theme = Theme::global_mut(cx);
        theme.light_theme = Rc::new(light_config());
        theme.dark_theme = Rc::new(dark_config());
    }
    // 恢复上次保存的主题模式
    let mode = match crate::data::load_theme_mode().as_deref() {
        Some("dark") => ThemeMode::Dark,
        _ => ThemeMode::Light,
    };
    Theme::change(mode, None, cx);
}

/// 切换主题模式并持久化。
pub fn set_mode(mode: ThemeMode, window: Option<&mut Window>, cx: &mut App) {
    crate::data::save_theme_mode(mode.name());
    Theme::change(mode, window, cx);
}

/// 卡片表面色（比页面背景略高一阶，产生层次感）
pub fn card(theme: &Theme) -> Hsla {
    if theme.mode.is_dark() {
        hsla(220. / 360., 0.16, 0.11, 1.0) // #171A20
    } else {
        hsla(0., 0., 1.0, 1.0) // #FFFFFF
    }
}

/// 卡片 hover 表面色
pub fn card_hover(theme: &Theme) -> Hsla {
    if theme.mode.is_dark() {
        hsla(220. / 360., 0.16, 0.14, 1.0)
    } else {
        hsla(210. / 360., 0.4, 0.98, 1.0) // #F8FAFC
    }
}

/// 卡片阴影（浅色偏深、深色偏弱）
pub fn card_shadow(theme: &Theme) -> Vec<gpui::BoxShadow> {
    if theme.mode.is_dark() {
        vec![gpui_component::box_shadow(
            0.,
            2.,
            8.,
            0.,
            hsla(0., 0., 0., 0.35),
        )]
    } else {
        vec![gpui_component::box_shadow(
            0.,
            2.,
            8.,
            0.,
            hsla(222. / 360., 0.47, 0.11, 0.08),
        )]
    }
}

/// 卡片悬停阴影（更大、更明显）
pub fn card_shadow_hover(theme: &Theme) -> Vec<gpui::BoxShadow> {
    if theme.mode.is_dark() {
        vec![gpui_component::box_shadow(
            0.,
            8.,
            24.,
            0.,
            hsla(0., 0., 0., 0.5),
        )]
    } else {
        vec![gpui_component::box_shadow(
            0.,
            10.,
            28.,
            0.,
            hsla(222. / 360., 0.47, 0.11, 0.16),
        )]
    }
}
