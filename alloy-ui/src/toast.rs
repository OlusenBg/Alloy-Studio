//! Toast notifications — floating stack in the bottom-right corner.

use crate::theme::*;
use floem::reactive::{RwSignal, SignalGet};
use floem::style::CursorStyle;
use floem::views::{container, dyn_stack, empty, h_stack, label, v_stack, Decorators};
use floem::{IntoView, View};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq)]
pub enum ToastTone {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone)]
pub struct ToastEntry {
    pub id: u64,
    pub tone: ToastTone,
    pub title: String,
    pub body: String,
    pub actions: Vec<ToastAction>,
}

#[derive(Clone)]
pub struct ToastAction {
    pub label: String,
    pub on_click: Arc<dyn Fn()>,
}

pub fn toast_overlay(toasts: RwSignal<Vec<ToastEntry>>, on_dismiss: Arc<dyn Fn(u64)>) -> impl View {
    container(
        v_stack((dyn_stack(move || toasts.get(), |t| t.id, {
            let dismiss = on_dismiss.clone();
            move |t| toast_card(t, dismiss.clone())
        })
        .style(|s| s.flex_col().gap(8.0).width_pct(100.0)),))
        .style(|s| {
            s.flex_direction(floem::taffy::style::FlexDirection::ColumnReverse)
                .gap(8.0)
                .width(360.0)
        }),
    )
    .style(move |s| {
        let s = s
            .absolute()
            .inset_bottom(UI_STATUS_HEIGHT + 8.0)
            .inset_right(12.0)
            .z_index(500);
        if toasts.get().is_empty() {
            s.hide()
        } else {
            s
        }
    })
}

fn toast_card(t: ToastEntry, on_dismiss: Arc<dyn Fn(u64)>) -> impl View {
    let (accent, icon) = match t.tone {
        ToastTone::Info => (STATUS_INFO, "i"),
        ToastTone::Success => (STATUS_SUCCESS, "ok"),
        ToastTone::Warning => (STATUS_WARNING, "!"),
        ToastTone::Error => (STATUS_ERROR, "x"),
    };
    let id = t.id;
    let title = t.title.clone();
    let body = t.body.clone();
    let actions = t.actions.clone();

    container(
        v_stack((
            h_stack((
                container(label(move || icon.to_string()).style(move |s| {
                    s.color(accent)
                        .font_size(T_TINY)
                        .font_weight(floem::text::FontWeight::BOLD)
                }))
                .style(move |s| {
                    s.width(18.0)
                        .height(18.0)
                        .border_radius(R_FULL)
                        .background(accent.with_alpha(0x22 as f32 / 255.0))
                        .items_center()
                        .justify_center()
                        .flex_shrink(0.0)
                }),
                v_stack((
                    label(move || title.clone()).style(|s| {
                        s.color(FG_1)
                            .font_size(T_SMALL)
                            .font_weight(floem::text::FontWeight::SEMI_BOLD)
                    }),
                    label(move || body.clone())
                        .style(|s| s.color(FG_3).font_size(T_TINY).margin_top(2.0)),
                ))
                .style(|s| s.flex_grow(1.0).min_width(0.0).gap(0.0)),
                container(label(|| "x".to_string()).style(|s| s.color(FG_3).font_size(T_TINY)))
                    .on_click_stop(move |_| (on_dismiss)(id))
                    .style(|s| {
                        s.padding(4.0)
                            .cursor(CursorStyle::Pointer)
                            .hover(|s| s.color(FG_1))
                    }),
            ))
            .style(|s| s.items_start().gap(10.0).width_pct(100.0)),
            if actions.is_empty() {
                container(empty()).style(|s| s.hide()).into_any()
            } else {
                h_stack((floem::views::stack_from_iter(actions.into_iter().map(|a| {
                    container(label(move || a.label.clone()).style(|s| {
                        s.color(FG_1)
                            .font_size(T_TINY)
                            .font_weight(floem::text::FontWeight::SEMI_BOLD)
                    }))
                    .on_click_stop(move |_| (a.on_click)())
                    .style(|s| {
                        s.padding_horiz(10.0)
                            .padding_vert(4.0)
                            .background(ALLOY_ORANGE)
                            .border_radius(R_4)
                            .cursor(CursorStyle::Pointer)
                            .hover(|s| s.background(ALLOY_ORANGE_DEEP))
                    })
                }))
                .style(|s| s.gap(6.0)),))
                .style(|s| s.margin_top(8.0))
                .into_any()
            },
        ))
        .style(|s| s.flex_col().gap(0.0).width_pct(100.0)),
    )
    .style(move |s| {
        s.padding(12.0)
            .background(BG_RAISED)
            .border_radius(R_8)
            .border_left(3.0)
            .border_color(accent)
            .box_shadow_blur(16.0)
            .box_shadow_color(floem::peniko::Color::from_rgba8(0, 0, 0, 0x5C))
            .box_shadow_v_offset(4.0)
            .width_pct(100.0)
    })
}
