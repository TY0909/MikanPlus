//! Scroll-position persistence regression test: an externally held ScrollHandle keeps its offset across page switches (window rebuilds).

use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::{
    Context, Render, ScrollDelta, ScrollHandle, ScrollWheelEvent, Size, TouchPhase, Window, div,
    point, prelude::*, px,
};

/// Minimal scroll container tracked by an external handle.
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

/// Opens a 1200x800 fixture window sharing the given scroll handle.
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

/// The handle records the offset after a scroll event; it keeps that offset when
/// the window is closed (page switch) and restores the scroll position when the
/// window is reopened (returning to the page).
#[gpui_kit::test]
fn scroll_offset_survives_window_recreation(cx: &mut TestAppContext) {
    let handle = ScrollHandle::default();

    // First visit: scroll into the middle.
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

    // Simulate a page switch: destroy the window.
    cx1.update(|window, _| window.remove_window());
    cx1.run_until_parked();

    // Simulate returning: recreate the window with the same handle.
    let mut cx2 = setup(cx, handle.clone());
    let offset_after_reopen = handle.offset().y;
    println!("offset after reopen: {:?}", offset_after_reopen);
    assert_eq!(
        offset_after_reopen, offset_after_scroll,
        "页面重建后 handle 应保留滚动偏移"
    );

    // Scroll again with the wheel; scrolling should still work in the new window.
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
