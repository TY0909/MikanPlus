//! Shared view building blocks: the scrolling page, loading / error views, and the
//! unsubscribe / filter modals.

use crate::prelude::*;

use crate::data::model::{UnsubscribeConfirmation, UnsubscribeWarning};
use crate::shell::state::GoBackCallback;

pub(crate) fn scroll_page(page: impl IntoElement, handle: &ScrollHandle) -> gpui_kit::AnyElement {
    ui::theme::layout::page_scroll(ui::theme::layout::MAX_PAGE_W, handle, page).into_any_element()
}

/// Centered floating warning window: tells the user the subscription has episodes downloading
/// and the unsubscribe was rejected.
///
/// Click the backdrop or the dismiss button to close; Esc (CloseFilterModal) is handled globally
/// by the caller.
pub(crate) fn render_unsubscribe_warning(
    theme: &gpui_kit::component::theme::Theme,
    warning: &UnsubscribeWarning,
    on_close: GoBackCallback,
) -> impl IntoElement {
    // Show at most 5 entries in the list; fold the rest into an "and N more" line
    const MAX_LISTED: usize = 5;
    let total = warning.active_titles.len();
    let listed: Vec<String> = warning
        .active_titles
        .iter()
        .take(MAX_LISTED)
        .cloned()
        .collect();
    let more = total.saturating_sub(MAX_LISTED);

    gpui_kit::div()
        .id("unsubscribe-warning-modal")
        .absolute()
        .inset_0()
        .bg(gpui_kit::hsla(0., 0., 0., 0.4))
        .flex()
        .items_center()
        .justify_center()
        .on_click({
            let on_close = on_close.clone();
            move |_, window, app| on_close(window, app)
        })
        .child(
            gpui_kit::div()
                .id("unsubscribe-warning-card")
                .w(px(440.))
                .rounded(px(12.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.background)
                .shadow_lg()
                .p(px(20.))
                .flex()
                .flex_col()
                .on_click(move |_, _, app: &mut App| {
                    app.stop_propagation();
                })
                .child(
                    gpui_kit::div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(ui::theme::icons::icon("info", 16.).text_color(theme.warning))
                        .child(
                            gpui_kit::div()
                                .text_base()
                                .font_semibold()
                                .text_color(theme.foreground)
                                .child("无法取消订阅"),
                        ),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(8.))
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(format!(
                            "「{} - {}」有 {total} 个剧集正在下载,请先等待下载完成或取消下载后再退订。",
                            warning.bangumi_name, warning.group_name
                        )),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(12.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .px(px(12.))
                        .py(px(8.))
                        .flex()
                        .flex_col()
                        .children(listed.iter().map(|t| {
                            gpui_kit::div()
                                .w_full()
                                .py(px(2.))
                                .text_sm()
                                .text_color(theme.foreground)
                                .truncate()
                                .child(t.clone())
                        }))
                        .when(more > 0, |this| {
                            this.child(
                                gpui_kit::div()
                                    .py(px(2.))
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(format!("…等 {more} 个")),
                            )
                        }),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(18.))
                        .flex()
                        .justify_end()
                        .child(
                            gpui_kit::div()
                                .id("unsubscribe-warning-ok")
                                .px(px(14.))
                                .py(px(6.))
                                .rounded(px(6.))
                                .text_sm()
                                .font_semibold()
                                .bg(theme.primary)
                                .text_color(theme.primary_foreground)
                                .cursor_pointer()
                                .hover(|style| style.bg(theme.primary_hover))
                                .on_click({
                                    let on_close = on_close.clone();
                                    move |_, window, app| on_close(window, app)
                                })
                                .child("知道了"),
                        ),
                ),
        )
}

