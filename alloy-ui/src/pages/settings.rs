//! Settings page — opens as a regular editor tab (not a modal).
//!
//! 9 categories on a left rail; scrollable content on the right.
//! Reference: kit/SettingsPage.jsx.

use std::sync::Arc;

use floem::reactive::{create_rw_signal, RwSignal, SignalGet, SignalUpdate};
use floem::style::CursorStyle;
use floem::views::{
    container, dyn_stack, empty, h_stack, label, scroll, text_input, v_stack, Decorators,
};
use floem::View;

use crate::theme::*;

static ALLOY_LOGO: &[u8] = include_bytes!("../../extra/images/logo.png");

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingsCategory {
    General,
    Editor,
    Appearance,
    Ftc,
    Ai,
    Git,
    Extensions,
    Keyboard,
    About,
}

impl SettingsCategory {
    fn label(self) -> &'static str {
        match self {
            SettingsCategory::General => "General",
            SettingsCategory::Editor => "Editor",
            SettingsCategory::Appearance => "Appearance",
            SettingsCategory::Ftc => "FTC",
            SettingsCategory::Ai => "AI",
            SettingsCategory::Git => "Git",
            SettingsCategory::Extensions => "Extensions",
            SettingsCategory::Keyboard => "Keyboard",
            SettingsCategory::About => "About",
        }
    }
    fn glyph(self) -> &'static str {
        match self {
            SettingsCategory::General => "⚙",
            SettingsCategory::Editor => "✎",
            SettingsCategory::Appearance => "◐",
            SettingsCategory::Ftc => "▶",
            SettingsCategory::Ai => "✸",
            SettingsCategory::Git => "⎇",
            SettingsCategory::Extensions => "⬚",
            SettingsCategory::Keyboard => "⌨",
            SettingsCategory::About => "ⓘ",
        }
    }
    fn all() -> &'static [SettingsCategory] {
        use SettingsCategory::*;
        &[
            General, Editor, Appearance, Ftc, Ai, Git, Extensions, Keyboard, About,
        ]
    }
}

pub fn settings_page(initial: SettingsCategory) -> impl View {
    let active_cat = create_rw_signal(initial);

    h_stack((left_rail(active_cat), right_content(active_cat)))
        .style(|s| s.width_pct(100.0).height_pct(100.0).background(BG_NAVY))
}

// ── Left rail ─────────────────────────────────────────────────────────────────
fn left_rail(active_cat: RwSignal<SettingsCategory>) -> impl View {
    let rows = SettingsCategory::all()
        .iter()
        .copied()
        .map(move |c| cat_row(c, active_cat));
    v_stack((
        label(|| "SETTINGS".to_string()).style(|s| {
            s.color(FG_3)
                .font_size(T_MICRO)
                .font_weight(floem::text::Weight::BOLD)
                .padding_horiz(10.0)
                .padding_top(4.0)
                .padding_bottom(8.0)
        }),
        floem::views::stack_from_iter(rows).style(|s| s.flex_col().gap(2.0).width_pct(100.0)),
    ))
    .style(|s| {
        s.width(200.0)
            .flex_shrink(0.0)
            .background(BG_SURFACE)
            .border_right(1.0)
            .border_color(BG_EDGE)
            .padding(8.0)
            .padding_top(16.0)
            .flex_col()
            .gap(2.0)
            .height_pct(100.0)
    })
}

fn cat_row(cat: SettingsCategory, active: RwSignal<SettingsCategory>) -> impl View {
    let glyph = cat.glyph();
    let lbl = cat.label();
    container(
        h_stack((
            // active stripe
            container(empty()).style(move |s| {
                let s = s
                    .absolute()
                    .width(2.0)
                    .height_pct(70.0)
                    .background(ALLOY_ORANGE)
                    .border_radius(R_2)
                    .margin_left(0.0);
                if active.get() == cat {
                    s
                } else {
                    s.hide()
                }
            }),
            label(move || glyph.to_string()).style(move |s| {
                let c = if active.get() == cat {
                    ALLOY_ORANGE
                } else {
                    FG_2
                };
                s.color(c).font_size(T_BASE).margin_right(10.0)
            }),
            label(move || lbl.to_string()).style(move |s| {
                let c = if active.get() == cat { FG_1 } else { FG_2 };
                s.color(c).font_size(T_SMALL)
            }),
        ))
        .style(|s| s.items_center()),
    )
    .on_click_stop(move |_| active.set(cat))
    .style(move |s| {
        let s = s
            .padding_horiz(10.0)
            .padding_vert(7.0)
            .border_radius(R_4)
            .cursor(CursorStyle::Pointer)
            .hover(|s| s.background(BG_HOVER));
        if active.get() == cat {
            s.background(BG_CURRENT)
        } else {
            s
        }
    })
}

