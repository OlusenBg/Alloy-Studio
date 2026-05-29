//! Editor shell — IDE chrome compositor.
//!
//! Wires together: title bar · activity bar · sidebar · tab strip ·
//! code editor placeholder · bottom panel · status bar.
//!
//! Reference: kit/EditorShell.jsx + kit/Sidebar.jsx.

use std::sync::Arc;

use floem::ext_event::create_ext_action;
use floem::reactive::{RwSignal, Scope, SignalGet, SignalUpdate};
use floem::style::CursorStyle;
use floem::views::{container, dyn_stack, h_stack, label, v_stack, Decorators};
use floem::View;

use crate::activity_bar::{activity_bar, ActivityTab};
use crate::bottom_panel::{bottom_panel, BottomPanelTab};
use crate::bridge::AppBridge;
use crate::panels::opmode::opmode_panel;
use crate::statusbar::{
    alloy_status_bar, LspState, RobotState, StatusBarHandlers, StatusBarSignals,
};
use crate::tab_strip::{tab_strip, EditorTab};
use crate::theme::*;
use crate::title_bar::{alloy_title_bar, TitleBarHandlers};

// ── File-tree node used by the Explorer sidebar ──────────────────────────────

#[derive(Clone, Copy)]
enum NodeKind {
    Dir,
    File,
}

#[derive(Clone, Copy)]
struct TreeNode {
    kind: NodeKind,
    name: &'static str,
    depth: u8,
    scm: Option<char>,
    open: bool,
}

const FILE_TREE: &[TreeNode] = &[
    TreeNode {
        kind: NodeKind::Dir,
        name: "TeamCode",
        depth: 0,
        scm: None,
        open: true,
    },
    TreeNode {
        kind: NodeKind::Dir,
        name: "src/main/java/org/firstinspires/ftc/teamcode",
        depth: 1,
        scm: None,
        open: true,
    },
    TreeNode {
        kind: NodeKind::File,
        name: "RobotHardware.java",
        depth: 2,
        scm: Some('A'),
        open: false,
    },
    TreeNode {
        kind: NodeKind::File,
        name: "OpMode.java",
        depth: 2,
        scm: Some('M'),
        open: false,
    },
    TreeNode {
        kind: NodeKind::File,
        name: "RedAutoStart.java",
        depth: 2,
        scm: None,
        open: false,
    },
    TreeNode {
        kind: NodeKind::File,
        name: "BlueAutoStart.java",
        depth: 2,
        scm: None,
        open: false,
    },
    TreeNode {
        kind: NodeKind::File,
        name: "TeleOpMain.java",
        depth: 2,
        scm: Some('M'),
        open: false,
    },
    TreeNode {
        kind: NodeKind::Dir,
        name: "FtcRobotController",
        depth: 0,
        scm: None,
        open: false,
    },
    TreeNode {
        kind: NodeKind::File,
        name: "build.gradle",
        depth: 0,
        scm: Some('M'),
        open: false,
    },
    TreeNode {
        kind: NodeKind::File,
        name: "settings.gradle",
        depth: 0,
        scm: None,
        open: false,
    },
    TreeNode {
        kind: NodeKind::File,
        name: "README.md",
        depth: 0,
        scm: None,
        open: false,
    },
];

// ── Dynamic file-tree entry (owned strings, for real data) ────────────────────

#[derive(Clone)]
pub struct DynTreeNode {
    pub is_dir: bool,
    pub name: String,
    pub depth: u8,
    pub scm: Option<char>,
}

// ── Public entry point ───────────────────────────────────────────────────────

