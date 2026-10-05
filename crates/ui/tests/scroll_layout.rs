//! Scroll-mechanism regression tests plus layout-environment investigation.

use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::{
    Context, Render, ScrollDelta, ScrollHandle, ScrollWheelEvent, Size, TouchPhase, Window, div,
    point, prelude::*, px,
};

/// Fixed 800px scroll container holding 3000px of content; used as a probe host.
struct ScrollFixture {
    handle: ScrollHandle,
}

/// Mirrors the real app's full layout chain using flex_1 and explicit heights.
struct RealAppFixture {
    handle: ScrollHandle,
}

/// Absolute-positioning approach in which every size is explicit.
struct AbsoluteFixture {
    handle: ScrollHandle,
}

impl Render for AbsoluteFixture {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("app-root")
            .debug_selector(|| "app-root".to_string())
            .size_full()
            .relative()
            .child(
                div()
                    .id("sidebar")
                    .debug_selector(|| "sidebar".to_string())
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w(px(216.)),
            )
            .child(
                div()
                    .id("main")
                    .debug_selector(|| "main".to_string())
                    .absolute()
                    .left(px(216.))
                    .right_0()
                    .top_0()
                    .bottom_0()
                    .child(
                        div()
                            .id("toolbar")
                            .debug_selector(|| "toolbar".to_string())
                            .absolute()
                            .top_0()
                            .left_0()
                            .right_0()
                            .h(px(48.)),
                    )
                    .child(
                        div()
                            .id("content-wrap")
                            .debug_selector(|| "content-wrap".to_string())
                            .absolute()
                            .top(px(48.))
                            .bottom_0()
                            .left_0()
                            .right_0()
                            .flex()
                            .flex_row()
                            .justify_center()
                            .child(
                                div()
                                    .id("scroller")
                                    .debug_selector(|| "scroller".to_string())
                                    .w_full()
                                    .max_w(px(1200.))
                                    .h_full()
                                    .overflow_y_scroll()
                                    .track_scroll(&self.handle)
                                    .child(div().h(px(3000.)).w_full()),
                            ),
                    ),
            )
    }
}

impl Render for RealAppFixture {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("app-root")
            .debug_selector(|| "app-root".to_string())
            .size_full()
            .flex_row()
            .child(
                div()
                    .id("sidebar")
                    .debug_selector(|| "sidebar".to_string())
                    .w(px(216.))
                    .h_full(),
            )
            .child(
                div()
                    .id("main")
                    .debug_selector(|| "main".to_string())
                    .flex_1()
                    .min_w_0()
                    .flex_col()
                    .h_full()
                    .child(
                        div()
                            .id("toolbar")
                            .debug_selector(|| "toolbar".to_string())
                            .h(px(48.)),
                    )
                    .child(
                        div()
                            .id("content-wrap")
                            .debug_selector(|| "content-wrap".to_string())
                            .flex_1()
                            .min_h_0()
                            .flex()
                            .flex_row()
                            .justify_center()
                            .h_full()
                            .child(
                                div()
                                    .id("scroller")
                                    .debug_selector(|| "scroller".to_string())
                                    .w_full()
                                    .max_w(px(1200.))
                                    .h_full()
                                    .overflow_y_scroll()
                                    .track_scroll(&self.handle)
                                    .child(div().h(px(3000.)).w_full()),
                            ),
                    ),
            )
    }
}

impl Render for ScrollFixture {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Scroll container: fixed 800px height with 3000px of content, so it always overflows.
        div()
            .id("scroller")
            .debug_selector(|| "scroller".to_string())
            .w_full()
            .h(px(800.))
            .overflow_y_scroll()
            .track_scroll(&self.handle)
            .child(
                div()
                    .id("tall")
                    .debug_selector(|| "tall".to_string())
                    .h(px(3000.))
                    .w_full(),
            )
    }
}

/// Opens a 1200x800 fixture window and returns its scroll handle and test context.
fn setup(cx: &mut TestAppContext) -> (ScrollHandle, VisualTestContext) {
    let handle = ScrollHandle::default();
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|_cx| ScrollFixture {
                handle: handle.clone(),
            })
        })
        .unwrap()
    });
    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(Size {
        width: px(1200.),
        height: px(800.),
    });
    cx.run_until_parked();
    (handle, cx)
}

