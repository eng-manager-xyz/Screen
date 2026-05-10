//! Atlas text backend (M-TEXT.4 / AUT-78).
//!
//! `AtlasText` is the simple/static/performance text path — bitmap font
//! atlases, batchable per-atlas, deterministic, no shaping or `BiDi`. It
//! preserves the M0.15 bitmap path (`scene::Text` + `text_pipeline`)
//! while expressing the same data through the wisp text trait surface
//! ([`WispTextEngine`], [`WispTextLayout`]).
//!
//! For styled, wrapped, or user-typed text use `FlexibleText` (Cosmic
//! Text + Glyphon, lands in M-TEXT.2/.3). The two backends are
//! complementary, not competitive — see `_docs/book/src/wisp/text/atlas-vs-flexible.md`
//! for the comparison table.
//!
//! Layout semantics:
//!
//! - `style.size_ndc` is the cell side length in NDC. font8x8 cells are
//!   square, so glyph width = glyph height = `size_ndc`.
//! - Glyphs advance horizontally by `size_ndc + style.letter_spacing_ndc`.
//! - Newlines (`\n`) advance the y cursor by
//!   `size_ndc * style.line_height`.
//! - `style.align` shifts whole lines (Left = no shift,
//!   Center = `(max_w - line_w)/2`, Right = `max_w - line_w`).
//! - `text.max_width_ndc` is **ignored** by `AtlasText` (no word
//!   wrapping). Captions / soft-wrap belong to `FlexibleText`.
//! - Codepoints absent from the bitmap atlas (anything ≥ 128) are
//!   silently dropped — same behavior as the M0.15 `scene::Text` node.

use glam::Vec2;

use super::{WispText, WispTextAlign, WispTextEngine, WispTextLayout, WispTextMetrics};
use crate::color::Color;
use crate::scene::text::{Font, GlyphMetrics};

/// One laid-out glyph — NDC-positioned quad plus atlas UV.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtlasGlyphInstance {
    /// Top-left of the glyph quad in NDC (x increases right, y the
    /// renderer's chosen convention — `AtlasText` preserves the
    /// `scene::Text` semantics where the layout flows top-down from
    /// `text.position` with line index as a positive offset).
    pub origin: Vec2,
    /// Width in NDC (= `style.size_ndc`).
    pub width: f32,
    /// Height in NDC (= `style.size_ndc`).
    pub height: f32,
    /// Atlas UV rect.
    pub uvs: GlyphMetrics,
    /// Solid tint pulled from `style.color` for convenience.
    pub color: Color,
}

/// Result of [`AtlasTextEngine::layout`] — a flat glyph list plus
/// metrics.
#[derive(Debug, Clone)]
pub struct AtlasTextLayout {
    /// One entry per drawable glyph (whitespace skipped, missing
    /// codepoints dropped).
    pub glyphs: Vec<AtlasGlyphInstance>,
    metrics: WispTextMetrics,
}

impl AtlasTextLayout {
    /// Borrow the underlying glyph instances.
    #[must_use]
    pub fn glyphs(&self) -> &[AtlasGlyphInstance] {
        &self.glyphs
    }
}

impl WispTextLayout for AtlasTextLayout {
    fn metrics(&self) -> WispTextMetrics {
        self.metrics
    }
}

/// Bitmap atlas text engine. Wraps a [`Font`] (M0.15 8×8 bitmap or
/// any future atlas) and lays out a [`WispText`] into per-glyph NDC
/// quads.
#[derive(Debug, Clone)]
pub struct AtlasTextEngine {
    font: Font,
}

impl AtlasTextEngine {
    /// Build the engine from a font.
    #[must_use]
    pub fn new(font: Font) -> Self {
        Self { font }
    }

    /// Borrow the font (so the renderer can reach the atlas texture).
    #[must_use]
    pub fn font(&self) -> &Font {
        &self.font
    }

