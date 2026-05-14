//! Table heatmap — `rows × cols` matrix of numeric values
//! rendered as a colour grid. Confusion matrices, hour×day
//! activity, regional×product sales.

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::heatmap::SequentialPalette;
use crate::theme::Theme;

/// Table heatmap value type.
#[derive(Clone, Debug)]
pub struct TableHeatmap {
    /// Row labels.
    pub rows: Vec<String>,
    /// Column labels.
    pub cols: Vec<String>,
    /// `values[r][c]` is the cell at row `r`, column `c`.
    pub values: Vec<Vec<f32>>,
    /// Colour palette.
    pub palette: SequentialPalette,
}

impl TableHeatmap {
    /// Construct with the blues palette as default.
    #[must_use]
    pub fn new(rows: Vec<String>, cols: Vec<String>, values: Vec<Vec<f32>>) -> Self {
        Self {
            rows,
            cols,
            values,
            palette: SequentialPalette::blues(),
        }
    }

    /// Override the palette.
    #[must_use]
    pub fn palette(mut self, palette: SequentialPalette) -> Self {
        self.palette = palette;
        self
    }

    /// Emit one filled rect per cell.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        if self.rows.is_empty() || self.cols.is_empty() || self.values.is_empty() {
            return g;
        }
        let pad = 16.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let plot_w = plot_right - plot_left;
        let plot_h = plot_bottom - plot_top;
        let cell_w = plot_w / usize_to_f32(self.cols.len());
        let cell_h = plot_h / usize_to_f32(self.rows.len());

        let (lo, hi) = numeric_extent(&self.values);
        let span = (hi - lo).max(f32::EPSILON);

        for (r, row) in self.values.iter().enumerate() {
            for (c, value) in row.iter().enumerate() {
                if c >= self.cols.len() || r >= self.rows.len() {
                    continue;
                }
                let t = (value - lo) / span;
                let color = self.palette.sample(t);
                g.fill(Fill::Solid(chart_to_wisp(color)));
                let x = plot_left + usize_to_f32(c) * cell_w;
                let y = plot_top + usize_to_f32(r) * cell_h;
                let rect = px_rect_to_ndc(x, y, cell_w - 1.0, cell_h - 1.0, viewport_px);
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
        reason = "row/col counts ≤ ~1000 fit f32 mantissa easily"
    )]
    {
        v as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_heatmap_emits_one_rect_per_cell() {
        let h = TableHeatmap::new(
            vec!["a".into(), "b".into(), "c".into()],
            vec!["x".into(), "y".into(), "z".into(), "w".into()],
            vec![
                vec![1.0, 2.0, 3.0, 4.0],
                vec![5.0, 6.0, 7.0, 8.0],
                vec![9.0, 10.0, 11.0, 12.0],
            ],
        );
        let theme = Theme::light();
        let g = h.emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 12);
    }

    #[test]
    fn empty_heatmap_emits_nothing() {
        let h = TableHeatmap::new(vec![], vec![], vec![]);
        let theme = Theme::light();
        let g = h.emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