// ── Right content ────────────────────────────────────────────────────────────
fn right_content(active_cat: RwSignal<SettingsCategory>) -> impl View {
    scroll(
        container(
            v_stack((
                container(general_page()).style(move |s| {
                    if active_cat.get() == SettingsCategory::General {
                        s
                    } else {
                        s.hide()
                    }
                }),
                container(editor_page()).style(move |s| {
                    if active_cat.get() == SettingsCategory::Editor {
                        s
                    } else {
                        s.hide()
                    }
                }),
                container(appearance_page()).style(move |s| {
                    if active_cat.get() == SettingsCategory::Appearance {
                        s
                    } else {
                        s.hide()
                    }
                }),
                container(ftc_page()).style(move |s| {
                    if active_cat.get() == SettingsCategory::Ftc {
                        s
                    } else {
                        s.hide()
                    }
                }),
                container(ai_page()).style(move |s| {
                    if active_cat.get() == SettingsCategory::Ai {
                        s
                    } else {
                        s.hide()
                    }
                }),
                container(git_page()).style(move |s| {
                    if active_cat.get() == SettingsCategory::Git {
                        s
                    } else {
                        s.hide()
                    }
                }),
                container(extensions_page()).style(move |s| {
                    if active_cat.get() == SettingsCategory::Extensions {
                        s
                    } else {
                        s.hide()
                    }
                }),
                container(keyboard_page()).style(move |s| {
                    if active_cat.get() == SettingsCategory::Keyboard {
                        s
                    } else {
                        s.hide()
                    }
                }),
                container(about_page()).style(move |s| {
                    if active_cat.get() == SettingsCategory::About {
                        s
                    } else {
                        s.hide()
                    }
                }),
            ))
            .style(|s| s.width_pct(100.0)),
        )
        .style(|s| {
            s.max_width(720.0)
                .padding(40.0)
                .padding_top(32.0)
                .padding_bottom(60.0)
        }),
    )
    .style(|s| s.flex_grow(1.0).width_pct(100.0).height_pct(100.0))
}

// ── Page-level primitives ────────────────────────────────────────────────────

fn page_h1(title: &'static str, sub: &'static str) -> impl View {
    v_stack((
        label(move || title.to_string()).style(|s| {
            s.color(FG_1)
                .font_size(T_3XL)
                .font_weight(floem::text::Weight::BOLD)
        }),
        label(move || sub.to_string()).style(|s| s.color(FG_3).font_size(T_SMALL).margin_top(6.0)),
    ))
    .style(|s| s.margin_bottom(24.0).gap(0.0))
}

fn group<C: View + 'static>(title: &'static str, children: C) -> impl View {
    v_stack((
        label(move || title.to_string()).style(|s| {
            s.color(FG_3)
                .font_size(T_MICRO)
                .font_weight(floem::text::Weight::BOLD)
                .margin_bottom(10.0)
                .padding_bottom(6.0)
                .width_pct(100.0)
                .border_bottom(1.0)
                .border_color(BG_EDGE)
        }),
        container(children).style(|s| s.flex_col().gap(14.0).width_pct(100.0)),
    ))
    .style(|s| s.width_pct(100.0).margin_bottom(28.0))
}

fn field<C: View + 'static>(label_text: &'static str, hint: &'static str, control: C) -> impl View {
    h_stack((
        v_stack((
            label(move || label_text.to_string()).style(|s| {
                s.color(FG_1)
                    .font_size(T_SMALL)
                    .font_weight(floem::text::Weight::MEDIUM)
            }),
            label(move || hint.to_string()).style(move |s| {
                let s = s
                    .color(FG_3)
                    .font_size(T_TINY)
                    .margin_top(3.0)
                    .line_height(1.45);
                if hint.is_empty() {
                    s.hide()
                } else {
                    s
                }
            }),
        ))
        .style(|s| s.width(200.0).flex_shrink(0.0).gap(0.0)),
        container(control).style(|s| s.flex_grow(1.0).flex_col().gap(4.0)),
    ))
    .style(|s| s.width_pct(100.0).gap(16.0).items_start())
}

