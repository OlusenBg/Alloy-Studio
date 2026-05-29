//! Alloy Editor UI library — theme tokens, panels, chrome, and full-tab pages.
//!
//! Everything visual specific to Alloy lives here.
//! `theme` is the single source of truth for design tokens.

pub mod theme;

// ── Chrome ────────────────────────────────────────────────────────────────────
pub mod statusbar;
pub mod title_bar;
pub mod command_palette;
pub mod toast;
pub mod welcome;
pub mod activity_bar;
pub mod tab_strip;

// ── Dock panels ───────────────────────────────────────────────────────────────
pub mod panels;

// ── Full-tab pages ────────────────────────────────────────────────────────────
pub mod pages;

// ── Main shell compositor ─────────────────────────────────────────────────────
pub mod shell;
pub mod bottom_panel;
