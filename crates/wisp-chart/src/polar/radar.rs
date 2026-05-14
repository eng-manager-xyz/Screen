//! Radar chart — multi-axis polygon overlay for multivariate
//! comparison. Each row of axes gets a polygon connecting the
//! per-axis values across the polygon's vertices.

use glam::Vec2;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// One axis of a radar chart — a labelled dimension with its
/// own value range.
#[derive(Clone, Debug, PartialEq)]
pub struct RadarAxis {
    /// Axis label.
    pub label: String,
    /// `(min, max)` domain.
    pub domain: (f32, f32),
}

impl RadarAxis {
    /// Construct from a label + domain.
    #[must_use]
    pub fn new(label: impl Into<String>, domain: (f32, f32)) -> Self {
        Self {
            label: label.into(),
            domain,
        }
    }
}

/// One series rendered as a polygon over the axes.
#[derive(Clone, Debug, PartialEq)]
pub struct RadarSeries {
    /// Series label (legend).
    pub label: String,
    /// Per-axis value — must have the same length as the chart's
    /// `axes` vec.
    pub values: Vec<f32>,
    /// Polygon fill colour.
    pub color: ChartColor,
}

impl RadarSeries {
    /// Construct from a label + values + colour.
    #[must_use]
    pub fn new(label: impl Into<String>, values: Vec<f32>, color: ChartColor) -> Self {
        Self {
            label: label.into(),
            values,
            color,
        }
    }
}

/// A radar chart.
#[derive(Clone, Debug)]
pub struct Radar {
    /// Axes in order. Render winds CCW from `+y` (top).
    pub axes: Vec<RadarAxis>,
    /// Series — one polygon per entry.
    pub series: Vec<RadarSeries>,
}

impl Radar {
    /// Construct from axes + series.
    #[must_use]
    pub fn new(axes: Vec<RadarAxis>, series: Vec<RadarSeries>) -> Self {
        Self { axes, series }
    }

    /// Emit grid (concentric polygons) + each series polygon as
    /// a `wisp::Graphics`.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let mut g = Graphics::new();
        if self.axes.len() < 3 {
            return g;
        }
        let centre_px = viewport_px * 0.5;
        let radius_px = (viewport_px.x.min(viewport_px.y)) * 0.4;
        let n = self.axes.len();
        // Wedge angles — start at top (`+π/2`), CCW.
        let angle_for = |i: usize| {
            std::f32::consts::FRAC_PI_2 - std::f32::consts::TAU * usize_to_f32(i) / usize_to_f32(n)
        };
        let axis_point = |i: usize, t: f32| {
            let a = angle_for(i);
            let r = radius_px * t;
            Vec2::new(centre_px.x + r * a.cos(), centre_px.y - r * a.sin())
        };

        // Gridlines — 4 concentric polygons at 25/50/75/100%.
        let grid_color = chart_to_wisp(theme.plot.gridline_minor.color);
        let grid_w_ndc = theme.plot.gridline_minor.width / viewport_px.y * 2.0;
        for level in [0.25_f32, 0.5, 0.75, 1.0] {
            g.fill(Fill::Solid(grid_color));
            for i in 0..n {
                let a = axis_point(i, level);
                let b = axis_point((i + 1) % n, level);
                g.draw_line(
                    pixel_to_ndc(a, viewport_px),
                    pixel_to_ndc(b, viewport_px),
                    grid_w_ndc,
                );
            }
        }

        // Per-axis spoke from centre outward.
        for i in 0..n {
            let p = axis_point(i, 1.0);
            g.draw_line(
                pixel_to_ndc(centre_px, viewport_px),
                pixel_to_ndc(p, viewport_px),
                grid_w_ndc,
            );
        }

        // Series polygons.
        for s in &self.series {
            if s.values.len() != n {
                continue;
            }
            let mut verts = Vec::with_capacity(n);
            for (i, axis) in self.axes.iter().enumerate() {
                let (lo, hi) = axis.domain;
                let span = (hi - lo).max(f32::EPSILON);
                let t = ((s.values[i] - lo) / span).clamp(0.0, 1.0);
                let p = axis_point(i, t);
                verts.push(pixel_to_ndc(p, viewport_px));
            }
            g.fill(Fill::Solid(chart_to_wisp(s.color)));
            // Polygon is convex-only in wisp v1, but a radar polygon
            // with all positive radii from a common centre is always
            // star-convex — safe for fan triangulation.
            g.draw_polygon(&verts);
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
        reason = "radar axis counts ≤ ~12 in practice; well within f32 mantissa"
    )]
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

    fn fixture() -> Radar {
        Radar::new(
            vec![
                RadarAxis::new("speed", (0.0, 100.0)),
                RadarAxis::new("range", (0.0, 100.0)),
                RadarAxis::new("comfort", (0.0, 100.0)),
                RadarAxis::new("efficiency", (0.0, 100.0)),
                RadarAxis::new("price", (0.0, 100.0)),
            ],
            vec![
                RadarSeries::new("A", vec![80.0, 70.0, 60.0, 90.0, 50.0], red()),
                RadarSeries::new("B", vec![60.0, 85.0, 75.0, 70.0, 80.0], green()),
            ],
        )
    }

    #[test]
    fn radar_emits_grid_plus_spokes_plus_polygons() {
        let theme = Theme::light();
        let g = fixture().emit_graphics(&theme, Vec2::new(300.0, 300.0));
        // 4 gridline levels * 5 sides + 5 spokes + 2 series polygons = 27.
        assert_eq!(g.primitive_count(), 27);
    }

    #[test]
    fn radar_with_too_few_axes_emits_nothing() {
        let r = Radar::new(
            vec![RadarAxis::new("only", (0.0, 1.0))],
            vec![RadarSeries::new("s", vec![1.0], red())],
        );
        let theme = Theme::light();
        let g = r.emit_graphics(&theme, Vec2::new(300.0, 300.0));
        assert_eq!(g.primitive_count(), 0);
    }

    #[test]
    fn radar_skips_series_with_wrong_value_count() {
        let r = Radar::new(
            vec![
                RadarAxis::new("a", (0.0, 1.0)),
                RadarAxis::new("b", (0.0, 1.0)),
                RadarAxis::new("c", (0.0, 1.0)),
            ],
            vec![
                RadarSeries::new("ok", vec![0.5, 0.6, 0.7], red()),
                RadarSeries::new("bad", vec![0.5], green()),
            ],
        );
        let theme = Theme::light();
        let g = r.emit_graphics(&theme, Vec2::new(300.0, 300.0));
        // 4 grid levels * 3 sides + 3 spokes + 1 valid polygon = 16.
        assert_eq!(g.primitive_count(), 16);
    }
}
