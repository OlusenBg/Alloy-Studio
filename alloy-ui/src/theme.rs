//! Alloy Editor design tokens — single source of truth.
//!
//! Mirrors `colors_and_type.css` from the design system.  Every other Alloy
//! file must consume from here — do not hard-code colors or sizes elsewhere.

use floem::peniko::Color;
use std::time::Duration;

// ── Brand ─────────────────────────────────────────────────────────────────────
pub const ALLOY_ORANGE: Color = Color::from_rgb8(0xFF, 0x6B, 0x2B);
pub const ALLOY_ORANGE_DEEP: Color = Color::from_rgb8(0xE0, 0x5A, 0x1F);
pub const ALLOY_ORANGE_SOFT: Color = Color::from_rgba8(0xFF, 0x6B, 0x2B, 0x22);
pub const ALLOY_ORANGE_GLOW: Color = Color::from_rgba8(0xFF, 0x6B, 0x2B, 0x59);
pub const FTC_BLUE: Color = Color::from_rgb8(0x00, 0x5C, 0xAA);
pub const FTC_RED: Color = Color::from_rgb8(0xED, 0x1C, 0x24);

// ── Surfaces (deep navy ladder) ───────────────────────────────────────────────
pub const BG_NAVY: Color = Color::from_rgb8(0x1A, 0x1F, 0x2E);
pub const BG_SURFACE: Color = Color::from_rgb8(0x23, 0x29, 0x38);
pub const BG_RAISED: Color = Color::from_rgb8(0x2A, 0x31, 0x44);
pub const BG_HOVER: Color = Color::from_rgb8(0x2E, 0x37, 0x4C);
pub const BG_CURRENT: Color = Color::from_rgb8(0x2C, 0x31, 0x3A);
pub const BG_EDGE: Color = Color::from_rgb8(0x14, 0x18, 0x2A);
pub const BG_GRID: Color = Color::from_rgb8(0x2A, 0x30, 0x42);

// ── Ink ───────────────────────────────────────────────────────────────────────
pub const FG_1: Color = Color::from_rgb8(0xE8, 0xEC, 0xF4);
pub const FG_2: Color = Color::from_rgb8(0xC0, 0xC8, 0xD8);
pub const FG_3: Color = Color::from_rgb8(0x88, 0x92, 0xA4);
pub const FG_4: Color = Color::from_rgb8(0x5C, 0x63, 0x70);

// ── Lines ─────────────────────────────────────────────────────────────────────
pub const LINE_RING: Color = Color::from_rgb8(0x3A, 0x40, 0x52);
pub const LINE_STRONG: Color = Color::from_rgb8(0x4A, 0x52, 0x66);

// ── Status ────────────────────────────────────────────────────────────────────
pub const STATUS_SUCCESS: Color = Color::from_rgb8(0x44, 0xCC, 0x88);
pub const STATUS_WARNING: Color = Color::from_rgb8(0xE5, 0xC0, 0x7B);
pub const STATUS_ERROR: Color = Color::from_rgb8(0xFF, 0x44, 0x44);
pub const STATUS_INFO: Color = Color::from_rgb8(0x61, 0xAF, 0xEF);

// ── SCM ───────────────────────────────────────────────────────────────────────
pub const SCM_ADDED: Color = Color::from_rgb8(0x44, 0xCC, 0x88);
pub const SCM_MODIFIED: Color = Color::from_rgb8(0x61, 0xAF, 0xEF);
pub const SCM_REMOVED: Color = Color::from_rgb8(0xFF, 0x44, 0x44);

// ── Syntax (One Dark) ─────────────────────────────────────────────────────────
pub const SYN_KEYWORD: Color = Color::from_rgb8(0xC6, 0x78, 0xDD);
pub const SYN_FUNCTION: Color = Color::from_rgb8(0x61, 0xAF, 0xEF);
pub const SYN_STRING: Color = Color::from_rgb8(0x98, 0xC3, 0x79);
pub const SYN_NUMBER: Color = Color::from_rgb8(0xE5, 0xC0, 0x7B);
pub const SYN_TYPE: Color = Color::from_rgb8(0xE5, 0xC0, 0x7B);
pub const SYN_VARIABLE: Color = Color::from_rgb8(0xE0, 0x6C, 0x75);
pub const SYN_BUILTIN: Color = Color::from_rgb8(0x56, 0xB6, 0xC2);
pub const SYN_COMMENT: Color = Color::from_rgb8(0x5C, 0x63, 0x70);
pub const SYN_PUNCT: Color = FG_2;
pub const SYN_ORANGE: Color = Color::from_rgb8(0xD1, 0x9A, 0x66);

// ── Type scale (px, base = 13) ────────────────────────────────────────────────
pub const T_MICRO: f32 = 10.0;
pub const T_TINY: f32 = 11.0;
pub const T_SMALL: f32 = 12.0;
pub const T_BASE: f32 = 13.0;
pub const T_MD: f32 = 14.0;
pub const T_LG: f32 = 16.0;
pub const T_XL: f32 = 18.0;
pub const T_2XL: f32 = 20.0;
pub const T_3XL: f32 = 22.0;
pub const T_4XL: f32 = 28.0;
pub const T_5XL: f32 = 36.0;

// ── Spacing (4-px grid) ───────────────────────────────────────────────────────
pub const S_1: f64 = 4.0;
pub const S_2: f64 = 8.0;
pub const S_3: f64 = 12.0;
pub const S_4: f64 = 16.0;
pub const S_5: f64 = 20.0;
pub const S_6: f64 = 24.0;
pub const S_7: f64 = 32.0;
pub const S_8: f64 = 40.0;
pub const S_9: f64 = 48.0;

// ── Radii ─────────────────────────────────────────────────────────────────────
pub const R_2: f64 = 2.0;
pub const R_4: f64 = 4.0;
pub const R_6: f64 = 6.0;
pub const R_8: f64 = 8.0;
pub const R_10: f64 = 10.0;
pub const R_14: f64 = 14.0;
pub const R_20: f64 = 20.0;
pub const R_FULL: f64 = 9999.0;

// ── Shell metrics ─────────────────────────────────────────────────────────────
pub const UI_HEADER_HEIGHT: f64 = 36.0;
pub const UI_STATUS_HEIGHT: f64 = 25.0;
pub const UI_TAB_MIN_WIDTH: f64 = 100.0;
pub const UI_ACTIVITY_WIDTH: f64 = 50.0;
pub const UI_SCROLL_WIDTH: f64 = 10.0;

// ── Motion ────────────────────────────────────────────────────────────────────
pub const MOTION_FAST: Duration = Duration::from_millis(120);
pub const MOTION_MED: Duration = Duration::from_millis(350);
pub const MOTION_SPRING: Duration = Duration::from_millis(550);

// ── Legacy aliases (keep backward-compat if anything imports these) ───────────
pub const BACKGROUND: Color = BG_NAVY;
pub const SURFACE: Color = BG_SURFACE;
pub const ACCENT: Color = ALLOY_ORANGE;
pub const TEXT_PRIMARY: Color = FG_1;
pub const TEXT_SECONDARY: Color = FG_2;
pub const TEXT_MUTED: Color = FG_3;
pub const BORDER: Color = LINE_RING;
