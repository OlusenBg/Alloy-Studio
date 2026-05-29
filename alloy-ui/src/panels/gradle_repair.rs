//! Gradle repair panel — AI-assisted build failure diagnostics.
//!
//! Reference: kit/GradleRepair.jsx.

use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
use floem::style::CursorStyle;
use floem::views::{container, dyn_stack, empty, h_stack, label, v_stack, Decorators};
use floem::View;

use crate::theme::*;

#[derive(Clone, Copy, PartialEq)]
pub enum BuildState {
    Idle,
    Running,
    Failed,
    Success,
}

#[derive(Clone, Copy, PartialEq)]
pub enum LogTone {
    Info,
    Err,
    Out,
}

#[derive(Clone, Copy)]
pub struct LogLine {
    pub tone: LogTone,
    pub text: &'static str,
}

const LOG_FAILED: &[LogLine] = &[
    LogLine {
        tone: LogTone::Info,
        text: "> Configure project :TeamCode",
    },
    LogLine {
        tone: LogTone::Info,
        text: "> Task :TeamCode:compileDebugJavaWithJavac",
    },
    LogLine {
        tone: LogTone::Info,
        text: "Resolving dependencies for configuration ':TeamCode:debugCompileClasspath'",
    },
    LogLine {
        tone: LogTone::Err,
        text: "FAILURE: Build failed with an exception.",
    },
    LogLine {
        tone: LogTone::Out,
        text: "* What went wrong:",
    },
    LogLine {
        tone: LogTone::Out,
        text: "Could not resolve all files for configuration ':TeamCode:debugCompileClasspath'.",
    },
    LogLine {
        tone: LogTone::Out,
        text: "  > Could not find com.qualcomm.robotcore:ftc-sdk:9.2.",
    },
    LogLine {
        tone: LogTone::Out,
        text: "    Required by: project :TeamCode",
    },
    LogLine {
        tone: LogTone::Info,
        text: "BUILD FAILED in 12s",
    },
];

pub fn gradle_repair_panel() -> impl View {
    let build_state = RwSignal::new(BuildState::Failed);
    let fix_applied = RwSignal::new(false);

    v_stack((
        status_banner(build_state),
        repair_card(build_state, fix_applied),
        log_view(),
        footer(build_state),
    ))
    .style(|s| {
        s.flex_col()
            .width_pct(100.0)
            .height_pct(100.0)
            .background(BG_SURFACE)
    })
}

fn status_banner(build_state: RwSignal<BuildState>) -> impl View {
    container(
        label(move || match build_state.get() {
            BuildState::Idle => "Gradle Repair — ready.".to_string(),
            BuildState::Running => "Building\u{2026}".to_string(),
            BuildState::Failed => {
                "BUILD FAILED — could not resolve com.qualcomm.robotcore:ftc-sdk:9.2".to_string()
            }
            BuildState::Success => "BUILD SUCCESSFUL".to_string(),
        })
        .style(move |s| {
            let color = match build_state.get() {
                BuildState::Idle => FG_2,
                BuildState::Running => STATUS_INFO,
                BuildState::Failed => STATUS_ERROR,
                BuildState::Success => STATUS_SUCCESS,
            };
            s.color(color).font_size(T_SMALL)
        }),
    )
    .style(move |s| {
        let bg = match build_state.get() {
            BuildState::Idle | BuildState::Running => BG_SURFACE,
            BuildState::Failed => floem::peniko::Color::from_rgb8(0x3A, 0x0D, 0x10),
            BuildState::Success => floem::peniko::Color::from_rgb8(0x0D, 0x3A, 0x23),
        };
        s.padding_horiz(14.0)
            .padding_vert(10.0)
            .background(bg)
            .border_bottom(1.0)
            .border_color(BG_EDGE)
            .flex_shrink(0.0)
            .width_pct(100.0)
    })
}

