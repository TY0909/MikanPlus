//! 海报封面组件。
//!
//! `poster()` 按数据来源分三档渲染:
//! - 本地文件路径 → 直接显示图片
//! - 远程 URL(http/https)→ **视口懒加载**:首次进入可视区域才下载,
//!   下载前显示渐变占位;已缓存则直接显示(每张图全生命周期只请求一次)
//! - 其他(空/无效)→ 按名称哈希生成的渐变占位封面

use gpui_kit::component::StyledExt;
use gpui_kit::{hsla, prelude::*, px};

/// 图片圆角方式(图片自身圆角,与所在容器的对应边一致)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CornerStyle {
    /// 四角全圆
    All,
    /// 只圆顶部两角(底部直角,便于与下方内容拼接)
    Top,
    /// 只圆底部两角
    Bottom,
    /// 只圆左侧两角(右侧直角,便于与右侧内容拼接)
    Left,
    /// 只圆右侧两角
    Right,
}

/// 按圆角方式给元素设置单边/单角圆角
fn with_corners<T: gpui_kit::Styled>(el: T, corners: CornerStyle, r: f32) -> T {
    match corners {
        CornerStyle::All => el.rounded(px(r)),
        CornerStyle::Top => el.rounded_t(px(r)),
        CornerStyle::Bottom => el.rounded_b(px(r)),
        CornerStyle::Left => el.rounded_l(px(r)),
        CornerStyle::Right => el.rounded_r(px(r)),
    }
}

/// 一组和谐的海报配色(hue 对)
const PALETTE: &[(f32, f32)] = &[
    (340.0, 260.0), // 玫红 → 紫
    (265.0, 320.0), // 紫 → 粉
    (210.0, 265.0), // 蓝 → 紫
    (200.0, 160.0), // 青 → 蓝绿
    (165.0, 120.0), // 绿 → 黄绿
    (35.0, 10.0),   // 琥珀 → 橙红
    (0.0, 40.0),    // 红 → 橙
    (250.0, 200.0), // 靛 → 青
    (330.0, 290.0), // 洋红 → 靛紫
    (150.0, 190.0), // 青绿 → 蓝
];

/// 由名称计算稳定的海报配色(两个 hue 值)
pub fn poster_hues(name: &str) -> (f32, f32) {
    let mut h: u64 = 5381;
    for b in name.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    let (a, b) = PALETTE[(h as usize) % PALETTE.len()];
    // 依据哈希微调色相,避免同一组内完全撞色
    let drift = ((h >> 8) % 24) as f32 - 12.0;
    (a + drift, b + drift)
}

/// 渲染一个海报封面。
///
/// 图片以 Cover 填满(`rounded` 半径与容器一致时绘制区域==显示区域,
/// 圆角精确);`corners` 控制圆角方向,便于与卡片其他部分拼接。
pub fn poster(
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
        // 本地资源(缓存文件 / 打包资源)
        with_corners(gpui_kit::img(path), corners, rounded)
            .w(width)
            .h(height)
            .object_fit(gpui_kit::ObjectFit::Cover)
            .into_any_element()
    } else if poster_path.starts_with("http://") || poster_path.starts_with("https://") {
        // 远程封面:视口懒加载
        lazy_cover(poster_path, name, is_dark, width, height, rounded, corners)
    } else {
        // 无效来源:渐变占位
        placeholder(name, is_dark, width, height, rounded, corners).into_any_element()
    }
}

/// 把封面 URL 的尺寸参数替换为竖版海报尺寸(服务器端裁剪,本地不改图)
fn cover_url(url: &str) -> String {
    let (base, query) = url.split_once('?').unwrap_or((url, ""));
    let kept: Vec<&str> = query
        .split('&')
        .filter(|p| !p.starts_with("width=") && !p.starts_with("height="))
        .collect();
    let mut parts = kept;
    parts.push("width=400");
    parts.push("height=560");
    format!("{base}?{}", parts.join("&"))
}

/// 远程封面:已缓存直接显示;未缓存显示占位,首次进入视口才下载。
/// 下载/缓存使用竖版海报 URL(400×560),与卡片比例一致,圆角精确。
fn lazy_cover(
    url: &str,
    name: &str,
    is_dark: bool,
    width: gpui_kit::Pixels,
    height: gpui_kit::Pixels,
    rounded: f32,
    corners: CornerStyle,
) -> gpui_kit::AnyElement {
    let fetch_url = cover_url(url);
    // 已缓存:直接显示
    if let Some(cache_path) = storage::cache::cached_image(&fetch_url) {
        return with_corners(gpui_kit::img(cache_path), corners, rounded)
            .w(width)
            .h(height)
            .object_fit(gpui_kit::ObjectFit::Cover)
            .into_any_element();
    }

    let url = fetch_url;
    let name = name.to_string();

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
        // 占位
        .child(placeholder_inner(
            &name,
            is_dark,
            rounded,
            corners,
            Some((width, height)),
        ))
        // 视口检测:每帧 paint 时判断元素是否进入可见区域,
        // 首次可见且未下载/未在途 → 启动后台下载(去重由 claim_image 保证)
        .child(
            gpui_kit::canvas(
                move |_bounds, _window, _app| {},
                move |bounds, _prepaint, window, _app| {
                    // 首次进入窗口可见区域才触发下载
                    let visible = window.bounds();
                    if bounds.intersects(&visible) && source::network::claim_image(&url) {
                        let url = url.clone();
                        std::thread::spawn(move || {
                            let ok = source::network::fetch_bytes(&url)
                                .and_then(|bytes| {
                                    storage::cache::store_image(&url, &bytes)
                                        .map(|_| ())
                                        .ok_or_else(|| "写入缓存失败".to_string())
                                })
                                .is_ok();
                            source::network::finish_image(&url, ok);
                        });
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .into_any_element()
}

/// 按名称哈希生成的渐变占位封面(固定尺寸)
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

/// 渐变占位封面(固定尺寸或铺满父容器)
fn placeholder_inner(
    name: &str,
    is_dark: bool,
    rounded: f32,
    corners: CornerStyle,
    fixed: Option<(gpui_kit::Pixels, gpui_kit::Pixels)>,
) -> impl IntoElement {
    let (hue_a, hue_b) = poster_hues(name);
    // 深色模式下饱和度、明度略微调低,保证与暗色背景协调
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
        // 右上角装饰圆
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
        // 底部压暗,为文字提供对比
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
