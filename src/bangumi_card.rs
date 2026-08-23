use std::rc::Rc;

use gpui::{App, Window, prelude::*, px};
use gpui_component::ActiveTheme;
use gpui_component::StyledExt;

use crate::app_theme;
use crate::poster::poster;

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

/// 卡片点击回调
pub type CardAction = Rc<dyn Fn(&mut Window, &mut App)>;

/// 番剧卡片:海报 + 名称 + 订阅状态。
/// 悬停时显示播放遮罩。
#[derive(IntoElement)]
pub struct BangumiCard {
    pub name: String,
    pub bangumi_id: Option<u32>,
    pub poster_url: String,
    pub format: BangumiFormat,
    /// 卡片唯一键(同一网格内不能重复——番剧名不可作 id,同名条目会串交互状态)
    pub key: String,
    pub on_click: Option<CardAction>,
}

impl BangumiCard {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bangumi_id: None,
            poster_url: String::new(),
            format: BangumiFormat::Tv,
            key: String::new(),
            on_click: None,
        }
    }
}

impl RenderOnce for BangumiCard {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let name = self.name;
        let on_click = self.on_click;
        let is_dark = cx.theme().mode.is_dark();
        let theme = cx.theme();

        // 调用方保证 key 唯一(未设置时退回名称,至少可运行)
        let card_id: gpui::SharedString = if self.key.is_empty() {
            name.clone().into()
        } else {
            self.key.into()
        };

        // 播放遮罩:默认透明,卡片悬停时淡入;圆角与海报区一致(只圆顶部)
        let play_overlay = gpui::div()
            .absolute()
            .inset_0()
            .rounded_t(px(10.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .bg(gpui::hsla(0., 0., 0., 0.55))
            .opacity(0.)
            .hover(|style| style.opacity(1.))
            .child(
                gpui::div()
                    .size(px(48.))
                    .rounded_full()
                    .bg(gpui::hsla(0., 0., 1.0, 0.95))
                    .flex()
                    .items_center()
                    .justify_center()
                    .shadow_lg()
                    .child(
                        gpui::div()
                            .text_xl()
                            .text_color(gpui::hsla(0., 0., 0.85, 0.95))
                            .pl(px(2.))
                            .child("▶"),
                    ),
            )
            .child(
                gpui::div()
                    .text_sm()
                    .font_semibold()
                    .text_color(gpui::hsla(0., 0., 1.0, 0.95))
                    .child("查看详情"),
            );

        let mut card = gpui::div()
            .w(px(180.))
            .bg(app_theme::card(theme))
            .rounded(px(10.))
            .overflow_hidden()
            .border_1()
            .border_color(theme.border)
            .id(card_id)
            .flex()
            .flex_col()
            .hover(|style| {
                style
                    .bg(app_theme::card_hover(theme))
                    .border_color(theme.primary)
                    .shadow(app_theme::card_shadow_hover(theme))
            });

        if let Some(cb) = on_click {
            card = card.cursor_pointer().on_click(move |_, window, app| {
                cb(window, app);
            });
        }

        // 海报区:只圆顶部两角(底部直角,与下方文字区拼接),溢出裁剪保证悬停遮罩不超出圆角
        let poster_area = gpui::div()
            .h(px(252.))
            .w_full()
            .relative()
            .rounded_t(px(10.))
            .overflow_hidden()
            .child(poster(
                &self.poster_url,
                &name,
                is_dark,
                px(180.),
                px(252.),
                10.,
                crate::poster::CornerStyle::Top,
            ))
            .child(
                // 左上角:格式徽章
                gpui::div()
                    .absolute()
                    .top(px(8.))
                    .left(px(8.))
                    .px(px(7.))
                    .py(px(2.))
                    .rounded_full()
                    .bg(gpui::hsla(0., 0., 0., 0.45))
                    .text_xs()
                    .text_color(gpui::hsla(0., 0., 1.0, 0.95))
                    .child(self.format.label().to_string()),
            )
            .child(play_overlay);

        // 名称区:仅显示番剧名;最多两行,超出显示省略号
        // (gpui 0.2.2 的 line_clamp 不带省略号,需配 text_ellipsis)
        let mut name_el = gpui::div()
            .id(gpui::SharedString::from(format!(
                "bangumi-card-name-{name}"
            )))
            .text_sm()
            .font_semibold()
            .text_color(theme.foreground)
            .w_full()
            .line_clamp(2)
            .text_ellipsis();
        // 名称可能被两行截断:悬停显示完整名称
        if crate::episode_row::exceeds_lines(&name, 160.0, 14.0, 2) {
            name_el = name_el.tooltip(crate::episode_row::title_tooltip(name.clone()));
        }
        name_el = name_el.child(name.clone());

        let label = gpui::div()
            .px(px(10.))
            .py(px(10.))
            .w_full()
            .flex()
            .flex_col()
            .child(name_el);

        card.child(poster_area).child(label)
    }
}
