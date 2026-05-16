//! Legend renderer — colour-coded swatches + labels mapping a
//! categorical encoding's values to their palette positions.
//!
//! Same shape as [`crate::axis`]: a `Legend` value is built by the
//! caller (or auto-built by `Plot` when a `Channel::Color` encoding
//! is present), then emits two outputs:
//!
//! * `emit_graphics(position, viewport_px) -> wisp::Graphics` — the
//!   optional rounded background rect + per-item swatch primitives.
//! * `emit_text_nodes(app, pipeline, position, viewport_px, ..., text_color) -> Vec<wisp::FlexText>`
//!   — the per-item label glyphs. Text requires a Font, which lives
//!   on a wgpu device, so the caller supplies it.
//!
//! Orientation:
//!
//! * [`LegendOrientation::Vertical`] stacks items top-down with
//!   `theme.legend.item_spacing_px` between them. The legend's
//!   width is the swatch + spacing + widest label.
//! * [`LegendOrientation::Horizontal`] lays items left-to-right.
//!   When the running x exceeds `viewport_px.x - position.x` the
//!   layout wraps to a new row.
//!
//! All positions are pixel-space (top-left origin); the emit
//! functions convert to NDC before pushing into `wisp::Graphics` /
//! `wisp::Text`.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::text::TextTexturePipeline;
use wisp::{Color, Fill, FlexText, Graphics, WispFontWeight};

use crate::chart_text::{ChartTextSpec, TextAnchor, build_text_node};

use crate::color::Color as ChartColor;
use crate::theme::LegendTheme;

/// Layout direction for legend items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LegendOrientation {
    /// Stack items top-down. Default — best for narrow side
    /// panels.
    #[default]
    Vertical,
    /// Lay items left-to-right with line wrapping. Best for
    /// chart-top / chart-bottom placement.
    Horizontal,
}

/// How an individual legend item's swatch is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwatchStyle {
    /// Solid filled square — used for bar / area / cell marks.
    ColorBox(ChartColor),
    /// Short horizontal line — used for line / trend marks.
    LineSample(ChartColor),
    /// Filled circle marker — used for scatter / dot marks.
    PointMarker(ChartColor),
}

impl SwatchStyle {
    fn color(self) -> ChartColor {
        match self {
            Self::ColorBox(c) | Self::LineSample(c) | Self::PointMarker(c) => c,
        }
    }
}

/// One legend entry — label string + swatch style.
#[derive(Clone, Debug, PartialEq)]
pub struct LegendItem {
    /// Display string rendered to the right of the swatch.
    pub label: String,
    /// How the colour swatch is drawn.
    pub swatch: SwatchStyle,
}

/// Composed legend — items + orientation + theme.
#[derive(Clone, Debug)]
pub struct Legend {
    items: Vec<LegendItem>,
    orientation: LegendOrientation,
}

