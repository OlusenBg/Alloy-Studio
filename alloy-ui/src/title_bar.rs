//! Alloy title bar — 36 px chrome strip across the top of the window.
//!
//! Layout (left → right):
//!   • traffic-light triplet (close / minimize / maximise — functional on all platforms)
//!   • drag region (double-click to toggle maximise)
//!   • Alloy logo + Home chevron
//!   • flex spacer (drag region continues through here)
//!   • centered command pill (project · team · branch · ⌘K)
//!   • flex spacer
//!   • Deploy button
//!   • Settings icon
//!   • update dot
//!   • right drag region

use std::sync::Arc;

use floem::reactive::{RwSignal, SignalGet};
use floem::style::CursorStyle;
use floem::views::{container, drag_window_area, empty, h_stack, label, Decorators};
use floem::View;

use crate::theme::*;

static ALLOY_LOGO: &[u8] = include_bytes!("../extra/images/logo.png");

#[derive(Clone)]
pub struct TitleBarHandlers {
    pub on_home: Arc<dyn Fn()>,
    pub on_palette: Arc<dyn Fn()>,
    pub on_settings: Arc<dyn Fn()>,
    pub on_run: Arc<dyn Fn()>,
}

pub fn alloy_title_bar(
    project_name: RwSignal<String>,
    team: RwSignal<String>,
    branch: RwSignal<String>,
    has_update: RwSignal<bool>,
    workspace_open: RwSignal<bool>,
    show_run: RwSignal<bool>,
    h: TitleBarHandlers,
) -> impl View {
    // Wrap the entire bar in a drag_window_area so users can grab anywhere
    // that isn't an interactive control to move the window.
    drag_window_area(
        h_stack((
            traffic_lights(),
            logo_and_home(workspace_open, h.on_home.clone()),
            spacer(),
            command_pill(project_name, team, branch, h.on_palette.clone()),
            spacer(),
            right_cluster(
                has_update,
                show_run,
                h.on_run.clone(),
                h.on_settings.clone(),
            ),
        ))
        .style(|s| {
            s.width_pct(100.0)
                .height(UI_HEADER_HEIGHT)
                .background(BG_SURFACE)
                .border_bottom(1.0)
                .border_color(BG_EDGE)
                .padding_horiz(8.0)
                .items_center()
                .gap(8.0)
        }),
    )
}

// ── pieces ────────────────────────────────────────────────────────────────────

fn spacer() -> impl View {
    container(empty()).style(|s| s.flex_grow(1.0f32))
}

/// Three traffic-light buttons — functional on every platform.
fn traffic_lights() -> impl View {
    h_stack((
        // Red — close
        traffic_dot(floem::peniko::Color::from_rgb8(0xFF, 0x5F, 0x57), || {
            floem::quit_app()
        }),
        // Yellow — minimise
        traffic_dot(floem::peniko::Color::from_rgb8(0xFE, 0xBC, 0x2E), || {
            floem::action::minimize_window()
        }),
        // Green — toggle maximise / fullscreen
        traffic_dot(floem::peniko::Color::from_rgb8(0x28, 0xC8, 0x40), || {
            floem::action::toggle_window_maximized()
        }),
    ))
    .style(|s| s.gap(8.0).padding_horiz(6.0).items_center())
}

fn traffic_dot(color: floem::peniko::Color, action: impl Fn() + 'static) -> impl View {
    container(empty())
        .on_click_stop(move |_| action())
        .style(move |s| {
            s.width(12.0)
                .height(12.0)
                .border_radius(R_FULL)
                .background(color)
                .cursor(CursorStyle::Pointer)
                .hover(|s| s.opacity(0.75))
        })
}

fn logo_and_home(workspace_open: RwSignal<bool>, on_home: Arc<dyn Fn()>) -> impl View {
    h_stack((
        floem::views::img(|| ALLOY_LOGO.to_vec()).style(|s| s.width(18.0).height(18.0)),
        container(home_chevron(on_home)).style(
            move |s| {
                if workspace_open.get() {
                    s
                } else {
                    s.hide()
                }
            },
        ),
    ))
    .style(|s| s.items_center().gap(8.0))
}

fn home_chevron(on_home: Arc<dyn Fn()>) -> impl View {
    container(
        h_stack((
            label(|| "‹".to_string()).style(|s| s.color(FG_2).font_size(T_SMALL).margin_right(4.0)),
            label(|| "Home".to_string()).style(|s| s.color(FG_2).font_size(T_TINY)),
        ))
        .style(|s| s.items_center()),
    )
    .on_click_stop(move |_| (on_home)())
    .style(|s| {
        s.padding_horiz(8.0)
            .padding_vert(3.0)
            .border_radius(R_4)
            .background(BG_RAISED)
            .cursor(CursorStyle::Pointer)
            .hover(|s| s.background(BG_HOVER))
    })
}

