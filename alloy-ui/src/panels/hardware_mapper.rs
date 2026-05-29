//! Hardware mapper panel — visual port assignment for FTC hubs.
//!
//! Reference: kit/HardwareMapper.jsx.

use floem::View;
use floem::reactive::{RwSignal, SignalGet, create_rw_signal};
use floem::style::CursorStyle;
use floem::views::{Decorators, container, dyn_stack, empty, h_stack, label, scroll, v_stack};

use crate::theme::*;

#[derive(Clone)]
pub struct Port {
    pub idx:  u8,
    pub name: Option<String>,
}

pub fn hardware_mapper_panel() -> impl View {
    let motors = create_rw_signal(vec![
        Port { idx: 0, name: Some("driveL".to_string()) },
        Port { idx: 1, name: Some("driveR".to_string()) },
        Port { idx: 2, name: Some("armMotor".to_string()) },
        Port { idx: 3, name: None },
    ]);

    let servos = create_rw_signal(vec![
        Port { idx: 0, name: Some("claw".to_string()) },
        Port { idx: 1, name: Some("wrist".to_string()) },
        Port { idx: 2, name: None },
        Port { idx: 3, name: None },
        Port { idx: 4, name: None },
        Port { idx: 5, name: None },
    ]);

    let status_msg = create_rw_signal("Ready.".to_string());

    v_stack((
        scroll(
            h_stack((
                motors_hub_block(motors, status_msg),
                servos_hub_block(servos, status_msg),
            ))
            .style(|s| s.padding(12.0).gap(12.0, 0.0).items_start()),
        )
        .style(|s| s.flex_grow(1.0).width_pct(100.0)),
        toolbar(motors, servos, status_msg),
        status_bar(status_msg),
    ))
    .style(|s| {
        s.flex_col()
            .width_pct(100.0)
            .height_pct(100.0)
            .background(BG_SURFACE)
    })
}

fn motors_hub_block(
    ports:      RwSignal<Vec<Port>>,
    status_msg: RwSignal<String>,
) -> impl View {
    v_stack((
        label(|| "MOTORS".to_string()).style(|s| {
            s.color(ALLOY_ORANGE)
                .font_size(T_TINY)
                .font_weight(floem::text::Weight::BOLD)
                .margin_bottom(10.0)
        }),
        dyn_stack(
            move || ports.get().into_iter().enumerate().collect::<Vec<_>>(),
            |(i, _)| *i,
            move |(_, port)| port_chip(port, ports, status_msg),
        )
        .style(|s| {
            use taffy::prelude::fr;
            s.display(floem::style::Display::Grid)
                .grid_template_columns([fr(1.), fr(1.)])
                .gap(6.0, 6.0)
        }),
    ))
    .style(|s| {
        s.background(BG_SURFACE)
            .border_radius(R_8)
            .border(1.0)
            .border_color(BG_EDGE)
            .padding(12.0)
            .min_width(280.0)
            .flex_col()
    })
}

fn servos_hub_block(
    ports:      RwSignal<Vec<Port>>,
    status_msg: RwSignal<String>,
) -> impl View {
    v_stack((
        label(|| "SERVOS".to_string()).style(|s| {
            s.color(STATUS_INFO)
                .font_size(T_TINY)
                .font_weight(floem::text::Weight::BOLD)
                .margin_bottom(10.0)
        }),
        dyn_stack(
            move || ports.get().into_iter().enumerate().collect::<Vec<_>>(),
            |(i, _)| *i,
            move |(_, port)| port_chip(port, ports, status_msg),
        )
        .style(|s| {
            use taffy::prelude::fr;
            s.display(floem::style::Display::Grid)
                .grid_template_columns([fr(1.), fr(1.), fr(1.)])
                .gap(6.0, 6.0)
        }),
    ))
    .style(|s| {
        s.background(BG_SURFACE)
            .border_radius(R_8)
            .border(1.0)
            .border_color(BG_EDGE)
            .padding(12.0)
            .min_width(280.0)
            .flex_col()
    })
}