/// Centered floating confirmation window: confirms the unsubscribe and explicitly asks whether to
/// delete the subtitle group's download directory and files.
pub(crate) fn render_unsubscribe_confirmation(
    theme: &gpui_kit::component::theme::Theme,
    confirmation: &UnsubscribeConfirmation,
    on_cancel: GoBackCallback,
    on_confirm: UnsubscribeConfirmCallback,
    on_toggle_remove: UnsubscribeConfirmCallback,
) -> impl IntoElement {
    let remove_downloads = confirmation.remove_downloads;
    let confirm_label = if remove_downloads {
        "退订并移除"
    } else {
        "确认退订"
    };

    gpui_kit::div()
        .id("unsubscribe-confirmation-modal")
        .absolute()
        .inset_0()
        .bg(gpui_kit::hsla(0., 0., 0., 0.4))
        .flex()
        .items_center()
        .justify_center()
        .on_click({
            let on_cancel = on_cancel.clone();
            move |_, window, app| on_cancel(window, app)
        })
        .child(
            gpui_kit::div()
                .id("unsubscribe-confirmation-card")
                .w(px(460.))
                .rounded(px(12.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.background)
                .shadow_lg()
                .p(px(20.))
                .flex()
                .flex_col()
                .on_click(move |_, _, app: &mut App| {
                    app.stop_propagation();
                })
                .child(
                    gpui_kit::div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(ui::theme::icons::icon("info", 16.).text_color(theme.warning))
                        .child(
                            gpui_kit::div()
                                .text_base()
                                .font_semibold()
                                .text_color(theme.foreground)
                                .child("确认取消订阅"),
                        ),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(10.))
                        .text_sm()
                        .text_color(theme.foreground)
                        .child(format!(
                            "确定要取消「{} - {}」的订阅吗?",
                            confirmation.bangumi_name, confirmation.group_name
                        )),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(14.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .px(px(12.))
                        .py(px(10.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .child(
                            gpui_kit::div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    gpui_kit::div()
                                        .text_sm()
                                        .text_color(theme.foreground)
                                        .child("同时移除下载目录和文件"),
                                )
                                .child(
                                    gpui_kit::div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(if remove_downloads {
                                            "已选择移除该字幕组的下载内容"
                                        } else {
                                            "默认保留下载任务和已下载内容"
                                        }),
                                ),
                        )
                        .child(
                            Switch::new("unsubscribe-remove-downloads")
                                .checked(remove_downloads)
                                .on_click(move |checked, window, app| {
                                    on_toggle_remove(*checked, window, app);
                                }),
                        ),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(18.))
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            gpui_kit::div()
                                .id("unsubscribe-confirmation-cancel")
                                .px(px(14.))
                                .py(px(6.))
                                .rounded(px(6.))
                                .border_1()
                                .border_color(theme.border)
                                .text_sm()
                                .text_color(theme.foreground)
                                .cursor_pointer()
                                .hover(|style| style.bg(theme.list_hover))
                                .on_click({
                                    let on_cancel = on_cancel.clone();
                                    move |_, window, app| on_cancel(window, app)
                                })
                                .child("取消"),
                        )
                        .child(
                            gpui_kit::div()
                                .id("unsubscribe-confirmation-confirm")
                                .px(px(14.))
                                .py(px(6.))
                                .rounded(px(6.))
                                .text_sm()
                                .font_semibold()
                                .text_color(if remove_downloads {
                                    theme.danger_foreground
                                } else {
                                    theme.primary_foreground
                                })
                                .cursor_pointer()
                                .when(remove_downloads, |this| {
                                    this.bg(theme.danger)
                                        .hover(|style| style.bg(theme.danger_hover))
                                })
                                .when(!remove_downloads, |this| {
                                    this.bg(theme.primary)
                                        .hover(|style| style.bg(theme.primary_hover))
                                })
                                .on_click(move |_, window, app| {
                                    on_confirm(remove_downloads, window, app);
                                })
                                .child(confirm_label),
                        ),
                ),
        )
}

