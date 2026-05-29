//! Git timeline panel — staged changes, commit message editor, and history.
//!
//! Reference: kit/GitTimeline.jsx.

use floem::reactive::{create_rw_signal, RwSignal, SignalGet, SignalUpdate};
use floem::style::CursorStyle;
use floem::views::{container, dyn_stack, empty, h_stack, label, text_input, v_stack, Decorators};
use floem::View;

use crate::theme::*;

#[derive(Clone, Copy)]
pub struct StagedFile {
    pub name: &'static str,
    pub scm: char,
    pub delta: &'static str,
}

#[derive(Clone, Copy)]
pub struct CommitEntry {
    pub hash: &'static str,
    pub message: &'static str,
    pub author: &'static str,
    pub when: &'static str,
}

static STAGED: &[StagedFile] = &[
    StagedFile {
        name: "RobotHardware.java",
        scm: 'A',
        delta: "+24 / −0",
    },
    StagedFile {
        name: "OpMode.java",
        scm: 'M',
        delta: "+8 / −3",
    },
    StagedFile {
        name: "build.gradle",
        scm: 'M',
        delta: "+1 / −1",
    },
];

static HISTORY: &[CommitEntry] = &[
    CommitEntry {
        hash: "a4f1c2e",
        message: "Tune arm PID for 12V battery",
        author: "Alex",
        when: "12 min ago",
    },
    CommitEntry {
        hash: "9b3d77a",
        message: "Add wrist servo to hardware map",
        author: "Sarah",
        when: "1 h ago",
    },
    CommitEntry {
        hash: "62e1a08",
        message: "Fix gradle dep version (9.2 → 9.2.0)",
        author: "Mentor",
        when: "yesterday",
    },
    CommitEntry {
        hash: "5512cd1",
        message: "Initial TeleOp scaffold",
        author: "Alex",
        when: "3 days ago",
    },
];

pub fn git_timeline_panel() -> impl View {
    let commit_msg = create_rw_signal(String::new());
    let status_text = create_rw_signal("Ready".to_string());

    v_stack((
        staged_section(),
        commit_section(commit_msg, status_text),
        history_section(),
        status_bar(status_text),
    ))
    .style(|s| {
        s.flex_col()
            .width_pct(100.0)
            .height_pct(100.0)
            .background(BG_SURFACE)
    })
}

fn section_label(text: &'static str) -> impl View {
    label(move || text.to_string()).style(|s| {
        s.color(FG_3)
            .font_size(T_MICRO)
            .font_weight(floem::text::FontWeight::BOLD)
            .padding_bottom(6.0)
    })
}

fn scm_color(ch: char) -> floem::peniko::Color {
    match ch {
        'A' => STATUS_SUCCESS,
        'M' => STATUS_INFO,
        'D' => STATUS_ERROR,
        _ => FG_3,
    }
}

fn staged_section() -> impl View {
    let staged_label = section_label("STAGED CHANGES (3)");

    let rows = dyn_stack(
        || STAGED.iter().enumerate().collect::<Vec<_>>(),
        |(i, _)| *i,
        |(_, file)| staged_row(*file),
    )
    .style(|s| s.flex_col().padding_vert(4.0).width_pct(100.0));

    let list_container = container(scroll(rows).style(|s| s.flex_grow(1.0).width_pct(100.0)))
        .style(|s| {
            s.background(BG_SURFACE)
                .border(1.0)
                .border_color(BG_EDGE)
                .border_radius(R_4)
                .width_pct(100.0)
                .max_height(120.0)
        });

    v_stack((staged_label, list_container)).style(|s| s.flex_col().padding(12.0).flex_shrink(0.0))
}

fn staged_row(file: StagedFile) -> impl View {
    let badge_color = scm_color(file.scm);
    let scm_ch = file.scm;

    h_stack((
        // file glyph icon
        label(|| "◈".to_string()).style(|s| s.color(FG_3).font_size(T_TINY).margin_right(6.0)),
        // filename
        label(move || file.name.to_string()).style(|s| {
            s.color(FG_1)
                .font_size(T_SMALL)
                .flex_grow(1.0f32)
                .min_width(0.0)
        }),
        // delta
        label(move || file.delta.to_string()).style(|s| {
            s.color(FG_3)
                .font_size(T_TINY)
                .font_family("monospace".to_string())
                .margin_right(8.0)
        }),
        // SCM badge letter
        label(move || scm_ch.to_string()).style(move |s| {
            s.color(badge_color)
                .font_size(T_MICRO)
                .font_weight(floem::text::FontWeight::BOLD)
                .width(14.0)
                .items_center()
                .justify_center()
        }),
    ))
    .style(|s| {
        s.items_center()
            .padding_horiz(8.0)
            .padding_vert(4.0)
            .width_pct(100.0)
            .hover(|s| s.background(BG_HOVER))
    })
}

