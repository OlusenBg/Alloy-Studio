//! Welcome / home screen shown before a project is opened.

use std::sync::Arc;
use floem::View;
use floem::reactive::{RwSignal, SignalGet, create_rw_signal};
use floem::style::CursorStyle;
use floem::views::{Decorators, container, empty, h_stack, label, scroll, v_stack, img};
use crate::theme::*;

static ALLOY_LOGO: &[u8] = include_bytes!("../extra/images/logo.png");
static FTC_LOGO:   &[u8] = include_bytes!("../extra/images/ftc-logo.png");

#[derive(Clone)]
pub struct RecentProject {
    pub name: String,
    pub path: String,
    pub when: String,
}

#[derive(Clone)]
pub struct WelcomeHandlers {
    pub on_open:      Arc<dyn Fn()>,
    pub on_clone:     Arc<dyn Fn()>,
    pub on_new:       Arc<dyn Fn()>,
    pub on_open_recent: Arc<dyn Fn(String)>,
}

pub fn welcome_screen(
    recent: RwSignal<Vec<RecentProject>>,
    h: WelcomeHandlers,
) -> impl View {
    v_stack((
        header_bar(),
        h_stack((
            left_column(recent, h.on_open_recent.clone()),
            right_column(h),
        ))
        .style(|s| {
            s.flex_grow(1.0).gap(0.0).width_pct(100.0).height_pct(100.0)
        }),
        footer(),
    ))
    .style(|s| {
        s.flex_col()
            .width_pct(100.0)
            .height_pct(100.0)
            .background(BG_NAVY)
    })
}

fn header_bar() -> impl View {
    h_stack((
        h_stack((
            img(|| ALLOY_LOGO.to_vec()).style(|s| s.width(32.0).height(32.0).margin_right(12.0)),
            v_stack((
                label(|| "ALLOY EDITOR".to_string()).style(|s| {
                    s.color(FG_1).font_size(T_2XL).font_weight(floem::text::Weight::BOLD)
                }),
                label(|| "FOR FIRST TECH CHALLENGE".to_string()).style(|s| {
                    s.color(ALLOY_ORANGE).font_size(T_MICRO).font_weight(floem::text::Weight::BOLD)
                }),
            )).style(|s| s.gap(2.0)),
        )).style(|s| s.items_center()),
        container(empty()).style(|s| s.flex_grow(1.0f32)),
        img(|| FTC_LOGO.to_vec()).style(|s| s.width(48.0).height(48.0)),
    ))
    .style(|s| {
        s.padding_horiz(32.0)
            .padding_vert(16.0)
            .background(BG_SURFACE)
            .border_bottom(1.0)
            .border_color(BG_EDGE)
            .items_center()
            .width_pct(100.0)
    })
}

fn left_column(
    recent: RwSignal<Vec<RecentProject>>,
    on_open: Arc<dyn Fn(String)>,
) -> impl View {
    v_stack((
        label(|| "RECENT PROJECTS".to_string()).style(|s| {
            s.color(FG_3).font_size(T_MICRO).font_weight(floem::text::Weight::BOLD)
                .margin_bottom(8.0)
        }),
        scroll(
            floem::views::dyn_stack(
                move || recent.get(),
                |p| p.name.clone(),
                move |p| recent_row(p, on_open.clone()),
            )
            .style(|s| s.flex_col().gap(2.0).width_pct(100.0)),
        )
        .style(|s| s.flex_grow(1.0).width_pct(100.0)),
    ))
    .style(|s| {
        s.flex_col()
            .flex_grow(1.0)
            .padding(32.0)
            .border_right(1.0)
            .border_color(BG_EDGE)
    })
}

