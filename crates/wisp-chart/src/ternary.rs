//! Ternary plot — show 3-component compositional data on an
//! equilateral triangle. Each point's position uniquely encodes
//! all three component ratios (which must sum to 1.0; the
//! constructor normalises).

use glam::Vec2;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// One ternary point — three component values + colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TernaryPoint {
    /// Bottom-left vertex component.
    pub a: f32,
    /// Bottom-right vertex component.
    pub b: f32,
    /// Top vertex component.
    pub c: f32,
    /// Marker colour.
    pub color: ChartColor,
}

impl TernaryPoint {
    /// Construct with the components normalised so they sum to 1.
    /// Non-positive sums fall back to `(1, 0, 0)`.
    #[must_use]
    pub fn new(a: f32, b: f32, c: f32, color: ChartColor) -> Self {
        let total = a + b + c;
        let (a, b, c) = if total > f32::EPSILON {
            (a / total, b / total, c / total)
        } else {
            (1.0, 0.0, 0.0)
        };
        Self { a, b, c, color }
    }
}

/// Ternary plot value type.
#[derive(Clone, Debug)]
pub struct TernaryPlot {
    /// Component-A label (bottom-left vertex).
    pub label_a: String,
    /// Component-B label (bottom-right vertex).
    pub label_b: String,
    /// Component-C label (top vertex).
    pub label_c: String,
    /// Points.
    pub points: Vec<TernaryPoint>,
    /// Marker radius in pixels.
    pub point_radius_px: f32,
}

impl TernaryPlot {
    /// Construct with default 4-px marker radius.
    #[must_use]
    pub fn new(
        label_a: impl Into<String>,
        label_b: impl Into<String>,
        label_c: impl Into<String>,
        points: Vec<TernaryPoint>,
    ) -> Self {
        Self {
            label_a: label_a.into(),
            label_b: label_b.into(),
            label_c: label_c.into(),
            points,
            point_radius_px: 4.0,
        }
    }

    /// Emit the triangle outline + grid + per-point ellipses.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let mut g = Graphics::new();
        let pad = 24.0_f32;
        let centre_x = viewport_px.x * 0.5;
        let max_w = viewport_px.x - pad * 2.0;
        let max_h = viewport_px.y - pad * 2.0;
        let side = max_w.min(max_h / (3.0_f32.sqrt() * 0.5));
        let height = side * 3.0_f32.sqrt() * 0.5;
        let bottom_y = pad + f32::midpoint(max_h, height);
        let top_y = bottom_y - height;
        let v_a = Vec2::new(centre_x - side * 0.5, bottom_y);
        let v_b = Vec2::new(centre_x + side * 0.5, bottom_y);
        let v_c = Vec2::new(centre_x, top_y);

        // Triangle outline.
        let outline_color = chart_to_wisp(theme.text_primary);
        let outline_w = 1.5 / viewport_px.x * 2.0;
        g.fill(Fill::Solid(outline_color));
        let edge = |a: Vec2, b: Vec2, out: &mut Graphics| {
            out.draw_line(
                pixel_to_ndc(a, viewport_px),
                pixel_to_ndc(b, viewport_px),
                outline_w,
            );
        };
        edge(v_a, v_b, &mut g);
        edge(v_b, v_c, &mut g);
        edge(v_c, v_a, &mut g);

        // Internal gridlines at 25 / 50 / 75 %.
        let grid_color = chart_to_wisp(theme.plot.gridline_minor.color);
        let grid_w = 0.5 / viewport_px.x * 2.0;
        g.fill(Fill::Solid(grid_color));
        for level in [0.25_f32, 0.5, 0.75] {
            // For each vertex pair, draw a line parallel to the
            // opposite edge at this fraction.
            let lerp = |from: Vec2, to: Vec2, t: f32| {
                Vec2::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t)
            };
            // Constant-A line: parallel to BC.
            let pa1 = lerp(v_a, v_b, level);
            let pa2 = lerp(v_a, v_c, level);
            edge(pa1, pa2, &mut g);
            // Constant-B line: parallel to AC.
            let pb1 = lerp(v_b, v_a, level);
            let pb2 = lerp(v_b, v_c, level);
            edge(pb1, pb2, &mut g);
            // Constant-C line: parallel to AB.
            let pc1 = lerp(v_c, v_a, level);
            let pc2 = lerp(v_c, v_b, level);
            edge(pc1, pc2, &mut g);
            // Silence unused warning if grid_w isn't used.
            let _ = grid_w;
        }

        // Points.
        let radii_ndc = Vec2::new(
            self.point_radius_px / viewport_px.x * 2.0,
            self.point_radius_px / viewport_px.y * 2.0,
        );
        for p in &self.points {
            // Barycentric to cartesian.
            let point_px = Vec2::new(
                v_a.x * p.a + v_b.x * p.b + v_c.x * p.c,
                v_a.y * p.a + v_b.y * p.b + v_c.y * p.c,
            );
            g.fill(Fill::Solid(chart_to_wisp(p.color)));
            g.draw_ellipse(pixel_to_ndc(point_px, viewport_px), radii_ndc);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn red() -> ChartColor {
        ChartColor::from_hex("#e74c3c").unwrap()
    }

    #[test]
    fn ternary_point_normalises_components_to_sum_one() {
        let p = TernaryPoint::new(2.0, 3.0, 5.0, red());
        let total = p.a + p.b + p.c;
        assert!((total - 1.0).abs() < 1e-5);
        assert!((p.a - 0.2).abs() < 1e-5);
        assert!((p.b - 0.3).abs() < 1e-5);
        assert!((p.c - 0.5).abs() < 1e-5);
    }

    #[test]
    fn ternary_emits_triangle_plus_grid_plus_per_point() {
        let plot = TernaryPlot::new(
            "Sand",
            "Silt",
            "Clay",
            vec![
                TernaryPoint::new(0.5, 0.3, 0.2, red()),
                TernaryPoint::new(0.2, 0.3, 0.5, red()),
            ],
        );
        let theme = Theme::light();
        let g = plot.emit_graphics(&theme, Vec2::new(360.0, 360.0));
        // 3 outline + 3*3 grid lines + 2 points = 14.
        assert_eq!(g.primitive_count(), 3 + 9 + 2);
    }

    #[test]
    fn ternary_point_zero_input_falls_back_to_first_corner() {
        let p = TernaryPoint::new(0.0, 0.0, 0.0, red());
        assert!((p.a - 1.0).abs() < 1e-5);
    }
}
