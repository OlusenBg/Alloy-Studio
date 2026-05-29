//! Source Control page — dedicated full Git surface that opens as an editor
//! tab. Distinct from the small SCM sidebar widget in lapce-app.
//!
//! Two columns: left = staged + unstaged + commit composer, right = history.
//! BranchPalette overlay anchors to the branch pill.
//!
//! Reference: kit/SourceControlPage.jsx.

use std::sync::Arc;

use floem::View;
use floem::reactive::{RwSignal, SignalGet, SignalUpdate, create_rw_signal};
use floem::style::CursorStyle;
use floem::views::{
    Decorators, container, dyn_stack, empty, h_stack, label, scroll, text_input, v_stack,
};

use crate::theme::*;

// ── Public model ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum ScmStatus { Added, Modified, Deleted }

impl ScmStatus {
    fn letter(self) -> &'static str {
        match self { Self::Added => "A", Self::Modified => "M", Self::Deleted => "D" }
    }
    fn color(self) -> floem::peniko::Color {
        match self { Self::Added => SCM_ADDED, Self::Modified => SCM_MODIFIED, Self::Deleted => SCM_REMOVED }
    }
}

#[derive(Clone)]
pub struct ChangedFile {
    pub name: String,
    pub path: String,
    pub status: ScmStatus,
    pub adds: u32,
    pub dels: u32,
}

#[derive(Clone)]
pub struct CommitNodeData {
    pub id:      String,
    pub message: String,
    pub author:  String,
    pub when:    String,
    pub files:   u32,
    pub mine:    bool,
}

#[derive(Clone)]
pub struct BranchInfo {
    pub name:    String,
    pub ahead:   u32,
    pub behind:  u32,
    pub current: bool,
    pub last:    String,
    pub remote:  bool,
}

#[derive(Clone)]
pub struct SourceControlSignals {
    pub branch:    RwSignal<String>,
    pub ahead:     RwSignal<u32>,
    pub behind:    RwSignal<u32>,
    pub staged:    RwSignal<Vec<ChangedFile>>,
    pub unstaged:  RwSignal<Vec<ChangedFile>>,
    pub commit_msg: RwSignal<String>,
    pub history:   RwSignal<Vec<CommitNodeData>>,
    pub branches:  RwSignal<Vec<BranchInfo>>,
    pub history_filter: RwSignal<String>,
}

#[derive(Clone)]
pub struct SourceControlHandlers {
    pub on_fetch:           Arc<dyn Fn()>,
    pub on_pull:            Arc<dyn Fn()>,
    pub on_push:            Arc<dyn Fn()>,
    pub on_stage:           Arc<dyn Fn(String)>,
    pub on_unstage:         Arc<dyn Fn(String)>,
    pub on_stage_all:       Arc<dyn Fn()>,
    pub on_unstage_all:     Arc<dyn Fn()>,
    pub on_generate_msg:    Arc<dyn Fn()>,
    pub on_commit:          Arc<dyn Fn()>,
    pub on_commit_push:     Arc<dyn Fn()>,
    pub on_switch_branch:   Arc<dyn Fn(String)>,
    pub on_new_branch:      Arc<dyn Fn()>,
    pub on_pick_commit:     Arc<dyn Fn(String)>,
}

// ── Entry point ──────────────────────────────────────────────────────────────

pub fn source_control_page(
    s: SourceControlSignals,
    h: SourceControlHandlers,
) -> impl View {
    let show_branches = create_rw_signal(false);

    h_stack((
        left_pane(s.clone(), h.clone(), show_branches),
        right_pane(s.clone()),
    ))
    .style(|s| {
        s.width_pct(100.0)
            .height_pct(100.0)
            .background(BG_NAVY)
    })
}

// ── Left pane (changes + commit) ─────────────────────────────────────────────

fn left_pane(
    sigs: SourceControlSignals,
    h:    SourceControlHandlers,
    show_branches: RwSignal<bool>,
) -> impl View {
    v_stack((
        top_bar(sigs.clone(), h.clone(), show_branches),
        lists_area(sigs.clone(), h.clone()),
        commit_area(sigs.clone(), h.clone()),
    ))
    .style(|s| {
        s.width(380.0)
            .flex_shrink(0.0)
            .background(BG_SURFACE)
            .border_right(1.0)
            .border_color(BG_EDGE)
            .flex_col()
            .height_pct(100.0)
    })
}

