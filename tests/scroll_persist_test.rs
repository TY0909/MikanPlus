//! 滚动位置保留回归测试:外部持有的 ScrollHandle 在页面切换(窗口重建)后仍保留偏移。

use gpui::TestAppContext;
use gpui::VisualTestContext;
use gpui::{
    Context, Render, ScrollDelta, ScrollHandle, ScrollWheelEvent, Size, TouchPhase, Window, div,
    point, prelude::*, px,
};

struct ScrollFixture {
    handle: ScrollHandle,
}

impl Render for ScrollFixture {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("scroller")
            .debug_selector(|| "scroller".to_string())
            .w_full()
            .h(px(600.))
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

fn setup(cx: &mut TestAppContext, handle: ScrollHandle) -> VisualTestContext {
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|_| ScrollFixture {
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
    cx
}

/// 滚动事件后 handle 保存偏移;窗口关闭(页面切换)后 handle 仍保留偏移,
/// 重新打开窗口(返回页面)后滚动位置得以恢复。
#[gpui::test]
fn scroll_offset_survives_window_recreation(cx: &mut TestAppContext) {
    let handle = ScrollHandle::default();

    // 第一次进入页面:滚动到中部
    let mut cx1 = setup(cx, handle.clone());
    cx1.simulate_event(ScrollWheelEvent {
        position: point(px(600.), px(400.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(-800.))),
        modifiers: Default::default(),
        touch_phase: TouchPhase::Moved,
    });
    cx1.run_until_parked();
    let offset_after_scroll = handle.offset().y;
    println!("offset after scroll: {:?}", offset_after_scroll);
    assert_ne!(offset_after_scroll, px(0.), "滚动后 offset 不应为 0");

    // 模拟页面切换:窗口销毁
    cx1.update(|window, _| window.remove_window());
    cx1.run_until_parked();

    // 模拟返回页面:同一 handle 重新创建窗口
    let mut cx2 = setup(cx, handle.clone());
    let offset_after_reopen = handle.offset().y;
    println!("offset after reopen: {:?}", offset_after_reopen);
    assert_eq!(
        offset_after_reopen, offset_after_scroll,
        "页面重建后 handle 应保留滚动偏移"
    );

    // 滚轮再次滚动,新窗口内的滚动应继续生效
    cx2.simulate_event(ScrollWheelEvent {
        position: point(px(600.), px(400.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(-400.))),
        modifiers: Default::default(),
        touch_phase: TouchPhase::Moved,
    });
    cx2.run_until_parked();
    println!("offset after second scroll: {:?}", handle.offset().y);
    assert_ne!(handle.offset().y, offset_after_scroll, "重建后仍可继续滚动");
}
