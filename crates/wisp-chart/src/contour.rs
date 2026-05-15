//! Contour plot — iso-lines over a 2D scalar field via the
//! marching-squares algorithm. v1 emits **line-only** contours
//! (no filled bands); filled bands are a follow-on once polygon
//! reconstruction lands.

use glam::Vec2;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::heatmap::SequentialPalette;
use crate::theme::Theme;

/// Contour plot value type.
#[derive(Clone, Debug)]
pub struct ContourPlot {
    /// Scalar field laid out row-major — `field[row * cols + col]`.
    pub field: Vec<f32>,
    /// Field width (number of x samples).
    pub cols: usize,
    /// Field height (number of y samples).
    pub rows: usize,
    /// Iso-levels to draw, in ascending order.
    pub levels: Vec<f32>,
    /// Palette — one stop per level, looked up by level index.
    pub palette: SequentialPalette,
}

impl ContourPlot {
    /// Construct from a field grid + iso-levels.
    #[must_use]
    pub fn new(field: Vec<f32>, cols: usize, rows: usize, levels: Vec<f32>) -> Self {
        Self {
            field,
            cols,
            rows,
            levels,
            palette: SequentialPalette::magma(),
        }
    }

    /// Override the palette.
    #[must_use]
    pub fn palette(mut self, palette: SequentialPalette) -> Self {
        self.palette = palette;
        self
    }

    /// Sample the field at `(col, row)` (out-of-bounds → 0.0).
    fn sample(&self, col: usize, row: usize) -> f32 {
        if col >= self.cols || row >= self.rows {
            0.0
        } else {
            self.field[row * self.cols + col]
        }
    }

    /// Emit one polyline segment per cell-edge crossing per
    /// level. Each line is a single primitive; complex contours
    /// produce many short lines that visually merge into smooth
    /// curves once antialiased.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        if self.cols < 2 || self.rows < 2 || self.levels.is_empty() {
            return g;
        }
        let pad = 16.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let plot_w = plot_right - plot_left;
        let plot_h = plot_bottom - plot_top;
        let cell_w = plot_w / usize_to_f32(self.cols - 1);
        let cell_h = plot_h / usize_to_f32(self.rows - 1);
        let map =
            |col: f32, row: f32| Vec2::new(plot_left + col * cell_w, plot_bottom - row * cell_h);
        let stroke = 1.5 / viewport_px.x * 2.0;
        let n_levels = self.levels.len();

        for (level_idx, &level) in self.levels.iter().enumerate() {
            let level_t = if n_levels == 1 {
                0.5
            } else {
                usize_to_f32(level_idx) / usize_to_f32(n_levels - 1)
            };
            let color = self.palette.sample(level_t);
            g.fill(Fill::Solid(chart_to_wisp(color)));

            for row in 0..(self.rows - 1) {
                for col in 0..(self.cols - 1) {
                    let v00 = self.sample(col, row);
                    let v10 = self.sample(col + 1, row);
                    let v11 = self.sample(col + 1, row + 1);
                    let v01 = self.sample(col, row + 1);
                    let case = marching_case(level, v00, v10, v11, v01);
                    // Lerp helpers: along each edge of the cell.
                    let edge_b = |a: f32, b: f32, c0x: f32, c0y: f32, c1x: f32, c1y: f32| {
                        let t = ((level - a) / (b - a)).clamp(0.0, 1.0);
                        Vec2::new(c0x + (c1x - c0x) * t, c0y + (c1y - c0y) * t)
                    };
                    let col_f = usize_to_f32(col);
                    let row_f = usize_to_f32(row);
                    // Edge points (b=bottom, t=top, l=left, r=right) in cell coords.
                    let bottom = edge_b(v00, v10, col_f, row_f, col_f + 1.0, row_f);
                    let right = edge_b(v10, v11, col_f + 1.0, row_f, col_f + 1.0, row_f + 1.0);
                    let top = edge_b(v01, v11, col_f, row_f + 1.0, col_f + 1.0, row_f + 1.0);
                    let left = edge_b(v00, v01, col_f, row_f, col_f, row_f + 1.0);
                    let segments = marching_segments(case, bottom, right, top, left);
                    for (a, b) in segments {
                        let p0 = pixel_to_ndc(map(a.x, a.y), viewport_px);
                        let p1 = pixel_to_ndc(map(b.x, b.y), viewport_px);
                        g.draw_line(p0, p1, stroke);
                    }
                }
            }
        }
        g
    }
}

fn marching_case(level: f32, v00: f32, v10: f32, v11: f32, v01: f32) -> u8 {
    let bit = |v: f32| u8::from(v >= level);
    bit(v00) | (bit(v10) << 1) | (bit(v11) << 2) | (bit(v01) << 3)
}

/// Returns 0, 1, or 2 line segments for the given marching-
/// squares case (ambiguous cases 5 and 10 use the simple
/// 2-segment resolution — accurate enough for visualisation).
fn marching_segments(
    case: u8,
    bottom: Vec2,
    right: Vec2,
    top: Vec2,
    left: Vec2,
) -> Vec<(Vec2, Vec2)> {
    match case {
        1 | 14 => vec![(left, bottom)],
        2 | 13 => vec![(bottom, right)],
        3 | 12 => vec![(left, right)],
        4 | 11 => vec![(right, top)],
        5 => vec![(left, bottom), (right, top)],
        6 | 9 => vec![(bottom, top)],
        7 | 8 => vec![(left, top)],
        10 => vec![(left, top), (bottom, right)],
        _ => Vec::new(),
    }
}

fn pixel_to_ndc(p: Vec2, viewport_px: Vec2) -> Vec2 {
    Vec2::new(
        p.x / viewport_px.x * 2.0 - 1.0,
        1.0 - p.y / viewport_px.y * 2.0,
    )
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
        reason = "field dim ≤ ~1024 fits f32 mantissa"
    )]
    {
        v as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marching_case_all_below_is_zero() {
        assert_eq!(marching_case(0.5, 0.0, 0.0, 0.0, 0.0), 0);
    }

    #[test]
    fn marching_case_all_above_is_fifteen() {
        assert_eq!(marching_case(0.5, 1.0, 1.0, 1.0, 1.0), 15);
    }

    #[test]
    fn marching_case_corner_set_is_one() {
        assert_eq!(marching_case(0.5, 1.0, 0.0, 0.0, 0.0), 1);
    }

    #[test]
    fn contour_emits_at_least_one_segment_for_radial_bump() {
        // Build a 5x5 radial bump where the centre is highest.
        let cols = 5_usize;
        let rows = 5_usize;
        let mut field = vec![0.0_f32; cols * rows];
        let cx = 2.0_f32;
        let cy = 2.0_f32;
        #[allow(
            clippy::cast_precision_loss,
            reason = "test grid is 5x5; cast is exact"
        )]
        for row in 0..rows {
            for col in 0..cols {
                let dx = col as f32 - cx;
                let dy = row as f32 - cy;
                field[row * cols + col] = (-(dx * dx + dy * dy) * 0.3).exp();
            }
        }
        let plot = ContourPlot::new(field, cols, rows, vec![0.3, 0.6]);
        let theme = Theme::light();
        let g = plot.emit_graphics(&theme, Vec2::new(400.0, 400.0));
        assert!(g.primitive_count() >= 2);
    }

    #[test]
    fn empty_field_emits_nothing() {
        let plot = ContourPlot::new(Vec::new(), 0, 0, vec![0.5]);
        let theme = Theme::light();
        let g = plot.emit_graphics(&theme, Vec2::new(400.0, 400.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
