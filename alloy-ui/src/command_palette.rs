//! Alloy command palette — overlay that drops from the title bar on ⌘K.
//!
//! This file is pure presentation. The actual fuzzy matching, command
//! dispatch, and rank-by-recency live in `lapce-app/src/palette.rs`. The
//! caller passes in a filtered list of `PaletteItem`s + current query +
//! selection cursor; this view renders them.
//!
//! Reference: kit/CommandPalette.jsx.

use std::sync::Arc;

use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
use floem::style::CursorStyle;
use floem::views::{container, dyn_stack, empty, h_stack, label, text_input, v_stack, Decorators};
use floem::{IntoView, View};

use crate::theme::*;

#[derive(Clone, PartialEq)]
pub enum PaletteTone {
    Default,
    Success,
    Info,
    Warning,
}

#[derive(Clone)]
pub struct PaletteItem {
    pub group: String,
    pub icon: &'static str, // codicon glyph fallback char or name
    pub label: String,
    pub sub: String,
    pub kbd: Option<String>,
    pub tone: PaletteTone,
    pub on_run: Arc<dyn Fn()>,
}

/// Render the palette. `open` controls visibility; `query` is bound to the
/// search input; `items` is the (already-filtered, already-grouped) list.
pub fn alloy_command_palette(
    open: RwSignal<bool>,
    query: RwSignal<String>,
    items: RwSignal<Vec<PaletteItem>>,
    selected: RwSignal<usize>,
) -> impl View {
    let backdrop = container(
        container(palette_card(query, items, selected, open))
            .style(|s| s.width(640.0).items_center()),
    )
    .on_click_stop(move |_| open.set(false))
    .style(move |s| {
        let s = s
            .absolute()
            .size_pct(100.0, 100.0)
            .background(floem::peniko::Color::from_rgba8(0x0A, 0x0D, 0x16, 0x73))
            .items_start()
            .justify_center()
            .padding_top(UI_HEADER_HEIGHT)
            .z_index(200);
        if open.get() {
            s
        } else {
            s.hide()
        }
    });

    backdrop
}

fn palette_card(
    query: RwSignal<String>,
    items: RwSignal<Vec<PaletteItem>>,
    selected: RwSignal<usize>,
    open: RwSignal<bool>,
) -> impl View {
    v_stack((
        search_row(query),
        results_list(items, selected, open),
        footer(items),
    ))
    .on_click_stop(|_| {}) // swallow clicks so backdrop doesn't close
    .style(|s| {
        s.width_pct(100.0)
            .max_height_pct(70.0)
            .background(BG_RAISED)
            .border_radius(R_10)
            .box_shadow_blur(48.0)
            .box_shadow_color(floem::peniko::Color::from_rgba8(0, 0, 0, 0x8C))
            .box_shadow_v_offset(18.0)
            .flex_col()
    })
}

fn search_row(query: RwSignal<String>) -> impl View {
    h_stack((
        label(|| "⌕".to_string()).style(|s| s.color(FG_3).font_size(T_MD).margin_right(10.0)),
        text_input(query)
            .placeholder("Search commands, files, OpModes…")
            .keyboard_navigable()
            .style(|s| {
                s.flex_grow(1.0)
                    .color(FG_1)
                    .background(floem::peniko::Color::TRANSPARENT)
                    .font_size(T_BASE)
            }),
        label(|| "Esc".to_string()).style(|s| {
            s.font_family("monospace".to_string())
                .font_size(T_MICRO)
                .color(FG_4)
                .background(BG_NAVY)
                .padding_horiz(5.0)
                .padding_vert(1.0)
                .border_radius(3.0)
        }),
    ))
    .style(|s| {
        s.height(44.0)
            .padding_horiz(14.0)
            .items_center()
            .border_bottom(1.0)
            .border_color(BG_EDGE)
    })
}

fn results_list(
    items: RwSignal<Vec<PaletteItem>>,
    selected: RwSignal<usize>,
    open: RwSignal<bool>,
) -> impl View {
    scroll(
        dyn_stack(
            move || {
                let v = items.get();
                let mut out: Vec<(usize, PaletteRow)> = Vec::with_capacity(v.len() * 2);
                let mut current_group = String::new();
                let mut idx: usize = 0;
                for (i, it) in v.into_iter().enumerate() {
                    if it.group != current_group {
                        current_group = it.group.clone();
                        out.push((100_000 + i, PaletteRow::Header(current_group.clone())));
                    }
                    out.push((idx, PaletteRow::Item(i, it)));
                    idx += 1;
                }
                out
            },
            |(k, _)| *k,
            move |(_, row)| match row {
                PaletteRow::Header(name) => v_stack((label(move || name.clone()).style(|s| {
                    s.padding_horiz(16.0)
                        .padding_top(10.0)
                        .padding_bottom(4.0)
                        .color(FG_3)
                        .font_size(T_MICRO)
                        .font_weight(floem::text::FontWeight::BOLD)
                }),))
                .into_any(),
                PaletteRow::Item(i, it) => palette_item_row(i, it, selected, open).into_any(),
            },
        )
        .style(|s| s.flex_col().width_pct(100.0)),
    )
    .style(|s| {
        s.width_pct(100.0)
            .flex_grow(1.0)
            .padding_vert(4.0)
            .background(BG_RAISED)
    })
}