fn port_chip(
    port:       Port,
    ports:      RwSignal<Vec<Port>>,
    status_msg: RwSignal<String>,
) -> impl View {
    let idx         = port.idx;
    let is_assigned = port.name.is_some();
    let chip_text   = match &port.name {
        Some(n) => format!("[{}] {}", idx, n),
        None    => format!("[{}] empty", idx),
    };

    container(
        label(move || chip_text.clone()).style(move |s| {
            let color = if is_assigned {
                floem::peniko::Color::WHITE
            } else {
                FG_3
            };
            s.color(color)
                .font_size(T_TINY)
                .font_family("monospace".to_string())
        }),
    )
    .on_click_stop(move |_| {
        ports.update(|v| {
            if let Some(p) = v.iter_mut().find(|p| p.idx == idx) {
                if p.name.is_some() {
                    p.name = None;
                    status_msg.set(format!("Port {} cleared.", idx));
                } else {
                    p.name = Some(format!("port{}", idx));
                    status_msg.set(format!("Port {} assigned.", idx));
                }
            }
        });
    })
    .style(move |s| {
        let bg = if is_assigned { ALLOY_ORANGE } else { BG_RAISED };
        s.height(38.0)
            .padding_horiz(8.0)
            .background(bg)
            .border_radius(R_4)
            .items_center()
            .justify_center()
            .cursor(CursorStyle::Pointer)
            .apply_if(!is_assigned, |s| s.hover(|s| s.background(BG_HOVER)))
    })
}

fn toolbar(
    motors:     RwSignal<Vec<Port>>,
    servos:     RwSignal<Vec<Port>>,
    status_msg: RwSignal<String>,
) -> impl View {
    h_stack((
        label(|| "Generate RobotHardware.java".to_string())
            .on_click_stop(move |_| {
                status_msg.set("RobotHardware.java generated.".to_string());
            })
            .style(|s| {
                s.padding_horiz(14.0)
                    .padding_vert(7.0)
                    .background(ALLOY_ORANGE)
                    .border_radius(R_4)
                    .color(floem::peniko::Color::WHITE)
                    .font_size(T_SMALL)
                    .font_weight(floem::text::Weight::SEMI_BOLD)
                    .cursor(CursorStyle::Pointer)
                    .margin_right(8.0)
                    .hover(|s| s.background(ALLOY_ORANGE_DEEP))
            }),
        label(|| "Clear All".to_string())
            .on_click_stop(move |_| {
                motors.update(|v| {
                    for p in v.iter_mut() {
                        p.name = None;
                    }
                });
                servos.update(|v| {
                    for p in v.iter_mut() {
                        p.name = None;
                    }
                });
                status_msg.set("All ports cleared.".to_string());
            })
            .style(|s| {
                s.padding_horiz(12.0)
                    .padding_vert(7.0)
                    .border(1.0)
                    .border_color(LINE_RING)
                    .border_radius(R_4)
                    .color(FG_2)
                    .font_size(T_SMALL)
                    .cursor(CursorStyle::Pointer)
                    .margin_right(8.0)
                    .hover(|s| s.background(BG_HOVER))
            }),
        container(empty()).style(|s| s.flex_grow(1.0f32)),
        label(|| "Read from hub".to_string())
            .on_click_stop(move |_| {
                status_msg.set("Reading from hub\u{2026}".to_string());
            })
            .style(|s| {
                s.padding_horiz(12.0)
                    .padding_vert(7.0)
                    .border(1.0)
                    .border_color(LINE_RING)
                    .border_radius(R_4)
                    .color(FG_2)
                    .font_size(T_SMALL)
                    .cursor(CursorStyle::Pointer)
                    .hover(|s| s.background(BG_HOVER))
            }),
    ))
    .style(|s| {
        s.height(48.0)
            .padding_horiz(14.0)
            .items_center()
            .border_top(1.0)
            .border_color(BG_EDGE)
            .flex_shrink(0.0)
    })
}

fn status_bar(status_msg: RwSignal<String>) -> impl View {
    container(
        label(move || status_msg.get()).style(|s| s.color(FG_2).font_size(T_TINY)),
    )
    .style(|s| {
        s.height(UI_STATUS_HEIGHT)
            .padding_horiz(14.0)
            .items_center()
            .border_top(1.0)
            .border_color(BG_EDGE)
            .background(BG_SURFACE)
            .flex_shrink(0.0)
            .width_pct(100.0)
    })
}
