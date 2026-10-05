//! Verifies the 60/40 flex layout of an episode row (gpui 0.2.2).
//!
//! Design requirement: the text column is fixed at 60% (overlong text is
//! truncated with an ellipsis) and the action column is fixed at 40% (the
//! download area is right-aligned).

use gpui_kit::component::StyledExt;
use gpui_kit::{Context, Render, Size, VisualTestContext, Window, div, prelude::*, px, relative};

/// Renders a realistic card: a header row plus a single episode row.
struct RowFixture {
    title: String,
    keyword: String,
}

impl Render for RowFixture {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Mimics the real structure: a card container (header row + episode row).
        div()
            .id("card")
            .debug_selector(|| "card".to_string())
            .w_full()
            .rounded(px(10.))
            .overflow_hidden()
            .border_1()
            .child(
                // Header row: "剧集列表" plus the filter button, spread to both ends.
                div()
                    .id("list-header")
                    .debug_selector(|| "list-header".to_string())
                    .w_full()
                    .px(px(14.))
                    .py(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_sm()
                            .font_semibold()
                            .child("剧集列表"),
                    )
                    .child(
                        div()
                            .id("filter-btn")
                            .debug_selector(|| "filter-btn".to_string())
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .px(px(10.))
                            .py(px(4.))
                            .rounded(px(6.))
                            .border_1()
                            .text_xs()
                            .font_semibold()
                            .child(
                                div()
                                    .max_w(px(180.))
                                    .truncate()
                                    .child(format!("筛选 · {}", self.keyword)),
                            ),
                    ),
            )
            .child(
                // Episode row: 60% text column + 40% action column.
                div()
                    .id("row-root")
                    .debug_selector(|| "row-root".to_string())
                    .w_full()
                    .px(px(14.))
                    .py(px(14.))
                    .flex()
                    .flex_row()
                    .items_center()
                    .child(
                        div()
                            .id("title-col")
                            .debug_selector(|| "title-col".to_string())
                            .w(relative(0.6))
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .id("title")
                                    .debug_selector(|| "title".to_string())
                                    .w_full()
                                    .text_sm()
                                    .truncate()
                                    .child(
                                        div()
                                            .id("title-inner")
                                            .debug_selector(|| "title-inner".to_string())
                                            .child(self.title.clone()),
                                    ),
                            )
                            .child(div().text_xs().child("1.2 GB · 2026/08/23")),
                    )
                    .child(
                        div()
                            .id("btn-col")
                            .debug_selector(|| "btn-col".to_string())
                            .w(relative(0.4))
                            .overflow_hidden()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_end()
                            .child(
                                // Simulates the downloading state: percent + two speeds + cancel.
                                div()
                                    .id("btn-area")
                                    .debug_selector(|| "btn-area".to_string())
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(div().w(px(68.)).text_xs().truncate().child("45.2%"))
                                    .child(div().text_xs().child("↓ 1.2 MB/s"))
                                    .child(div().text_xs().child("↑ 0 B/s"))
                                    .child(div().text_xs().child("取消")),
                            ),
                    ),
            )
    }
}

/// Opens a fixture window of the given width and returns its visual test context.
fn setup(
    cx: &mut gpui_kit::TestAppContext,
    width: f32,
    title: &str,
    keyword: &str,
) -> VisualTestContext {
    let title = title.to_string();
    let keyword = keyword.to_string();
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|_cx| RowFixture { title, keyword })
        })
        .unwrap()
    });
    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(Size {
        width: px(width),
        height: px(400.),
    });
    cx.run_until_parked();
    cx
}

