//! Polar coordinate chart — angular x + radial y. Renders a
//! wind-rose-style chart: concentric grid + radial spokes + one
//! filled radial sector per category.
//!
//! For richer polar marks (lines / points), the same coord
//! conversion `(θ, r) → (centre.x + r·cos θ, centre.y - r·sin θ)`
//! is exposed via [`PolarCoord::to_pixel`] so callers can emit
//! `wisp::Graphics` primitives on a polar layout without going
//! through this module's pre-baked bar variant.

use glam::Vec2;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// Polar coord system — centre + outer radius. Drop-in helper
/// for converting `(θ, r)` to pixel positions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolarCoord {
    /// Centre in pixel space.
    pub centre: Vec2,
    /// Outer radius (the chart's `r = 1.0` boundary).
    pub radius_px: f32,
}

impl PolarCoord {
    /// Project a polar `(angle_rad, r ∈ [0, 1])` to pixel space.
    /// `angle_rad = 0` points right (`+x`); positive angles go
    /// counter-clockwise. Screen `+y` is *down*, so `sin(θ)` is
    /// negated to keep the conventional "0 = right, π/2 = top"
    /// orientation.
    #[must_use]
    pub fn to_pixel(&self, angle_rad: f32, r_normalised: f32) -> Vec2 {
        let r = self.radius_px * r_normalised.clamp(0.0, 1.0);
        Vec2::new(
            self.centre.x + r * angle_rad.cos(),
            self.centre.y - r * angle_rad.sin(),
        )
    }
}

/// Polar-bar / wind-rose chart.
#[derive(Clone, Debug)]
pub struct PolarPlot {
    /// One label per angular sector. Sectors are evenly spaced
    /// around the circle, starting at angle `π/2` (top) and
    /// going clockwise — the conventional compass orientation.
    pub categories: Vec<String>,
    /// One value per category — drives the sector's outer radius
    /// (mapped via `value / max(values) * outer_radius`).
    pub values: Vec<f32>,
    /// Per-category fill colour. Cycled when shorter than
    /// `categories`.
    pub palette: Vec<ChartColor>,
}

impl PolarPlot {
    /// Construct with a default Wong palette (cycled).
    #[must_use]
    pub fn new(categories: Vec<String>, values: Vec<f32>) -> Self {
        let palette = vec![
            ChartColor::from_hex("#0072b2").unwrap(),
            ChartColor::from_hex("#d55e00").unwrap(),
            ChartColor::from_hex("#009e73").unwrap(),
            ChartColor::from_hex("#cc79a7").unwrap(),
            ChartColor::from_hex("#f0e442").unwrap(),
            ChartColor::from_hex("#56b4e9").unwrap(),
            ChartColor::from_hex("#e69f00").unwrap(),
            ChartColor::from_hex("#999999").unwrap(),
        ];
        Self {
            categories,
            values,
            palette,
        }
    }

    /// Override the palette.
    #[must_use]
    pub fn palette(mut self, palette: Vec<ChartColor>) -> Self {
        self.palette = palette;
        self
    }

    /// Emit grid + spokes + radial sectors as a `wisp::Graphics`.
    /// The chart centres on `viewport_px / 2` with outer radius
    /// `min(width, height) * 0.45` (leaves room for labels).
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let mut out = Graphics::new();
        let n = self.categories.len();
        if n < 2 || self.values.len() != n {
            return out;
        }
        let centre_px = viewport_px * 0.5;
        let radius_px = (viewport_px.x.min(viewport_px.y)) * 0.45;
        let coord = PolarCoord {
            centre: centre_px,
            radius_px,
        };
        let centre_ndc = pixel_to_ndc(centre_px, viewport_px);
        let r_outer_ndc = radius_px / viewport_px.y * 2.0;

        // Concentric gridlines at 25 / 50 / 75 / 100 %.
        let grid_color = chart_to_wisp(theme.plot.gridline_minor.color);
        out.fill(Fill::Solid(grid_color));
        for level in [0.25_f32, 0.5, 0.75, 1.0] {
            let r_ndc = r_outer_ndc * level;
            // Outline-only ring via annular_sector with a 1-px
            // band. Cheaper than tracing the polyline of the
            // circle.
            let outer = r_ndc + 0.5 / viewport_px.y * 2.0;
            let inner = r_ndc - 0.5 / viewport_px.y * 2.0;
            out.draw_annular_sector(centre_ndc, inner, outer, 0.0, std::f32::consts::TAU);
        }