/// A wheel event over the scroller changes its scroll offset.
#[gpui_kit::test]
fn scroll_wheel_moves_scroll_offset(cx: &mut TestAppContext) {
    let (handle, mut cx) = setup(cx);

    let scroller = cx
        .debug_bounds("scroller")
        .expect("scroller should be laid out");
    let tall = cx.debug_bounds("tall").expect("tall should be laid out");
    println!("scroller={:?} tall={:?}", scroller, tall);
    assert_eq!(scroller.size.height, px(800.));
    assert_eq!(tall.size.height, px(3000.));

    cx.simulate_event(ScrollWheelEvent {
        position: point(px(600.), px(400.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(-400.))),
        modifiers: Default::default(),
        touch_phase: TouchPhase::Moved,
    });
    cx.run_until_parked();

    let offset = handle.offset();
    println!("offset after scroll: {:?}", offset);
    assert_ne!(offset.y, px(0.), "滚轮事件后滚动位置应发生变化");
}

/// Reports window bounds and probes cross-axis sizing for fixed and stretch children.
#[gpui_kit::test]
fn window_bounds_and_flex(cx: &mut TestAppContext) {
    let (_, mut cx) = setup(cx);

    // Window logical bounds.
    let bounds = cx.update(|window, _| window.bounds());
    println!("window.bounds() = {:?}", bounds);

    // Render a flex_row with a fixed child plus a stretch child to inspect cross-axis height.
    cx.draw(
        point(px(0.), px(0.)),
        Size::new(px(1200.), px(800.)),
        |_window, _cx| {
            div()
                .id("row")
                .debug_selector(|| "row".to_string())
                .size_full()
                .flex_row()
                .child(
                    div()
                        .id("a")
                        .debug_selector(|| "a".to_string())
                        .w(px(200.))
                        .h(px(50.)),
                )
                .child(div().id("b").debug_selector(|| "b".to_string()).flex_1())
        },
    );

    let row = cx.debug_bounds("row").expect("row");
    let a = cx.debug_bounds("a").expect("a");
    let b = cx.debug_bounds("b").expect("b");
    println!("draw-test row={:?} a={:?} b={:?}", row, a, b);

    // Diagnostic experiment: does an explicit 100% cross-axis height take effect?
    cx.draw(
        point(px(0.), px(0.)),
        Size::new(px(1200.), px(800.)),
        |_window, _cx| {
            div()
                .id("row2")
                .debug_selector(|| "row2".to_string())
                .size_full()
                .flex_row()
                .child(
                    div()
                        .id("c")
                        .debug_selector(|| "c".to_string())
                        .w(px(200.))
                        .h_full(),
                )
                .child(
                    div()
                        .id("d")
                        .debug_selector(|| "d".to_string())
                        .flex_1()
                        .h_full(),
                )
        },
    );
    let c = cx.debug_bounds("c").expect("c");
    let d = cx.debug_bounds("d").expect("d");
    println!("pct-test c={:?} d={:?}", c, d);
}

/// The full app chain with explicit h_full heights keeps the scroller at viewport height.
#[gpui_kit::test]
fn real_app_chain_with_h_full(cx: &mut TestAppContext) {
    // Full real-app chain, but every height uses an explicit h_full instead of relying on stretch.
    let handle = ScrollHandle::default();
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|_cx| RealAppFixture {
                handle: handle.clone(),
            })
        })
        .unwrap()
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(Size {
        width: px(1200.),
        height: px(800.),
    });
    cx.run_until_parked();

    let root = cx.debug_bounds("app-root").expect("app-root");
    let main = cx.debug_bounds("main").expect("main");
    let wrap = cx.debug_bounds("content-wrap").expect("content-wrap");
    let scroller = cx.debug_bounds("scroller").expect("scroller");
    println!(
        "chain root={:?} main={:?} wrap={:?} scroller={:?}",
        root, main, wrap, scroller
    );
    assert!(
        scroller.size.height < px(1200.),
        "滚动容器高度应为视口高度,实际 {:?}",
        scroller.size.height
    );
}

/// Probes whether flex_1 fills the remaining height in a flex_col under draw.
#[gpui_kit::test]
fn draw_env_flex_behavior(cx: &mut TestAppContext) {
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|_cx| ScrollFixture {
                handle: ScrollHandle::default(),
            })
        })
        .unwrap()
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(Size {
        width: px(1200.),
        height: px(800.),
    });

    // In a draw call with an explicit root size, check whether flex_1 works in a flex_col.
    cx.draw(
        point(px(0.), px(0.)),
        Size::new(px(1200.), px(800.)),
        |_window, _cx| {
            div()
                .id("col")
                .debug_selector(|| "col".to_string())
                .size_full()
                .flex_col()
                .child(
                    div()
                        .id("head")
                        .debug_selector(|| "head".to_string())
                        .h(px(48.)),
                )
                .child(
                    div()
                        .id("rest")
                        .debug_selector(|| "rest".to_string())
                        .flex_1()
                        .min_h_0(),
                )
        },
    );
    let col = cx.debug_bounds("col").expect("col");
    let head = cx.debug_bounds("head").expect("head");
    let rest = cx.debug_bounds("rest").expect("rest");
    println!("grow-test col={:?} head={:?} rest={:?}", col, head, rest);
}

