//! Bounded list viewport shared by the application and offscreen layout tests.
use gpui::{prelude::*, *};
use gpui_component::{
    scroll::{Scrollbar, ScrollbarShow},
    v_flex,
};

pub fn scroll_viewport(list: Stateful<Div>, scroll: &ScrollHandle) -> impl IntoElement {
    v_flex()
        .id("command-list-viewport")
        .relative()
        .size_full()
        .min_h_0()
        .overflow_hidden()
        .child(
            list.size_full()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(scroll),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .right_0()
                .bottom_0()
                .w(px(16.0))
                .child(Scrollbar::vertical(scroll).scrollbar_show(ScrollbarShow::Scrolling)),
        )
}