        // Radial spokes — one per category boundary.
        let spoke_w_ndc = 1.0 / viewport_px.x * 2.0;
        for i in 0..n {
            let angle = sector_boundary_angle(i, n);
            let outer = coord.to_pixel(angle, 1.0);
            out.draw_line(centre_ndc, pixel_to_ndc(outer, viewport_px), spoke_w_ndc);
        }

        // Sectors.
        let max_value = self
            .values
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        if max_value.abs() < f32::EPSILON {
            return out;
        }
        for (i, value) in self.values.iter().enumerate() {
            let start = sector_boundary_angle(i, n);
            let end = sector_boundary_angle(i + 1, n);
            let (lo, hi) = if start <= end {
                (start, end)
            } else {
                // Wrap the last sector across the 2π seam.
                (start, end + std::f32::consts::TAU)
            };
            let r_norm = (value / max_value).clamp(0.0, 1.0);
            let r_ndc = r_outer_ndc * r_norm;
            let color = self.palette[i % self.palette.len()];
            out.fill(Fill::Solid(chart_to_wisp(color)));
            out.draw_annular_sector(centre_ndc, 0.0, r_ndc, lo, hi);
        }
        out
    }
}

/// Angle (radians) of the boundary BEFORE sector `i`. Sectors
/// are clockwise starting at the top (`π/2`), which is the
/// compass convention.
fn sector_boundary_angle(i: usize, n: usize) -> f32 {
    let step = std::f32::consts::TAU / usize_to_f32(n);
    // π/2 = top. Clockwise from the top means decreasing angle
    // in the CCW convention. So sector 0 starts at
    // `π/2 + step/2` (left edge of the top wedge).
    std::f32::consts::FRAC_PI_2 + step * 0.5 - step * usize_to_f32(i)
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
        reason = "category counts ≤ ~36 in practice (e.g. wind direction in 10° increments)"
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

    fn fixture() -> PolarPlot {
        PolarPlot::new(
            vec![
                "N".into(),
                "NE".into(),
                "E".into(),
                "SE".into(),
                "S".into(),
                "SW".into(),
                "W".into(),
                "NW".into(),
            ],
            vec![12.0, 18.0, 22.0, 30.0, 25.0, 16.0, 14.0, 8.0],
        )
    }

    #[test]
    fn polar_to_pixel_zero_angle_points_right() {
        let coord = PolarCoord {
            centre: Vec2::new(100.0, 100.0),
            radius_px: 50.0,
        };
        let p = coord.to_pixel(0.0, 1.0);
        assert!((p.x - 150.0).abs() < 1e-4);
        assert!((p.y - 100.0).abs() < 1e-4);
    }

    #[test]
    fn polar_to_pixel_pi_over_2_points_up() {
        let coord = PolarCoord {
            centre: Vec2::new(100.0, 100.0),
            radius_px: 50.0,
        };
        let p = coord.to_pixel(std::f32::consts::FRAC_PI_2, 1.0);
        assert!((p.x - 100.0).abs() < 1e-4);
        // Screen +y is down, so π/2 maps to (centre.y - radius) = 50.
        assert!((p.y - 50.0).abs() < 1e-4);
    }

    #[test]
    fn polar_emits_grid_plus_spokes_plus_sectors() {
        let theme = Theme::light();
        let g = fixture().emit_graphics(&theme, Vec2::new(320.0, 320.0));
        // 4 gridline rings + 8 spokes + 8 sectors = 20.
        assert_eq!(g.primitive_count(), 20);
    }

    #[test]
    fn polar_with_mismatched_lengths_emits_nothing() {
        let plot = PolarPlot::new(vec!["A".into(), "B".into()], vec![1.0, 2.0, 3.0]);
        let theme = Theme::light();
        let g = plot.emit_graphics(&theme, Vec2::new(300.0, 300.0));
        assert_eq!(g.primitive_count(), 0);
    }

    #[test]
    fn polar_with_one_category_emits_nothing() {
        let plot = PolarPlot::new(vec!["only".into()], vec![1.0]).palette(vec![red()]);
        let theme = Theme::light();
        let g = plot.emit_graphics(&theme, Vec2::new(300.0, 300.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
