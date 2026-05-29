//! OpMode panel — lists TeleOp and Autonomous op modes with run/open callbacks.
//!
//! Reference: kit/OpModePanel.jsx.

use std::sync::Arc;

use floem::style::CursorStyle;
use floem::views::{container, dyn_stack, empty, h_stack, label, scroll, v_stack, Decorators};
use floem::View;

use crate::theme::*;

#[derive(Clone, Copy)]
pub struct OpMode {
    pub cls: &'static str,
    pub file: &'static str,
    pub group: &'static str,
    pub disabled: bool,
}

static TELEOP: &[OpMode] = &[
    OpMode {
        cls: "CenterStageTeleOp",
        file: "OpMode.java",
        group: "Driver",
        disabled: false,
    },
    OpMode {
        cls: "ArmTuner",
        file: "ArmTuner.java",
        group: "Tuning",
        disabled: true,
    },
];

static AUTO: &[OpMode] = &[
    OpMode {
        cls: "RedAutoStart",
        file: "RedAutoStart.java",
        group: "Red Side",
        disabled: false,
    },
    OpMode {
        cls: "BlueAutoStart",
        file: "BlueAutoStart.java",
        group: "Blue Side",
        disabled: false,
    },
    OpMode {
        cls: "ParkOnly",
        file: "ParkOnly.java",
        group: "Backup",
        disabled: false,
    },
];

