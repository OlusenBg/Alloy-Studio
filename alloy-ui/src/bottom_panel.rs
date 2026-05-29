//! Bottom panel — tabbed dock with Problems, Terminal, Telemetry, Hardware,
//! Git Timeline, and Gradle Repair.
//!
//! Reference: kit/BottomPanel.jsx.

use std::sync::Arc;

use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
use floem::style::CursorStyle;
use floem::views::{container, dyn_stack, empty, h_stack, label, v_stack, Decorators};
use floem::{IntoView, View};

use crate::panels::git_timeline::git_timeline_panel;
use crate::panels::gradle_repair::gradle_repair_panel;
use crate::panels::hardware_mapper::hardware_mapper_panel;
use crate::panels::telemetry::telemetry_panel;
use crate::theme::*;

#[derive(Clone, Copy, PartialEq)]
pub enum BottomPanelTab {
    Problems,
    Terminal,
    Telemetry,
    Hardware,
    Git,
    Gradle,
}

pub fn bottom_panel(
    tab: RwSignal<BottomPanelTab>,
    on_maximize: Arc<dyn Fn()>,
    on_close: Arc<dyn Fn()>,
    terminal_lines: RwSignal<Vec<String>>,
) -> impl View {
    v_stack((
        tab_strip(tab, on_maximize, on_close),
        content_area(tab, terminal_lines),
    ))
    .style(|s| {
        s.flex_col()
            .width_pct(100.0)
            .height_pct(100.0)
            .background(BG_SURFACE)
    })
}

// ── Tab strip ────────────────────────────────────────────────────────────────

fn tab_strip(
    tab: RwSignal<BottomPanelTab>,
    on_maximize: Arc<dyn Fn()>,
    on_close: Arc<dyn Fn()>,
) -> impl View {
    h_stack((
        tab_btn(
            BottomPanelTab::Problems,
            "Problems",
            Some(("2", ALLOY_ORANGE)),
            tab,
        ),
        tab_btn(BottomPanelTab::Terminal, "Terminal", None, tab),
        tab_btn(BottomPanelTab::Telemetry, "Telemetry", None, tab),
        tab_btn(BottomPanelTab::Hardware, "Hardware Mapper", None, tab),
        tab_btn(BottomPanelTab::Git, "Git Timeline", None, tab),
        tab_btn(
            BottomPanelTab::Gradle,
            "Gradle Repair",
            Some(("1", STATUS_ERROR)),
            tab,
        ),
        // spacer
        container(empty()).style(|s| s.flex_grow(1.0f32)),
        // window controls
        icon_btn("\u{2922}", move || (on_maximize)()),
        icon_btn("\u{2227}", || {}),
        icon_btn("\u{00D7}", move || (on_close)()),
    ))
    .style(|s| {
        s.height(32.0)
            .background(BG_SURFACE)
            .border_bottom(1.0)
            .border_color(BG_EDGE)
            .items_center()
            .flex_shrink(0.0)
            .width_pct(100.0)
    })
}

