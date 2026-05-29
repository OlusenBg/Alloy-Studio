//! Activity bar — 50 px wide vertical icon strip on the far left.
//!
//! Reference: kit/ActivityBar.jsx.

use crate::theme::*;
use floem::reactive::{RwSignal, SignalGet};
use floem::style::CursorStyle;
use floem::views::{container, empty, img, label, v_stack, Decorators};
use floem::View;
use std::sync::Arc;

static ALLOY_LOGO: &[u8] = include_bytes!("../extra/images/logo.png");

#[derive(Clone, Copy, PartialEq)]
pub enum ActivityTab {
    Files,
    Search,
    SourceControl,
    OpModes,
    Extensions,
}

pub fn activity_bar(active: RwSignal<ActivityTab>, on_settings: Arc<dyn Fn()>) -> impl View {
    v_stack((
        // Logo at top
        container(img(|| ALLOY_LOGO.to_vec()).style(|s| s.width(26.0).height(26.0))).style(|s| {
            s.width(UI_ACTIVITY_WIDTH)
                .height(UI_ACTIVITY_WIDTH)
                .items_center()
                .justify_center()
        }),
        // Main tabs
        activity_btn(ActivityTab::Files, "files", "F", active.clone()),
        activity_btn(ActivityTab::Search, "search", "S", active.clone()),
        activity_btn(ActivityTab::SourceControl, "scm", "G", active.clone()),
        activity_btn(ActivityTab::OpModes, "opmodes", "O", active.clone()),
        activity_btn(ActivityTab::Extensions, "ext", "E", active.clone()),
        // Spacer
        container(empty()).style(|s| s.flex_grow(1.0f32)),
        // Settings at bottom
        container(label(|| "gear".to_string()).style(|s| s.color(FG_3).font_size(T_MD)))
            .on_click_stop(move |_| (on_settings)())
            .style(|s| {
                s.width(UI_ACTIVITY_WIDTH)
                    .height(UI_ACTIVITY_WIDTH)
                    .items_center()
                    .justify_center()
                    .cursor(CursorStyle::Pointer)
                    .hover(|s| s.color(FG_1))
            }),
    ))
    .style(|s| {
        s.width(UI_ACTIVITY_WIDTH)
            .flex_shrink(0.0)
            .height_pct(100.0)
            .background(BG_SURFACE)
            .border_right(1.0)
            .border_color(BG_EDGE)
            .flex_col()
    })
}

fn activity_btn(
    tab: ActivityTab,
    _name: &'static str,
    glyph: &'static str,
    active: RwSignal<ActivityTab>,
) -> impl View {
    container(
        v_stack((
            // active stripe on left
            container(empty()).style(move |s| {
                let s = s
                    .absolute()
                    .width(2.0)
                    .height(18.0)
                    .background(ALLOY_ORANGE)
                    .border_radius(R_2)
                    .margin_left(0.0);
                if active.get() == tab {
                    s
                } else {
                    s.hide()
                }
            }),
            label(move || glyph.to_string()).style(move |s| {
                let c = if active.get() == tab { FG_1 } else { FG_3 };
                s.color(c).font_size(T_LG)
            }),
        ))
        .style(|s| s.items_center().justify_center()),
    )
    .on_click_stop(move |_| active.set(tab))
    .style(|s| {
        s.width(UI_ACTIVITY_WIDTH)
            .height(UI_ACTIVITY_WIDTH)
            .items_center()
            .justify_center()
            .cursor(CursorStyle::Pointer)
            .relative()
            .hover(|s| s.background(BG_HOVER))
    })
}