/// Centered floating filter window: translucent backdrop + centered card
/// (title / description / input box / button row).
///
/// Click the backdrop to cancel; clicks inside the card don't bubble; Esc (CloseFilterModal)
/// and Enter (Input PressEnter) are handled globally by the caller.
pub(crate) fn render_filter_modal(
    theme: &gpui_kit::component::theme::Theme,
    input: Entity<InputState>,
    keyword: &str,
    on_clear: GoBackCallback,
    on_cancel: GoBackCallback,
    on_confirm: GoBackCallback,
) -> impl IntoElement {
    let has_keyword = !keyword.is_empty();

    let btn = |label: &str, primary: bool, cb: GoBackCallback| {
        gpui_kit::div()
            .px(px(14.))
            .py(px(6.))
            .rounded(px(6.))
            .text_sm()
            .font_semibold()
            .cursor_pointer()
            .id(gpui_kit::SharedString::from(format!("filter-btn-{label}")))
            .when(primary, |this| {
                this.bg(theme.primary)
                    .text_color(theme.primary_foreground)
                    .hover(|style| style.bg(theme.primary_hover))
            })
            .when(!primary, |this| {
                this.border_1()
                    .border_color(theme.border)
                    .text_color(theme.foreground)
                    .hover(|style| style.bg(theme.list_hover))
            })
            .on_click(move |_, window, app| cb(window, app))
            .child(label.to_string())
    };

    gpui_kit::div()
        .id("filter-modal")
        .absolute()
        .inset_0()
        .bg(gpui_kit::hsla(0., 0., 0., 0.4))
        .flex()
        .items_center()
        .justify_center()
        .on_click({
            let on_cancel = on_cancel.clone();
            move |_, window, app| on_cancel(window, app)
        })
        .child(
            gpui_kit::div()
                .id("filter-card")
                .w(px(380.))
                .rounded(px(12.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.background)
                .shadow_lg()
                .p(px(20.))
                .flex()
                .flex_col()
                .on_click(move |_, _, app: &mut App| {
                    app.stop_propagation();
                })
                .child(
                    gpui_kit::div()
                        .text_base()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .child("筛选剧集"),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(6.))
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("仅显示标题包含以下关键词的剧集,留空则显示全部"),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(14.))
                        .child(Input::new(&input).w_full()),
                )
                .child(
                    gpui_kit::div()
                        .mt(px(18.))
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .when(has_keyword, |this| {
                            this.child(btn("清除筛选", false, on_clear))
                        })
                        .child(btn("取消", false, on_cancel.clone()))
                        .child(btn("确定", true, on_confirm)),
                ),
        )
}

/// Loading view: a slowly rotating loading icon + hint text.
///
/// gpui-component's spinner has a fixed 0.8s per rotation and isn't configurable, so this
/// redraws a 1.6s-per-rotation version following its internal implementation (only the period
/// changes; behavior is otherwise identical), making the wait feel calmer.
pub(crate) fn loading_view(theme: &gpui_kit::component::theme::Theme) -> gpui_kit::Div {
    gpui_kit::div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .flex_col()
        .child(
            // Circular container + slowly rotating loading icon: more layered than a bare spinner
            // (the rotating icon is shared with the subtitle-group loading state via ui::components::spinner, for consistency)
            gpui_kit::div()
                .size(px(64.))
                .rounded_full()
                .border_1()
                .border_color(theme.border)
                .flex()
                .items_center()
                .justify_center()
                .child(ui::components::spinner::spinner(
                    "loading-spin",
                    30.,
                    theme.primary,
                )),
        )
        .child(
            gpui_kit::div()
                .mt(px(16.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(3.))
                .child(
                    gpui_kit::div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("正在加载…"),
                ),
        )
}

/// Load failure view: error message + retry button
pub(crate) fn error_view(
    err: &SourceError,
    retry: GoBackCallback,
    theme: &gpui_kit::component::theme::Theme,
) -> gpui_kit::Div {
    // Show only information the user can understand; don't expose low-level error details
    let message = err.user_message().to_string();
    let hint = err.user_hint().to_string();
    gpui_kit::div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(
            gpui_kit::div()
                .text_sm()
                .text_color(theme.danger)
                .child(message),
        )
        .when(!hint.is_empty(), |this| {
            this.child(
                gpui_kit::div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(hint),
            )
        })
        .child(
            gpui_kit::div()
                .px(px(14.))
                .py(px(6.))
                .rounded(px(6.))
                .bg(theme.primary)
                .text_sm()
                .text_color(theme.primary_foreground)
                .cursor_pointer()
                .id("retry-load")
                .hover(|style| style.bg(theme.primary_hover))
                .on_click(move |_, window, app| retry(window, app))
                .child("重试"),
        )
}
