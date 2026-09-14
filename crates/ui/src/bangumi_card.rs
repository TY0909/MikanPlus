use std::{rc::Rc, time::Duration};

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::StyledExt;
use gpui_kit::{App, Window, prelude::*, px};

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

struct BangumiCardHoverState {
    hovered: bool,
}

/// 番剧卡片:海报 + 名称 + 订阅状态。
/// 悬停时整体向上移动,并通过边框与阴影提供轻量反馈。
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let name = self.name;
        let on_click = self.on_click;
        let is_dark = cx.theme().mode.is_dark();

        // 调用方保证 key 唯一(未设置时退回名称,至少可运行)
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

        // 海报区:只圆顶部两角(底部直角,与下方文字区拼接)
        let poster_area = gpui_kit::div()
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
            ));

        // 名称区:仅显示番剧名;最多两行,超出显示省略号
        // (gpui 0.2.2 的 line_clamp 不带省略号,需配 text_ellipsis)
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
        // 名称可能被两行截断:悬停显示完整名称
        if crate::episode_row::exceeds_lines(&name, 160.0, 14.0, 2) {
            name_el = name_el.tooltip(crate::episode_row::title_tooltip(name.clone()));
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
