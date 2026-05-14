//! Parallel coordinates plot — N vertical axes (one per
//! dimension), one polyline per row connecting per-dim values.

use glam::Vec2;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// One axis (one dimension) in a parallel-coords chart.
#[derive(Clone, Debug, PartialEq)]
pub struct ParallelAxis {
    /// Display label.
    pub label: String,
    /// `(min, max)` domain — each axis carries its own scale.
    pub domain: (f32, f32),
}

impl ParallelAxis {
    /// Construct from a label + domain.
    #[must_use]
    pub fn new(label: impl Into<String>, domain: (f32, f32)) -> Self {
        Self {
            label: label.into(),
            domain,
        }
    }
}

/// One row of values across all axes.
#[derive(Clone, Debug, PartialEq)]
pub struct ParallelRow {
    /// Per-axis value — same length as the chart's `axes`.
    pub values: Vec<f32>,
    /// Polyline colour.
    pub color: ChartColor,
}

impl ParallelRow {
    /// Construct from values + colour.
    #[must_use]
    pub fn new(values: Vec<f32>, color: ChartColor) -> Self {
        Self { values, color }
    }
}

/// Parallel-coords chart.
#[derive(Clone, Debug)]
pub struct ParallelCoords {
    /// Axes in left-to-right order.
    pub axes: Vec<ParallelAxis>,
    /// Rows — one polyline each.
    pub rows: Vec<ParallelRow>,
}

impl ParallelCoords {
    /// Construct from axes + rows.
    #[must_use]
    pub const fn new(axes: Vec<ParallelAxis>, rows: Vec<ParallelRow>) -> Self {
        Self { axes, rows }
    }

    /// Emit axes (vertical lines) + per-row polylines as a
    /// `wisp::Graphics`.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut out = Graphics::new();
        if self.axes.len() < 2 {
            return out;
        }
        let pad = 16.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let plot_w = plot_right - plot_left;
        let plot_h = plot_bottom - plot_top;
        let n_axes = self.axes.len();
        let axis_x = |i: usize| plot_left + usize_to_f32(i) * plot_w / usize_to_f32(n_axes - 1);
        let map_y = |axis: &ParallelAxis, value: f32| {
            let (lo, hi) = axis.domain;
            let span = (hi - lo).max(f32::EPSILON);
            let normed = ((value - lo) / span).clamp(0.0, 1.0);
            plot_bottom - normed * plot_h
        };

        // Axes.
        let axis_w_ndc = 1.0 / viewport_px.x * 2.0;
        let axis_color = chart_to_wisp(theme.text_muted);
        out.fill(Fill::Solid(axis_color));
        for axis_idx in 0..n_axes {
            let px = axis_x(axis_idx);
            let top_ndc = pixel_to_ndc(Vec2::new(px, plot_top), viewport_px);
            let bot_ndc = pixel_to_ndc(Vec2::new(px, plot_bottom), viewport_px);
            out.draw_line(top_ndc, bot_ndc, axis_w_ndc);
        }

        // Polylines.
        let line_w_ndc = 1.0 / viewport_px.x * 2.0;
        for row in &self.rows {
            if row.values.len() != n_axes {
                continue;
            }
            out.fill(Fill::Solid(chart_to_wisp(row.color)));
            for seg in 0..(n_axes - 1) {
                let x0 = axis_x(seg);
                let x1 = axis_x(seg + 1);
                let y0 = map_y(&self.axes[seg], row.values[seg]);
                let y1 = map_y(&self.axes[seg + 1], row.values[seg + 1]);
                let p0 = pixel_to_ndc(Vec2::new(x0, y0), viewport_px);
                let p1 = pixel_to_ndc(Vec2::new(x1, y1), viewport_px);
                out.draw_line(p0, p1, line_w_ndc);
            }
        }
        out
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
    #[allow(clippy::cast_precision_loss, reason = "axis counts ≤ ~20 in practice")]
    {
        v as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red() -> ChartColor {
        ChartColor::from_hex("#e74c3c").unwrap()
    }
    fn green() -> ChartColor {
        ChartColor::from_hex("#27ae60").unwrap()
    }

    fn fixture() -> ParallelCoords {
        ParallelCoords::new(
            vec![
                ParallelAxis::new("mpg", (10.0, 50.0)),
                ParallelAxis::new("cyl", (3.0, 8.0)),
                ParallelAxis::new("hp", (60.0, 300.0)),
                ParallelAxis::new("wt", (1.5, 5.5)),
            ],
            vec![
                ParallelRow::new(vec![25.0, 4.0, 110.0, 2.8], red()),
                ParallelRow::new(vec![18.0, 6.0, 180.0, 3.5], green()),
            ],
        )
    }

    #[test]
    fn parallel_emits_axes_plus_per_row_polyline_segments() {
        let theme = Theme::light();
        let g = fixture().emit_graphics(&theme, Vec2::new(400.0, 240.0));
        // 4 axes + 2 rows × 3 segments = 4 + 6 = 10.
        assert_eq!(g.primitive_count(), 10);
    }

    #[test]
    fn parallel_with_too_few_axes_emits_nothing() {
        let pc = ParallelCoords::new(
            vec![ParallelAxis::new("only", (0.0, 1.0))],
            vec![ParallelRow::new(vec![0.5], red())],
        );
        let theme = Theme::light();
        let g = pc.emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 0);
    }

    #[test]
    fn parallel_skips_rows_with_wrong_value_count() {
        let pc = ParallelCoords::new(
            vec![
                ParallelAxis::new("a", (0.0, 1.0)),
                ParallelAxis::new("b", (0.0, 1.0)),
                ParallelAxis::new("c", (0.0, 1.0)),
            ],
            vec![
                ParallelRow::new(vec![0.1, 0.5, 0.9], red()),
                ParallelRow::new(vec![0.2], green()),
            ],
        );
        let theme = Theme::light();
        let g = pc.emit_graphics(&theme, Vec2::new(400.0, 240.0));
        // 3 axes + 1 valid row × 2 segments = 5.
        assert_eq!(g.primitive_count(), 5);
    }
}