impl Legend {
    /// Construct an empty legend with default orientation
    /// ([`LegendOrientation::Vertical`]).
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            orientation: LegendOrientation::default(),
        }
    }

    /// Add an item built from a label + swatch style.
    #[must_use]
    pub fn item(mut self, label: impl Into<String>, swatch: SwatchStyle) -> Self {
        self.items.push(LegendItem {
            label: label.into(),
            swatch,
        });
        self
    }

    /// Set the layout direction.
    #[must_use]
    pub const fn orientation(mut self, orientation: LegendOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Number of items in the legend.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the legend has no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Read-only access to the item list — used by tests and by
    /// upstream `Plot` integration to inspect what the legend
    /// will draw.
    #[must_use]
    pub fn items(&self) -> &[LegendItem] {
        &self.items
    }

    /// Compute the pixel-space position of each item's swatch
    /// top-left given the legend's `position` (top-left in pixel
    /// space) and the `viewport_px` it lives inside.
    ///
    /// Exposed so tests can verify layout without rendering.
    #[must_use]
    pub fn item_positions(
        &self,
        position: Vec2,
        viewport_px: Vec2,
        theme: &LegendTheme,
        font_cell_px: f32,
    ) -> Vec<Vec2> {
        let mut out = Vec::with_capacity(self.items.len());
        let swatch = theme.swatch_size_px;
        let spacing = theme.item_spacing_px;
        match self.orientation {
            LegendOrientation::Vertical => {
                let row_h = swatch.max(theme.item_font_size) + spacing;
                for (i, _) in self.items.iter().enumerate() {
                    let y = position.y + usize_to_f32(i) * row_h;
                    out.push(Vec2::new(position.x, y));
                }
            }
            LegendOrientation::Horizontal => {
                let mut cur = position;
                let row_h = swatch.max(theme.item_font_size) + spacing;
                let glyph_w = font_cell_px * theme.item_font_size / font_cell_px;
                // (font_cell_px gives one px per glyph cell; the
                // ratio above is a placeholder kept for clarity.)
                let _ = glyph_w;
                for item in &self.items {
                    let label_w = label_width_px(&item.label, theme.item_font_size, font_cell_px);
                    let item_w = swatch + spacing + label_w;
                    if cur.x + item_w > position.x + viewport_px.x - position.x {
                        cur.x = position.x;
                        cur.y += row_h;
                    }
                    out.push(cur);
                    cur.x += item_w + spacing;
                }
            }
        }
        out
    }

    /// Emit the legend's swatches (and optional background) as a
    /// `wisp::Graphics`.
    #[must_use]
    pub fn emit_graphics(
        &self,
        position: Vec2,
        viewport_px: Vec2,
        theme: &LegendTheme,
        font_cell_px: f32,
    ) -> Graphics {
        let mut g = Graphics::new();
        if self.items.is_empty() {
            return g;
        }
        let swatch = theme.swatch_size_px;
        let positions = self.item_positions(position, viewport_px, theme, font_cell_px);

        for (item, p) in self.items.iter().zip(positions.iter()) {
            let wisp = chart_to_wisp(item.swatch.color());
            g.fill(Fill::Solid(wisp));
            match item.swatch {
                SwatchStyle::ColorBox(_) => {
                    let r = pixel_rect_to_ndc(*p, Vec2::splat(swatch), viewport_px);
                    g.draw_rounded_rect(r, ndc_length_px(2.0, viewport_px));
                }
                SwatchStyle::LineSample(_) => {
                    // Short horizontal line, centred vertically in
                    // the swatch box.
                    let y = p.y + swatch * 0.5;
                    let a = pixel_to_ndc(Vec2::new(p.x, y), viewport_px);
                    let b = pixel_to_ndc(Vec2::new(p.x + swatch, y), viewport_px);
                    g.draw_line(a, b, ndc_length_px(2.0, viewport_px));
                }
                SwatchStyle::PointMarker(_) => {
                    let centre = pixel_to_ndc(
                        Vec2::new(p.x + swatch * 0.5, p.y + swatch * 0.5),
                        viewport_px,
                    );
                    let radii = Vec2::splat(ndc_length_px(swatch * 0.5, viewport_px));
                    g.draw_ellipse(centre, radii);
                }
            }
        }
        g
    }

    /// Emit the legend's label text as Inter-rendered [`FlexText`]
    /// nodes (late-pass, paint on top of every chart Graphics
    /// primitive).
    #[must_use]
    pub fn emit_text_nodes(
        &self,
        app: &Application,
        pipeline: &TextTexturePipeline,
        position: Vec2,
        viewport_px: Vec2,
        theme: &LegendTheme,
        text_color: ChartColor,
    ) -> Vec<FlexText> {
        // Use the item font size as the "cell pixel" proxy for the
        // legend's item_positions math — it's an approximation
        // already and inter glyph widths run ~0.5 em on average,
        // close enough to keep the existing layout passable.
        let positions = self.item_positions(position, viewport_px, theme, theme.item_font_size);
        let swatch = theme.swatch_size_px;
        let spacing = theme.item_spacing_px;

        let mut out = Vec::with_capacity(self.items.len());
        for (item, p) in self.items.iter().zip(positions.iter()) {
            let spec = ChartTextSpec {
                content: item.label.clone(),
                anchor_px: Vec2::new(p.x + swatch + spacing, p.y + swatch * 0.5),
                size_px: theme.item_font_size,
                color: text_color,
                anchor: TextAnchor::MiddleLeft,
                weight: WispFontWeight::Regular,
            };
            out.push(build_text_node(app, pipeline, viewport_px, &spec));
        }
        out
    }
}

impl Default for Legend {
    fn default() -> Self {
        Self::new()
    }
}

fn pixel_to_ndc(p: Vec2, viewport_px: Vec2) -> Vec2 {
    Vec2::new(
        p.x / viewport_px.x * 2.0 - 1.0,
        1.0 - p.y / viewport_px.y * 2.0,
    )
}

fn pixel_rect_to_ndc(top_left: Vec2, size: Vec2, viewport_px: Vec2) -> Rect {
    let x = top_left.x / viewport_px.x * 2.0 - 1.0;
    let y = 1.0 - (top_left.y + size.y) / viewport_px.y * 2.0;
    let w = size.x / viewport_px.x * 2.0;
    let h = size.y / viewport_px.y * 2.0;
    Rect::new(x, y, w, h)
}