fn repair_card(build_state: RwSignal<BuildState>, fix_applied: RwSignal<bool>) -> impl View {
    container(
        v_stack((
            // Card header
            container(
                label(|| "Alloy AI · Proposed Fix".to_string()).style(|s| {
                    s.color(ALLOY_ORANGE)
                        .font_size(T_TINY)
                        .font_weight(floem::text::FontWeight::BOLD)
                }),
            )
            .style(|s| {
                s.background(BG_RAISED)
                    .padding_horiz(12.0)
                    .padding_vert(8.0)
                    .border_bottom(1.0)
                    .border_color(BG_EDGE)
                    .width_pct(100.0)
            }),
            // Card body
            v_stack((
                label(|| "The dependency version 9.2 does not exist. Update the FTC SDK version to 9.1 in your build.gradle.".to_string())
                    .style(|s| s.color(FG_1).font_size(T_SMALL).margin_bottom(10.0)),
                // Diff block
                v_stack((
                    label(|| "- implementation 'com.qualcomm.robotcore:ftc-sdk:9.2'".to_string())
                        .style(|s| {
                            s.color(STATUS_ERROR)
                                .font_size(T_TINY)
                                .font_family("monospace".to_string())
                        }),
                    label(|| "+ implementation 'com.qualcomm.robotcore:ftc-sdk:9.1'".to_string())
                        .style(|s| {
                            s.color(STATUS_SUCCESS)
                                .font_size(T_TINY)
                                .font_family("monospace".to_string())
                        }),
                ))
                .style(|s| {
                    s.background(BG_NAVY)
                        .padding(10.0)
                        .border_radius(R_4)
                        .margin_bottom(12.0)
                        .width_pct(100.0)
                }),
                // Action buttons
                h_stack((
                    label(move || {
                        if fix_applied.get() {
                            "Fix Applied".to_string()
                        } else {
                            "Apply Fix & Rebuild".to_string()
                        }
                    })
                    .on_click_stop(move |_| {
                        if !fix_applied.get() {
                            fix_applied.set(true);
                        }
                    })
                    .style(move |s| {
                        let applied = fix_applied.get();
                        let bg = if applied { FG_4 } else { ALLOY_ORANGE };
                        s.padding_horiz(12.0)
                            .padding_vert(6.0)
                            .background(bg)
                            .border_radius(R_4)
                            .color(floem::peniko::Color::WHITE)
                            .font_size(T_SMALL)
                            .font_weight(floem::text::FontWeight::SEMI_BOLD)
                            .cursor(CursorStyle::Pointer)
                            .margin_right(8.0)
                    }),
                    label(|| "Dismiss".to_string())
                        .style(|s| {
                            s.padding_horiz(12.0)
                                .padding_vert(6.0)
                                .border(1.0)
                                .border_color(LINE_RING)
                                .border_radius(R_4)
                                .color(FG_2)
                                .font_size(T_SMALL)
                                .cursor(CursorStyle::Pointer)
                                .hover(|s| s.background(BG_HOVER))
                        }),
                ))
                .style(|s| s.items_center()),
            ))
            .style(|s| s.padding(12.0).flex_col()),
        ))
        .style(|s| {
            s.border(1.0)
                .border_color(ALLOY_ORANGE_GLOW)
                .border_radius(R_6)
                .background(BG_SURFACE)
                .margin(12.0)
                .flex_shrink(0.0)
        }),
    )
    .style(move |s| {
        let hidden = build_state.get() != BuildState::Failed;
        s.apply_if(hidden, |s| s.hide()).flex_shrink(0.0)
    })
}

fn log_view() -> impl View {
    scroll(
        dyn_stack(
            || LOG_FAILED.iter().enumerate().collect::<Vec<_>>(),
            |(i, _)| *i,
            |(_, line)| {
                let color = match line.tone {
                    LogTone::Err => STATUS_ERROR,
                    LogTone::Info => ALLOY_ORANGE,
                    LogTone::Out => FG_2,
                };
                let text = line.text;
                label(move || text.to_string()).style(move |s| {
                    s.color(color)
                        .font_size(T_SMALL)
                        .font_family("monospace".to_string())
                        .padding_vert(1.0)
                })
            },
        )
        .style(|s| s.flex_col().padding(12.0).width_pct(100.0)),
    )
    .style(|s| s.flex_grow(1.0).background(BG_NAVY).width_pct(100.0))
}

fn footer(build_state: RwSignal<BuildState>) -> impl View {
    h_stack((
        // Rebuild button
        label(|| "Rebuild".to_string())
            .on_click_stop(move |_| {
                build_state.set(BuildState::Running);
            })
            .style(|s| {
                s.padding_horiz(14.0)
                    .padding_vert(6.0)
                    .background(ALLOY_ORANGE)
                    .border_radius(R_4)
                    .color(floem::peniko::Color::WHITE)
                    .font_size(T_SMALL)
                    .font_weight(floem::text::FontWeight::SEMI_BOLD)
                    .cursor(CursorStyle::Pointer)
                    .margin_right(8.0)
                    .hover(|s| s.background(ALLOY_ORANGE_DEEP))
            }),
        // Stop button
        label(|| "Stop".to_string())
            .on_click_stop(move |_| {
                build_state.set(BuildState::Idle);
            })
            .style(|s| {
                s.padding_horiz(14.0)
                    .padding_vert(6.0)
                    .border(1.0)
                    .border_color(LINE_RING)
                    .border_radius(R_4)
                    .color(FG_2)
                    .font_size(T_SMALL)
                    .cursor(CursorStyle::Pointer)
                    .hover(|s| s.background(BG_HOVER))
            }),
        // Spacer
        container(empty()).style(|s| s.flex_grow(1.0f32)),
        // Gradle / JDK info
        label(|| "gradle 8.4 · jdk 17.0.10".to_string()).style(|s| {
            s.color(FG_3)
                .font_size(T_MICRO)
                .font_family("monospace".to_string())
        }),
    ))
    .style(|s| {
        s.height(44.0)
            .padding_horiz(14.0)
            .items_center()
            .border_top(1.0)
            .border_color(BG_EDGE)
            .flex_shrink(0.0)
    })
}
use floem::views::scroll::scroll;
