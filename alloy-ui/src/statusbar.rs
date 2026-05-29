//! Alloy status bar — 25 px strip at the bottom of every window.
//!
//! Left cluster:  branch · LSP · Robot · Problems
//! Right cluster: cursor · file lang · encoding · indent · gear
//!
//! Reference: kit/StatusBar.jsx.

use std::sync::Arc;

use floem::reactive::{RwSignal, SignalGet};
use floem::style::CursorStyle;
use floem::views::{container, empty, h_stack, label, Decorators};
use floem::View;

use crate::theme::*;

#[derive(Clone, Copy, PartialEq)]
pub enum LspState {
    Loading,
    Ready,
    Error,
}

#[derive(Clone, Copy, PartialEq)]
pub enum RobotState {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Clone)]
pub struct StatusBarSignals {
    pub branch: RwSignal<String>,
    pub ahead: RwSignal<u32>,
    pub behind: RwSignal<u32>,
    pub lsp: RwSignal<LspState>,
    pub robot: RwSignal<RobotState>,
    pub error_count: RwSignal<u32>,
    pub warn_count: RwSignal<u32>,
    pub cursor_line: RwSignal<u32>,
    pub cursor_col: RwSignal<u32>,
    pub file_lang: RwSignal<String>,
    pub encoding: RwSignal<String>,
    pub indent: RwSignal<String>,
}

#[derive(Clone)]
pub struct StatusBarHandlers {
    pub on_branch: Arc<dyn Fn()>,
    pub on_lsp: Arc<dyn Fn()>,
    pub on_robot: Arc<dyn Fn()>,
    pub on_problems: Arc<dyn Fn()>,
    pub on_lang: Arc<dyn Fn()>,
    pub on_encoding: Arc<dyn Fn()>,
    pub on_indent: Arc<dyn Fn()>,
    pub on_settings: Arc<dyn Fn()>,
}

pub fn alloy_status_bar(sigs: StatusBarSignals, h: StatusBarHandlers) -> impl View {
    h_stack((
        branch_chip(&sigs, h.on_branch.clone()),
        lsp_chip(&sigs, h.on_lsp.clone()),
        robot_chip(&sigs, h.on_robot.clone()),
        problems_chip(&sigs, h.on_problems.clone()),
        container(empty()).style(|s| s.flex_grow(1.0f32)),
        cursor_chip(&sigs),
        plain_chip(sigs.file_lang, h.on_lang.clone()),
        plain_chip(sigs.encoding, h.on_encoding.clone()),
        plain_chip(sigs.indent, h.on_indent.clone()),
        gear_chip(h.on_settings.clone()),
    ))
    .style(|s| {
        s.width_pct(100.0)
            .height(UI_STATUS_HEIGHT)
            .background(BG_SURFACE)
            .border_top(1.0)
            .border_color(BG_EDGE)
            .padding_horiz(6.0)
            .items_center()
            .gap(2.0)
    })
}

// ── Chip primitives ──────────────────────────────────────────────────────────

fn chip<C: View + 'static>(body: C, on_click: Arc<dyn Fn()>) -> impl View {
    container(body)
        .on_click_stop(move |_| (on_click)())
        .style(|s| {
            s.padding_horiz(8.0)
                .padding_vert(2.0)
                .height(20.0)
                .items_center()
                .border_radius(R_4)
                .cursor(CursorStyle::Pointer)
                .hover(|s| s.background(BG_HOVER))
        })
}

fn dot(color: floem::peniko::Color) -> impl View {
    container(empty()).style(move |s| {
        s.width(6.0)
            .height(6.0)
            .border_radius(R_FULL)
            .background(color)
            .margin_right(6.0)
    })
}

// ── Specific chips ───────────────────────────────────────────────────────────

fn branch_chip(s: &StatusBarSignals, on_click: Arc<dyn Fn()>) -> impl View {
    let branch = s.branch;
    let ahead = s.ahead;
    let behind = s.behind;
    chip(
        h_stack((
            label(|| "⎇".to_string())
                .style(|s| s.color(ALLOY_ORANGE).font_size(T_TINY).margin_right(6.0)),
            label(move || branch.get()).style(|s| {
                s.color(FG_1)
                    .font_size(T_TINY)
                    .font_family("monospace".to_string())
            }),
            label(move || format!("  ↑{} ↓{}", ahead.get(), behind.get())).style(|s| {
                s.color(FG_3)
                    .font_size(T_MICRO)
                    .font_family("monospace".to_string())
            }),
        ))
        .style(|s| s.items_center()),
        on_click,
    )
}

