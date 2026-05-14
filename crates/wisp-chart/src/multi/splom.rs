//! Scatterplot matrix (SPLOM) — N×N grid of pairwise mini
//! scatters. v1 renders off-diagonal cells as point clouds and
//! leaves the diagonal blank (a future ticket replaces the
//! diagonal with per-dim histograms / density plots).

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// One dimension of a SPLOM.
#[derive(Clone, Debug, PartialEq)]
pub struct SplomDimension {
    /// Display label.
    pub label: String,
    /// Per-row values — same length across all dimensions.
    pub values: Vec<f32>,
}

impl SplomDimension {
    /// Construct from a label + values.
    #[must_use]
    pub fn new(label: impl Into<String>, values: Vec<f32>) -> Self {
        Self {
            label: label.into(),
            values,
        }
    }

    fn extent(&self) -> (f32, f32) {
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for v in &self.values {
            lo = lo.min(*v);
            hi = hi.max(*v);
        }
        if lo.is_infinite() {
            (0.0, 1.0)
        } else {
            (lo, hi)
        }
    }
}

/// SPLOM value type.
#[derive(Clone, Debug)]
pub struct Splom {
    /// Dimensions — must share the same `values.len()`.
    pub dims: Vec<SplomDimension>,
    /// Marker colour.
    pub point_color: ChartColor,
    /// Marker radius in pixels.
    pub point_radius_px: f32,
}

impl Splom {
    /// Construct with default Wong-navy points + 2 px radius.
    #[must_use]
    pub fn new(dims: Vec<SplomDimension>) -> Self {
        Self {
            dims,
            point_color: ChartColor::from_hex("#0072b2").unwrap(),
            point_radius_px: 2.0,
        }
    }

    /// Emit one off-diagonal scatter per `(row_dim, col_dim)`
    /// cell + cell-grid borders.
    #[must_use]
    #[allow(
        clippy::many_single_char_names,
        reason = "geometry code with conventional short names — x/y for pixel coords, r for row, n for dim count. Renaming all of them would obscure the layout math."
    )]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let mut g = Graphics::new();
        let n = self.dims.len();
        if n < 2 {
            return g;
        }
        let pad = 16.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let cell_w = (plot_right - plot_left) / usize_to_f32(n);
        let cell_h = (plot_bottom - plot_top) / usize_to_f32(n);

        // Border grid.
        let border_color = chart_to_wisp(theme.plot.gridline_minor.color);
        let border_w_ndc = 1.0 / viewport_px.y * 2.0;
        for i in 0..=n {
            let x = plot_left + usize_to_f32(i) * cell_w;
            g.fill(Fill::Solid(border_color));
            let a = pixel_to_ndc(Vec2::new(x, plot_top), viewport_px);
            let b = pixel_to_ndc(Vec2::new(x, plot_bottom), viewport_px);
            g.draw_line(a, b, border_w_ndc);
            let y = plot_top + usize_to_f32(i) * cell_h;
            let a = pixel_to_ndc(Vec2::new(plot_left, y), viewport_px);
            let b = pixel_to_ndc(Vec2::new(plot_right, y), viewport_px);
            g.draw_line(a, b, border_w_ndc);
        }

        // Off-diagonal scatters.
        let radii_ndc = Vec2::new(
            self.point_radius_px / viewport_px.x * 2.0,
            self.point_radius_px / viewport_px.y * 2.0,
        );
        let fill_color = chart_to_wisp(self.point_color);
        let row_count = self.dims[0].values.len();
        for row_dim in 0..n {
            for col_dim in 0..n {
                if row_dim == col_dim {
                    continue;
                }
                let (col_lo, col_hi) = self.dims[col_dim].extent();
                let (row_lo, row_hi) = self.dims[row_dim].extent();
                let col_span = (col_hi - col_lo).max(f32::EPSILON);
                let row_span = (row_hi - row_lo).max(f32::EPSILON);
                let cell_x = plot_left + usize_to_f32(col_dim) * cell_w;
                let cell_y = plot_top + usize_to_f32(row_dim) * cell_h;
                let pad_inner = 4.0;
                g.fill(Fill::Solid(fill_color));
                for r in 0..row_count {
                    if r >= self.dims[col_dim].values.len() || r >= self.dims[row_dim].values.len()
                    {
                        continue;
                    }
                    let xv = self.dims[col_dim].values[r];
                    let yv = self.dims[row_dim].values[r];
                    let nx = (xv - col_lo) / col_span;
                    let ny = (yv - row_lo) / row_span;
                    let px = cell_x + pad_inner + nx * (cell_w - pad_inner * 2.0);
                    let py = cell_y + cell_h - pad_inner - ny * (cell_h - pad_inner * 2.0);
                    let centre = pixel_to_ndc(Vec2::new(px, py), viewport_px);
                    g.draw_ellipse(centre, radii_ndc);
                }
            }
        }
        g
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
        reason = "SPLOM dim counts ≤ ~10 in practice"
    )]
    {
        v as f32
    }
}

#[allow(dead_code)]
const _RECT_ANCHOR: Option<Rect> = None;

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Splom {
        Splom::new(vec![
            SplomDimension::new("mpg", vec![32.0, 22.0, 18.0, 14.0]),
            SplomDimension::new("cyl", vec![4.0, 6.0, 6.0, 8.0]),
            SplomDimension::new("hp", vec![95.0, 150.0, 200.0, 280.0]),
            SplomDimension::new("wt", vec![2.2, 3.0, 3.6, 4.4]),
        ])
    }

    #[test]
    fn splom_emits_cell_borders_plus_per_off_diagonal_points() {
        let theme = Theme::light();
        let g = fixture().emit_graphics(&theme, Vec2::new(400.0, 400.0));
        let n = 4_usize;
        // Borders: (n+1) vert + (n+1) horiz = 10.
        // Off-diagonal cells: n*n - n = 12; each draws 4 ellipses.
        // Total: 10 + 12*4 = 58.
        assert_eq!(g.primitive_count(), 10 + (n * n - n) * 4);
    }

    #[test]
    fn splom_with_one_dim_emits_nothing() {
        let s = Splom::new(vec![SplomDimension::new("a", vec![1.0, 2.0])]);
        let theme = Theme::light();
        let g = s.emit_graphics(&theme, Vec2::new(400.0, 400.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
