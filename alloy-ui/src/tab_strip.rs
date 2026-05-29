//! Tab strip — row of editor tabs + breadcrumb bar below.
//!
//! Reference: kit/TabStrip.jsx.

use crate::theme::*;
use floem::reactive::{RwSignal, SignalGet};
use floem::style::CursorStyle;
use floem::views::{container, dyn_stack, empty, h_stack, label, v_stack, Decorators};
use floem::{IntoView, View};
use std::sync::Arc;

#[derive(Clone)]
pub struct EditorTab {
    pub id: String,
    pub name: String,
    pub lang: String,
    pub dirty: bool,
}

pub fn tab_strip(
    tabs: RwSignal<Vec<EditorTab>>,
    active: RwSignal<String>,
    breadcrumb: RwSignal<Vec<String>>,
    on_select: Arc<dyn Fn(String)>,
    on_close: Arc<dyn Fn(String)>,
) -> impl View {
    v_stack((
        // Tab row
        h_stack((
            dyn_stack(move || tabs.get(), |t| t.id.clone(), {
                let on_select = on_select.clone();
                let on_close = on_close.clone();
                move |t| tab_item(t, active, on_select.clone(), on_close.clone())
            })
            .style(|s| s.flex_row()),
            container(empty()).style(|s| s.flex_grow(1.0f32).background(BG_SURFACE)),
        ))
        .style(|s| {
            s.height(35.0)
                .width_pct(100.0)
                .background(BG_SURFACE)
                .border_bottom(1.0)
                .border_color(BG_EDGE)
                .items_end()
        }),
        // Breadcrumb
        breadcrumb_bar(breadcrumb),
    ))
    .style(|s| s.flex_col().width_pct(100.0).flex_shrink(0.0))
}

fn tab_item(
    t: EditorTab,
    active: RwSignal<String>,
    on_select: Arc<dyn Fn(String)>,
    on_close: Arc<dyn Fn(String)>,
) -> impl View {
    let id_sel = t.id.clone();
    let id_close = t.id.clone();
    let id_active = t.id.clone();
    let name = t.name.clone();
    let dirty = t.dirty;

    container(
        h_stack((
            label(move || name.clone()).style(|s| s.color(FG_2).font_size(T_SMALL)),
            container(if dirty {
                label(|| "•".to_string())
                    .style(|s| s.color(FG_2).font_size(T_TINY))
                    .into_any()
            } else {
                container(label(|| "x".to_string()).style(|s| s.color(FG_4).font_size(T_MICRO)))
                    .on_click_stop(move |_| (on_close)(id_close.clone()))
                    .style(|s| {
                        s.width(14.0)
                            .height(14.0)
                            .border_radius(R_4)
                            .items_center()
                            .justify_center()
                            .cursor(CursorStyle::Pointer)
                            .hover(|s| s.background(BG_HOVER).color(FG_1))
                    })
                    .into_any()
            })
            .style(|s| s.margin_left(8.0).items_center().justify_center()),
        ))
        .style(|s| s.items_center()),
    )
    .on_click_stop(move |_| (on_select)(id_sel.clone()))
    .style(move |s| {
        let is_active = active.get() == id_active;
        let s = s
            .min_width(UI_TAB_MIN_WIDTH)
            .height(35.0)
            .padding_horiz(12.0)
            .items_center()
            .cursor(CursorStyle::Pointer)
            .border_right(1.0)
            .border_color(BG_EDGE);
        if is_active {
            s.background(BG_NAVY)
                .border_bottom(2.0)
                .border_color(floem::peniko::Color::from_rgb8(0x52, 0x8B, 0xFF))
                .color(FG_1)
        } else {
            s.background(BG_SURFACE)
                .color(FG_3)
                .hover(|s| s.background(BG_RAISED).color(FG_2))
        }
    })
}

fn breadcrumb_bar(segments: RwSignal<Vec<String>>) -> impl View {
    h_stack((dyn_stack(
        move || {
            let v = segments.get();
            let total = v.len();
            v.into_iter().enumerate().collect::<Vec<_>>()
        },
        |(i, _)| *i,
        move |(i, seg)| {
            let is_last = i + 1 == segments.get().len();
            h_stack((
                label(move || seg.clone()).style(move |s| {
                    let s = s.font_size(T_TINY).font_family("monospace".to_string());
                    if is_last {
                        s.color(FG_2)
                    } else {
                        s.color(FG_4)
                    }
                }),
                if !is_last {
                    label(|| " > ".to_string())
                        .style(|s| s.color(FG_4).font_size(T_TINY))
                        .into_any()
                } else {
                    container(empty()).style(|s| s.hide()).into_any()
                },
            ))
            .style(|s| s.items_center())
        },
    )
    .style(|s| s.flex_row().items_center()),))
    .style(|s| {
        s.height(22.0)
            .padding_horiz(12.0)
            .background(BG_SURFACE)
            .border_bottom(1.0)
            .border_color(BG_EDGE)
            .items_center()
    })
}