enum PaletteRow {
    Header(String),
    Item(usize, PaletteItem),
}

fn palette_item_row(
    idx: usize,
    it: PaletteItem,
    selected: RwSignal<usize>,
    open: RwSignal<bool>,
) -> impl View {
    let label_text = it.label.clone();
    let sub_text = it.sub.clone();
    let kbd = it.kbd.clone();
    let on_run = it.on_run.clone();
    let tone_color = match it.tone {
        PaletteTone::Success => STATUS_SUCCESS,
        PaletteTone::Info => STATUS_INFO,
        PaletteTone::Warning => STATUS_WARNING,
        PaletteTone::Default => FG_3,
    };
    let icon_glyph = it.icon;

    h_stack((
        // active stripe
        container(empty()).style(move |s| {
            let s = s
                .absolute()
                .width(2.0)
                .height_pct(80.0)
                .background(ALLOY_ORANGE)
                .border_radius(R_2)
                .margin_left(0.0);
            if selected.get() == idx {
                s
            } else {
                s.hide()
            }
        }),
        // icon
        label(move || icon_glyph.to_string())
            .style(move |s| s.color(tone_color).font_size(T_MD).margin_right(10.0)),
        // text
        v_stack((
            label(move || label_text.clone()).style(|s| s.color(FG_1).font_size(T_BASE)),
            label(move || sub_text.clone())
                .style(|s| s.color(FG_3).font_size(T_TINY).text_ellipsis()),
        ))
        .style(|s| s.flex_grow(1.0).min_width(0.0).gap(2.0)),
        // kbd
        container(label(move || kbd.clone().unwrap_or_default()).style(|s| {
            s.font_family("monospace".to_string())
                .font_size(T_MICRO)
                .color(FG_3)
                .padding_horiz(6.0)
                .padding_vert(2.0)
                .border_radius(3.0)
                .background(BG_NAVY)
        }))
        .style(move |s| {
            let has = it.kbd.is_some();
            if has {
                s
            } else {
                s.hide()
            }
        }),
    ))
    .on_event_stop(floem::event::listener::PointerEnter, move |_cx, _info| {
        selected.set(idx);
    })
    .on_click_stop(move |_| {
        (on_run)();
        open.set(false);
    })
    .style(move |s| {
        let active = selected.get() == idx;
        s.width_pct(100.0)
            .padding_horiz(16.0)
            .padding_vert(7.0)
            .gap(0.0)
            .items_center()
            .cursor(CursorStyle::Pointer)
            .apply_if(active, |s| s.background(BG_HOVER))
    })
}

fn footer(items: RwSignal<Vec<PaletteItem>>) -> impl View {
    h_stack((
        kbd_hint("↵", "Open"),
        kbd_hint("↑↓", "Navigate"),
        kbd_hint("⌘P", "Files only"),
        container(empty()).style(|s| s.flex_grow(1.0f32)),
        label(move || format!("{} results", items.get().len()))
            .style(|s| s.color(FG_3).font_size(T_MICRO)),
    ))
    .style(|s| {
        s.height(28.0)
            .padding_horiz(14.0)
            .gap(16.0)
            .items_center()
            .background(BG_SURFACE)
            .border_top(1.0)
            .border_color(BG_EDGE)
            .color(FG_3)
            .font_size(T_MICRO)
    })
}

fn kbd_hint(k: &'static str, lbl: &'static str) -> impl View {
    h_stack((
        label(move || k.to_string()).style(|s| {
            s.font_family("monospace".to_string())
                .font_size(9.0)
                .color(FG_2)
                .padding_horiz(4.0)
                .padding_vert(1.0)
                .border_radius(R_2)
                .background(BG_RAISED)
                .border(1.0)
                .border_color(BG_EDGE)
                .margin_right(4.0)
        }),
        label(move || lbl.to_string()).style(|s| s.color(FG_3).font_size(T_MICRO)),
    ))
    .style(|s| s.items_center())
}
use floem::views::scroll::scroll;