fn top_bar(
    sigs: SourceControlSignals,
    h:    SourceControlHandlers,
    show_branches: RwSignal<bool>,
) -> impl View {
    let branch = sigs.branch;
    let ahead = sigs.ahead;
    let behind = sigs.behind;
    let on_fetch = h.on_fetch.clone();
    let on_pull = h.on_pull.clone();
    let on_push = h.on_push.clone();
    let branches_sig = sigs.branches;
    let on_switch = h.on_switch_branch.clone();
    let on_new = h.on_new_branch.clone();

    container(
        h_stack((
            branch_pill(branch, ahead, behind, show_branches),
            container(empty()).style(|s| s.flex_grow(1.0f32)),
            small_panel_btn("Fetch", on_fetch),
            small_panel_btn("Pull", on_pull),
            small_primary_btn_with_count("Push", ahead, on_push),
            branch_palette(show_branches, branches_sig, on_switch, on_new),
        ))
        .style(|s| s.items_center().gap(6.0)),
    )
    .style(|s| {
        s.padding_horiz(12.0)
            .padding_vert(10.0)
            .border_bottom(1.0)
            .border_color(BG_EDGE)
            .background(BG_SURFACE)
            .relative()
    })
}

fn branch_pill(
    branch: RwSignal<String>,
    ahead:  RwSignal<u32>,
    behind: RwSignal<u32>,
    show:   RwSignal<bool>,
) -> impl View {
    container(
        h_stack((
            label(|| "⎇".to_string()).style(|s| s.color(ALLOY_ORANGE).font_size(T_SMALL).margin_right(6.0)),
            label(move || branch.get()).style(|s| s.color(FG_1).font_size(T_SMALL)),
            label(move || format!("  +{} -{}", ahead.get(), behind.get()))
                .style(|s| s.color(FG_3).font_size(T_TINY).font_family("monospace".to_string()).margin_left(6.0)),
            label(|| " v".to_string()).style(|s| s.color(FG_3).font_size(T_MICRO).margin_left(6.0)),
        ))
        .style(|s| s.items_center()),
    )
    .on_click_stop(move |_| show.update(|v| *v = !*v))
    .style(|s| {
        s.padding_horiz(10.0)
            .padding_vert(4.0)
            .background(BG_RAISED)
            .border(1.0)
            .border_color(BG_EDGE)
            .border_radius(R_4)
            .cursor(CursorStyle::Pointer)
            .hover(|s| s.background(BG_HOVER))
    })
}

fn lists_area(sigs: SourceControlSignals, h: SourceControlHandlers) -> impl View {
    let staged = sigs.staged;
    let unstaged = sigs.unstaged;
    let on_stage_all = h.on_stage_all.clone();
    let on_unstage_all = h.on_unstage_all.clone();
    let on_stage = h.on_stage.clone();
    let on_unstage = h.on_unstage.clone();

    scroll(
        v_stack((
            section("STAGED CHANGES", staged, true, on_unstage, on_unstage_all),
            section("CHANGES", unstaged, false, on_stage, on_stage_all),
        ))
        .style(|s| s.flex_col().gap(18.0).padding(12.0).width_pct(100.0)),
    )
    .style(|s| s.flex_grow(1.0).width_pct(100.0))
}

fn section(
    label_text: &'static str,
    files: RwSignal<Vec<ChangedFile>>,
    staged: bool,
    on_each: Arc<dyn Fn(String)>,
    on_all:  Arc<dyn Fn()>,
) -> impl View {
    let action_text = if staged { "Unstage all" } else { "Stage all" };
    v_stack((
        h_stack((
            label(move || label_text.to_string()).style(|s| {
                s.color(FG_2).font_size(T_MICRO).font_weight(floem::text::Weight::BOLD)
            }),
            label(move || files.get().len().to_string()).style(|s| {
                s.color(FG_4).font_size(T_MICRO).font_family("monospace".to_string()).margin_left(8.0)
            }),
            container(empty()).style(|s| s.flex_grow(1.0f32)),
            container(label(move || action_text.to_string())
                .style(|s| s.color(FG_3).font_size(T_MICRO)))
                .on_click_stop(move |_| (on_all)())
                .style(|s| s.cursor(CursorStyle::Pointer).hover(|s| s.color(FG_1))),
        )).style(|s| s.items_center().margin_bottom(6.0).width_pct(100.0)),
        dyn_stack(
            move || files.get(),
            |f: &ChangedFile| f.name.clone(),
            move |f| file_row(f, staged, on_each.clone()),
        ).style(|s| s.flex_col().gap(2.0).width_pct(100.0)),
    ))
    .style(|s| s.flex_col().width_pct(100.0))
}