fn commit_section(commit_msg: RwSignal<String>, status_text: RwSignal<String>) -> impl View {
    let msg_label = section_label("COMMIT MESSAGE");

    let input = text_input(commit_msg).style(|s| {
        s.width_pct(100.0)
            .min_height(84.0)
            .background(BG_SURFACE)
            .color(FG_1)
            .font_size(T_SMALL)
            .border(1.0)
            .border_color(LINE_RING)
            .border_radius(R_4)
            .padding(8.0)
    });

    let btn_generate = label(|| "⚙ Generate Message".to_string())
        .on_click_stop(move |_| status_text.set("Generating…".to_string()))
        .style(|s| {
            s.padding_horiz(10.0)
                .padding_vert(5.0)
                .border(1.0)
                .border_color(LINE_RING)
                .border_radius(R_4)
                .color(ALLOY_ORANGE)
                .font_size(T_TINY)
                .cursor(CursorStyle::Pointer)
                .hover(|s| s.background(BG_HOVER))
        });

    let spacer = container(empty()).style(|s| s.flex_grow(1.0f32));

    let btn_pull = action_btn("Pull", false);
    let btn_push = action_btn("Push", false);

    let btn_commit = label(|| "Commit".to_string())
        .on_click_stop(move |_| status_text.set("Committed.".to_string()))
        .style(|s| {
            s.padding_horiz(12.0)
                .padding_vert(5.0)
                .background(ALLOY_ORANGE)
                .color(FG_1)
                .font_size(T_TINY)
                .font_weight(floem::text::FontWeight::BOLD)
                .border_radius(R_4)
                .cursor(CursorStyle::Pointer)
                .hover(|s| s.background(ALLOY_ORANGE_DEEP))
        });

    let buttons = h_stack((btn_generate, spacer, btn_pull, btn_push, btn_commit))
        .style(|s| s.items_center().col_gap(6.0).row_gap(0.0).margin_top(8.0));

    v_stack((msg_label, input, buttons)).style(|s| {
        s.flex_col()
            .padding_left(12.0)
            .padding_right(12.0)
            .padding_bottom(12.0)
            .flex_shrink(0.0)
    })
}

fn action_btn(text: &'static str, _primary: bool) -> impl View {
    label(move || text.to_string()).style(|s| {
        s.padding_horiz(10.0)
            .padding_vert(5.0)
            .border(1.0)
            .border_color(LINE_RING)
            .border_radius(R_4)
            .color(FG_2)
            .font_size(T_TINY)
            .cursor(CursorStyle::Pointer)
            .hover(|s| s.background(BG_HOVER))
    })
}

fn history_section() -> impl View {
    let hist_label = section_label("COMMIT HISTORY · origin/main");

    let rows = dyn_stack(
        || HISTORY.iter().enumerate().collect::<Vec<_>>(),
        |(i, _)| *i,
        |(_, entry)| history_row(*entry),
    )
    .style(|s| s.flex_col().padding_vert(4.0).width_pct(100.0));

    let list_container = container(scroll(rows).style(|s| s.flex_grow(1.0).width_pct(100.0)))
        .style(|s| {
            s.background(BG_SURFACE)
                .border(1.0)
                .border_color(BG_EDGE)
                .border_radius(R_4)
                .width_pct(100.0)
                .flex_grow(1.0f32)
        });

    v_stack((hist_label, list_container)).style(|s| {
        s.flex_col()
            .flex_grow(1.0f32)
            .padding_left(12.0)
            .padding_right(12.0)
            .padding_bottom(12.0)
    })
}

fn history_row(entry: CommitEntry) -> impl View {
    h_stack((
        // short hash
        label(move || entry.hash.to_string()).style(|s| {
            s.color(ALLOY_ORANGE)
                .font_size(T_TINY)
                .font_family("monospace".to_string())
                .width(56.0)
                .flex_shrink(0.0)
        }),
        // commit message
        label(move || entry.message.to_string()).style(|s| {
            s.color(FG_1)
                .font_size(T_SMALL)
                .flex_grow(1.0f32)
                .min_width(0.0)
        }),
        // author
        label(move || entry.author.to_string())
            .style(|s| s.color(FG_3).font_size(T_TINY).margin_right(8.0)),
        // when
        label(move || entry.when.to_string()).style(|s| {
            s.color(FG_4)
                .font_size(T_MICRO)
                .font_family("monospace".to_string())
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

fn status_bar(status_text: RwSignal<String>) -> impl View {
    label(move || status_text.get()).style(|s| {
        s.padding_horiz(14.0)
            .padding_vert(5.0)
            .color(FG_3)
            .font_size(T_TINY)
            .background(BG_SURFACE)
            .border_top(1.0)
            .border_color(BG_EDGE)
            .flex_shrink(0.0)
            .width_pct(100.0)
    })
}
use floem::views::scroll::scroll;