fn lsp_chip(s: &StatusBarSignals, on_click: Arc<dyn Fn()>) -> impl View {
    let lsp = s.lsp;
    chip(
        h_stack((
            container(empty()).style(move |s| {
                let c = match lsp.get() {
                    LspState::Ready => STATUS_SUCCESS,
                    LspState::Loading => STATUS_WARNING,
                    LspState::Error => STATUS_ERROR,
                };
                s.width(6.0)
                    .height(6.0)
                    .border_radius(R_FULL)
                    .background(c)
                    .margin_right(6.0)
            }),
            label(move || match lsp.get() {
                LspState::Ready => "JDTLS ready".to_string(),
                LspState::Loading => "JDTLS loading…".to_string(),
                LspState::Error => "JDTLS error".to_string(),
            })
            .style(|s| s.color(FG_2).font_size(T_TINY)),
        ))
        .style(|s| s.items_center()),
        on_click,
    )
}

fn robot_chip(s: &StatusBarSignals, on_click: Arc<dyn Fn()>) -> impl View {
    let r = s.robot;
    chip(
        h_stack((
            container(empty()).style(move |s| {
                let c = match r.get() {
                    RobotState::Connected => STATUS_SUCCESS,
                    RobotState::Connecting => STATUS_WARNING,
                    RobotState::Disconnected => STATUS_ERROR,
                };
                s.width(6.0)
                    .height(6.0)
                    .border_radius(R_FULL)
                    .background(c)
                    .margin_right(6.0)
            }),
            label(move || match r.get() {
                RobotState::Connected => "Robot connected".to_string(),
                RobotState::Connecting => "Robot connecting…".to_string(),
                RobotState::Disconnected => "Robot offline".to_string(),
            })
            .style(|s| s.color(FG_2).font_size(T_TINY)),
        ))
        .style(|s| s.items_center()),
        on_click,
    )
}

fn problems_chip(s: &StatusBarSignals, on_click: Arc<dyn Fn()>) -> impl View {
    let e = s.error_count;
    let w = s.warn_count;
    chip(
        h_stack((
            label(|| "⊘".to_string())
                .style(|s| s.color(STATUS_ERROR).font_size(T_TINY).margin_right(4.0)),
            label(move || e.get().to_string())
                .style(|s| s.color(FG_2).font_size(T_TINY).margin_right(8.0)),
            label(|| "⚠".to_string())
                .style(|s| s.color(STATUS_WARNING).font_size(T_TINY).margin_right(4.0)),
            label(move || w.get().to_string()).style(|s| s.color(FG_2).font_size(T_TINY)),
        ))
        .style(|s| s.items_center()),
        on_click,
    )
}

fn cursor_chip(s: &StatusBarSignals) -> impl View {
    let l = s.cursor_line;
    let c = s.cursor_col;
    container(
        label(move || format!("Ln {}, Col {}", l.get(), c.get())).style(|s| {
            s.color(FG_2)
                .font_size(T_TINY)
                .font_family("monospace".to_string())
        }),
    )
    .style(|s| s.padding_horiz(8.0).items_center())
}

fn plain_chip(text_sig: RwSignal<String>, on_click: Arc<dyn Fn()>) -> impl View {
    chip(
        label(move || text_sig.get()).style(|s| s.color(FG_2).font_size(T_TINY)),
        on_click,
    )
}

fn gear_chip(on_click: Arc<dyn Fn()>) -> impl View {
    container(label(|| "⚙".to_string()).style(|s| s.color(FG_3).font_size(T_BASE)))
        .on_click_stop(move |_| (on_click)())
        .style(|s| {
            s.width(22.0)
                .height(22.0)
                .border_radius(R_4)
                .items_center()
                .justify_center()
                .cursor(CursorStyle::Pointer)
                .hover(|s| s.background(BG_HOVER).color(FG_1))
        })
}