fn file_row(f: ChangedFile, staged: bool, on_each: Arc<dyn Fn(String)>) -> impl View {
    let name = f.name.clone();
    let path = f.path.clone();
    let status_color = f.status.color();
    let status_letter = f.status.letter();
    let adds = f.adds;
    let dels = f.dels;
    let row_name = f.name.clone();

    h_stack((
        label(move || status_letter.to_string()).style(move |s| {
            s.color(status_color)
                .font_family("monospace".to_string())
                .font_weight(floem::text::Weight::BOLD)
                .font_size(T_TINY)
                .width(14.0)
        }),
        v_stack((
            label(move || name.clone()).style(|s| s.color(FG_1).font_size(T_SMALL)),
            label(move || path.clone()).style(|s| {
                s.color(FG_3).font_size(T_MICRO).font_family("monospace".to_string())
            }),
        )).style(|s| s.flex_grow(1.0).min_width(0.0).gap(2.0)),
        delta_bars(adds, dels),
        container(label(move || (if staged { "-" } else { "+" }).to_string())
            .style(|s| s.color(FG_1).font_size(T_MD)))
            .on_click_stop(move |_| (on_each)(row_name.clone()))
            .style(|s| s.width(16.0).items_center().justify_center().cursor(CursorStyle::Pointer)),
    ))
    .style(|s| {
        s.padding_horiz(8.0)
            .padding_vert(6.0)
            .border_radius(R_4)
            .items_center()
            .gap(8.0)
            .cursor(CursorStyle::Pointer)
            .hover(|s| s.background(BG_HOVER))
    })
}

fn delta_bars(add: u32, del: u32) -> impl View {
    let total = (add + del).max(1);
    let blocks: u32 = 5;
    let add_blocks = ((add as f64 / total as f64) * blocks as f64).round() as u32;
    let add_blocks = add_blocks.min(blocks);
    let del_blocks = ((del as f64 / total as f64) * blocks as f64).round() as u32;
    let del_blocks = del_blocks.min(blocks - add_blocks);
    let cells = (0..blocks).map(move |i| {
        let c = if i < add_blocks {
            STATUS_SUCCESS
        } else if i < add_blocks + del_blocks {
            STATUS_ERROR
        } else {
            BG_EDGE
        };
        container(empty()).style(move |s| {
            s.width(5.0).height(9.0).background(c).border_radius(1.0)
        })
    });
    floem::views::stack_from_iter(cells).style(|s| s.gap(1.0).items_center())
}

fn commit_area(sigs: SourceControlSignals, h: SourceControlHandlers) -> impl View {
    let msg = sigs.commit_msg;
    let on_generate = h.on_generate_msg.clone();
    let on_commit = h.on_commit.clone();
    let on_commit_push = h.on_commit_push.clone();
    v_stack((
        label(|| "COMMIT MESSAGE".to_string()).style(|s| {
            s.color(FG_3).font_size(T_MICRO).font_weight(floem::text::Weight::BOLD)
        }),
        text_input(msg)
            .keyboard_navigable()
            .style(|s| {
                s.width_pct(100.0)
                    .min_height(84.0)
                    .background(BG_NAVY)
                    .border(1.0)
                    .border_color(LINE_RING)
                    .color(FG_1)
                    .padding_horiz(10.0)
                    .padding_vert(8.0)
                    .font_size(T_SMALL)
                    .border_radius(R_4)
            }),
        h_stack((
            panel_btn_small_glyph("Generate Message", ALLOY_ORANGE, on_generate),
            container(empty()).style(|s| s.flex_grow(1.0f32)),
            label(move || format!("{} / 72",
                msg.get().lines().next().map(|l| l.chars().count()).unwrap_or(0))
            ).style(|s| s.color(FG_4).font_size(T_MICRO).font_family("monospace".to_string())),
        )).style(|s| s.items_center().width_pct(100.0).gap(6.0)),
        h_stack((
            panel_btn_grow("Commit", on_commit),
            primary_btn_grow("Commit & Push", on_commit_push),
        )).style(|s| s.gap(6.0).width_pct(100.0)),
    ))
    .style(|s| {
        s.padding(12.0)
            .border_top(1.0)
            .border_color(BG_EDGE)
            .background(BG_SURFACE)
            .flex_col()
            .gap(8.0)
            .width_pct(100.0)
    })
}

