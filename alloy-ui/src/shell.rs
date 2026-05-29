//! Editor shell — IDE chrome compositor.
//!
//! Wires together: title bar · activity bar · sidebar · tab strip ·
//! code editor placeholder · bottom panel · status bar.
//!
//! Reference: kit/EditorShell.jsx + kit/Sidebar.jsx.

use std::sync::Arc;

use floem::View;
use floem::reactive::{RwSignal, SignalGet, create_rw_signal};
use floem::style::CursorStyle;
use floem::views::{Decorators, container, dyn_stack, empty, h_stack, label, scroll, v_stack};

use crate::theme::*;
use crate::activity_bar::{ActivityTab, activity_bar};
use crate::tab_strip::{EditorTab, tab_strip};
use crate::statusbar::{
    LspState, RobotState, StatusBarHandlers, StatusBarSignals, alloy_status_bar,
};
use crate::title_bar::{TitleBarHandlers, alloy_title_bar};
use crate::bottom_panel::{BottomPanelTab, bottom_panel};
use crate::panels::opmode::opmode_panel;

// ── File-tree node used by the Explorer sidebar ──────────────────────────────

#[derive(Clone, Copy)]
enum NodeKind { Dir, File }

#[derive(Clone, Copy)]
struct TreeNode {
    kind:  NodeKind,
    name:  &'static str,
    depth: u8,
    scm:   Option<char>,
    open:  bool,
}

const FILE_TREE: &[TreeNode] = &[
    TreeNode { kind: NodeKind::Dir,  name: "TeamCode",                                                  depth: 0, scm: None,      open: true  },
    TreeNode { kind: NodeKind::Dir,  name: "src/main/java/org/firstinspires/ftc/teamcode",              depth: 1, scm: None,      open: true  },
    TreeNode { kind: NodeKind::File, name: "RobotHardware.java",                                        depth: 2, scm: Some('A'), open: false },
    TreeNode { kind: NodeKind::File, name: "OpMode.java",                                               depth: 2, scm: Some('M'), open: false },
    TreeNode { kind: NodeKind::File, name: "RedAutoStart.java",                                         depth: 2, scm: None,      open: false },
    TreeNode { kind: NodeKind::File, name: "BlueAutoStart.java",                                        depth: 2, scm: None,      open: false },
    TreeNode { kind: NodeKind::File, name: "TeleOpMain.java",                                           depth: 2, scm: Some('M'), open: false },
    TreeNode { kind: NodeKind::Dir,  name: "FtcRobotController",                                        depth: 0, scm: None,      open: false },
    TreeNode { kind: NodeKind::File, name: "build.gradle",                                              depth: 0, scm: Some('M'), open: false },
    TreeNode { kind: NodeKind::File, name: "settings.gradle",                                           depth: 0, scm: None,      open: false },
    TreeNode { kind: NodeKind::File, name: "README.md",                                                 depth: 0, scm: None,      open: false },
];

// ── Public entry point ───────────────────────────────────────────────────────

