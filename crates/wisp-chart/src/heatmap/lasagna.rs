//! Lasagna plot — dense time-series-per-row heatmap. One entity
//! per row, time across columns. Reads patterns across hundreds
//! of entities that a multi-line chart spaghettis up.

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::heatmap::SequentialPalette;
use crate::theme::Theme;

/// Lasagna value type — same data shape as
/// [`crate::heatmap::TableHeatmap`] but rendered with no inter-
/// cell gap (one continuous stripe per entity) and a default
/// magma palette.
#[derive(Clone, Debug)]
pub struct LasagnaHeatmap {
    /// Entity labels (one per row).
    pub entities: Vec<String>,
    /// Time labels (one per column).
    pub times: Vec<String>,
    /// `values[r][c]` is entity `r` at time `c`.
    pub values: Vec<Vec<f32>>,
    /// Colour palette.
    pub palette: SequentialPalette,
}

impl LasagnaHeatmap {
    /// Construct with the magma palette as default.
    #[must_use]
    pub fn new(entities: Vec<String>, times: Vec<String>, values: Vec<Vec<f32>>) -> Self {
        Self {
            entities,
            times,
            values,
            palette: SequentialPalette::magma(),
        }
    }

    /// Override the palette.
    #[must_use]
    pub fn palette(mut self, palette: SequentialPalette) -> Self {
        self.palette = palette;
        self
    }

    /// Emit one rect per cell (no gap between cells).
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        if self.entities.is_empty() || self.times.is_empty() || self.values.is_empty() {
            return g;
        }
        let pad = 16.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let plot_w = plot_right - plot_left;
        let plot_h = plot_bottom - plot_top;
        let cell_w = plot_w / usize_to_f32(self.times.len());
        let cell_h = plot_h / usize_to_f32(self.entities.len());

        let (lo, hi) = numeric_extent(&self.values);
        let span = (hi - lo).max(f32::EPSILON);

        for (r, row) in self.values.iter().enumerate() {
            if r >= self.entities.len() {
                continue;
            }
            for (c, value) in row.iter().enumerate() {
                if c >= self.times.len() {
                    continue;
                }
                let t = (value - lo) / span;
                let color = self.palette.sample(t);
                g.fill(Fill::Solid(chart_to_wisp(color)));
                let x = plot_left + usize_to_f32(c) * cell_w;
                let y = plot_top + usize_to_f32(r) * cell_h;
                let rect = px_rect_to_ndc(x, y, cell_w, cell_h, viewport_px);
                g.draw_rect(rect);
            }
        }
        g
    }
}

fn numeric_extent(values: &[Vec<f32>]) -> (f32, f32) {
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for row in values {
        for v in row {
            lo = lo.min(*v);
            hi = hi.max(*v);
        }
    }
    if lo.is_infinite() {
        (0.0, 1.0)
    } else {
        (lo, hi)
    }
}

fn px_rect_to_ndc(x: f32, y: f32, w: f32, h: f32, viewport_px: Vec2) -> Rect {
    let nx = x / viewport_px.x * 2.0 - 1.0;
    let ny = 1.0 - (y + h) / viewport_px.y * 2.0;
    Rect::new(nx, ny, w / viewport_px.x * 2.0, h / viewport_px.y * 2.0)
}

fn chart_to_wisp(c: ChartColor) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn usize_to_f32(v: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "row/col counts ≤ ~1000 fit f32 mantissa"
    )]
    {
        v as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lasagna_emits_one_rect_per_cell() {
        let h = LasagnaHeatmap::new(
            vec!["a".into(), "b".into(), "c".into()],
            (1..=10).map(|i| format!("t{i}")).collect(),
            (0..3)
                .map(|r| (0..10).map(|c| usize_to_f32(r * 10 + c)).collect())
                .collect(),
        );
        let theme = Theme::light();
        let g = h.emit_graphics(&theme, Vec2::new(600.0, 200.0));
        assert_eq!(g.primitive_count(), 30);
    }
}