// ── Right pane (history timeline) ────────────────────────────────────────────

fn right_pane(s: SourceControlSignals) -> impl View {
    let filter = s.history_filter;
    let branch = s.branch;
    let history = s.history;

    v_stack((
        h_stack((
            label(move || format!("COMMIT HISTORY - {}", branch.get().to_uppercase())).style(|s| {
                s.color(FG_3).font_size(T_MICRO).font_weight(floem::text::Weight::BOLD)
            }),
            container(empty()).style(|s| s.flex_grow(1.0f32)),
            text_input(filter)
                .placeholder("Filter commits...")
                .keyboard_navigable()
                .style(|s| {
                    s.background(BG_NAVY)
                        .border(1.0)
                        .border_color(LINE_RING)
                        .color(FG_1)
                        .padding_horiz(10.0)
                        .padding_vert(4.0)
                        .font_size(T_TINY)
                        .width(180.0)
                        .border_radius(R_4)
                }),
        ))
        .style(|s| {
            s.padding_horiz(16.0)
                .padding_vert(10.0)
                .border_bottom(1.0)
                .border_color(BG_EDGE)
                .background(BG_SURFACE)
                .items_center()
                .gap(10.0)
                .width_pct(100.0)
        }),
        scroll(timeline(history, filter))
            .style(|s| s.flex_grow(1.0).width_pct(100.0).padding(16.0)),
    ))
    .style(|s| s.flex_col().flex_grow(1.0).width_pct(100.0).height_pct(100.0))
}

fn timeline(
    history: RwSignal<Vec<CommitNodeData>>,
    filter:  RwSignal<String>,
) -> impl View {
    container(
        v_stack((
            container(empty()).style(|s| {
                s.absolute()
                    .width(1.0)
                    .height_pct(100.0)
                    .background(LINE_RING)
                    .margin_left(9.0)
                    .margin_top(8.0)
            }),
            dyn_stack(
                move || {
                    let f = filter.get().to_lowercase();
                    history.get().into_iter()
                        .filter(|c| f.is_empty()
                            || c.message.to_lowercase().contains(&f)
                            || c.author.to_lowercase().contains(&f)
                            || c.id.contains(&f))
                        .collect::<Vec<_>>()
                },
                |c| c.id.clone(),
                commit_node,
            ).style(|s| s.flex_col().padding_left(24.0).width_pct(100.0)),
        )),
    ).style(|s| s.relative().width_pct(100.0))
}

fn commit_node(c: CommitNodeData) -> impl View {
    let mine = c.mine;
    let id = c.id.clone();
    let msg = c.message.clone();
    let author = c.author.clone();
    let when = c.when.clone();
    let files = c.files;
    let avatar_initials = c.author.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2).collect::<String>().to_uppercase();
    let avatar_color = avatar_palette(&c.author);

    h_stack((
        container(empty()).style(move |s| {
            s.width(10.0)
                .height(10.0)
                .border_radius(R_FULL)
                .background(if mine { ALLOY_ORANGE } else { FG_4 })
                .margin_left(-2.0)
                .flex_shrink(0.0)
        }),
        container(label(move || avatar_initials.clone()).style(|s| {
            s.color(floem::peniko::Color::BLACK)
                .font_size(T_MICRO)
                .font_weight(floem::text::Weight::BOLD)
        }))
        .style(move |s| {
            s.width(20.0)
                .height(20.0)
                .border_radius(R_FULL)
                .background(avatar_color)
                .items_center()
                .justify_center()
                .flex_shrink(0.0)
                .margin_horiz(10.0)
        }),
        v_stack((
            label(move || msg.clone())
                .style(|s| s.color(FG_1).font_size(T_SMALL)),
            h_stack((
                label(move || author.clone()).style(|s| s.color(FG_3).font_size(T_MICRO)),
                label(|| " · ".to_string()).style(|s| s.color(FG_3).font_size(T_MICRO)),
                label(move || when.clone()).style(|s| s.color(FG_3).font_size(T_MICRO)),
                label(|| " · ".to_string()).style(|s| s.color(FG_3).font_size(T_MICRO)),
                label(move || format!("{files} files")).style(|s| s.color(FG_3).font_size(T_MICRO)),
            )).style(|s| s.items_center().margin_top(3.0)),
        )).style(|s| s.flex_grow(1.0).min_width(0.0).gap(0.0)),
        label(move || id.clone()).style(|s| {
            s.font_family("monospace".to_string()).color(FG_3).font_size(T_MICRO)
        }),
    ))
    .style(|s| {
        s.padding_horiz(10.0)
            .padding_vert(8.0)
            .border_radius(R_4)
            .items_center()
            .gap(0.0)
            .margin_left(-8.0)
            .margin_bottom(4.0)
            .cursor(CursorStyle::Pointer)
            .hover(|s| s.background(BG_SURFACE))
    })
}