fn text_field(initial: &str, mono: bool) -> impl View {
    let sig = create_rw_signal(initial.to_string());
    text_input(sig).keyboard_navigable().style(move |s| {
        let s = s
            .flex_grow(1.0)
            .background(BG_SURFACE)
            .border(1.0)
            .border_color(LINE_RING)
            .color(FG_1)
            .padding_horiz(10.0)
            .padding_vert(7.0)
            .font_size(T_SMALL)
            .border_radius(R_4);
        if mono {
            s.font_family("monospace".to_string())
        } else {
            s
        }
    })
}

fn select_pill(value: &str, _options: &[&str]) -> impl View {
    let v = value.to_string();
    h_stack((
        label(move || v.clone()).style(|s| s.color(FG_1).font_size(T_SMALL).flex_grow(1.0)),
        label(|| "▾".to_string()).style(|s| s.color(FG_3).font_size(T_TINY)),
    ))
    .style(|s| {
        s.background(BG_SURFACE)
            .border(1.0)
            .border_color(LINE_RING)
            .padding_horiz(10.0)
            .padding_vert(6.0)
            .border_radius(R_4)
            .items_center()
            .cursor(CursorStyle::Pointer)
            .width_pct(100.0)
            .hover(|s| s.background(BG_HOVER))
    })
}

fn toggle(initial: bool, lbl: &'static str) -> impl View {
    let sig = create_rw_signal(initial);
    h_stack((
        container(container(empty()).style(move |s| {
            let on = sig.get();
            let s = s
                .width(14.0)
                .height(14.0)
                .border_radius(R_FULL)
                .background(floem::peniko::Color::WHITE);
            if on {
                s.margin_left(16.0)
            } else {
                s.margin_left(2.0)
            }
        }))
        .on_click_stop(move |_| sig.update(|v| *v = !*v))
        .style(move |s| {
            let on = sig.get();
            s.width(32.0)
                .height(18.0)
                .border_radius(R_FULL)
                .background(if on { ALLOY_ORANGE } else { BG_RAISED })
                .cursor(CursorStyle::Pointer)
                .items_center()
                .flex_shrink(0.0)
        }),
        label(move || lbl.to_string())
            .style(|s| s.color(FG_2).font_size(T_SMALL).margin_left(10.0)),
    ))
    .style(|s| s.items_center())
}

// ── Individual category pages ────────────────────────────────────────────────

fn general_page() -> impl View {
    v_stack((
        page_h1("General", "Core editor behaviour."),
        group(
            "Workspace",
            v_stack((
                field(
                    "Auto-save",
                    "Saves modified files after a delay.",
                    toggle(true, "After 1s idle"),
                ),
                field(
                    "Restore on launch",
                    "Reopen last project + tabs.",
                    toggle(true, "Enabled"),
                ),
                field(
                    "Telemetry to Alloy",
                    "Anonymous usage stats, off by default.",
                    toggle(false, "Disabled"),
                ),
            ))
            .style(|s| s.flex_col().gap(14.0)),
        ),
        group(
            "Updates",
            v_stack((field(
                "Update channel",
                "Pre-built binaries are not yet available — track main for now.",
                select_pill("alpha (main)", &["alpha (main)", "beta", "stable"]),
            ),))
            .style(|s| s.flex_col().gap(14.0)),
        ),
    ))
    .style(|s| s.flex_col())
}

fn editor_page() -> impl View {
    v_stack((
        page_h1("Editor", "Type, indentation, and formatting."),
        group(
            "Font",
            v_stack((
                field(
                    "Editor font family",
                    "",
                    select_pill(
                        "JetBrains Mono",
                        &["JetBrains Mono", "Fira Code", "Berkeley Mono", "monospace"],
                    ),
                ),
                field("Editor font size", "", text_field("13", false)),
                field("Line height", "", text_field("1.55", false)),
                field("Ligatures", "", toggle(true, "Enabled")),
            ))
            .style(|s| s.flex_col().gap(14.0)),
        ),
        group(
            "Indent",
            v_stack((
                field("Tab size", "", text_field("4", false)),
                field("Insert spaces", "", toggle(true, "Enabled")),
            ))
            .style(|s| s.flex_col().gap(14.0)),
        ),
    ))
    .style(|s| s.flex_col())
}