/// Asserts the 60/40 split, no overflow, and right-alignment of the download area.
fn check(cx: &mut VisualTestContext, label: &str) {
    let root = cx.debug_bounds("row-root").expect("row-root laid out");
    let title_col = cx.debug_bounds("title-col").expect("title-col laid out");
    let title = cx.debug_bounds("title").expect("title laid out");
    let title_inner = cx
        .debug_bounds("title-inner")
        .expect("title-inner laid out");
    let btn_col = cx.debug_bounds("btn-col").expect("btn-col laid out");
    let btn_area = cx.debug_bounds("btn-area").expect("btn-area laid out");
    let header = cx
        .debug_bounds("list-header")
        .expect("list-header laid out");
    let filter_btn = cx.debug_bounds("filter-btn").expect("filter-btn laid out");

    // Usable row width (minus 28px of left/right padding).
    let row_inner: f32 = (root.size.width - px(28.)).into();
    let title_ratio: f32 = title_col.size.width.into();
    let btn_ratio: f32 = btn_col.size.width.into();
    let title_ratio = title_ratio / row_inner;
    let btn_ratio = btn_ratio / row_inner;

    let container_right = root.origin.x + root.size.width;
    let btn_right = btn_col.origin.x + btn_col.size.width;
    let btn_area_right = btn_area.origin.x + btn_area.size.width;
    let header_right = header.origin.x + header.size.width;
    let filter_right = filter_btn.origin.x + filter_btn.size.width;

    println!(
        "[{label}] row={root:?} title_col={title_col:?} (ratio={title_ratio:.2}) btn_col={btn_col:?} (ratio={btn_ratio:.2})"
    );
    println!(
        "[{label}] title={title:?} title_inner={title_inner:?} (title_inner.right - title.right = {:?})",
        (title_inner.origin.x + title_inner.size.width) - (title.origin.x + title.size.width)
    );
    println!(
        "[{label}] btn_area={btn_area:?} btn_area_right={btn_area_right:?} btn_col_right={btn_right:?}"
    );
    println!("[{label}] header={header:?} filter_btn={filter_btn:?}");

    // 60/40 ratio (tolerating a 1% error).
    assert!(
        (title_ratio - 0.6).abs() < 0.01,
        "[{label}] 文本列应占 60%,实际 {title_ratio:.2}"
    );
    assert!(
        (btn_ratio - 0.4).abs() < 0.01,
        "[{label}] 操作列应占 40%,实际 {btn_ratio:.2}"
    );

    // No overflow.
    assert!(
        btn_right <= container_right + px(0.1),
        "[{label}] 操作列被挤出容器:btn_right={btn_right:?} > container_right={container_right:?}"
    );
    assert!(
        filter_right <= header_right + px(0.1),
        "[{label}] 筛选按钮超出标题行:filter_right={filter_right:?} > header_right={header_right:?}"
    );

    // Download area right-aligned: the button area's right edge matches the action column's right edge.
    assert!(
        (btn_area_right - btn_right).abs() < px(0.1),
        "[{label}] 下载区应右对齐:btn_area_right={btn_area_right:?} != btn_col_right={btn_right:?}"
    );
}

const LONG_TITLE: &str = "【某某字幕组】这个番剧的名字真的非常非常长以至于肯定装不下一行还会继续往后延伸很长很长很长很长很长很长";
const LONG_ASCII_TITLE: &str = "[SomeFanSub] Some.Anime.Title.S2.E01.1080p.WEB-DL.AAC.x264-SomeFanSub.ThisIsAReallyLongFileNameThatNeverBreaks";
const LONG_KEYWORD: &str = "关键词也非常非常长以至于筛选按钮肯定放不下需要截断";

/// The 60/40 layout holds in an 800px-wide window with long Chinese text.
#[gpui_kit::test]
async fn sixty_forty_wide_window(cx: &mut gpui_kit::TestAppContext) {
    let mut cx = setup(cx, 800., LONG_TITLE, LONG_KEYWORD);
    check(&mut cx, "wide");
}

/// The 60/40 layout holds in a 420px-wide window with long Chinese text.
#[gpui_kit::test]
async fn sixty_forty_narrow_window(cx: &mut gpui_kit::TestAppContext) {
    let mut cx = setup(cx, 420., LONG_TITLE, LONG_KEYWORD);
    check(&mut cx, "narrow");
}

/// The 60/40 layout holds in a 500px-wide window with a long unbreakable ASCII title.
#[gpui_kit::test]
async fn sixty_forty_ascii_title(cx: &mut gpui_kit::TestAppContext) {
    let mut cx = setup(cx, 500., LONG_ASCII_TITLE, LONG_KEYWORD);
    check(&mut cx, "ascii");
}
