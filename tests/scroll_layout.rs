//! 滚动机制回归测试 + 布局环境调查。

use gpui::TestAppContext;
use gpui::VisualTestContext;
use gpui::{
    Context, Render, ScrollDelta, ScrollHandle, ScrollWheelEvent, Size, TouchPhase, Window, div,
    point, prelude::*, px,
};

struct ScrollFixture {
    handle: ScrollHandle,
}

/// 模拟真实应用完整布局链
struct RealAppFixture {
    handle: ScrollHandle,
}

/// 绝对定位方案:所有尺寸显式
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
        // 滚动容器:确定高度 800px;内容 3000px,必然溢出
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

#[gpui::test]
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

#[gpui::test]
fn window_bounds_and_flex(cx: &mut TestAppContext) {
    let (_, mut cx) = setup(cx);

    // 窗口逻辑尺寸
    let bounds = cx.update(|window, _| window.bounds());
    println!("window.bounds() = {:?}", bounds);

    // 渲染一个 flex_row:固定项 + stretch 项,检查交叉轴高度
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

    // 判别实验:交叉轴显式 100% 高度是否生效
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

#[gpui::test]
fn real_app_chain_with_h_full(cx: &mut TestAppContext) {
    // 模拟真实应用完整链:所有高度用显式 h_full,不依赖 stretch
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

#[gpui::test]
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

    // 在 draw(明确根尺寸)环境中验证 flex_col 的 flex_1 是否生效
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

#[gpui::test]
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

    // 验证 flex_row 交叉轴 stretch
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

#[gpui::test]
fn absolute_layout_scroll_height(cx: &mut TestAppContext) {
    // 验证绝对定位 + 显式高度方案:所有尺寸显式,不依赖 flex grow/stretch
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
    // 视口 800 - 工具栏 48 = 752
    assert!(
        scroller.size.height > px(700.) && scroller.size.height < px(780.),
        "绝对定位方案滚动容器高度应为 752,实际 {:?}",
        scroller.size.height
    );
}

#[gpui::test]
fn horizontal_flex_grow_width(cx: &mut TestAppContext) {
    // 验证水平方向 flex_1(宽度)是否可靠
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

#[gpui::test]
fn absolute_inset_with_padding(cx: &mut TestAppContext) {
    // 验证绝对定位在带 padding 的 relative 父容器中的坐标基准(详情页 Hero 右列定位依赖)
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