fn recent_row(p: RecentProject, on_open: Arc<dyn Fn(String)>) -> impl View {
    let path_for_click = p.path.clone();
    let name = p.name.clone();
    let path = p.path.clone();
    let when = p.when.clone();
    container(
        h_stack((
            v_stack((
                label(move || name.clone()).style(|s| {
                    s.color(FG_1).font_size(T_SMALL).font_weight(floem::text::Weight::SEMIBOLD)
                }),
                label(move || path.clone()).style(|s| {
                    s.color(FG_3).font_size(T_MICRO).font_family("monospace".to_string())
                }),
            )).style(|s| s.flex_grow(1.0).min_width(0.0).gap(3.0)),
            label(move || when.clone()).style(|s| {
                s.color(FG_4).font_size(T_MICRO)
            }),
        ))
        .style(|s| s.items_center().width_pct(100.0)),
    )
    .on_click_stop(move |_| (on_open)(path_for_click.clone()))
    .style(|s| {
        s.padding_horiz(12.0)
            .padding_vert(10.0)
            .border_radius(R_6)
            .border_bottom(1.0)
            .border_color(BG_EDGE)
            .cursor(CursorStyle::Pointer)
            .hover(|s| s.background(BG_RAISED))
    })
}

fn right_column(h: WelcomeHandlers) -> impl View {
    v_stack((
        label(|| "GET STARTED".to_string()).style(|s| {
            s.color(FG_3).font_size(T_MICRO).font_weight(floem::text::Weight::BOLD).margin_bottom(12.0)
        }),
        action_btn("Open FTC Project...", true, h.on_open.clone()),
        action_btn("Clone from GitHub", false, h.on_clone.clone()),
        action_btn("+ New FTC Project  (coming soon)", false, {
            let _ = h.on_new.clone();
            std::sync::Arc::new(|| {})
        }),
        label(|| "FTC RESOURCES".to_string()).style(|s| {
            s.color(FG_3).font_size(T_MICRO).font_weight(floem::text::Weight::BOLD)
                .margin_top(24.0).margin_bottom(12.0)
        }),
        resource_link("SDK Documentation"),
        resource_link("Current Game Manual"),
        resource_link("FTC Team Resources"),
        resource_link("firstinspires.org"),
    ))
    .style(|s| {
        s.flex_col()
            .width(320.0)
            .flex_shrink(0.0)
            .padding(32.0)
            .gap(8.0)
    })
}

fn action_btn(text: &'static str, primary: bool, on_click: Arc<dyn Fn()>) -> impl View {
    container(label(move || text.to_string()).style(move |s| {
        let s = s.font_size(T_SMALL);
        if primary { s.color(FG_1).font_weight(floem::text::Weight::SEMIBOLD) } else { s.color(FG_2) }
    }))
    .on_click_stop(move |_| (on_click)())
    .style(move |s| {
        let s = s
            .width_pct(100.0)
            .padding_vert(10.0)
            .padding_horiz(16.0)
            .border_radius(R_6)
            .items_center()
            .justify_center()
            .cursor(CursorStyle::Pointer);
        if primary {
            s.background(ALLOY_ORANGE)
                .box_shadow_blur(12.0)
                .box_shadow_color(ALLOY_ORANGE_GLOW)
                .box_shadow_v_offset(4.0)
                .hover(|s| s.background(ALLOY_ORANGE_DEEP))
        } else {
            s.background(BG_SURFACE)
                .border(1.0)
                .border_color(BG_EDGE)
                .hover(|s| s.background(BG_RAISED))
        }
    })
}

fn resource_link(text: &'static str) -> impl View {
    h_stack((
        label(|| "->".to_string()).style(|s| s.color(ALLOY_ORANGE).font_size(T_SMALL).margin_right(10.0)),
        label(move || text.to_string()).style(|s| s.color(FG_2).font_size(T_SMALL)),
    ))
    .style(|s| {
        s.items_center()
            .padding_vert(4.0)
            .cursor(CursorStyle::Pointer)
            .hover(|s| s.color(FG_1))
    })
}

fn footer() -> impl View {
    label(|| "Alloy Editor - Built for FTC Robotics".to_string())
        .style(|s| {
            s.color(FG_4)
                .font_size(T_MICRO)
                .width_pct(100.0)
                .padding_vert(8.0)
                .padding_horiz(32.0)
                .border_top(1.0)
                .border_color(BG_EDGE)
        })
}