fn avatar_palette(name: &str) -> floem::peniko::Color {
    let palette = [ALLOY_ORANGE, STATUS_INFO, STATUS_SUCCESS, SYN_KEYWORD, SYN_NUMBER];
    let h: usize = name.bytes().fold(0usize, |a, b| a.wrapping_add(b as usize));
    palette[h % palette.len()]
}

// ── Branch palette overlay ───────────────────────────────────────────────────

fn branch_palette(
    show: RwSignal<bool>,
    branches: RwSignal<Vec<BranchInfo>>,
    on_switch: Arc<dyn Fn(String)>,
    on_new:    Arc<dyn Fn()>,
) -> impl View {
    let query = create_rw_signal(String::new());
    container(
        v_stack((
            h_stack((
                label(|| "search".to_string()).style(|s| s.color(FG_3).font_size(T_TINY).margin_right(8.0)),
                text_input(query)
                    .placeholder("Switch to or create branch...")
                    .keyboard_navigable()
                    .style(|s| s.flex_grow(1.0).color(FG_1).font_size(T_SMALL).background(floem::peniko::Color::TRANSPARENT)),
                container(label(|| "x".to_string()).style(|s| s.color(FG_3).font_size(T_TINY)))
                    .on_click_stop(move |_| show.set(false))
                    .style(|s| s.padding(2.0).cursor(CursorStyle::Pointer)),
            )).style(|s| {
                s.height(36.0).padding_horiz(12.0).items_center()
                    .border_bottom(1.0).border_color(BG_EDGE)
            }),
            scroll(
                dyn_stack(
                    move || {
                        let q = query.get().to_lowercase();
                        branches.get().into_iter()
                            .filter(|b| q.is_empty() || b.name.to_lowercase().contains(&q))
                            .collect::<Vec<_>>()
                    },
                    |b| b.name.clone(),
                    {
                        let on_switch = on_switch.clone();
                        move |b| branch_row(b, on_switch.clone())
                    },
                ).style(|s| s.flex_col().padding_vert(4.0).width_pct(100.0))
            ).style(|s| s.max_height(320.0).width_pct(100.0)),
            container(
                h_stack((
                    label(|| "+ ".to_string()).style(|s| s.color(ALLOY_ORANGE).font_size(T_TINY).margin_right(8.0)),
                    label(|| "Create branch from main...".to_string()).style(|s| s.color(FG_3).font_size(T_TINY)),
                )).style(|s| s.items_center())
            )
            .on_click_stop(move |_| (on_new)())
            .style(|s| {
                s.padding_horiz(12.0).padding_vert(6.0)
                    .background(BG_SURFACE).border_top(1.0).border_color(BG_EDGE)
                    .cursor(CursorStyle::Pointer).hover(|s| s.background(BG_HOVER))
            }),
        ))
        .style(|s| {
            s.width(320.0)
                .background(BG_RAISED)
                .border_radius(R_8)
                .box_shadow_blur(36.0)
                .box_shadow_color(floem::peniko::Color::from_rgba8(0, 0, 0, 0x8C))
                .box_shadow_v_offset(12.0)
                .flex_col()
        })
    )
    .style(move |s| {
        let s = s.absolute().margin_top(40.0).margin_left(12.0).z_index(50);
        if show.get() { s } else { s.hide() }
    })
}