/// Full IDE shell. Creates all reactive state internally.
/// Returns a view that fills the window.
///
/// When `bridge` is `Some`, uses real backend data; falls back to demo data otherwise.
pub fn editor_shell(bridge: Option<Arc<AppBridge>>) -> impl View {
    let cx = Scope::new();

    // ── Activity / sidebar ─────────────────────────────────────────────────
    let activity = RwSignal::new(ActivityTab::Files);

    // ── Editor tabs ────────────────────────────────────────────────────────
    let tabs: RwSignal<Vec<EditorTab>> = RwSignal::new(vec![
        EditorTab {
            id: "1".into(),
            name: "OpMode.java".into(),
            lang: "java".into(),
            dirty: true,
        },
        EditorTab {
            id: "2".into(),
            name: "RobotHardware.java".into(),
            lang: "java".into(),
            dirty: false,
        },
        EditorTab {
            id: "3".into(),
            name: "build.gradle".into(),
            lang: "gradle".into(),
            dirty: false,
        },
        EditorTab {
            id: "4".into(),
            name: "README.md".into(),
            lang: "markdown".into(),
            dirty: false,
        },
    ]);
    let active_tab = RwSignal::new("1".to_string());
    let open_file = RwSignal::new("OpMode.java".to_string());
    let breadcrumb = RwSignal::new(vec![
        "TeamCode".to_string(),
        "src/main/java".to_string(),
        "OpMode.java".to_string(),
    ]);

    // ── Bottom panel ───────────────────────────────────────────────────────
    let bottom_tab = RwSignal::new(BottomPanelTab::Telemetry);
    let bottom_hidden = RwSignal::new(false);
    let bottom_max = RwSignal::new(false);

    // ── Terminal lines signal (updated by gradle runner) ───────────────────
    let terminal_lines: RwSignal<Vec<String>> = RwSignal::new(vec![
        "> Configure project :TeamCode".to_string(),
        "> Task :TeamCode:assembleDebug".to_string(),
        "BUILD SUCCESSFUL in 11s".to_string(),
    ]);

    // ── Title bar signals ──────────────────────────────────────────────────
    let project_name = RwSignal::new("CenterStage-7842".to_string());
    let team = RwSignal::new("Team 7842".to_string());
    let branch = RwSignal::new("main".to_string());
    let has_update = RwSignal::new(false);
    let workspace_open = RwSignal::new(true);
    let show_run = RwSignal::new(true);

    // ── Status bar signals ─────────────────────────────────────────────────
    let sb_branch = RwSignal::new("main".to_string());
    let sb_ahead = RwSignal::new(0u32);
    let sb_behind = RwSignal::new(0u32);
    let sb_robot = RwSignal::new(RobotState::Disconnected);

    let sb = StatusBarSignals {
        branch: sb_branch,
        ahead: sb_ahead,
        behind: sb_behind,
        lsp: RwSignal::new(LspState::Ready),
        robot: sb_robot,
        error_count: RwSignal::new(0u32),
        warn_count: RwSignal::new(0u32),
        cursor_line: RwSignal::new(1u32),
        cursor_col: RwSignal::new(1u32),
        file_lang: RwSignal::new("Java".to_string()),
        encoding: RwSignal::new("UTF-8".to_string()),
        indent: RwSignal::new("Spaces: 4".to_string()),
    };

    // ── Dynamic file tree signal ───────────────────────────────────────────
    let dyn_file_tree: RwSignal<Vec<DynTreeNode>> = RwSignal::new(vec![]);
    let use_real_tree = RwSignal::new(false);

    // ── Wire real data from bridge ─────────────────────────────────────────
    if let Some(ref b) = bridge {
        // Wire file tree
        {
            let workspace = Arc::clone(&b.workspace);
            let action = create_ext_action(cx, move |entries: Vec<DynTreeNode>| {
                dyn_file_tree.set(entries);
                use_real_tree.set(true);
            });
            b.tokio.spawn(async move {
                let entries = workspace.file_tree();
                let nodes: Vec<DynTreeNode> = entries
                    .into_iter()
                    .map(|e| {
                        let name = if e.depth == 0 {
                            e.relative_path.clone()
                        } else {
                            e.path
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| e.relative_path.clone())
                        };
                        DynTreeNode {
                            is_dir: e.is_dir,
                            name,
                            depth: e.depth.min(255) as u8,
                            scm: None,
                        }
                    })
                    .collect();
                action(nodes);
            });
        }

        // Wire project name from workspace root
        {
            let root = b.workspace_root.clone();
            let action = create_ext_action(cx, move |name: String| {
                project_name.set(name);
            });
            let name = root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Alloy Studio".to_string());
            action(name);
        }

        // Wire git branch to status bar and title bar
        if let Some(ref git) = b.git_repo {
            let repo = Arc::clone(git);
            let action = create_ext_action(cx, move |(br, name): (String, String)| {
                sb_branch.set(br.clone());
                branch.set(br);
                project_name.set(name);
            });
            let workspace_name = b
                .workspace_root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Alloy Studio".to_string());
            b.tokio.spawn(async move {
                let br = repo
                    .with_repo(|r| {
                        let head = r.head().ok();
                        let branch_name = head
                            .as_ref()
                            .and_then(|h| h.shorthand().map(|s| s.to_string()))
                            .unwrap_or_else(|| "HEAD".to_string());
                        Ok(branch_name)
                    })
                    .await
                    .unwrap_or_else(|_| "HEAD".to_string());
                action((br, workspace_name));
            });
        }

        // Wire ongoing telemetry robot state updates
        {
            let telemetry = Arc::clone(&b.telemetry);
            let handle = b.tokio.clone();
            // Use a RwSignal<Option<RobotState>> as intermediate for update_signal_from_channel
            let robot_state_opt: RwSignal<Option<RobotState>> = RwSignal::new(None);
            let (tx, rx) = std::sync::mpsc::channel::<RobotState>();
            floem::ext_event::update_signal_from_channel(robot_state_opt.write_only(), rx);
            // Drive sb_robot from the intermediate signal
            floem::reactive::create_effect(move |_| {
                if let Some(state) = robot_state_opt.get() {
                    sb_robot.set(state);
                }
            });
            handle.spawn(async move {
                let mut sub = telemetry.subscribe();
                loop {
                    match tokio::time::timeout(std::time::Duration::from_secs(5), sub.recv()).await
                    {
                        Ok(Ok(_)) => {
                            let _ = tx.send(RobotState::Connected);
                        }
                        Ok(Err(_)) => {
                            break;
                        }
                        Err(_) => {
                            let _ = tx.send(RobotState::Disconnected);
                        }
                    }
                }
            });
        }
    }

    // ── Deploy button handler ──────────────────────────────────────────────
    let on_run: Arc<dyn Fn()> = if let Some(ref b) = bridge {
        if let Some(ref ftc) = b.ftc_project {
            let project = Arc::new(ftc.clone());
            let tokio_handle = b.tokio.clone();
            let cx2 = cx;
            let term_lines_clone = terminal_lines;
            let bottom_tab_clone = bottom_tab;
            Arc::new(move || {
                // Switch to Terminal tab
                bottom_tab_clone.set(BottomPanelTab::Terminal);

                let project = Arc::clone(&project);
                let action = create_ext_action(cx2, move |lines: Vec<String>| {
                    term_lines_clone.set(lines);
                });

                tokio_handle.spawn(async move {
                    let runner = alloy_gradle::runner::GradleRunner::new(project);
                    let (mut rx_event, _handle) =
                        runner.run(&[alloy_gradle::runner::GradleTask::AssembleDebug]);

                    let mut all_lines: Vec<String> = Vec::new();
                    loop {
                        match rx_event.recv().await {
                            Ok(event) => match event {
                                alloy_rpc::types::BuildEvent::OutputLine(text) => {
                                    all_lines.push(text);
                                }
                                alloy_rpc::types::BuildEvent::ErrorDetected(err) => {
                                    all_lines.push(format!("ERROR: {}", err.message));
                                }
                                alloy_rpc::types::BuildEvent::Finished { exit_code, .. } => {
                                    let msg = if exit_code == 0 {
                                        "BUILD SUCCESSFUL".to_string()
                                    } else {
                                        format!("BUILD FAILED (exit code {})", exit_code)
                                    };
                                    all_lines.push(msg);
                                    break;
                                }
                            },
                            Err(_) => break,
                        }
                    }
                    action(all_lines);
                });
            })
        } else {
            Arc::new(|| {})
        }
    } else {
        Arc::new(|| {})
    };

    let noop = || Arc::new(|| {}) as Arc<dyn Fn()>;

    let title_h = TitleBarHandlers {
        on_home: noop(),
        on_palette: noop(),
        on_settings: noop(),
        on_run: on_run.clone(),
    };
    let sb_h = StatusBarHandlers {
        on_branch: noop(),
        on_lsp: noop(),
        on_robot: noop(),
        on_problems: noop(),
        on_lang: noop(),
        on_encoding: noop(),
        on_indent: noop(),
        on_settings: noop(),
    };

    // ── Tab callbacks ──────────────────────────────────────────────────────
    let tabs_for_select = tabs;
    let active_for_select = active_tab;
    let open_for_select = open_file;
    let on_select: Arc<dyn Fn(String)> = Arc::new(move |id: String| {
        active_for_select.set(id.clone());
        let name = tabs_for_select
            .get()
            .into_iter()
            .find(|t| t.id == id)
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
    let on_run_opmode: Arc<dyn Fn(&'static str)> = Arc::new(|_cls| {});
    let on_open_opmode: Arc<dyn Fn(&'static str)> = Arc::new(|_file| {});

    // ── Layout ─────────────────────────────────────────────────────────────
    v_stack((
        alloy_title_bar(
            project_name,
            team,
            branch,
            has_update,
            workspace_open,
            show_run,
            title_h,
        ),
        h_stack((
            activity_bar(activity, noop()),
            sidebar(
                activity,
                open_file,
                dyn_file_tree,
                use_real_tree,
                bridge.clone(),
            ),
            editor_column(
                tabs,
                active_tab,
                breadcrumb,
                on_select,
                on_close,
                open_file,
                bottom_tab,
                bottom_hidden,
                bottom_max,
                on_run_opmode,
                on_open_opmode,
                terminal_lines,
            ),
        ))
        .style(|s| s.flex_grow(1.0f32).min_height(0.0).width_pct(100.0)),
        alloy_status_bar(sb, sb_h),
    ))
    .style(|s| {
        s.flex_col()
            .width_pct(100.0)
            .height_pct(100.0)
            .background(BG_NAVY)
    })
}

// ── Sidebar ──────────────────────────────────────────────────────────────────

fn sidebar(
    activity: RwSignal<ActivityTab>,
    open_file: RwSignal<String>,
    dyn_file_tree: RwSignal<Vec<DynTreeNode>>,
    use_real_tree: RwSignal<bool>,
    bridge: Option<Arc<AppBridge>>,
) -> impl View {
    container(
        v_stack((
            // Files pane
            container(explorer(open_file, dyn_file_tree, use_real_tree)).style(move |s| {
                let s = s.flex_col().width_pct(100.0).height_pct(100.0);
                if activity.get() == ActivityTab::Files {
                    s
                } else {
                    s.hide()
                }
            }),
            // Search pane
            container(search_panel()).style(move |s| {
                let s = s.flex_col().width_pct(100.0).height_pct(100.0);
                if activity.get() == ActivityTab::Search {
                    s
                } else {
                    s.hide()
                }
            }),
            // Source control pane
            container(scm_sidebar()).style(move |s| {
                let s = s.flex_col().width_pct(100.0).height_pct(100.0);
                if activity.get() == ActivityTab::SourceControl {
                    s
                } else {
                    s.hide()
                }
            }),
            // OpModes pane
            container(opmode_panel_wrapper(bridge)).style(move |s| {
                let s = s.flex_col().width_pct(100.0).height_pct(100.0);
                if activity.get() == ActivityTab::OpModes {
                    s
                } else {
                    s.hide()
                }
            }),
            // Extensions pane
            container(extensions_panel()).style(move |s| {
                let s = s.flex_col().width_pct(100.0).height_pct(100.0);
                if activity.get() == ActivityTab::Extensions {
                    s
                } else {
                    s.hide()
                }
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

// ── OpMode panel wrapper that wires real data ─────────────────────────────────

fn opmode_panel_wrapper(_bridge: Option<Arc<AppBridge>>) -> impl View {
    opmode_panel(Arc::new(|_| {}), Arc::new(|_| {}))
}

// ── Explorer sidebar pane ─────────────────────────────────────────────────────

fn explorer(
    open_file: RwSignal<String>,
    dyn_file_tree: RwSignal<Vec<DynTreeNode>>,
    use_real_tree: RwSignal<bool>,
) -> impl View {
    v_stack((
        panel_header("Explorer"),
        label(|| "  ⌄ Workspace".to_string()).style(|s| {
            s.color(FG_2)
                .font_size(T_TINY)
                .font_weight(floem::text::FontWeight::BOLD)
                .padding_horiz(8.0)
                .padding_vert(4.0)
        }),
        scroll(
            v_stack((
                // Real file tree (shown when bridge has data)
                container(
                    dyn_stack(
                        move || {
                            dyn_file_tree
                                .get()
                                .into_iter()
                                .enumerate()
                                .collect::<Vec<_>>()
                        },
                        |(i, _)| *i,
                        move |(_, node)| dyn_file_row(node, open_file),
                    )
                    .style(|s| s.flex_col()),
                )
                .style(move |s| if use_real_tree.get() { s } else { s.hide() }),
                // Demo file tree (shown when no bridge)
                container(
                    dyn_stack(
                        || FILE_TREE.iter().enumerate().collect::<Vec<_>>(),
                        |(i, _)| *i,
                        move |(_, node)| file_row(*node, open_file),
                    )
                    .style(|s| s.flex_col()),
                )
                .style(move |s| if use_real_tree.get() { s.hide() } else { s }),
            ))
            .style(|s| s.flex_col()),
        )
        .style(|s| s.flex_grow(1.0f32)),
    ))
    .style(|s| s.flex_col().height_pct(100.0))
}

fn dyn_file_row(node: DynTreeNode, open_file: RwSignal<String>) -> impl View {
    let indent = 8.0 + (node.depth as f64) * 14.0;
    let is_dir = node.is_dir;
    let name = node.name.clone();
    let name_for_click = name.clone();
    let name_for_active = name.clone();

    container(
        h_stack((
            label(move || {
                if is_dir {
                    "⌄ ".to_string()
                } else {
                    "  ".to_string()
                }
            })
            .style(|s| s.color(FG_3).font_size(T_TINY).min_width(16.0)),
            label({
                let n = name.clone();
                move || n.clone()
            })
            .style(move |s| {
                s.flex_grow(1.0f32)
                    .font_size(T_SMALL)
                    .color(FG_2)
                    .apply_if(is_dir, |s| {
                        s.color(FG_1)
                            .font_weight(floem::text::FontWeight::SEMI_BOLD)
                    })
            }),
            label(move || node.scm.map(|c| c.to_string()).unwrap_or_default()).style(move |s| {
                let s = s
                    .font_size(T_TINY)
                    .font_family("monospace".to_string())
                    .font_weight(floem::text::FontWeight::BOLD);
                match node.scm {
                    Some('A') => s.color(SCM_ADDED),
                    Some('M') => s.color(SCM_MODIFIED),
                    Some('D') => s.color(SCM_REMOVED),
                    _ => s.color(FG_4),
                }
            }),
        ))
        .style(|s| s.items_center().width_pct(100.0)),
    )
    .on_click_stop(move |_| {
        if !is_dir {
            open_file.set(name_for_click.clone());
        }
    })
    .style(move |s| {
        let active = !is_dir && open_file.get() == name_for_active;
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
                s.flex_grow(1.0f32)
                    .font_size(T_SMALL)
                    .color(FG_2)
                    .apply_if(is_dir, |s| {
                        s.color(FG_1)
                            .font_weight(floem::text::FontWeight::SEMI_BOLD)
                    })
            }),
            label(move || node.scm.map(|c| c.to_string()).unwrap_or_default())
                .style(|s| {
                    s.font_size(T_TINY)
                        .font_family("monospace".to_string())
                        .font_weight(floem::text::FontWeight::BOLD)
                })
                .style(move |s| match node.scm {
                    Some('A') => s.color(SCM_ADDED),
                    Some('M') => s.color(SCM_MODIFIED),
                    Some('D') => s.color(SCM_REMOVED),
                    _ => s.color(FG_4),
                }),
        ))
        .style(|s| s.items_center().width_pct(100.0)),
    )
    .on_click_stop(move |_| {
        if !is_dir {
            open_file.set(name.to_string());
        }
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
            label(|| "hardwareMap.get".to_string()).style(|s| {
                s.width_pct(100.0)
                    .padding(6.0)
                    .background(BG_NAVY)
                    .border(1.0)
                    .border_color(LINE_RING)
                    .border_radius(R_4)
                    .color(FG_1)
                    .font_size(T_SMALL)
                    .font_family("monospace".to_string())
            }),
            label(|| "3 results in 2 files".to_string())
                .style(|s| s.color(FG_3).font_size(T_TINY).padding_top(8.0)),
            label(|| "OpMode.java".to_string())
                .style(|s| s.color(FG_1).font_size(T_SMALL).padding_top(6.0)),
            label(|| "  arm = hardwareMap.get(…);".to_string()).style(|s| {
                s.color(FG_3)
                    .font_size(T_TINY)
                    .font_family("monospace".to_string())
            }),
            label(|| "  claw = hardwareMap.get(…);".to_string()).style(|s| {
                s.color(FG_3)
                    .font_size(T_TINY)
                    .font_family("monospace".to_string())
            }),
            label(|| "RobotHardware.java".to_string())
                .style(|s| s.color(FG_1).font_size(T_SMALL).padding_top(6.0)),
            label(|| "  drive = hardwareMap.get(…);".to_string()).style(|s| {
                s.color(FG_3)
                    .font_size(T_TINY)
                    .font_family("monospace".to_string())
            }),
        ))
        .style(|s| s.flex_col().padding(12.0).gap(2.0)),
    ))
    .style(|s| s.flex_col())
}

// ── SCM sidebar pane ──────────────────────────────────────────────────────────

fn scm_sidebar() -> impl View {
    let commit_msg = RwSignal::new("add ArmController unit tests".to_string());

    v_stack((
        panel_header("Source Control"),
        v_stack((
            label(move || commit_msg.get()).style(|s| {
                s.width_pct(100.0)
                    .min_height(56.0)
                    .padding(8.0)
                    .background(BG_NAVY)
                    .border(1.0)
                    .border_color(LINE_RING)
                    .border_radius(R_4)
                    .color(FG_1)
                    .font_size(T_SMALL)
            }),
            container(
                label(|| "✓ Commit".to_string())
                    .style(|s| s.color(floem::peniko::Color::WHITE).font_size(T_SMALL)),
            )
            .style(|s| {
                s.width_pct(100.0)
                    .height(28.0)
                    .items_center()
                    .justify_center()
                    .background(ALLOY_ORANGE)
                    .border_radius(R_4)
                    .cursor(CursorStyle::Pointer)
                    .margin_top(8.0)
                    .hover(|s| s.background(ALLOY_ORANGE_DEEP))
            }),
            label(|| "Changes (3)".to_string()).style(|s| {
                s.color(FG_2)
                    .font_size(T_TINY)
                    .font_weight(floem::text::FontWeight::BOLD)
                    .padding_top(14.0)
                    .padding_bottom(6.0)
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
        _ => SCM_REMOVED,
    };
    h_stack((
        label(|| "· ".to_string()).style(|s| s.color(FG_3).font_size(T_SMALL)),
        label(move || file.to_string())
            .style(|s| s.flex_grow(1.0f32).color(FG_1).font_size(T_SMALL)),
        label(move || scm.to_string()).style(move |s| {
            s.color(color)
                .font_family("monospace".to_string())
                .font_weight(floem::text::FontWeight::BOLD)
                .font_size(T_TINY)
        }),
    ))
    .style(|s| s.items_center().padding_vert(4.0).gap(8.0))
}

// ── Extensions sidebar pane ───────────────────────────────────────────────────

fn extensions_panel() -> impl View {
    const EXTS: &[(&str, &str, bool)] = &[
        ("FTC SDK Bindings", "alloy.ftc-sdk", true),
        ("JDTLS (Java)", "redhat.java", true),
        ("Gradle Repair AI", "alloy.gradle-repair", true),
        ("Robot Telemetry", "alloy.telemetry", true),
        ("Onbot Java", "alloy.onbot-java", false),
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
    let status_color = if installed {
        STATUS_SUCCESS
    } else {
        ALLOY_ORANGE
    };
    let status_text = if installed {
        "Installed"
    } else {
        "Recommended"
    };

    h_stack((
        container(label(|| "⊞".to_string()).style(|s| s.color(ALLOY_ORANGE).font_size(T_LG)))
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
                    .font_weight(floem::text::FontWeight::SEMI_BOLD)
            }),
            label(move || id.to_string()).style(|s| {
                s.color(FG_3)
                    .font_size(T_MICRO)
                    .font_family("monospace".to_string())
            }),
            label(move || status_text.to_string())
                .style(move |s| s.color(status_color).font_size(T_MICRO).padding_top(2.0)),
        ))
        .style(|s| s.flex_col().flex_grow(1.0f32).margin_left(10.0)),
    ))
    .style(|s| {
        s.items_center()
            .padding(10.0)
            .border_bottom(1.0)
            .border_color(BG_EDGE)
    })
}

// ── Editor column ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn editor_column(
    tabs: RwSignal<Vec<EditorTab>>,
    active_tab: RwSignal<String>,
    breadcrumb: RwSignal<Vec<String>>,
    on_select: Arc<dyn Fn(String)>,
    on_close: Arc<dyn Fn(String)>,
    open_file: RwSignal<String>,
    bottom_tab: RwSignal<BottomPanelTab>,
    bottom_hidden: RwSignal<bool>,
    bottom_max: RwSignal<bool>,
    _on_run_opmode: Arc<dyn Fn(&'static str)>,
    _on_open_opmode: Arc<dyn Fn(&'static str)>,
    terminal_lines: RwSignal<Vec<String>>,
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
        container(code_placeholder(open_file)).style(move |s| {
            let s = s.flex_grow(1.0f32).min_height(0.0).width_pct(100.0);
            if bottom_max.get() {
                s.hide()
            } else {
                s
            }
        }),
        // Bottom panel
        container(bottom_panel(
            bottom_tab,
            on_maximize,
            on_close_panel,
            terminal_lines,
        ))
        .style(move |s| {
            let s = s
                .flex_shrink(0.0)
                .width_pct(100.0)
                .border_top(1.0)
                .border_color(BG_EDGE);
            if bottom_hidden.get() {
                s.hide()
            } else if bottom_max.get() {
                s.flex_grow(1.0f32).min_height(0.0)
            } else {
                s.height(280.0)
            }
        }),
    ))
    .style(|s| {
        s.flex_col()
            .flex_grow(1.0f32)
            .min_width(0.0)
            .height_pct(100.0)
    })
}

// ── Code placeholder ─────────────────────────────────────────────────────────

fn code_placeholder(open_file: RwSignal<String>) -> impl View {
    container(
        v_stack((
            label(move || open_file.get()).style(|s| {
                s.color(FG_4)
                    .font_size(T_3XL)
                    .font_weight(floem::text::FontWeight::BOLD)
                    .font_family("monospace".to_string())
            }),
            label(|| "Code editor  —  coming soon".to_string())
                .style(|s| s.color(FG_4).font_size(T_SMALL).padding_top(8.0)),
        ))
        .style(|s| s.flex_col().items_center()),
    )
    .style(|s| {
        s.width_pct(100.0)
            .height_pct(100.0)
            .background(BG_NAVY)
            .items_center()
            .justify_center()
    })
}

// ── Shared helpers ────────────────────────────────────────────────────────────

fn panel_header(title: &'static str) -> impl View {
    h_stack((label(move || title.to_string()).style(|s| {
        s.flex_grow(1.0f32)
            .color(FG_2)
            .font_size(T_TINY)
            .font_weight(floem::text::FontWeight::BOLD)
    }),))
    .style(|s| {
        s.height(36.0)
            .padding_horiz(14.0)
            .items_center()
            .border_bottom(1.0)
            .border_color(BG_EDGE)
            .flex_shrink(0.0)
    })
}
use floem::views::scroll::scroll;
