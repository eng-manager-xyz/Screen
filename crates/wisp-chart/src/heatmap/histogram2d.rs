//! 2D histogram heatmap — bin a point cloud over a 2D grid and
//! render each cell with intensity from a sequential palette.
//! Fallback for overplotted scatterplots (>5k points).

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::heatmap::SequentialPalette;
use crate::theme::Theme;

/// 2D-histogram value type.
#[derive(Clone, Debug)]
pub struct Histogram2D {
    /// Counts laid out row-major — `counts[row * cols + col]`.
    pub counts: Vec<u32>,
    /// Grid width (cols).
    pub cols: usize,
    /// Grid height (rows).
    pub rows: usize,
    /// Colour ramp.
    pub palette: SequentialPalette,
}

impl Histogram2D {
    /// Bin a point cloud into a `cols × rows` grid.
    /// `extent` clips both axes; samples outside are dropped.
    #[must_use]
    pub fn from_points(
        points: &[(f32, f32)],
        cols: usize,
        rows: usize,
        extent: Option<((f32, f32), (f32, f32))>,
    ) -> Self {
        let palette = SequentialPalette::magma();
        if cols == 0 || rows == 0 {
            return Self {
                counts: Vec::new(),
                cols: 0,
                rows: 0,
                palette,
            };
        }
        let ((x_lo, x_hi), (y_lo, y_hi)) = extent.unwrap_or_else(|| {
            let mut x_lo = f32::INFINITY;
            let mut x_hi = f32::NEG_INFINITY;
            let mut y_lo = f32::INFINITY;
            let mut y_hi = f32::NEG_INFINITY;
            for &(x, y) in points {
                x_lo = x_lo.min(x);
                x_hi = x_hi.max(x);
                y_lo = y_lo.min(y);
                y_hi = y_hi.max(y);
            }
            ((x_lo, x_hi), (y_lo, y_hi))
        });
        let x_span = (x_hi - x_lo).max(f32::EPSILON);
        let y_span = (y_hi - y_lo).max(f32::EPSILON);
        let mut counts = vec![0_u32; cols * rows];
        for &(x, y) in points {
            if x < x_lo || x > x_hi || y < y_lo || y > y_hi {
                continue;
            }
            let col_norm = ((x - x_lo) / x_span).clamp(0.0, 1.0);
            let row_norm = ((y - y_lo) / y_span).clamp(0.0, 1.0);
            let col_idx = f32_to_clamped_index(col_norm * usize_to_f32(cols), cols);
            let row_idx = f32_to_clamped_index(row_norm * usize_to_f32(rows), rows);
            counts[row_idx * cols + col_idx] += 1;
        }
        Self {
            counts,
            cols,
            rows,
            palette,
        }
    }

    /// Override the colour palette.
    #[must_use]
    pub fn palette(mut self, palette: SequentialPalette) -> Self {
        self.palette = palette;
        self
    }

    /// Emit one rect per non-zero cell.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        if self.counts.is_empty() {
            return g;
        }
        let pad = 16.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let plot_w = plot_right - plot_left;
        let plot_h = plot_bottom - plot_top;
        let cell_w = plot_w / usize_to_f32(self.cols);
        let cell_h = plot_h / usize_to_f32(self.rows);
        let max_count = self.counts.iter().copied().max().unwrap_or(0);
        if max_count == 0 {
            return g;
        }
        let max_f = u32_to_f32(max_count);

        for row in 0..self.rows {
            for col in 0..self.cols {
                let count = self.counts[row * self.cols + col];
                if count == 0 {
                    continue;
                }
                let t = u32_to_f32(count) / max_f;
                let color = self.palette.sample(t);
                g.fill(Fill::Solid(chart_to_wisp(color)));
                let x = plot_left + usize_to_f32(col) * cell_w;
                // y-row 0 = bottom (lowest y bin) to match chart
                // convention (low values at bottom).
                let y = plot_bottom - (usize_to_f32(row) + 1.0) * cell_h;
                let rect = px_rect_to_ndc(x, y, cell_w, cell_h, viewport_px);
                g.draw_rect(rect);
            }
        }
        g
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

fn f32_to_clamped_index(v: f32, dim: usize) -> usize {
    let limit = dim.saturating_sub(1);
    if v <= 0.0 {
        return 0;
    }
    let limit_f = usize_to_f32(limit);
    if v >= limit_f {
        return limit;
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "v in [0, limit_f]; non-negative, within usize range"
    )]
    {
        v as usize
    }
}

fn usize_to_f32(v: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "grid dim ≤ ~1024 fits f32 mantissa easily"
    )]
    {
        v as f32
    }
}

fn u32_to_f32(v: u32) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "cell counts ≤ ~16M fit f32 mantissa precisely"
    )]
    {
        v as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histogram2d_bins_concentrate_count_in_correct_cell() {
        // 10 points all at (0.5, 0.5) should land in middle cell.
        let points: Vec<(f32, f32)> = (0..10).map(|_| (0.5_f32, 0.5_f32)).collect();
        let h2d = Histogram2D::from_points(&points, 3, 3, Some(((0.0, 1.0), (0.0, 1.0))));
        // Middle cell index = row 1, col 1.
        assert_eq!(h2d.counts[3 + 1], 10);
    }

    #[test]
    fn histogram2d_emits_one_rect_per_nonzero_cell() {
        let points = vec![(0.1, 0.1), (0.9, 0.9), (0.5, 0.5)];
        let h2d = Histogram2D::from_points(&points, 5, 5, Some(((0.0, 1.0), (0.0, 1.0))));
        let theme = Theme::light();
        let g = h2d.emit_graphics(&theme, Vec2::new(400.0, 400.0));
        // 3 distinct cells.
        assert_eq!(g.primitive_count(), 3);
    }

    #[test]
    fn empty_points_emits_nothing() {
        let h2d = Histogram2D::from_points(&[], 5, 5, None);
        let theme = Theme::light();
        let g = h2d.emit_graphics(&theme, Vec2::new(400.0, 400.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