fn command_pill(
    project_name: RwSignal<String>,
    team: RwSignal<String>,
    branch: RwSignal<String>,
    on_palette: Arc<dyn Fn()>,
) -> impl View {
    container(
        h_stack((
            label(|| "▸".to_string())
                .style(|s| s.color(ALLOY_ORANGE).font_size(T_SMALL).margin_right(2.0)),
            label(move || project_name.get()).style(|s| {
                s.color(FG_1)
                    .font_size(T_SMALL)
                    .font_weight(floem::text::FontWeight::MEDIUM)
            }),
            label(|| "·".to_string()).style(|s| s.color(FG_4).font_size(T_SMALL).margin_horiz(6.0)),
            label(move || team.get()).style(|s| s.color(FG_3).font_size(T_SMALL)),
            label(|| "·".to_string()).style(|s| s.color(FG_4).font_size(T_SMALL).margin_horiz(6.0)),
            label(|| "⎇".to_string()).style(|s| s.color(FG_3).font_size(T_TINY).margin_right(4.0)),
            label(move || branch.get()).style(|s| s.color(FG_3).font_size(T_SMALL)),
            container(empty()).style(|s| s.flex_grow(1.0f32)),
            label(|| "⌘K".to_string()).style(|s| {
                s.font_family("monospace".to_string())
                    .font_size(T_MICRO)
                    .color(FG_4)
                    .background(BG_NAVY)
                    .padding_horiz(5.0)
                    .padding_vert(1.0)
                    .border_radius(3.0)
            }),
        ))
        .style(|s| s.items_center().width_pct(100.0)),
    )
    .on_click_stop(move |_| (on_palette)())
    .style(|s| {
        s.height(24.0)
            .min_width(380.0)
            .max_width(520.0)
            .padding_horiz(10.0)
            .background(BG_RAISED)
            .border(1.0)
            .border_color(BG_EDGE)
            .border_radius(R_6)
            .cursor(CursorStyle::Text)
            .items_center()
            .hover(|s| s.background(BG_HOVER))
    })
}

fn right_cluster(
    has_update: RwSignal<bool>,
    show_run: RwSignal<bool>,
    on_run: Arc<dyn Fn()>,
    on_settings: Arc<dyn Fn()>,
) -> impl View {
    h_stack((
        container(run_button(on_run.clone())).style(
            move |s| {
                if show_run.get() {
                    s
                } else {
                    s.hide()
                }
            },
        ),
        title_icon_button("⚙", on_settings.clone()),
        update_dot(has_update),
    ))
    .style(|s| s.items_center().gap(4.0))
}

fn run_button(on_run: Arc<dyn Fn()>) -> impl View {
    container(
        h_stack((
            label(|| "▶".to_string()).style(|s| s.color(FG_1).font_size(T_SMALL).margin_right(6.0)),
            label(|| "Deploy".to_string()).style(|s| {
                s.color(FG_1)
                    .font_size(T_TINY)
                    .font_weight(floem::text::FontWeight::SEMI_BOLD)
            }),
        ))
        .style(|s| s.items_center()),
    )
    .on_click_stop(move |_| (on_run)())
    .style(|s| {
        s.padding_horiz(10.0)
            .padding_vert(4.0)
            .border_radius(R_4)
            .background(ALLOY_ORANGE)
            .cursor(CursorStyle::Pointer)
            .box_shadow_blur(12.0)
            .box_shadow_color(ALLOY_ORANGE_GLOW)
            .box_shadow_v_offset(4.0)
            .border(1.0)
            .border_color(ALLOY_ORANGE_GLOW)
            .hover(|s| s.background(ALLOY_ORANGE_DEEP))
    })
}

fn title_icon_button(glyph: &'static str, on_click: Arc<dyn Fn()>) -> impl View {
    container(label(move || glyph.to_string()).style(|s| s.color(FG_3).font_size(T_MD)))
        .on_click_stop(move |_| (on_click)())
        .style(|s| {
            s.width(26.0)
                .height(26.0)
                .border_radius(R_4)
                .items_center()
                .justify_center()
                .cursor(CursorStyle::Pointer)
                .hover(|s| s.background(BG_HOVER).color(FG_1))
        })
}

fn update_dot(has_update: RwSignal<bool>) -> impl View {
    container(empty()).style(move |s| {
        let s = s
            .width(7.0)
            .height(7.0)
            .border_radius(R_FULL)
            .background(ALLOY_ORANGE)
            .margin_left(2.0)
            .margin_right(6.0)
            .box_shadow_blur(6.0)
            .box_shadow_color(ALLOY_ORANGE_GLOW);
        if has_update.get() {
            s
        } else {
            s.hide()
        }
    })
}