fn appearance_page() -> impl View {
    v_stack((
        page_h1("Appearance",
                "The editor's look. Built around two layers: deep navy substrate and molten orange accent."),
        group("Theme", v_stack((
            field("Color theme", "",
                  select_pill("Alloy Dark (default)",
                              &["Alloy Dark (default)", "Alloy Light (coming soon)", "One Dark (Lapce)"])),
            field("Icon theme",  "",
                  select_pill("Codicons", &["Codicons", "Material Icons"])),
        )).style(|s| s.flex_col().gap(14.0))),
        group("Accent", v_stack((
            field("Brand accent",
                  "Welcome CTA, active tab underline, chart line.",
                  accent_swatches()),
            field("Compact mode", "Reduces padding throughout panels.",
                  toggle(false, "Off")),
        )).style(|s| s.flex_col().gap(14.0))),
    ))
    .style(|s| s.flex_col())
}

fn accent_swatches() -> impl View {
    let colors: [floem::peniko::Color; 5] = [
        ALLOY_ORANGE,
        FTC_RED,
        STATUS_SUCCESS,
        STATUS_INFO,
        SYN_KEYWORD,
    ];
    let chosen = create_rw_signal(0usize);
    let swatches = colors.into_iter().enumerate().map(|(i, c)| {
        container(empty())
            .on_click_stop(move |_| chosen.set(i))
            .style(move |s| {
                let active = chosen.get() == i;
                let s = s
                    .width(24.0)
                    .height(24.0)
                    .border_radius(R_4)
                    .background(c)
                    .cursor(CursorStyle::Pointer);
                if active {
                    s.border(2.0).border_color(FG_1)
                } else {
                    s
                }
            })
    });
    floem::views::stack_from_iter(swatches).style(|s| s.gap(6.0))
}

fn ftc_page() -> impl View {
    v_stack((
        page_h1(
            "FTC",
            "Robotics-specific options. Surface only when an FTC project is detected.",
        ),
        group(
            "Project",
            v_stack((
                field(
                    "Team number",
                    "Shown in the title bar.",
                    text_field("7842", false),
                ),
                field(
                    "SDK version",
                    "Detected from build.gradle.",
                    select_pill("9.2.0 (detected)", &["9.2.0 (detected)", "9.1.0", "9.0.1"]),
                ),
                field(
                    "Auto-detect FTC projects",
                    "Look for FtcRobotController/ on open.",
                    toggle(true, "Enabled"),
                ),
            ))
            .style(|s| s.flex_col().gap(14.0)),
        ),
        group(
            "Robot",
            v_stack((
                field(
                    "Robot Controller IP",
                    "Default is 192.168.43.1 (DS hotspot).",
                    text_field("192.168.43.1", true),
                ),
                field(
                    "Telemetry UDP port",
                    "alloy-telemetry-bridge.apk listens here.",
                    text_field("9988", true),
                ),
                field(
                    "Deploy method",
                    "",
                    select_pill(
                        "adb push (USB)",
                        &["adb push (USB)", "adb wifi", "OnBot Java"],
                    ),
                ),
            ))
            .style(|s| s.flex_col().gap(14.0)),
        ),
    ))
    .style(|s| s.flex_col())
}

fn ai_page() -> impl View {
    v_stack((
        page_h1("AI",
                "Alloy uses Anthropic's Claude API for Gradle Repair, commit messages, and telemetry diagnostics. Your API key is stored locally in the OS keychain."),
        group("Anthropic API", v_stack((
            field(
                "Claude API key",
                "Stored in the OS keychain — never written to disk.",
                h_stack((
                    text_field("••••••••••••••••••••••••••••", true),
                    primary_btn_sm("Test"),
                )).style(|s| s.gap(8.0).items_center()),
            ),
            field("Model", "Haiku is fastest and cheapest. Sonnet for harder repair cases.",
                  select_pill("claude-haiku-4-5",
                              &["claude-haiku-4-5", "claude-sonnet-4-5", "claude-opus-4-5"])),
            field("Monthly budget cap", "Alloy pauses AI features for the month once this is reached.",
                  text_field("$5.00", false)),
        )).style(|s| s.flex_col().gap(14.0))),
        group("Features", v_stack((
            field("Gradle Repair", "Diagnose build failures and propose patches.",
                  toggle(true, "Enabled")),
            field("Commit messages", "Draft conventional commits from staged diffs.",
                  toggle(true, "Enabled")),
            field("Telemetry Diagnostics", "Spot anomalies in robot telemetry streams.",
                  toggle(false, "Disabled · coming soon")),
        )).style(|s| s.flex_col().gap(14.0))),
    ))
    .style(|s| s.flex_col())
}