pub fn opmode_panel(
    on_run: Arc<dyn Fn(&'static str)>,
    on_open_file: Arc<dyn Fn(&'static str)>,
) -> impl View {
    let on_run_teleop = on_run.clone();
    let on_run_auto = on_run.clone();
    let on_open_teleop = on_open_file.clone();
    let on_open_auto = on_open_file.clone();

    v_stack((
        panel_header(),
        scroll(
            v_stack((
                section_header("TELEOP", STATUS_SUCCESS, TELEOP.len()),
                teleop_rows(on_run_teleop, on_open_teleop),
                container(empty()).style(|s| s.height(12.0)),
                section_header("AUTONOMOUS", STATUS_INFO, AUTO.len()),
                auto_rows(on_run_auto, on_open_auto),
            ))
            .style(|s| {
                s.flex_col()
                    .padding_horiz(8.0)
                    .padding_vert(8.0)
                    .width_pct(100.0)
            }),
        )
        .style(|s| s.flex_grow(1.0).width_pct(100.0)),
        panel_footer(),
    ))
    .style(|s| {
        s.flex_col()
            .width_pct(100.0)
            .height_pct(100.0)
            .background(BG_SURFACE)
    })
}

fn panel_header() -> impl View {
    h_stack((
        label(|| "OpModes".to_string()).style(|s| {
            s.color(FG_2)
                .font_size(T_TINY)
                .font_weight(floem::text::Weight::BOLD)
                .flex_grow(1.0f32)
        }),
        label(|| "⟳".to_string()).style(|s| {
            s.color(FG_3)
                .font_size(T_BASE)
                .cursor(CursorStyle::Pointer)
                .hover(|s| s.color(FG_1))
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

fn section_header(title: &'static str, dot_color: floem::peniko::Color, count: usize) -> impl View {
    h_stack((
        // colored square dot
        container(empty()).style(move |s| {
            s.width(6.0)
                .height(6.0)
                .background(dot_color)
                .border_radius(R_2)
                .margin_right(6.0)
                .flex_shrink(0.0)
        }),
        label(move || title.to_string()).style(|s| {
            s.color(FG_3)
                .font_size(T_MICRO)
                .font_weight(floem::text::Weight::BOLD)
                .margin_right(6.0)
        }),
        label(move || count.to_string()).style(|s| s.color(FG_4).font_size(T_MICRO)),
        // horizontal rule
        container(empty()).style(|s| {
            s.flex_grow(1.0f32)
                .height(1.0)
                .background(BG_EDGE)
                .margin_left(8.0)
        }),
    ))
    .style(|s| s.items_center().padding_vert(6.0).width_pct(100.0))
}

fn teleop_rows(
    on_run: Arc<dyn Fn(&'static str)>,
    on_open_file: Arc<dyn Fn(&'static str)>,
) -> impl View {
    dyn_stack(
        || TELEOP.iter().enumerate().collect::<Vec<_>>(),
        |(i, _)| *i,
        move |(_, op)| opmode_row(*op, STATUS_SUCCESS, on_run.clone(), on_open_file.clone()),
    )
    .style(|s| s.flex_col().width_pct(100.0))
}

fn auto_rows(
    on_run: Arc<dyn Fn(&'static str)>,
    on_open_file: Arc<dyn Fn(&'static str)>,
) -> impl View {
    dyn_stack(
        || AUTO.iter().enumerate().collect::<Vec<_>>(),
        |(i, _)| *i,
        move |(_, op)| opmode_row(*op, STATUS_INFO, on_run.clone(), on_open_file.clone()),
    )
    .style(|s| s.flex_col().width_pct(100.0))
}

fn opmode_row(
    op: OpMode,
    accent_color: floem::peniko::Color,
    on_run: Arc<dyn Fn(&'static str)>,
    on_open_file: Arc<dyn Fn(&'static str)>,
) -> impl View {
    let disabled = op.disabled;
    let open_cb = on_open_file.clone();

    container(
        h_stack((
            // 2px × 28px accent bar
            container(empty()).style(move |s| {
                s.width(2.0)
                    .height(28.0)
                    .background(accent_color)
                    .border_radius(R_2)
                    .margin_right(8.0)
                    .flex_shrink(0.0)
            }),
            // body: class name + file name + disabled note
            v_stack((
                label(move || op.cls.to_string()).style(|s| {
                    s.color(FG_1)
                        .font_size(T_SMALL)
                        .font_weight(floem::text::Weight::SEMIBOLD)
                        .font_family("monospace".to_string())
                        .min_width(0.0)
                }),
                h_stack((
                    label(move || op.file.to_string()).style(|s| s.color(FG_3).font_size(T_MICRO)),
                    label(move || if disabled { " (disabled)" } else { "" }.to_string())
                        .style(|s| s.color(FG_4).font_size(T_MICRO).margin_left(4.0)),
                ))
                .style(|s| s.items_center()),
            ))
            .style(|s| s.flex_grow(1.0f32).min_width(0.0).justify_center()),
            // Run button
            label(|| "▶".to_string())
                .on_click_stop(move |_| (on_run)(op.cls))
                .style(move |s| {
                    s.color(ALLOY_ORANGE)
                        .font_size(T_MICRO)
                        .font_weight(floem::text::Weight::BOLD)
                        .padding_horiz(8.0)
                        .padding_vert(3.0)
                        .border_radius(R_4)
                        .cursor(CursorStyle::Pointer)
                        .apply_if(disabled, |s| s.hide())
                        .hover(|s| s.background(ALLOY_ORANGE_SOFT))
                }),
        ))
        .style(|s| s.items_center().width_pct(100.0)),
    )
    .on_click_stop(move |_| (open_cb)(op.file))
    .style(move |s| {
        let s = s
            .background(BG_RAISED)
            .border_radius(R_4)
            .padding_horiz(10.0)
            .padding_vert(8.0)
            .margin_bottom(4.0)
            .width_pct(100.0)
            .cursor(CursorStyle::Pointer)
            .hover(|s| s.background(BG_HOVER));
        if disabled {
            s.opacity(0.55)
        } else {
            s
        }
    })
}

fn panel_footer() -> impl View {
    v_stack((
        footer_new_btn("+ New TeleOp from template\u{2026}", STATUS_SUCCESS),
        footer_new_btn("+ New Autonomous from template\u{2026}", STATUS_INFO),
    ))
    .style(|s| {
        s.background(BG_SURFACE)
            .border_top(1.0)
            .border_color(BG_EDGE)
            .padding_horiz(10.0)
            .padding_vert(8.0)
            .flex_col()
            .gap(0.0, 4.0)
            .flex_shrink(0.0)
    })
}

fn footer_new_btn(text: &'static str, color: floem::peniko::Color) -> impl View {
    label(move || text.to_string()).style(move |s| {
        s.width_pct(100.0)
            .padding_horiz(10.0)
            .padding_vert(5.0)
            .color(color)
            .font_size(T_TINY)
            .border(1.0)
            .border_color(LINE_RING)
            .border_radius(R_4)
            .cursor(CursorStyle::Pointer)
            .hover(|s| s.background(BG_HOVER))
    })
}