fn branch_row(b: BranchInfo, on_switch: Arc<dyn Fn(String)>) -> impl View {
    let name_for_click = b.name.clone();
    let name_for_label = b.name.clone();
    let last_for_label = b.last.clone();
    let current = b.current;
    let remote = b.remote;
    let ahead = b.ahead;
    let behind = b.behind;

    h_stack((
        label(move || if current { "check".to_string() } else { " ".to_string() })
            .style(|s| s.color(ALLOY_ORANGE).font_size(T_TINY).width(12.0)),
        label(move || (if remote { "cloud" } else { "branch" }).to_string())
            .style(|s| s.color(FG_3).font_size(T_TINY).margin_horiz(6.0)),
        label(move || name_for_label.clone()).style(|s| {
            s.color(FG_1).font_size(T_SMALL).font_family("monospace".to_string())
        }),
        container(label(move || format!("+{ahead} -{behind}"))
            .style(|s| s.color(FG_3).font_size(T_MICRO).font_family("monospace".to_string()).margin_left(6.0)))
            .style(move |s| {
                if !remote && (ahead > 0 || behind > 0) { s } else { s.hide() }
            }),
        container(empty()).style(|s| s.flex_grow(1.0f32)),
        label(move || last_for_label.clone()).style(|s| {
            s.color(FG_4).font_size(T_MICRO).max_width(140.0)
        }),
    ))
    .on_click_stop(move |_| (on_switch)(name_for_click.clone()))
    .style(|s| {
        s.padding_horiz(14.0)
            .padding_vert(5.0)
            .items_center()
            .cursor(CursorStyle::Pointer)
            .hover(|s| s.background(BG_HOVER))
    })
}

// ── Button helpers ───────────────────────────────────────────────────────────

fn small_panel_btn(text: &'static str, on_click: Arc<dyn Fn()>) -> impl View {
    container(label(move || text.to_string()).style(|s| s.color(FG_1).font_size(T_TINY)))
        .on_click_stop(move |_| (on_click)())
        .style(|s| {
            s.padding_horiz(8.0).padding_vert(4.0)
                .background(BG_RAISED).border(1.0).border_color(BG_EDGE)
                .border_radius(R_4).cursor(CursorStyle::Pointer)
                .hover(|s| s.background(BG_HOVER))
        })
}

fn small_primary_btn_with_count(
    text: &'static str,
    count: RwSignal<u32>,
    on_click: Arc<dyn Fn()>,
) -> impl View {
    container(label(move || format!("{} {}", text, count.get()))
        .style(|s| s.color(FG_1).font_size(T_TINY).font_weight(floem::text::Weight::SEMIBOLD)))
        .on_click_stop(move |_| (on_click)())
        .style(|s| {
            s.padding_horiz(8.0).padding_vert(4.0)
                .background(ALLOY_ORANGE).border_radius(R_4)
                .cursor(CursorStyle::Pointer)
                .box_shadow_blur(8.0).box_shadow_color(ALLOY_ORANGE_GLOW)
                .hover(|s| s.background(ALLOY_ORANGE_DEEP))
        })
}

fn panel_btn_small_glyph(
    text: &'static str,
    _glyph_color: floem::peniko::Color,
    on_click: Arc<dyn Fn()>,
) -> impl View {
    container(label(move || text.to_string()).style(|s| s.color(FG_1).font_size(T_TINY)))
        .on_click_stop(move |_| (on_click)())
        .style(|s| {
            s.padding_horiz(10.0).padding_vert(5.0)
                .background(BG_RAISED).border_radius(R_4)
                .cursor(CursorStyle::Pointer)
                .hover(|s| s.background(BG_HOVER))
        })
}

fn panel_btn_grow(text: &'static str, on_click: Arc<dyn Fn()>) -> impl View {
    container(label(move || text.to_string()).style(|s| s.color(FG_1).font_size(T_TINY)))
        .on_click_stop(move |_| (on_click)())
        .style(|s| {
            s.padding_vert(7.0).flex_grow(1.0)
                .background(BG_RAISED).border_radius(R_4)
                .items_center().justify_center()
                .cursor(CursorStyle::Pointer)
                .hover(|s| s.background(BG_HOVER))
        })
}

fn primary_btn_grow(text: &'static str, on_click: Arc<dyn Fn()>) -> impl View {
    container(label(move || text.to_string()).style(|s| {
        s.color(FG_1).font_size(T_TINY).font_weight(floem::text::Weight::SEMIBOLD)
    }))
    .on_click_stop(move |_| (on_click)())
    .style(|s| {
        s.padding_vert(7.0).flex_grow(1.0)
            .background(ALLOY_ORANGE).border_radius(R_4)
            .items_center().justify_center()
            .cursor(CursorStyle::Pointer)
            .box_shadow_blur(8.0).box_shadow_color(ALLOY_ORANGE_GLOW)
            .hover(|s| s.background(ALLOY_ORANGE_DEEP))
    })
}