fn git_page() -> impl View {
    v_stack((
        page_h1("Git", "Source control."),
        group(
            "Identity",
            v_stack((
                field("Name", "", text_field("Alex Chen", false)),
                field("Email", "", text_field("alex@7842.team", false)),
            ))
            .style(|s| s.flex_col().gap(14.0)),
        ),
        group(
            "Accounts",
            v_stack((field(
                "GitHub",
                "Sign in to clone private repos and open pull requests.",
                primary_btn_sm("⎇ Connect GitHub…"),
            ),))
            .style(|s| s.flex_col().gap(14.0)),
        ),
        group(
            "Behaviour",
            v_stack((
                field("Default branch name", "", text_field("main", true)),
                field(
                    "Auto-fetch",
                    "Pulls new commits from origin in the background.",
                    select_pill(
                        "every 5 minutes",
                        &["off", "every 5 minutes", "every 15 minutes", "every hour"],
                    ),
                ),
            ))
            .style(|s| s.flex_col().gap(14.0)),
        ),
    ))
    .style(|s| s.flex_col())
}

fn extensions_page() -> impl View {
    let installed = vec![
        ("FTC SDK Bindings", "alloy.ftc-sdk", "1.2.0"),
        ("JDTLS (Java)", "redhat.java", "1.31.0"),
        ("Gradle Repair AI", "alloy.gradle-repair", "0.6.1"),
        ("Robot Telemetry", "alloy.telemetry", "0.4.0"),
    ];
    let cards = installed
        .into_iter()
        .map(|(name, id, ver)| extension_card(name, id, ver));
    v_stack((
        page_h1("Extensions", "Installed extensions and their preferences."),
        group(
            "Installed",
            floem::views::stack_from_iter(cards).style(|s| s.flex_col().gap(8.0).width_pct(100.0)),
        ),
    ))
    .style(|s| s.flex_col())
}

fn extension_card(name: &'static str, id: &'static str, ver: &'static str) -> impl View {
    h_stack((
        container(label(|| "⬚".to_string()).style(|s| s.color(ALLOY_ORANGE).font_size(T_XL)))
            .style(|s| {
                s.width(32.0)
                    .height(32.0)
                    .border_radius(R_4)
                    .background(BG_RAISED)
                    .items_center()
                    .justify_center()
            }),
        v_stack((
            label(move || name.to_string()).style(|s| {
                s.color(FG_1)
                    .font_size(T_SMALL)
                    .font_weight(floem::text::Weight::SEMIBOLD)
            }),
            label(move || format!("{id} · v{ver}")).style(|s| {
                s.color(FG_3)
                    .font_size(T_TINY)
                    .font_family("monospace".to_string())
            }),
        ))
        .style(|s| s.flex_grow(1.0).min_width(0.0).margin_horiz(12.0).gap(2.0)),
        toggle(true, ""),
    ))
    .style(|s| {
        s.padding(10.0)
            .background(BG_SURFACE)
            .border_radius(R_6)
            .items_center()
            .width_pct(100.0)
    })
}

fn keyboard_page() -> impl View {
    let shortcuts: Vec<(&'static str, &'static str)> = vec![
        ("Open command palette", "⌘K"),
        ("Open file…", "⌘P"),
        ("Find in files", "⌘⇧F"),
        ("Toggle terminal", "⌃`"),
        ("Run current OpMode", "⇧F10"),
        ("Stop deploy", "⌘F2"),
        ("Generate RobotHardware", "⌘⇧H"),
        ("Generate commit message", "⌘⇧M"),
        ("Apply Gradle Repair fix", "⌘⇧R"),
        ("Toggle Hardware Mapper", "⌘⇧K"),
    ];
    let total = shortcuts.len();
    let rows = shortcuts.into_iter().enumerate().map(|(i, (lbl, kbd))| {
        let is_last = i + 1 == total;
        h_stack((
            label(move || lbl.to_string())
                .style(|s| s.color(FG_1).font_size(T_SMALL).flex_grow(1.0)),
            label(move || kbd.to_string()).style(|s| {
                s.font_family("monospace".to_string())
                    .color(FG_2)
                    .background(BG_RAISED)
                    .padding_horiz(8.0)
                    .padding_vert(2.0)
                    .border_radius(3.0)
                    .font_size(T_TINY)
            }),
        ))
        .style(move |s| {
            let s = s
                .padding_horiz(14.0)
                .padding_vert(10.0)
                .items_center()
                .font_size(T_SMALL);
            if !is_last {
                s.border_bottom(1.0).border_color(BG_EDGE)
            } else {
                s
            }
        })
    });
    v_stack((
        page_h1(
            "Keyboard",
            "Default bindings. Custom keymaps coming in beta.",
        ),
        container(floem::views::stack_from_iter(rows).style(|s| s.flex_col().width_pct(100.0)))
            .style(|s| s.background(BG_SURFACE).border_radius(R_6).width_pct(100.0)),
    ))
    .style(|s| s.flex_col())
}