    /// Layout `text` into a concrete [`AtlasTextLayout`]. The trait
    /// version (`<Self as WispTextEngine>::layout`) boxes this for
    /// dyn dispatch; this method preserves the concrete type so the
    /// renderer side can read `glyphs()` directly without downcasting.
    #[must_use]
    pub fn layout_concrete(&self, text: &WispText) -> AtlasTextLayout {
        layout_atlas(&self.font, text)
    }
}

impl WispTextEngine for AtlasTextEngine {
    fn layout(&self, text: &WispText) -> Box<dyn WispTextLayout> {
        Box::new(layout_atlas(&self.font, text))
    }
}

fn layout_atlas(font: &Font, text: &WispText) -> AtlasTextLayout {
    let style = text.style;
    let cell_w = style.size_ndc;
    let cell_h = style.size_ndc;
    let advance = cell_w + style.letter_spacing_ndc;
    let line_step = cell_h * style.line_height;

    let lines: Vec<&str> = text.content.split('\n').collect();
    let mut line_widths: Vec<f32> = Vec::with_capacity(lines.len());
    for line in &lines {
        let glyph_count_usize = line.chars().filter(|c| font.glyph(*c).is_some()).count();
        #[expect(
            clippy::cast_precision_loss,
            reason = "line glyph counts are small (< 2^23 in practice)"
        )]
        let glyph_count = glyph_count_usize as f32;
        let width = if glyph_count > 0.0 {
            (glyph_count - 1.0).max(0.0) * advance + cell_w
        } else {
            0.0
        };
        line_widths.push(width);
    }
    let max_width = line_widths.iter().copied().fold(0.0_f32, f32::max);

    let mut glyphs: Vec<AtlasGlyphInstance> = Vec::new();
    for (line_idx, line) in lines.iter().enumerate() {
        let line_width = line_widths[line_idx];
        let x_offset = match style.align {
            WispTextAlign::Left => 0.0,
            WispTextAlign::Center => (max_width - line_width) * 0.5,
            WispTextAlign::Right => max_width - line_width,
        };
        let mut x = text.position.x + x_offset;
        #[expect(
            clippy::cast_precision_loss,
            reason = "line_idx bounded by line count — fits losslessly in f32"
        )]
        let line_y = text.position.y + (line_idx as f32) * line_step;
        for c in line.chars() {
            if let Some(uvs) = font.glyph(c) {
                glyphs.push(AtlasGlyphInstance {
                    origin: Vec2::new(x, line_y),
                    width: cell_w,
                    height: cell_h,
                    uvs,
                    color: style.color,
                });
            }
            x += advance;
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "line_count is small (< 2^32) by construction"
    )]
    let line_count = lines.len() as u32;
    let extra_lines = lines.len().saturating_sub(1);
    #[expect(
        clippy::cast_precision_loss,
        reason = "line counts are small (< 2^23 in practice)"
    )]
    let extra_lines_f = extra_lines as f32;
    let total_height = if lines.is_empty() {
        0.0
    } else {
        cell_h + extra_lines_f * line_step
    };

    AtlasTextLayout {
        glyphs,
        metrics: WispTextMetrics {
            line_count,
            max_width_ndc: max_width,
            total_height_ndc: total_height,
            baseline_ndc: cell_h,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{AppConfig, Application};
    use crate::text::{WispFontWeight, WispText, WispTextStyle};

    fn boot() -> Application {
        pollster::block_on(Application::new(AppConfig::default())).expect("init")
    }

    fn engine() -> AtlasTextEngine {
        let app = boot();
        AtlasTextEngine::new(Font::bitmap_8x8(&app))
    }

    #[test]
    fn empty_text_yields_no_glyphs_and_metric_zero_width() {
        let eng = engine();
        let layout = eng.layout_concrete(&WispText::new(""));
        assert!(layout.glyphs().is_empty());
        assert_eq!(layout.metrics().line_count, 1);
        assert!(layout.metrics().max_width_ndc.abs() < f32::EPSILON);
    }

    #[test]
    fn single_line_emits_one_glyph_per_ascii_char() {
        let eng = engine();
        let layout = eng.layout_concrete(&WispText::new("Hello"));
        assert_eq!(layout.glyphs().len(), 5);
        assert_eq!(layout.metrics().line_count, 1);
        // 5 chars at default 0.06 size, no letter spacing → width = 5 * 0.06 = 0.30.
        let expected = 0.30_f32;
        assert!(
            (layout.metrics().max_width_ndc - expected).abs() < 1e-5,
            "got {} expected {expected}",
            layout.metrics().max_width_ndc
        );
    }

    #[test]
    fn newline_starts_a_new_line_and_advances_y() {
        let eng = engine();
        let layout = eng.layout_concrete(&WispText::new("ab\ncd").with_position(Vec2::ZERO));
        assert_eq!(layout.metrics().line_count, 2);
        assert_eq!(layout.glyphs().len(), 4);
        // First glyph at y=0; third glyph (start of line 2) at y = 0.06 * 1.2 = 0.072.
        let expected_step = 0.06_f32 * 1.2;
        let g0 = layout.glyphs()[0];
        let g2 = layout.glyphs()[2];
        assert!(g0.origin.y.abs() < 1e-6);
        assert!(
            (g2.origin.y - expected_step).abs() < 1e-5,
            "g2.y={} expected={expected_step}",
            g2.origin.y
        );
    }

    #[test]
    fn non_ascii_codepoints_are_dropped_silently() {
        let eng = engine();
        let layout = eng.layout_concrete(&WispText::new("aé"));
        // 'é' is past 128, dropped; only 'a' survives.
        assert_eq!(layout.glyphs().len(), 1);
    }

    #[test]
    fn center_align_shifts_short_line_to_match_long_line() {
        let eng = engine();
        let style = WispTextStyle::default().with_align(WispTextAlign::Center);
        // "ab" (2 chars wide) on top of "abcd" (4 chars wide).
        let layout = eng.layout_concrete(
            &WispText::new("ab\nabcd")
                .with_style(style)
                .with_position(Vec2::ZERO),
        );
        assert_eq!(layout.metrics().line_count, 2);
        // Line widths: 2 * 0.06 = 0.12, 4 * 0.06 = 0.24. Center shift for first
        // line = (0.24 - 0.12) / 2 = 0.06. So first glyph x = 0.06.
        let g0 = layout.glyphs()[0];
        assert!(
            (g0.origin.x - 0.06).abs() < 1e-5,
            "g0.x={} expected 0.06",
            g0.origin.x
        );
    }

    #[test]
    fn weight_and_italic_do_not_change_atlas_layout() {
        // AtlasText is bitmap; weight + style are no-ops at this layer
        // (FlexibleText surfaces them). This locks that contract.
        let eng = engine();
        let plain = eng.layout_concrete(&WispText::new("test"));
        let bold_italic = eng.layout_concrete(
            &WispText::new("test").with_style(
                WispTextStyle::default()
                    .with_weight(WispFontWeight::Bold)
                    .italic(),
            ),
        );
        assert_eq!(plain.glyphs().len(), bold_italic.glyphs().len());
        for (a, b) in plain.glyphs().iter().zip(bold_italic.glyphs().iter()) {
            assert!((a.origin.x - b.origin.x).abs() < 1e-6);
            assert!((a.origin.y - b.origin.y).abs() < 1e-6);
        }
    }

    #[test]
    fn metrics_total_height_matches_line_count() {
        let eng = engine();
        let layout = eng.layout_concrete(&WispText::new("a\nb\nc"));
        // 3 lines. height = 0.06 + 2 * 0.06 * 1.2 = 0.06 + 0.144 = 0.204.
        let expected = 0.06_f32 + 2.0 * 0.06 * 1.2;
        assert!(
            (layout.metrics().total_height_ndc - expected).abs() < 1e-5,
            "total={} expected={expected}",
            layout.metrics().total_height_ndc
        );
    }
}