fn tab_btn(
    this_tab: BottomPanelTab,
    name: &'static str,
    badge: Option<(&'static str, floem::peniko::Color)>,
    active: RwSignal<BottomPanelTab>,
) -> impl View {
    let row = h_stack((
        label(move || name.to_string()).style(move |s| {
            let color = if active.get() == this_tab { FG_1 } else { FG_3 };
            s.color(color)
                .font_size(T_MICRO)
                .font_weight(floem::text::FontWeight::BOLD)
        }),
        // optional badge
        {
            if let Some((count, bg)) = badge {
                label(move || count.to_string())
                    .style(move |s| {
                        s.background(bg)
                            .color(FG_1)
                            .font_size(T_MICRO)
                            .font_weight(floem::text::FontWeight::BOLD)
                            .border_radius(R_FULL)
                            .padding_horiz(6.0)
                            .padding_vert(1.0)
                            .margin_left(4.0)
                    })
                    .into_any()
            } else {
                label(|| String::new()).style(|s| s.hide()).into_any()
            }
        },
        // active indicator bar (bottom)
        container(empty()).style(move |s| {
            let s = s
                .absolute()
                .width_pct(100.0)
                .height(2.0)
                .background(ALLOY_ORANGE)
                .inset_bottom(0.0);
            if active.get() == this_tab {
                s
            } else {
                s.hide()
            }
        }),
    ))
    .style(|s| s.items_center());

    container(row)
        .on_click_stop(move |_| active.set(this_tab))
        .style(move |s| {
            let s = s
                .height(32.0)
                .padding_horiz(14.0)
                .items_center()
                .cursor(CursorStyle::Pointer);
            if active.get() == this_tab {
                s.color(FG_1)
            } else {
                s.color(FG_3).hover(|s| s.color(FG_2))
            }
        })
}

fn icon_btn(glyph: &'static str, mut on_click: impl FnMut() + 'static) -> impl View {
    label(move || glyph.to_string())
        .on_click_stop(move |_cx| on_click())
        .style(|s| {
            s.color(FG_3)
                .font_size(T_BASE)
                .padding_horiz(8.0)
                .height(32.0)
                .items_center()
                .cursor(CursorStyle::Pointer)
                .hover(|s| s.color(FG_1))
        })
}

// ── Content area ─────────────────────────────────────────────────────────────

fn content_area(tab: RwSignal<BottomPanelTab>, terminal_lines: RwSignal<Vec<String>>) -> impl View {
    container(
        h_stack((
            container(problems_view()).style(move |s| {
                if tab.get() == BottomPanelTab::Problems {
                    s.flex_grow(1.0f32).height_pct(100.0)
                } else {
                    s.hide()
                }
            }),
            container(terminal_view(terminal_lines)).style(move |s| {
                if tab.get() == BottomPanelTab::Terminal {
                    s.flex_grow(1.0f32).height_pct(100.0)
                } else {
                    s.hide()
                }
            }),
            container(telemetry_panel()).style(move |s| {
                if tab.get() == BottomPanelTab::Telemetry {
                    s.flex_grow(1.0f32).height_pct(100.0)
                } else {
                    s.hide()
                }
            }),
            container(hardware_mapper_panel()).style(move |s| {
                if tab.get() == BottomPanelTab::Hardware {
                    s.flex_grow(1.0f32).height_pct(100.0)
                } else {
                    s.hide()
                }
            }),
            container(git_timeline_panel()).style(move |s| {
                if tab.get() == BottomPanelTab::Git {
                    s.flex_grow(1.0f32).height_pct(100.0)
                } else {
                    s.hide()
                }
            }),
            container(gradle_repair_panel()).style(move |s| {
                if tab.get() == BottomPanelTab::Gradle {
                    s.flex_grow(1.0f32).height_pct(100.0)
                } else {
                    s.hide()
                }
            }),
        ))
        .style(|s| s.flex_row().width_pct(100.0).height_pct(100.0)),
    )
    .style(|s| s.flex_grow(1.0f32).width_pct(100.0).min_height(0.0))
}

// ── Inline Problems view ──────────────────────────────────────────────────────

struct Diagnostic {
    level: DiagLevel,
    message: &'static str,
    file: &'static str,
    loc: &'static str,
}

#[derive(Clone, Copy)]
enum DiagLevel {
    Error,
    Warning,
}

static DIAGNOSTICS: &[Diagnostic] = &[
    Diagnostic {
        level: DiagLevel::Error,
        message: "cannot find symbol: armMotor \u{2014} did you mean \u{2018}arm\u{2019}?",
        file: "OpMode.java",
        loc: "16:53",
    },
    Diagnostic {
        level: DiagLevel::Warning,
        message: "unchecked call to add(E)",
        file: "OpMode.java",
        loc: "24:12",
    },
    Diagnostic {
        level: DiagLevel::Warning,
        message: "Servo field \u{2018}wrist\u{2019} is never read",
        file: "RobotHardware.java",
        loc: "8:3",
    },
];

fn problems_view() -> impl View {
    scroll(
        v_stack((
            diag_row(&DIAGNOSTICS[0]),
            diag_row(&DIAGNOSTICS[1]),
            diag_row(&DIAGNOSTICS[2]),
        ))
        .style(|s| s.flex_col().padding(8.0).width_pct(100.0)),
    )
    .style(|s| s.flex_grow(1.0).width_pct(100.0))
}

fn diag_row(d: &'static Diagnostic) -> impl View {
    let (icon, color) = match d.level {
        DiagLevel::Error => ("✖", STATUS_ERROR),
        DiagLevel::Warning => ("⚠", STATUS_WARNING),
    };

    h_stack((
        label(move || icon.to_string()).style(move |s| {
            s.color(color)
                .font_size(T_SMALL)
                .margin_right(8.0)
                .flex_shrink(0.0)
        }),
        label(move || d.message.to_string()).style(|s| {
            s.color(FG_1)
                .font_size(T_SMALL)
                .flex_grow(1.0f32)
                .min_width(0.0)
        }),
        label(move || format!("{} {}", d.file, d.loc)).style(|s| {
            s.color(FG_3)
                .font_size(T_MICRO)
                .font_family("monospace".to_string())
                .margin_left(12.0)
                .flex_shrink(0.0)
        }),
    ))
    .style(|s| {
        s.items_center()
            .padding_horiz(8.0)
            .padding_vert(5.0)
            .width_pct(100.0)
            .hover(|s| s.background(BG_HOVER))
    })
}

// ── Inline Terminal view ──────────────────────────────────────────────────────

fn terminal_view(terminal_lines: RwSignal<Vec<String>>) -> impl View {
    scroll(
        v_stack((
            terminal_prompt_line(),
            dyn_stack(
                move || {
                    terminal_lines
                        .get()
                        .into_iter()
                        .enumerate()
                        .collect::<Vec<_>>()
                },
                |(i, _)| *i,
                |(_, line)| terminal_line(line),
            )
            .style(|s| s.flex_col()),
        ))
        .style(|s| {
            s.flex_col()
                .padding(12.0)
                .width_pct(100.0)
                .background(BG_NAVY)
        }),
    )
    .style(|s| s.flex_grow(1.0).width_pct(100.0).background(BG_NAVY))
}

fn terminal_line(text: String) -> impl View {
    let color = if text.contains("BUILD SUCCESSFUL") {
        STATUS_SUCCESS
    } else if text.contains("BUILD FAILED") || text.contains("ERROR:") {
        STATUS_ERROR
    } else if text.starts_with("> Task") || text.starts_with("> Configure") {
        ALLOY_ORANGE
    } else {
        FG_2
    };
    label(move || text.clone()).style(move |s| {
        s.color(color)
            .font_size(T_SMALL)
            .font_family("monospace".to_string())
            .padding_vert(2.0)
    })
}

fn terminal_prompt_line() -> impl View {
    h_stack((
        label(|| "alex@centerstage".to_string()).style(|s| {
            s.color(STATUS_SUCCESS)
                .font_size(T_SMALL)
                .font_family("monospace".to_string())
        }),
        label(|| ":".to_string()).style(|s| {
            s.color(FG_1)
                .font_size(T_SMALL)
                .font_family("monospace".to_string())
        }),
        label(|| "~/Projects/ftc/CenterStage".to_string()).style(|s| {
            s.color(STATUS_INFO)
                .font_size(T_SMALL)
                .font_family("monospace".to_string())
        }),
        label(|| " (main)".to_string()).style(|s| {
            s.color(FG_3)
                .font_size(T_SMALL)
                .font_family("monospace".to_string())
        }),
        label(|| "$ ./gradlew assembleDebug".to_string()).style(|s| {
            s.color(FG_1)
                .font_size(T_SMALL)
                .font_family("monospace".to_string())
                .margin_left(4.0)
        }),
    ))
    .style(|s| s.items_center().margin_bottom(4.0))
}

use floem::views::scroll::scroll;