fn ndc_length_px(px: f32, viewport_px: Vec2) -> f32 {
    px / viewport_px.y * 2.0
}

fn chart_to_wisp(c: ChartColor) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn label_width_px(label: &str, font_size: f32, cell_px: f32) -> f32 {
    // Approximate: bitmap font glyphs are square (cell × cell).
    // Scale by font_size / cell_px to get pixel width per glyph,
    // then multiply by char count.
    let glyph_w = font_size;
    let _ = cell_px;
    glyph_w * usize_to_f32(label.chars().count()) * 0.6
}

fn usize_to_f32(v: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "legend item counts + label char counts fit in f32 mantissa (~8M chars). Practical max <100."
    )]
    {
        v as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color as ChartColor;
    use crate::theme::Theme;

    fn theme() -> LegendTheme {
        Theme::light().legend
    }

    fn red() -> ChartColor {
        ChartColor::from_hex("#e74c3c").unwrap()
    }
    fn green() -> ChartColor {
        ChartColor::from_hex("#27ae60").unwrap()
    }
    fn blue() -> ChartColor {
        ChartColor::from_hex("#2980b9").unwrap()
    }

    #[test]
    fn vertical_layout_has_uniform_spacing() {
        let legend = Legend::new()
            .item("Red", SwatchStyle::ColorBox(red()))
            .item("Green", SwatchStyle::ColorBox(green()))
            .item("Blue", SwatchStyle::ColorBox(blue()));
        let positions = legend.item_positions(
            Vec2::new(20.0, 20.0),
            Vec2::new(480.0, 320.0),
            &theme(),
            8.0,
        );
        assert_eq!(positions.len(), 3);
        let dy1 = positions[1].y - positions[0].y;
        let dy2 = positions[2].y - positions[1].y;
        assert!(
            (dy1 - dy2).abs() < 1e-3,
            "vertical spacing should be uniform: dy1={dy1}, dy2={dy2}"
        );
        // All same x.
        assert!((positions[0].x - positions[2].x).abs() < 1e-6);
    }

    #[test]
    fn horizontal_layout_advances_x_per_item() {
        let legend = Legend::new()
            .item("Red", SwatchStyle::ColorBox(red()))
            .item("Green", SwatchStyle::ColorBox(green()))
            .item("Blue", SwatchStyle::ColorBox(blue()))
            .orientation(LegendOrientation::Horizontal);
        let positions = legend.item_positions(
            Vec2::new(20.0, 20.0),
            Vec2::new(2000.0, 320.0),
            &theme(),
            8.0,
        );
        assert_eq!(positions.len(), 3);
        // Same y for items on the same row.
        assert!((positions[0].y - positions[1].y).abs() < 1e-6);
        assert!(
            positions[1].x > positions[0].x,
            "second item should be right of first"
        );
        assert!(
            positions[2].x > positions[1].x,
            "third item should be right of second"
        );
    }

    #[test]
    fn empty_legend_emits_empty_graphics() {
        let legend = Legend::new();
        let g = legend.emit_graphics(Vec2::ZERO, Vec2::new(480.0, 320.0), &theme(), 8.0);
        assert_eq!(g.primitive_count(), 0);
    }

    #[test]
    fn one_item_per_swatch_primitive() {
        let legend = Legend::new()
            .item("Red", SwatchStyle::ColorBox(red()))
            .item("Green", SwatchStyle::LineSample(green()))
            .item("Blue", SwatchStyle::PointMarker(blue()));
        let g = legend.emit_graphics(
            Vec2::new(20.0, 20.0),
            Vec2::new(480.0, 320.0),
            &theme(),
            8.0,
        );
        assert_eq!(g.primitive_count(), 3);
    }

    #[test]
    fn horizontal_layout_wraps_when_exceeding_viewport() {
        let legend = Legend::new()
            .item("AAAAAAAAAA", SwatchStyle::ColorBox(red()))
            .item("BBBBBBBBBB", SwatchStyle::ColorBox(green()))
            .item("CCCCCCCCCC", SwatchStyle::ColorBox(blue()))
            .orientation(LegendOrientation::Horizontal);
        let positions = legend.item_positions(
            Vec2::new(0.0, 0.0),
            Vec2::new(80.0, 240.0), // Narrow — wrap expected.
            &theme(),
            8.0,
        );
        assert_eq!(positions.len(), 3);
        // At least one wrap should have occurred: y should
        // increase somewhere.
        let max_y = positions.iter().map(|p| p.y).fold(0.0_f32, f32::max);
        assert!(
            max_y > 0.0,
            "narrow viewport should wrap to a new row, got positions {positions:?}"
        );
    }
}
