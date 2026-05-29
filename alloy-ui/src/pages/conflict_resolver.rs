//! Conflict resolver — full-tab merge-conflict UI with AI summaries and a
//! two-pane diff per file.

use std::sync::Arc;

use floem::reactive::{RwSignal, SignalGet};
use floem::style::CursorStyle;
use floem::views::{container, dyn_stack, empty, h_stack, label, scroll, v_stack, Decorators};
use floem::View;

use crate::theme::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Side {
    Yours,
    Theirs,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SynTok {
    Keyword,
    Function,
    String,
    Number,
    Type,
    Variable,
    Builtin,
    Punct,
}

impl SynTok {
    fn color(self) -> floem::peniko::Color {
        match self {
            Self::Keyword => SYN_KEYWORD,
            Self::Function => SYN_FUNCTION,
            Self::String => SYN_STRING,
            Self::Number => SYN_NUMBER,
            Self::Type => SYN_TYPE,
            Self::Variable => SYN_VARIABLE,
            Self::Builtin => SYN_BUILTIN,
            Self::Punct => SYN_PUNCT,
        }
    }
}

pub type CodeLine = Vec<(SynTok, String)>;

#[derive(Clone)]
pub struct ConflictSideData {
    pub author: String,
    pub when: String,
    pub explain: String,
    pub code: Vec<CodeLine>,
}

#[derive(Clone)]
pub struct ConflictFile {
    pub name: String,
    pub path: String,
    pub summary: String,
    pub yours: ConflictSideData,
    pub theirs: ConflictSideData,
    pub resolution: Option<Side>,
}

#[derive(Clone)]
pub struct ConflictHandlers {
    pub on_keep: Arc<dyn Fn(String, Side)>,
    pub on_open_in_editor: Arc<dyn Fn(String)>,
    pub on_abort_merge: Arc<dyn Fn()>,
    pub on_discard: Arc<dyn Fn()>,
    pub on_continue: Arc<dyn Fn()>,
}

pub fn conflict_resolver(conflicts: RwSignal<Vec<ConflictFile>>, h: ConflictHandlers) -> impl View {
    v_stack((
        warning_banner(conflicts, h.on_abort_merge.clone()),
        scroll(
            v_stack((
                cards(conflicts, h.clone()),
                footer_bar(conflicts, h.clone()),
            ))
            .style(|s| s.flex_col().padding(20.0).gap(16.0).width_pct(100.0)),
        )
        .style(|s| s.flex_grow(1.0).width_pct(100.0)),
    ))
    .style(|s| {
        s.flex_col()
            .width_pct(100.0)
            .height_pct(100.0)
            .background(BG_NAVY)
    })
}

fn warning_banner(conflicts: RwSignal<Vec<ConflictFile>>, on_abort: Arc<dyn Fn()>) -> impl View {
    h_stack((
        label(|| "!".to_string())
            .style(|s| s.color(ALLOY_ORANGE).font_size(T_LG).margin_right(12.0)),
        v_stack((
            label(move || {
                format!(
                    "Merge conflict in {} files — Alloy will help you resolve them.",
                    conflicts.get().len()
                )
            })
            .style(|s| {
                s.color(FG_1)
                    .font_size(T_BASE)
                    .font_weight(floem::text::Weight::BOLD)
            }),
            label(move || {
                let total = conflicts.get().len();
                let resolved = conflicts
                    .get()
                    .iter()
                    .filter(|c| c.resolution.is_some())
                    .count();
                format!("Pulling from origin/main · Resolved {resolved} of {total}")
            })
            .style(|s| s.color(FG_3).font_size(T_TINY).margin_top(2.0)),
        ))
        .style(|s| s.flex_grow(1.0).gap(0.0)),
        container(label(|| "Abort merge".to_string()).style(|s| s.color(FG_1).font_size(T_TINY)))
            .on_click_stop(move |_| (on_abort)())
            .style(|s| {
                s.padding_horiz(10.0)
                    .padding_vert(5.0)
                    .background(BG_RAISED)
                    .border_radius(R_4)
                    .cursor(CursorStyle::Pointer)
                    .hover(|s| s.background(BG_HOVER))
            }),
    ))
    .style(|s| {
        s.padding_horiz(20.0)
            .padding_vert(12.0)
            .background(floem::peniko::Color::from_rgb8(0x3A, 0x1D, 0x0A))
            .border_bottom(1.0)
            .border_color(floem::peniko::Color::from_rgb8(0x5A, 0x2A, 0x10))
            .items_center()
            .gap(12.0)
            .flex_shrink(0.0)
            .width_pct(100.0)
    })
}

fn cards(conflicts: RwSignal<Vec<ConflictFile>>, h: ConflictHandlers) -> impl View {
    dyn_stack(
        move || conflicts.get(),
        |c| c.name.clone(),
        move |c| conflict_card(c, h.clone()),
    )
    .style(|s| s.flex_col().gap(16.0).width_pct(100.0))
}

fn conflict_card(f: ConflictFile, h: ConflictHandlers) -> impl View {
    let resolved = f.resolution.is_some();
    let name = f.name.clone();
    let path = f.path.clone();
    let summary = f.summary.clone();
    let resolution = f.resolution;

    v_stack((
        h_stack((
            label(move || name.clone()).style(|s| {
                s.color(FG_1)
                    .font_size(T_BASE)
                    .font_weight(floem::text::Weight::SEMIBOLD)
            }),
            label(move || path.clone()).style(|s| {
                s.color(FG_3)
                    .font_size(T_TINY)
                    .font_family("monospace".to_string())
                    .margin_left(8.0)
            }),
            container(empty()).style(|s| s.flex_grow(1.0f32)),
            status_pill(resolution),
        ))
        .style(|s| {
            s.padding_horiz(16.0)
                .padding_vert(12.0)
                .background(BG_RAISED)
                .border_bottom(1.0)
                .border_color(BG_EDGE)
                .items_center()
                .gap(10.0)
                .width_pct(100.0)
        }),
        h_stack((
            label(|| "*".to_string()).style(|s| {
                s.color(ALLOY_ORANGE)
                    .font_size(T_BASE)
                    .margin_top(2.0)
                    .margin_right(10.0)
            }),
            label(move || summary.clone())
                .style(|s| s.color(FG_2).font_size(T_SMALL).flex_grow(1.0)),
        ))
        .style(|s| {
            s.padding_horiz(16.0)
                .padding_vert(10.0)
                .background(BG_SURFACE)
                .items_start()
                .gap(10.0)
                .width_pct(100.0)
        }),
        h_stack((
            side_view(
                "Your version",
                STATUS_INFO,
                f.yours.clone(),
                Side::Yours,
                resolution == Some(Side::Yours),
                f.name.clone(),
                h.on_keep.clone(),
            ),
            container(empty()).style(|s| s.width(1.0).background(BG_EDGE).height_pct(100.0)),
            side_view(
                "Their version",
                STATUS_WARNING,
                f.theirs.clone(),
                Side::Theirs,
                resolution == Some(Side::Theirs),
                f.name.clone(),
                h.on_keep.clone(),
            ),
        ))
        .style(|s| s.width_pct(100.0)),
        container(
            container(
                label(|| "Open in editor".to_string()).style(|s| s.color(FG_1).font_size(T_TINY)),
            )
            .on_click_stop({
                let n = f.name.clone();
                let cb = h.on_open_in_editor.clone();
                move |_| (cb)(n.clone())
            })
            .style(|s| {
                s.padding_horiz(10.0)
                    .padding_vert(5.0)
                    .background(BG_RAISED)
                    .border_radius(R_4)
                    .cursor(CursorStyle::Pointer)
                    .hover(|s| s.background(BG_HOVER))
                    .items_center()
                    .justify_center()
                    .width_pct(100.0)
            }),
        )
        .style(|s| {
            s.padding(10.0)
                .background(BG_SURFACE)
                .border_top(1.0)
                .border_color(BG_EDGE)
                .width_pct(100.0)
        }),
    ))
    .style(move |s| {
        s.background(BG_SURFACE)
            .border_radius(R_8)
            .border_left(3.0)
            .border_color(if resolved {
                STATUS_SUCCESS
            } else {
                STATUS_ERROR
            })
            .width_pct(100.0)
            .flex_col()
    })
}

fn status_pill(resolution: Option<Side>) -> impl View {
    h_stack((
        label(move || match resolution {
            Some(_) => "ok".to_string(),
            None => " ".to_string(),
        })
        .style(move |s| {
            let c = if resolution.is_some() {
                STATUS_SUCCESS
            } else {
                STATUS_ERROR
            };
            s.color(c).font_size(T_TINY).margin_right(4.0)
        }),
        label(move || match resolution {
            Some(Side::Yours) => "Resolved - kept yours".to_string(),
            Some(Side::Theirs) => "Resolved - kept theirs".to_string(),
            None => "Unresolved".to_string(),
        })
        .style(move |s| {
            let c = if resolution.is_some() {
                STATUS_SUCCESS
            } else {
                STATUS_ERROR
            };
            s.color(c)
                .font_size(T_TINY)
                .font_weight(floem::text::Weight::SEMIBOLD)
        }),
    ))
    .style(|s| s.items_center())
}

fn side_view(
    title: &'static str,
    accent: floem::peniko::Color,
    side: ConflictSideData,
    s_kind: Side,
    chosen: bool,
    file_name: String,
    on_keep: Arc<dyn Fn(String, Side)>,
) -> impl View {
    let author = side.author.clone();
    let when = side.when.clone();
    let explain = side.explain.clone();
    let code = side.code.clone();
    let lbl = match s_kind {
        Side::Yours => "yours",
        Side::Theirs => "theirs",
    };

    v_stack((
        h_stack((
            container(empty()).style(move |s| {
                s.width(6.0)
                    .height(6.0)
                    .background(accent)
                    .border_radius(R_2)
            }),
            label(move || title.to_uppercase()).style(move |s| {
                s.color(accent)
                    .font_size(T_MICRO)
                    .font_weight(floem::text::Weight::BOLD)
                    .margin_left(8.0)
            }),
            label(move || format!(" - {} - {}", author, when))
                .style(|s| s.color(FG_3).font_size(T_MICRO)),
        ))
        .style(|s| s.items_center()),
        label(move || explain.clone()).style(|s| s.color(FG_2).font_size(T_TINY)),
        v_stack((dyn_stack(
            move || {
                code.iter()
                    .enumerate()
                    .map(|(i, l)| (i, l.clone()))
                    .collect::<Vec<_>>()
            },
            |(i, _)| *i,
            |(_, line)| {
                h_stack((dyn_stack(
                    move || {
                        line.iter()
                            .enumerate()
                            .map(|(i, t)| (i, t.clone()))
                            .collect::<Vec<_>>()
                    },
                    |(i, _)| *i,
                    |(_, (tok, text))| {
                        label(move || text.clone()).style(move |s| {
                            s.color(tok.color())
                                .font_size(T_SMALL)
                                .font_family("monospace".to_string())
                        })
                    },
                )
                .style(|s| s.flex_row()),))
                .style(|s| s.min_height(18.0))
            },
        )
        .style(|s| {
            s.flex_col()
                .padding_horiz(10.0)
                .padding_vert(8.0)
                .width_pct(100.0)
        }),))
        .style(|s| s.background(BG_NAVY).border_radius(R_4).width_pct(100.0)),
        container(
            label(move || {
                if chosen {
                    format!("Kept {lbl}")
                } else {
                    format!("Keep {lbl}")
                }
            })
            .style(|s| {
                s.color(FG_1)
                    .font_size(T_TINY)
                    .font_weight(floem::text::Weight::SEMIBOLD)
            }),
        )
        .on_click_stop({
            let on_keep = on_keep.clone();
            let n = file_name.clone();
            move |_| (on_keep)(n.clone(), s_kind)
        })
        .style(move |s| {
            let s = s
                .padding_horiz(12.0)
                .padding_vert(6.0)
                .border_radius(R_4)
                .cursor(CursorStyle::Pointer);
            if chosen {
                s.background(ALLOY_ORANGE)
                    .box_shadow_blur(8.0)
                    .box_shadow_color(ALLOY_ORANGE_GLOW)
                    .hover(|s| s.background(ALLOY_ORANGE_DEEP))
            } else {
                s.background(BG_RAISED).hover(|s| s.background(BG_HOVER))
            }
        }),
    ))
    .style(move |s| {
        let s = s
            .padding(14.0)
            .gap(8.0)
            .flex_col()
            .flex_grow(1.0)
            .flex_basis(0.0);
        if chosen {
            s.background(floem::peniko::Color::from_rgba8(0x44, 0xCC, 0x88, 0x14))
        } else {
            s.background(BG_SURFACE)
        }
    })
}

fn footer_bar(conflicts: RwSignal<Vec<ConflictFile>>, h: ConflictHandlers) -> impl View {
    h_stack((
        label(move || {
            let v = conflicts.get();
            let total = v.len();
            let unresolved = v.iter().filter(|c| c.resolution.is_none()).count();
            if unresolved == 0 {
                "All conflicts resolved.".to_string()
            } else {
                format!(
                    "{unresolved} conflict{} left.",
                    if unresolved == 1 { "" } else { "s" }
                )
            }
        })
        .style(|s| s.color(FG_3).font_size(T_SMALL)),
        container(empty()).style(|s| s.flex_grow(1.0f32)),
        container(
            label(|| "Discard changes".to_string()).style(|s| s.color(FG_1).font_size(T_TINY)),
        )
        .on_click_stop(move |_| (h.on_discard)())
        .style(|s| {
            s.padding_horiz(10.0)
                .padding_vert(5.0)
                .background(BG_RAISED)
                .border_radius(R_4)
                .cursor(CursorStyle::Pointer)
                .hover(|s| s.background(BG_HOVER))
        }),
        container(label(|| "Continue merge".to_string()).style(|s| {
            s.color(FG_1)
                .font_size(T_TINY)
                .font_weight(floem::text::Weight::SEMIBOLD)
        }))
        .on_click_stop({
            let on_cont = h.on_continue.clone();
            move |_| (on_cont)()
        })
        .style(move |s| {
            let all_resolved = conflicts.get().iter().all(|c| c.resolution.is_some());
            let s = s.padding_horiz(10.0).padding_vert(5.0).border_radius(R_4);
            if all_resolved {
                s.background(ALLOY_ORANGE)
                    .box_shadow_blur(8.0)
                    .box_shadow_color(ALLOY_ORANGE_GLOW)
                    .cursor(CursorStyle::Pointer)
                    .hover(|s| s.background(ALLOY_ORANGE_DEEP))
            } else {
                s.background(BG_RAISED).color(FG_4)
            }
        }),
    ))
    .style(|s| {
        s.items_center()
            .gap(8.0)
            .padding(16.0)
            .margin_top(4.0)
            .border_top(1.0)
            .border_color(BG_EDGE)
            .width_pct(100.0)
    })
}
