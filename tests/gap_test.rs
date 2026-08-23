//! 临时验证:flex 布局行为(gpui 0.2.2 crates.io 版)

use gpui::TestAppContext;
use gpui::VisualTestContext;
use gpui::{Context, Render, Size, Window, div, prelude::*, px};

struct FlexFixture;
impl Render for FlexFixture {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("root")
            .debug_selector(|| "root".to_string())
            .size_full()
            .flex()
            .flex_row()
            .gap(px(18.))
            .child(
                div()
                    .id("a")
                    .debug_selector(|| "a".to_string())
                    .w(px(100.))
                    .h(px(50.)),
            )
            .child(
                div()
                    .id("b")
                    .debug_selector(|| "b".to_string())
                    .w(px(100.))
                    .h(px(50.)),
            )
            .child(
                div()
                    .id("c")
                    .debug_selector(|| "c".to_string())
                    .w(px(100.))
                    .h(px(50.)),
            )
    }
}

struct ColGapFixture;
impl Render for ColGapFixture {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("root")
            .debug_selector(|| "root".to_string())
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(
                div()
                    .id("a")
                    .debug_selector(|| "a".to_string())
                    .h(px(50.))
                    .w(px(200.)),
            )
            .child(
                div()
                    .id("b")
                    .debug_selector(|| "b".to_string())
                    .h(px(50.))
                    .w(px(200.)),
            )
    }
}

struct CenterFixture;
impl Render for CenterFixture {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("root")
            .debug_selector(|| "root".to_string())
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(16.))
            .child(
                div()
                    .id("a")
                    .debug_selector(|| "a".to_string())
                    .h(px(30.))
                    .w(px(120.)),
            )
            .child(
                div()
                    .id("b")
                    .debug_selector(|| "b".to_string())
                    .h(px(30.))
                    .w(px(120.)),
            )
    }
}

fn open<T: Render + 'static>(
    cx: &mut TestAppContext,
    label: &str,
    make: impl FnOnce() -> T,
) -> VisualTestContext {
    let window = cx
        .update(|cx| cx.open_window(Default::default(), |_, cx| cx.new(|_| make())))
        .unwrap();
    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(Size {
        width: px(1200.),
        height: px(800.),
    });
    cx.run_until_parked();
    println!("--- {label} ---");
    cx
}

#[gpui::test]
fn flex_row_horizontal(cx: &mut TestAppContext) {
    let mut cx = open(cx, "flex_row", || FlexFixture);
    for id in ["root", "a", "b", "c"] {
        println!("{id} = {:?}", cx.debug_bounds(id).expect(id));
    }
    let a = cx.debug_bounds("a").unwrap();
    let b = cx.debug_bounds("b").unwrap();
    println!(
        "b.x - (a.x + a.w) = {:?}",
        b.origin.x - (a.origin.x + a.size.width)
    );
    assert_eq!(b.origin.x, px(118.), "flex_row 子项应水平排列且 gap 生效");
}

#[gpui::test]
fn flex_col_gap(cx: &mut TestAppContext) {
    let mut cx = open(cx, "flex_col+gap", || ColGapFixture);
    for id in ["root", "a", "b"] {
        println!("{id} = {:?}", cx.debug_bounds(id).expect(id));
    }
    let a = cx.debug_bounds("a").unwrap();
    let b = cx.debug_bounds("b").unwrap();
    println!("gap = {:?}", b.origin.y - (a.origin.y + a.size.height));
    assert_eq!(
        b.origin.y - (a.origin.y + a.size.height),
        px(24.),
        "flex_col gap 应生效"
    );
}

#[gpui::test]
fn flex_col_center(cx: &mut TestAppContext) {
    let mut cx = open(cx, "flex_col+center", || CenterFixture);
    for id in ["root", "a", "b"] {
        println!("{id} = {:?}", cx.debug_bounds(id).expect(id));
    }
    let a = cx.debug_bounds("a").unwrap();
    let b = cx.debug_bounds("b").unwrap();
    // 水平居中:x = (1200 - 120) / 2 = 540
    assert_eq!(a.origin.x, px(540.), "items_center 应水平居中");
    // 垂直居中:总高 30+16+30 = 76,起始 y = (800-76)/2 = 362
    assert_eq!(a.origin.y, px(362.), "justify_center 应垂直居中");
    assert_eq!(b.origin.y - (a.origin.y + a.size.height), px(16.), "gap 16");
}
