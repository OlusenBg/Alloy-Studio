//! Telemetry panel — live robot metric charts over UDP.
//!
//! Reference: kit/TelemetryPanel.jsx.

use floem::peniko::Color;
use floem::reactive::{create_rw_signal, RwSignal, SignalGet, SignalUpdate};
use floem::style::CursorStyle;
use floem::views::{container, dyn_stack, empty, h_stack, label, v_stack, Decorators};
use floem::View;

use crate::theme::*;

#[derive(Clone, Copy, PartialEq)]
pub enum RobotConnState {
    Disconnected,
    Connected,
}

#[derive(Clone)]
pub struct TelemetryMetric {
    pub key: String,
    pub value: f64,
    pub unit: String,
}

impl TelemetryMetric {
    pub fn default_set() -> Vec<TelemetryMetric> {
        vec![
            TelemetryMetric {
                key: "leftPower".to_string(),
                value: 0.72,
                unit: "%".to_string(),
            },
            TelemetryMetric {
                key: "rightPower".to_string(),
                value: 0.68,
                unit: "%".to_string(),
            },
            TelemetryMetric {
                key: "heading".to_string(),
                value: 42.3,
                unit: "°".to_string(),
            },
            TelemetryMetric {
                key: "armAngle".to_string(),
                value: 118.0,
                unit: "°".to_string(),
            },
            TelemetryMetric {
                key: "batteryVolt".to_string(),
                value: 13.1,
                unit: "V".to_string(),
            },
            TelemetryMetric {
                key: "loopTime".to_string(),
                value: 8.4,
                unit: "ms".to_string(),
            },
        ]
    }
}

pub fn telemetry_panel() -> impl View {
    let conn_state = create_rw_signal(RobotConnState::Connected);
    let metrics = create_rw_signal(TelemetryMetric::default_set());

    v_stack((conn_header(conn_state), chart_grid(metrics))).style(|s| {
        s.flex_col()
            .width_pct(100.0)
            .height_pct(100.0)
            .background(BG_SURFACE)
    })
}

fn conn_header(conn_state: RwSignal<RobotConnState>) -> impl View {
    h_stack((
        // Status dot
        container(empty()).style(move |s| {
            let color = if conn_state.get() == RobotConnState::Connected {
                STATUS_SUCCESS
            } else {
                STATUS_ERROR
            };
            s.width(8.0)
                .height(8.0)
                .border_radius(R_FULL)
                .background(color)
                .flex_shrink(0.0)
                .margin_right(8.0)
        }),
        // Connection label
        label(move || {
            if conn_state.get() == RobotConnState::Connected {
                "Robot Connected".to_string()
            } else {
                "Robot Disconnected".to_string()
            }
        })
        .style(|s| {
            s.color(FG_2)
                .font_size(T_SMALL)
                .flex_grow(0.0)
                .margin_right(12.0)
        }),
        // Address / rate
        label(|| "UDP :9988 · 5 Hz".to_string()).style(|s| {
            s.color(FG_3)
                .font_size(T_TINY)
                .font_family("monospace".to_string())
        }),
        // Spacer
        container(empty()).style(|s| s.flex_grow(1.0f32)),
        // Reconnect button
        label(|| "Reconnect".to_string())
            .on_click_stop(move |_| {
                conn_state.set(RobotConnState::Connected);
            })
            .style(|s| {
                s.padding_horiz(10.0)
                    .padding_vert(4.0)
                    .border(1.0)
                    .border_color(LINE_RING)
                    .border_radius(R_4)
                    .color(FG_2)
                    .font_size(T_TINY)
                    .cursor(CursorStyle::Pointer)
                    .hover(|s| s.background(BG_HOVER))
            }),
    ))
    .style(|s| {
        s.height(40.0)
            .padding_horiz(14.0)
            .items_center()
            .border_bottom(1.0)
            .border_color(BG_EDGE)
            .flex_shrink(0.0)
    })
}

fn chart_grid(metrics: RwSignal<Vec<TelemetryMetric>>) -> impl View {
    scroll(
        dyn_stack(
            move || metrics.get().into_iter().enumerate().collect::<Vec<_>>(),
            |(i, _)| *i,
            move |(_, metric)| chart_card(metric),
        )
        .style(|s| {
            s.flex_row()
                .flex_wrap(floem::style::FlexWrap::Wrap)
                .padding(6.0)
                .width_pct(100.0)
        }),
    )
    .style(|s| s.flex_grow(1.0).width_pct(100.0))
}

fn chart_card(metric: TelemetryMetric) -> impl View {
    let key = metric.key.clone();
    let value = metric.value;
    let unit = metric.unit.clone();

    v_stack((
        // Header row: key + value + unit
        h_stack((
            label({
                let k = key.clone();
                move || k.clone()
            })
            .style(|s| s.color(FG_2).font_size(T_TINY).flex_grow(1.0f32)),
            h_stack((
                label(move || format!("{:.1}", value)).style(|s| {
                    s.color(ALLOY_ORANGE)
                        .font_size(T_XL)
                        .font_weight(floem::text::FontWeight::BOLD)
                        .font_family("monospace".to_string())
                }),
                label({
                    let u = unit.clone();
                    move || u.clone()
                })
                .style(|s| {
                    s.color(FG_3)
                        .font_size(T_MICRO)
                        .margin_left(3.0)
                        .align_self(floem::taffy::style::AlignItems::FlexEnd)
                        .margin_bottom(2.0)
                }),
            ))
            .style(|s| s.items_end()),
        ))
        .style(|s| s.items_center().margin_bottom(6.0)),
        // Chart area
        chart_area(),
    ))
    .style(|s| {
        s.width(220.0)
            .padding(12.0)
            .background(BG_SURFACE)
            .border_radius(R_6)
            .border(1.0)
            .border_color(BG_EDGE)
            .margin(6.0)
    })
}

fn chart_area() -> impl View {
    // Chart background with horizontal grid lines represented as thin containers.
    // The orange fill tint sits at the bottom.
    container(
        v_stack((
            // Three horizontal grid lines dividing the 60px chart into 4 bands
            container(empty()).style(|s| s.width_pct(100.0).height(1.0).background(BG_GRID)),
            container(empty()).style(|s| s.flex_grow(1.0f32)),
            container(empty()).style(|s| s.width_pct(100.0).height(1.0).background(BG_GRID)),
            container(empty()).style(|s| s.flex_grow(1.0f32)),
            container(empty()).style(|s| s.width_pct(100.0).height(1.0).background(BG_GRID)),
            container(empty()).style(|s| s.flex_grow(1.0f32)),
            // Orange tint fill at the bottom
            container(empty()).style(|s| {
                s.width_pct(100.0)
                    .height(14.0)
                    .background(Color::from_rgba8(0xFF, 0x6B, 0x2B, 90))
            }),
        ))
        .style(|s| s.flex_col().width_pct(100.0).height_pct(100.0)),
    )
    .style(|s| {
        s.width_pct(100.0)
            .height(60.0)
            .background(BG_NAVY)
            .border_radius(R_4)
            .overflow_x(floem::taffy::style::Overflow::Hidden)
            .overflow_y(floem::taffy::style::Overflow::Hidden)
    })
}
use floem::views::scroll::scroll;
