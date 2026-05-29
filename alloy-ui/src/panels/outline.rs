//! Outline panel — right-rail document-symbol tree.
//!
//! Pure presentation. The data source is `lapce-app::editor::active editor's
//! document symbols` (already an LSP result with depth via parent links).
//! Caller maps `lsp_types::DocumentSymbol` to `OutlineSymbol` and feeds it.
//!
//! Reference: kit/OutlinePanel.jsx.

use floem::View;
use floem::reactive::{RwSignal, SignalGet};
use floem::style::CursorStyle;
use floem::views::{Decorators, container, dyn_stack, empty, h_stack, label, scroll, v_stack};

use crate::theme::*;

#[derive(Clone, Copy, PartialEq)]
pub enum SymKind {
    Class,
    Interface,
    Enum,
    Method,
    Function,
    Field,
    Variable,
    Constant,
    Property,
    Constructor,
    Module,
    Namespace,
    Other,
}

impl SymKind {
    fn glyph(self) -> &'static str {
        // Single-char glyphs so we don't need codicon SVG plumbing here.
        // lapce-app should swap these for `svg(LapceIcons::SYMBOL_*)` calls.
        match self {
            SymKind::Class       => "ⓒ",
            SymKind::Interface   => "ⓘ",
            SymKind::Enum        => "ⓔ",
            SymKind::Method      => "ⓜ",
            SymKind::Function    => "ƒ",
            SymKind::Field       => "▪",
            SymKind::Variable    => "ν",
            SymKind::Constant    => "κ",
            SymKind::Property    => "π",
            SymKind::Constructor => "ⓒ",
            SymKind::Module      => "▤",
            SymKind::Namespace   => "▦",
            SymKind::Other       => "·",
        }
    }
    fn color(self) -> floem::peniko::Color {
        match self {
            SymKind::Class | SymKind::Interface | SymKind::Enum => SYN_TYPE,
            SymKind::Method | SymKind::Function | SymKind::Constructor => SYN_FUNCTION,
            SymKind::Field | SymKind::Property | SymKind::Variable => SYN_VARIABLE,
            SymKind::Constant => SYN_NUMBER,
            SymKind::Module | SymKind::Namespace => SYN_BUILTIN,
            SymKind::Other => FG_3,
        }
    }
}

#[derive(Clone)]
pub struct OutlineSymbol {
    pub kind:        SymKind,
    pub name:        String,
    pub return_type: String,
    pub line:        u32,
    pub depth:       u8,
    pub active:      bool,
}

pub fn outline_panel(
    file_name: RwSignal<String>,
    symbols:   RwSignal<Vec<OutlineSymbol>>,
    on_pick:   std::sync::Arc<dyn Fn(u32)>, // jump to line N in active editor
) -> impl View {
    v_stack((
        header(file_name),
        body(symbols, on_pick),
        footer(symbols),
    ))
    .style(|s| {
        s.flex_col()
            .width_pct(100.0)
            .height_pct(100.0)
            .background(BG_SURFACE)
    })
}

fn header(file_name: RwSignal<String>) -> impl View {
    h_stack((
        label(|| "Outline".to_string()).style(|s| {
            s.flex_grow(1.0)
                .color(FG_2)
                .font_size(T_TINY)
                .font_weight(floem::text::Weight::BOLD)
        }),
        label(move || file_name.get()).style(|s| {
            s.color(FG_3)
                .font_size(T_MICRO)
                .font_family("monospace".to_string())
        }),
    ))
    .style(|s| {
        s.height(36.0)
            .padding_horiz(14.0)
            .items_center()
            .border_bottom(1.0)
            .border_color(BG_EDGE)
            .flex_shrink(0.0)
    })
}

fn body(
    symbols: RwSignal<Vec<OutlineSymbol>>,
    on_pick: std::sync::Arc<dyn Fn(u32)>,
) -> impl View {
    scroll(
        dyn_stack(
            move || symbols.get().into_iter().enumerate().collect::<Vec<_>>(),
            |(i, _)| *i,
            move |(_, sym)| symbol_row(sym, on_pick.clone()),
        )
        .style(|s| s.flex_col().padding_vert(6.0).width_pct(100.0)),
    )
    .style(|s| s.flex_grow(1.0).width_pct(100.0))
}

fn symbol_row(sym: OutlineSymbol, on_pick: std::sync::Arc<dyn Fn(u32)>) -> impl View {
    let glyph = sym.kind.glyph();
    let icon_color = sym.kind.color();
    let active = sym.active;
    let depth_pad = 12.0 + (sym.depth as f64) * 14.0;
    let line_no = sym.line;
    let pick = on_pick.clone();

    container(
        h_stack((
            // active stripe
            container(empty()).style(move |s| {
                let s = s
                    .absolute()
                    .width(2.0)
                    .height_pct(80.0)
                    .background(ALLOY_ORANGE)
                    .border_radius(R_2);
                if active { s } else { s.hide() }
            }),
            // icon
            label(move || glyph.to_string())
                .style(move |s| s.color(icon_color).font_size(T_BASE).margin_right(8.0)),
            // name + type
            h_stack((
                label({
                    let n = sym.name.clone();
                    move || n.clone()
                })
                .style(move |s| {
                    let bold = matches!(sym.kind, SymKind::Class | SymKind::Interface | SymKind::Enum);
                    s.color(FG_1).font_size(T_SMALL).apply_if(bold, |s| {
                        s.font_weight(floem::text::Weight::BOLD)
                    })
                }),
                label({
                    let r = sym.return_type.clone();
                    move || format!(": {}", r)
                })
                .style(|s| s.color(FG_4).font_size(T_TINY).margin_left(6.0)),
            ))
            .style(|s| s.flex_grow(1.0).min_width(0.0).items_center()),
            // line number
            label(move || line_no.to_string()).style(|s| {
                s.color(FG_4).font_size(T_MICRO).font_family("monospace".to_string())
            }),
        ))
        .style(|s| s.items_center().width_pct(100.0)),
    )
    .on_click_stop(move |_| (pick)(line_no))
    .style(move |s| {
        let s = s
            .padding_top(4.0)
            .padding_bottom(4.0)
            .padding_left(depth_pad)
            .padding_right(14.0)
            .width_pct(100.0)
            .font_family("monospace".to_string())
            .cursor(CursorStyle::Pointer)
            .hover(|s| s.background(BG_HOVER));
        if active {
            s.background(BG_CURRENT).color(FG_1)
        } else {
            s.color(FG_2)
        }
    })
}

fn footer(symbols: RwSignal<Vec<OutlineSymbol>>) -> impl View {
    label(move || {
        let v = symbols.get();
        let classes = v.iter().filter(|s| matches!(s.kind, SymKind::Class | SymKind::Interface | SymKind::Enum)).count();
        let methods = v.iter().filter(|s| matches!(s.kind, SymKind::Method | SymKind::Function | SymKind::Constructor)).count();
        format!("Symbols provided by LSP · {classes} {} · {methods} {}",
            if classes == 1 { "class" } else { "classes" },
            if methods == 1 { "method" } else { "methods" })
    })
    .style(|s| {
        s.padding_horiz(14.0)
            .padding_vert(6.0)
            .color(FG_3)
            .font_size(T_MICRO)
            .border_top(1.0)
            .border_color(BG_EDGE)
            .flex_shrink(0.0)
    })
}
