//! Alloy Editor UI library — theme tokens, panels, chrome, and full-tab pages.
//!
//! Everything visual specific to Alloy lives here.
//! `theme` is the single source of truth for design tokens.

// UI crate under active development — dead code and unused items are expected
// while panels and features are still being wired up. Floem's function-style
// view constructors (container, label, v_stack, …) are deprecated in favour of
// struct constructors but the migration is deferred; suppress until then.
#![allow(dead_code, unused, deprecated)]

pub mod bridge;
pub mod theme;

// ── Chrome ────────────────────────────────────────────────────────────────────
pub mod activity_bar;
pub mod command_palette;
pub mod statusbar;
pub mod tab_strip;
pub mod title_bar;
pub mod toast;
pub mod welcome;

// ── Dock panels ───────────────────────────────────────────────────────────────
pub mod panels;

// ── Full-tab pages ────────────────────────────────────────────────────────────
pub mod pages;

// ── Main shell compositor ─────────────────────────────────────────────────────
pub mod bottom_panel;
pub mod shell;
