#![cfg(feature = "ui-tests")]
//! GPUI TestPlatform only: no native window, mouse or keyboard input is sent.
use command_manager::list_view::scroll_viewport;
use gpui::{
    Context, InputEvent, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Render, ScrollDelta, ScrollHandle, ScrollWheelEvent, TestAppContext, TouchPhase, Window,
    WindowHandle, div, point, prelude::*, px, size,
};
use gpui_component::v_flex;

struct LongList {
    count: usize,
    scroll: ScrollHandle,
}
impl Render for LongList {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut list = v_flex().id("test-command-list").gap_1();
        for _ in 0..self.count {
            // Every row (folder or command) shares one height token.
            list = list.child(
                div()
                    .h(px(60.0))
                    .w_full()
                    .flex_shrink_0(),
            );
        }
        v_flex()
            .size_full()
            .child(div().h(px(84.0)).w_full().flex_shrink_0())
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(scroll_viewport(list, &self.scroll)),
            )
            .child(div().h(px(88.0)).w_full().flex_shrink_0())
    }
}

fn setup(cx: &mut TestAppContext, count: usize) -> (WindowHandle<LongList>, ScrollHandle) {
    cx.update(gpui_component::init);
    let scroll = ScrollHandle::new();
    let shared = scroll.clone();
    let window = cx.open_window(size(px(430.0), px(360.0)), move |_, _| LongList {
        count,
        scroll: shared,
    });
    draw(cx, window);
    (window, scroll)
}

fn draw(cx: &mut TestAppContext, handle: WindowHandle<LongList>) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
    })
    .unwrap();
}

fn wheel(cx: &mut TestAppContext, handle: WindowHandle<LongList>, delta: ScrollDelta) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            ScrollWheelEvent {
                position: point(px(150.0), px(150.0)),
                delta,
                touch_phase: TouchPhase::Moved,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        window.draw(cx).clear(cx);
    })
    .unwrap();
}

#[gpui::test]
fn long_list_has_bounded_viewport_and_wheel_scrolls_both_directions(cx: &mut TestAppContext) {
    let (window, scroll) = setup(cx, 200);
    assert_eq!(scroll.bounds().size.height, px(188.0));
    assert!(scroll.max_offset().y > px(10_000.0));
    wheel(cx, window, ScrollDelta::Lines(point(0.0, -6.0)));
    assert!(
        scroll.offset().y < px(0.0),
        "wheel down must move the command list"
    );
    wheel(
        cx,
        window,
        ScrollDelta::Pixels(point(px(0.0), px(50_000.0))),
    );
    assert_eq!(
        scroll.offset().y,
        px(0.0),
        "wheel up must return to the first command"
    );
    assert_eq!(
        scroll.bounds().size.height,
        px(188.0),
        "header/footer must stay fixed"
    );
}

#[gpui::test]
fn selection_reveals_last_row_and_can_return_to_first(cx: &mut TestAppContext) {
    let (window, scroll) = setup(cx, 200);
    scroll.scroll_to_item(199);
    draw(cx, window);
    assert!(scroll.offset().y < px(-10_000.0));
    let last = scroll.bounds_for_item(199).unwrap();
    assert!(last.bottom() + scroll.offset().y <= scroll.bounds().bottom() + px(1.0));
    assert!(last.top() + scroll.offset().y >= scroll.bounds().top() - px(1.0));
    scroll.scroll_to_top_of_item(0);
    draw(cx, window);
    assert_eq!(scroll.offset().y, px(0.0));
}

#[gpui::test]
fn scrollbar_offset_and_filtered_short_or_empty_results_are_clamped(cx: &mut TestAppContext) {
    let (window, scroll) = setup(cx, 200);
    scroll.set_offset(point(px(0.0), -scroll.max_offset().y));
    draw(cx, window);
    assert!(scroll.offset().y < px(-10_000.0));
    window
        .update(cx, |this, _, cx| {
            this.count = 1;
            cx.notify();
        })
        .unwrap();
    draw(cx, window);
    assert_eq!(scroll.max_offset().y, px(0.0));
    assert_eq!(scroll.offset().y, px(0.0));
    window
        .update(cx, |this, _, cx| {
            this.count = 0;
            cx.notify();
        })
        .unwrap();
    draw(cx, window);
    assert_eq!(scroll.offset().y, px(0.0));
    assert_eq!(scroll.bounds().size.height, px(188.0));
}

#[gpui::test]
fn dragging_scrollbar_reaches_bottom_and_top(cx: &mut TestAppContext) {
    let (handle, scroll) = setup(cx, 200);
    // The thumb only fades in while scrolling, so wake it up before grabbing it.
    wheel(cx, handle, ScrollDelta::Lines(point(0.0, -1.0)));
    wheel(cx, handle, ScrollDelta::Lines(point(0.0, -1.0)));
    let start = point(
        scroll.bounds().right() - px(8.0),
        scroll.bounds().top() + px(12.0),
    );
    let bottom = point(start.x, scroll.bounds().bottom() - px(4.0));
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            MouseDownEvent {
                button: MouseButton::Left,
                position: start,
                click_count: 1,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        // The scrollbar limits drag updates using real elapsed time.
        std::thread::sleep(std::time::Duration::from_millis(25));
        window.dispatch_event(
            MouseMoveEvent {
                position: bottom,
                pressed_button: Some(MouseButton::Left),
                modifiers: Default::default(),
            }
            .to_platform_input(),
            cx,
        );
        window.dispatch_event(
            MouseUpEvent {
                button: MouseButton::Left,
                position: bottom,
                click_count: 1,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        window.draw(cx).clear(cx);
    })
    .unwrap();
    assert!(
        scroll.offset().y < px(-10_000.0),
        "dragging the thumb must scroll real content"
    );
    let grabbed = point(start.x, scroll.bounds().bottom() - px(16.0));
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            MouseDownEvent {
                button: MouseButton::Left,
                position: grabbed,
                click_count: 1,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        // The scrollbar limits drag updates using real elapsed time.
        std::thread::sleep(std::time::Duration::from_millis(25));
        window.dispatch_event(
            MouseMoveEvent {
                position: start,
                pressed_button: Some(MouseButton::Left),
                modifiers: Default::default(),
            }
            .to_platform_input(),
            cx,
        );
        window.dispatch_event(
            MouseUpEvent {
                button: MouseButton::Left,
                position: start,
                click_count: 1,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        window.draw(cx).clear(cx);
    })
    .unwrap();
    assert_eq!(scroll.offset().y, px(0.0));
}