/// Probes cross-axis stretch for a width-only child in a flex_row under draw.
#[gpui_kit::test]
fn draw_env_stretch_behavior(cx: &mut TestAppContext) {
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|_cx| ScrollFixture {
                handle: ScrollHandle::default(),
            })
        })
        .unwrap()
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(Size {
        width: px(1200.),
        height: px(800.),
    });

    // Check cross-axis stretch in a flex_row.
    cx.draw(
        point(px(0.), px(0.)),
        Size::new(px(1200.), px(800.)),
        |_window, _cx| {
            div()
                .id("row")
                .debug_selector(|| "row".to_string())
                .size_full()
                .flex_row()
                .child(
                    div()
                        .id("only")
                        .debug_selector(|| "only".to_string())
                        .w(px(300.)),
                )
        },
    );
    let only = cx.debug_bounds("only").expect("only");
    println!("stretch-test only={:?}", only);
}

/// Absolute positioning with explicit sizes yields the expected scroller height.
#[gpui_kit::test]
fn absolute_layout_scroll_height(cx: &mut TestAppContext) {
    // Verify the absolute-positioning plus explicit-height approach: every size is explicit and nothing relies on flex grow/stretch.
    let handle = ScrollHandle::default();
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|_cx| AbsoluteFixture {
                handle: handle.clone(),
            })
        })
        .unwrap()
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(Size {
        width: px(1200.),
        height: px(800.),
    });
    cx.run_until_parked();

    let wrap = cx.debug_bounds("content-wrap").expect("content-wrap");
    let scroller = cx.debug_bounds("scroller").expect("scroller");
    println!("ABS wrap={:?} scroller={:?}", wrap, scroller);
    // Viewport 800 - toolbar 48 = 752.
    assert!(
        scroller.size.height > px(700.) && scroller.size.height < px(780.),
        "绝对定位方案滚动容器高度应为 752,实际 {:?}",
        scroller.size.height
    );
}

/// Probes whether flex_1 reliably grows width horizontally.
#[gpui_kit::test]
fn horizontal_flex_grow_width(cx: &mut TestAppContext) {
    // Check whether horizontal flex_1 (width) grows reliably.
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|_cx| ScrollFixture {
                handle: ScrollHandle::default(),
            })
        })
        .unwrap()
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(Size {
        width: px(1200.),
        height: px(800.),
    });

    cx.draw(
        point(px(0.), px(0.)),
        Size::new(px(1200.), px(800.)),
        |_window, _cx| {
            div()
                .id("row")
                .debug_selector(|| "row".to_string())
                .size_full()
                .flex_row()
                .child(
                    div()
                        .id("side")
                        .debug_selector(|| "side".to_string())
                        .w(px(216.)),
                )
                .child(
                    div()
                        .id("grow")
                        .debug_selector(|| "grow".to_string())
                        .flex_1()
                        .child(div().w_full()),
                )
        },
    );
    let side = cx.debug_bounds("side").expect("side");
    let grow = cx.debug_bounds("grow").expect("grow");
    println!("HGROW side={:?} grow={:?}", side, grow);
}

/// Probes the coordinate origin for absolute children inside a padded relative parent.
#[gpui_kit::test]
fn absolute_inset_with_padding(cx: &mut TestAppContext) {
    // Verify the coordinate origin for absolute positioning inside a padded relative parent (the detail page's Hero right column depends on this).
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|_cx| ScrollFixture {
                handle: ScrollHandle::default(),
            })
        })
        .unwrap()
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(Size {
        width: px(1200.),
        height: px(800.),
    });

    cx.draw(
        point(px(0.), px(0.)),
        Size::new(px(1200.), px(800.)),
        |_window, _cx| {
            div()
                .id("wrap")
                .debug_selector(|| "wrap".to_string())
                .w(px(500.))
                .h(px(300.))
                .relative()
                .p(px(20.))
                .child(
                    div()
                        .id("flow")
                        .debug_selector(|| "flow".to_string())
                        .w(px(160.))
                        .h(px(200.)),
                )
                .child(
                    div()
                        .id("abs")
                        .debug_selector(|| "abs".to_string())
                        .absolute()
                        .left(px(180.))
                        .top_0()
                        .right_0()
                        .h(px(60.)),
                )
        },
    );
    let wrap = cx.debug_bounds("wrap").expect("wrap");
    let flow = cx.debug_bounds("flow").expect("flow");
    let abs = cx.debug_bounds("abs").expect("abs");
    println!("ABSINSET wrap={:?} flow={:?} abs={:?}", wrap, flow, abs);
}