/// Full IDE shell. Creates all reactive state internally.
/// Returns a view that fills the window.
pub fn editor_shell() -> impl View {
    // ── Activity / sidebar ─────────────────────────────────────────────────
    let activity     = create_rw_signal(ActivityTab::Files);

    // ── Editor tabs ────────────────────────────────────────────────────────
    let tabs: RwSignal<Vec<EditorTab>> = create_rw_signal(vec![
        EditorTab { id: "1".into(), name: "OpMode.java".into(),        lang: "java".into(), dirty: true  },
        EditorTab { id: "2".into(), name: "RobotHardware.java".into(), lang: "java".into(), dirty: false },
        EditorTab { id: "3".into(), name: "build.gradle".into(),       lang: "gradle".into(), dirty: false },
        EditorTab { id: "4".into(), name: "README.md".into(),          lang: "markdown".into(), dirty: false },
    ]);
    let active_tab   = create_rw_signal("1".to_string());
    let open_file    = create_rw_signal("OpMode.java".to_string());
    let breadcrumb   = create_rw_signal(vec![
        "TeamCode".to_string(), "src/main/java".to_string(), "OpMode.java".to_string(),
    ]);

    // ── Bottom panel ───────────────────────────────────────────────────────
    let bottom_tab    = create_rw_signal(BottomPanelTab::Telemetry);
    let bottom_hidden = create_rw_signal(false);
    let bottom_max    = create_rw_signal(false);

    // ── Title bar signals ──────────────────────────────────────────────────
    let project_name  = create_rw_signal("CenterStage-7842".to_string());
    let team          = create_rw_signal("Team 7842".to_string());
    let branch        = create_rw_signal("main".to_string());
    let has_update    = create_rw_signal(false);
    let workspace_open = create_rw_signal(true);
    let show_run      = create_rw_signal(true);

    // ── Status bar signals ─────────────────────────────────────────────────
    let sb = StatusBarSignals {
        branch:      create_rw_signal("main".to_string()),
        ahead:       create_rw_signal(2u32),
        behind:      create_rw_signal(0u32),
        lsp:         create_rw_signal(LspState::Ready),
        robot:       create_rw_signal(RobotState::Connected),
        error_count: create_rw_signal(1u32),
        warn_count:  create_rw_signal(2u32),
        cursor_line: create_rw_signal(16u32),
        cursor_col:  create_rw_signal(53u32),
        file_lang:   create_rw_signal("Java".to_string()),
        encoding:    create_rw_signal("UTF-8".to_string()),
        indent:      create_rw_signal("Spaces: 4".to_string()),
    };

    let noop = || Arc::new(|| {}) as Arc<dyn Fn()>;

    let title_h = TitleBarHandlers {
        on_home:     noop(),
        on_palette:  noop(),
        on_settings: noop(),
        on_run:      noop(),
    };
    let sb_h = StatusBarHandlers {
        on_branch:   noop(),
        on_lsp:      noop(),
        on_robot:    noop(),
        on_problems: noop(),
        on_lang:     noop(),
        on_encoding: noop(),
        on_indent:   noop(),
        on_settings: noop(),
    };

    // ── Tab callbacks ──────────────────────────────────────────────────────
    let tabs_for_select = tabs;
    let active_for_select = active_tab;
    let open_for_select = open_file;
    let on_select: Arc<dyn Fn(String)> = Arc::new(move |id: String| {
        active_for_select.set(id.clone());
        let name = tabs_for_select.get()
            .into_iter().find(|t| t.id == id)
            .map(|t| t.name.clone())
            .unwrap_or_default();
        open_for_select.set(name);
    });
    let tabs_for_close = tabs;
    let active_for_close = active_tab;
    let on_close: Arc<dyn Fn(String)> = Arc::new(move |id: String| {
        tabs_for_close.update(|ts| ts.retain(|t| t.id != id));
        if active_for_close.get() == id {
            let remaining = tabs_for_close.get();
            if let Some(first) = remaining.first() {
                active_for_close.set(first.id.clone());
            }
        }
    });

    // ── OpMode callbacks ───────────────────────────────────────────────────
    let on_run_opmode:   Arc<dyn Fn(&'static str)> = Arc::new(|_cls| {});
    let on_open_opmode:  Arc<dyn Fn(&'static str)> = Arc::new(|_file| {});

    // ── Layout ─────────────────────────────────────────────────────────────
    v_stack((
        alloy_title_bar(project_name, team, branch, has_update, workspace_open, show_run, title_h),
        h_stack((
            activity_bar(activity, noop()),
            sidebar(activity, open_file),
            editor_column(
                tabs, active_tab, breadcrumb, on_select, on_close,
                open_file,
                bottom_tab, bottom_hidden, bottom_max,
                on_run_opmode, on_open_opmode,
            ),
        ))
        .style(|s| s.flex_grow(1.0f32).min_height(0.0).width_pct(100.0)),
        alloy_status_bar(sb, sb_h),
    ))
    .style(|s| s.flex_col().width_pct(100.0).height_pct(100.0).background(BG_NAVY))
}

// ── Sidebar ──────────────────────────────────────────────────────────────────

fn sidebar(activity: RwSignal<ActivityTab>, open_file: RwSignal<String>) -> impl View {
    container(
        v_stack((
            // Files pane
            container(explorer(open_file))
                .style(move |s| {
                    let s = s.flex_col().width_pct(100.0).height_pct(100.0);
                    if activity.get() == ActivityTab::Files { s } else { s.hide() }
                }),
            // Search pane
            container(search_panel())
                .style(move |s| {
                    let s = s.flex_col().width_pct(100.0).height_pct(100.0);
                    if activity.get() == ActivityTab::Search { s } else { s.hide() }
                }),
            // Source control pane
            container(scm_sidebar())
                .style(move |s| {
                    let s = s.flex_col().width_pct(100.0).height_pct(100.0);
                    if activity.get() == ActivityTab::SourceControl { s } else { s.hide() }
                }),
            // OpModes pane
            container(opmode_panel(
                Arc::new(|_| {}),
                Arc::new(|_| {}),
            ))
            .style(move |s| {
                let s = s.flex_col().width_pct(100.0).height_pct(100.0);
                if activity.get() == ActivityTab::OpModes { s } else { s.hide() }
            }),
            // Extensions pane
            container(extensions_panel())
                .style(move |s| {
                    let s = s.flex_col().width_pct(100.0).height_pct(100.0);
                    if activity.get() == ActivityTab::Extensions { s } else { s.hide() }
                }),
        ))
        .style(|s| s.flex_col().width_pct(100.0).height_pct(100.0)),
    )
    .style(|s| {
        s.width(280.0)
            .flex_shrink(0.0)
            .height_pct(100.0)
            .background(BG_SURFACE)
            .border_right(1.0)
            .border_color(BG_EDGE)
    })
}

// ── Explorer sidebar pane ─────────────────────────────────────────────────────

fn explorer(open_file: RwSignal<String>) -> impl View {
    v_stack((
        panel_header("Explorer"),
        label(|| "  ⌄ CenterStage-7842".to_string())
            .style(|s| {
                s.color(FG_2).font_size(T_TINY).font_weight(floem::text::Weight::BOLD)
                    .padding_horiz(8.0).padding_vert(4.0)
            }),
        scroll(
            dyn_stack(
                || FILE_TREE.iter().enumerate().collect::<Vec<_>>(),
                |(i, _)| *i,
                move |(_, node)| file_row(*node, open_file),
            )
            .style(|s| s.flex_col()),
        )
        .style(|s| s.flex_grow(1.0f32)),
    ))
    .style(|s| s.flex_col().height_pct(100.0))
}

fn file_row(node: TreeNode, open_file: RwSignal<String>) -> impl View {
    let indent = 8.0 + (node.depth as f64) * 14.0;
    let is_dir = matches!(node.kind, NodeKind::Dir);
    let name = node.name;

    container(
        h_stack((
            label(move || {
                if is_dir {
                    (if node.open { "⌄ " } else { "› " }).to_string()
                } else {
                    "  ".to_string()
                }
            })
            .style(|s| s.color(FG_3).font_size(T_TINY).min_width(16.0)),
            label(move || name.to_string()).style(move |s| {
                s.flex_grow(1.0f32).font_size(T_SMALL).color(FG_2)
                    .apply_if(is_dir, |s| s.color(FG_1).font_weight(floem::text::Weight::SEMIBOLD))
            }),
            label(move || {
                node.scm.map(|c| c.to_string()).unwrap_or_default()
            })
            .style(|s| {
                s.font_size(T_TINY).font_family("monospace".to_string())
                    .font_weight(floem::text::Weight::BOLD)
            })
            .style(move |s| match node.scm {
                Some('A') => s.color(SCM_ADDED),
                Some('M') => s.color(SCM_MODIFIED),
                Some('D') => s.color(SCM_REMOVED),
                _         => s.color(FG_4),
            }),
        ))
        .style(|s| s.items_center().width_pct(100.0)),
    )
    .on_click_stop(move |_| {
        if !is_dir { open_file.set(name.to_string()); }
    })
    .style(move |s| {
        let active = !is_dir && open_file.get() == name;
        s.padding_left(indent)
            .padding_right(10.0)
            .height(22.0)
            .width_pct(100.0)
            .items_center()
            .apply_if(!is_dir, |s| s.cursor(CursorStyle::Pointer))
            .apply_if(active, |s| s.background(BG_CURRENT))
            .hover(|s| if !active { s.background(BG_HOVER) } else { s })
    })
}

// ── Search sidebar pane ───────────────────────────────────────────────────────

fn search_panel() -> impl View {
    v_stack((
        panel_header("Search"),
        v_stack((
            label(|| "hardwareMap.get".to_string())
                .style(|s| {
                    s.width_pct(100.0).padding(6.0).background(BG_NAVY)
                        .border(1.0).border_color(LINE_RING).border_radius(R_4)
                        .color(FG_1).font_size(T_SMALL).font_family("monospace".to_string())
                }),
            label(|| "3 results in 2 files".to_string())
                .style(|s| s.color(FG_3).font_size(T_TINY).padding_top(8.0)),
            label(|| "OpMode.java".to_string())
                .style(|s| s.color(FG_1).font_size(T_SMALL).padding_top(6.0)),
            label(|| "  arm = hardwareMap.get(…);".to_string())
                .style(|s| s.color(FG_3).font_size(T_TINY).font_family("monospace".to_string())),
            label(|| "  claw = hardwareMap.get(…);".to_string())
                .style(|s| s.color(FG_3).font_size(T_TINY).font_family("monospace".to_string())),
            label(|| "RobotHardware.java".to_string())
                .style(|s| s.color(FG_1).font_size(T_SMALL).padding_top(6.0)),
            label(|| "  drive = hardwareMap.get(…);".to_string())
                .style(|s| s.color(FG_3).font_size(T_TINY).font_family("monospace".to_string())),
        ))
        .style(|s| s.flex_col().padding(12.0).gap(2.0)),
    ))
    .style(|s| s.flex_col())
}

// ── SCM sidebar pane ──────────────────────────────────────────────────────────

fn scm_sidebar() -> impl View {
    let commit_msg = create_rw_signal("add ArmController unit tests".to_string());

    v_stack((
        panel_header("Source Control"),
        v_stack((
            label(move || commit_msg.get())
                .style(|s| {
                    s.width_pct(100.0).min_height(56.0).padding(8.0)
                        .background(BG_NAVY).border(1.0).border_color(LINE_RING).border_radius(R_4)
                        .color(FG_1).font_size(T_SMALL)
                }),
            container(
                label(|| "✓ Commit".to_string())
                    .style(|s| s.color(floem::peniko::Color::WHITE).font_size(T_SMALL)),
            )
            .style(|s| {
                s.width_pct(100.0).height(28.0).items_center().justify_center()
                    .background(ALLOY_ORANGE).border_radius(R_4).cursor(CursorStyle::Pointer)
                    .margin_top(8.0)
                    .hover(|s| s.background(ALLOY_ORANGE_DEEP))
            }),
            label(|| "Changes (3)".to_string())
                .style(|s| {
                    s.color(FG_2).font_size(T_TINY).font_weight(floem::text::Weight::BOLD)
                        .padding_top(14.0).padding_bottom(6.0)
                }),
            scm_row("OpMode.java", 'M'),
            scm_row("build.gradle", 'M'),
            scm_row("RobotHardware.java", 'A'),
        ))
        .style(|s| s.flex_col().padding(12.0)),
    ))
    .style(|s| s.flex_col())
}

fn scm_row(file: &'static str, scm: char) -> impl View {
    let color = match scm {
        'A' => SCM_ADDED,
        'M' => SCM_MODIFIED,
        _   => SCM_REMOVED,
    };
    h_stack((
        label(|| "· ".to_string()).style(|s| s.color(FG_3).font_size(T_SMALL)),
        label(move || file.to_string()).style(|s| s.flex_grow(1.0f32).color(FG_1).font_size(T_SMALL)),
        label(move || scm.to_string()).style(move |s| {
            s.color(color).font_family("monospace".to_string())
                .font_weight(floem::text::Weight::BOLD).font_size(T_TINY)
        }),
    ))
    .style(|s| s.items_center().padding_vert(4.0).gap(8.0))
}

// ── Extensions sidebar pane ───────────────────────────────────────────────────

fn extensions_panel() -> impl View {
    const EXTS: &[(&str, &str, bool)] = &[
        ("FTC SDK Bindings",  "alloy.ftc-sdk",       true),
        ("JDTLS (Java)",       "redhat.java",         true),
        ("Gradle Repair AI",   "alloy.gradle-repair", true),
        ("Robot Telemetry",    "alloy.telemetry",     true),
        ("Onbot Java",         "alloy.onbot-java",    false),
    ];

    v_stack((
        panel_header("Extensions"),
        scroll(
            dyn_stack(
                || EXTS.iter().enumerate().collect::<Vec<_>>(),
                |(i, _)| *i,
                |(_, ext)| ext_row(ext.0, ext.1, ext.2),
            )
            .style(|s| s.flex_col()),
        )
        .style(|s| s.flex_grow(1.0f32)),
    ))
    .style(|s| s.flex_col().height_pct(100.0))
}

fn ext_row(name: &'static str, id: &'static str, installed: bool) -> impl View {
    let status_color = if installed { STATUS_SUCCESS } else { ALLOY_ORANGE };
    let status_text  = if installed { "Installed" } else { "Recommended" };

    h_stack((
        container(
            label(|| "⊞".to_string()).style(|s| s.color(ALLOY_ORANGE).font_size(T_LG)),
        )
        .style(|s| {
            s.width(32.0).height(32.0).border_radius(R_4)
                .background(BG_RAISED).items_center().justify_center()
        }),
        v_stack((
            label(move || name.to_string())
                .style(|s| s.color(FG_1).font_size(T_SMALL).font_weight(floem::text::Weight::SEMIBOLD)),
            label(move || id.to_string())
                .style(|s| s.color(FG_3).font_size(T_MICRO).font_family("monospace".to_string())),
            label(move || status_text.to_string())
                .style(move |s| s.color(status_color).font_size(T_MICRO).padding_top(2.0)),
        ))
        .style(|s| s.flex_col().flex_grow(1.0f32).margin_left(10.0)),
    ))
    .style(|s| {
        s.items_center().padding(10.0).border_bottom(1.0).border_color(BG_EDGE)
    })
}

// ── Editor column ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn editor_column(
    tabs:       RwSignal<Vec<EditorTab>>,
    active_tab: RwSignal<String>,
    breadcrumb: RwSignal<Vec<String>>,
    on_select:  Arc<dyn Fn(String)>,
    on_close:   Arc<dyn Fn(String)>,
    open_file:  RwSignal<String>,
    bottom_tab:    RwSignal<BottomPanelTab>,
    bottom_hidden: RwSignal<bool>,
    bottom_max:    RwSignal<bool>,
    _on_run_opmode: Arc<dyn Fn(&'static str)>,
    _on_open_opmode: Arc<dyn Fn(&'static str)>,
) -> impl View {
    let on_maximize = {
        let bm = bottom_max;
        Arc::new(move || bm.update(|v| *v = !*v)) as Arc<dyn Fn()>
    };
    let on_close_panel = {
        let bh = bottom_hidden;
        Arc::new(move || bh.set(true)) as Arc<dyn Fn()>
    };

    v_stack((
        tab_strip(tabs, active_tab, breadcrumb, on_select, on_close),
        // Code editor placeholder
        container(code_placeholder(open_file))
            .style(move |s| {
                let s = s.flex_grow(1.0f32).min_height(0.0).width_pct(100.0);
                if bottom_max.get() { s.hide() } else { s }
            }),
        // Bottom panel
        container(
            bottom_panel(bottom_tab, on_maximize, on_close_panel),
        )
        .style(move |s| {
            let s = s.flex_shrink(0.0).width_pct(100.0)
                .border_top(1.0).border_color(BG_EDGE);
            if bottom_hidden.get() {
                s.hide()
            } else if bottom_max.get() {
                s.flex_grow(1.0f32).min_height(0.0)
            } else {
                s.height(280.0)
            }
        }),
    ))
    .style(|s| s.flex_col().flex_grow(1.0f32).min_width(0.0).height_pct(100.0))
}

// ── Code placeholder ─────────────────────────────────────────────────────────

fn code_placeholder(open_file: RwSignal<String>) -> impl View {
    container(
        v_stack((
            label(move || open_file.get())
                .style(|s| {
                    s.color(FG_4).font_size(T_3XL)
                        .font_weight(floem::text::Weight::BOLD)
                        .font_family("monospace".to_string())
                }),
            label(|| "Code editor  —  coming soon".to_string())
                .style(|s| s.color(FG_4).font_size(T_SMALL).padding_top(8.0)),
        ))
        .style(|s| s.flex_col().items_center()),
    )
    .style(|s| {
        s.width_pct(100.0).height_pct(100.0)
            .background(BG_NAVY)
            .items_center()
            .justify_center()
    })
}

// ── Shared helpers ────────────────────────────────────────────────────────────

fn panel_header(title: &'static str) -> impl View {
    h_stack((
        label(move || title.to_string()).style(|s| {
            s.flex_grow(1.0f32).color(FG_2).font_size(T_TINY)
                .font_weight(floem::text::Weight::BOLD)
        }),
    ))
    .style(|s| {
        s.height(36.0).padding_horiz(14.0).items_center()
            .border_bottom(1.0).border_color(BG_EDGE).flex_shrink(0.0)
    })
}