fn about_page() -> impl View {
    v_stack((
        v_stack((
            floem::views::img(|| ALLOY_LOGO.to_vec()).style(|s| s.width(80.0).height(80.0)),
            label(|| "ALLOY EDITOR".to_string()).style(|s| {
                s.color(FG_1)
                    .font_size(T_4XL)
                    .font_weight(floem::text::Weight::BOLD)
                    .margin_top(16.0)
            }),
            label(|| "v0.1.0-alpha · built for FIRST Tech Challenge".to_string()).style(|s| {
                s.color(ALLOY_ORANGE)
                    .font_size(T_TINY)
                    .font_weight(floem::text::Weight::SEMIBOLD)
                    .margin_top(8.0)
            }),
            h_stack((
                primary_btn_sm("Check for updates"),
                panel_btn_sm("Release notes"),
            ))
            .style(|s| s.gap(8.0).margin_top(16.0)),
        ))
        .style(|s| s.items_center().padding_vert(32.0).gap(0.0)),
        group("Resources", resources_grid()),
        label(|| "Forked from Lapce · Apache-2.0 · Brand marks © respective owners".to_string())
            .style(|s| {
                s.color(FG_4)
                    .font_size(T_MICRO)
                    .margin_top(24.0)
                    .width_pct(100.0)
            }),
    ))
    .style(|s| s.flex_col().width_pct(100.0))
}

fn resources_grid() -> impl View {
    let items: [(&'static str, &'static str); 4] = [
        ("GitHub repository", "github.com/OlusenBg/alloy-editor"),
        ("Documentation", "alloy.dev/docs"),
        ("FTC Discord", "discord.gg/firsttech"),
        ("Report a bug", "GitHub Issues"),
    ];
    let rows = items.into_iter().map(|(lbl, url)| {
        h_stack((
            label(|| "→".to_string())
                .style(|s| s.color(ALLOY_ORANGE).font_size(T_BASE).margin_right(10.0)),
            v_stack((
                label(move || lbl.to_string()).style(|s| s.color(FG_1).font_size(T_SMALL)),
                label(move || url.to_string()).style(|s| {
                    s.color(FG_3)
                        .font_size(T_MICRO)
                        .font_family("monospace".to_string())
                }),
            ))
            .style(|s| s.flex_grow(1.0).min_width(0.0).gap(2.0)),
            label(|| "↗".to_string()).style(|s| s.color(FG_3).font_size(T_TINY)),
        ))
        .style(|s| {
            s.padding_horiz(12.0)
                .padding_vert(10.0)
                .background(BG_SURFACE)
                .border_radius(R_6)
                .items_center()
                .cursor(CursorStyle::Pointer)
                .hover(|s| s.background(BG_HOVER))
        })
    });
    floem::views::stack_from_iter(rows).style(|s| s.flex_col().gap(6.0).width_pct(100.0))
}

fn primary_btn_sm(text: &'static str) -> impl View {
    container(label(move || text.to_string()).style(|s| {
        s.color(FG_1)
            .font_size(T_TINY)
            .font_weight(floem::text::Weight::SEMIBOLD)
    }))
    .style(|s| {
        s.padding_horiz(12.0)
            .padding_vert(6.0)
            .background(ALLOY_ORANGE)
            .border_radius(R_4)
            .cursor(CursorStyle::Pointer)
            .hover(|s| s.background(ALLOY_ORANGE_DEEP))
    })
}

fn panel_btn_sm(text: &'static str) -> impl View {
    container(label(move || text.to_string()).style(|s| s.color(FG_1).font_size(T_TINY))).style(
        |s| {
            s.padding_horiz(12.0)
                .padding_vert(6.0)
                .background(BG_RAISED)
                .border_radius(R_4)
                .cursor(CursorStyle::Pointer)
                .hover(|s| s.background(BG_HOVER))
        },
    )
}
