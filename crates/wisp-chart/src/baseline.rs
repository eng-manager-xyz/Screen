//! Baseline chart — area chart split by a horizontal reference
//! value. Fill above the baseline in one colour (profit), below
//! in another (loss).

use glam::Vec2;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// Baseline chart.
#[derive(Clone, Debug)]
pub struct BaselineChart {
    /// `(x, y)` points in left-to-right order.
    pub points: Vec<(f32, f32)>,
    /// Reference `y` value that splits above / below fills.
    pub baseline: f32,
    /// Fill colour for the region where `y > baseline`.
    pub above_color: ChartColor,
    /// Fill colour for the region where `y < baseline`.
    pub below_color: ChartColor,
}

impl BaselineChart {
    /// Construct with default green-up / red-down colours.
    #[must_use]
    pub fn new(points: Vec<(f32, f32)>, baseline: f32) -> Self {
        Self {
            points,
            baseline,
            above_color: ChartColor::from_hex("#27ae60").unwrap(),
            below_color: ChartColor::from_hex("#e74c3c").unwrap(),
        }
    }

    /// Emit one quad per segment, coloured by which side of the
    /// baseline the segment's average sits on.
    ///
    /// Convex-quad-per-segment instead of one big polygon —
    /// wisp's `draw_polygon` is convex-only in v1, and an area
    /// split by a baseline produces non-convex shapes.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        self.emit_with_interaction(theme, viewport_px).graphics
    }

    /// Like [`emit_graphics`](Self::emit_graphics) but returns the
    /// reverse-lookup table — each polygon maps to
    /// [`ChartElementId::Bar`](crate::interaction::ChartElementId::Bar)
    /// keyed by the segment index `0..self.points.len()-1`.
    #[must_use]
    pub fn emit_with_interaction(
        &self,
        theme: &Theme,
        viewport_px: Vec2,
    ) -> crate::interaction::EmittedChart {
        let _ = theme;
        let mut g = Graphics::new();
        let mut elements: Vec<(usize, crate::interaction::ChartElementId)> = Vec::new();
        if self.points.len() < 2 {
            return crate::interaction::EmittedChart {
                graphics: g,
                elements,
            };
        }
        let pad = 16.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let plot_w = plot_right - plot_left;
        let plot_h = plot_bottom - plot_top;

        let x_lo = self
            .points
            .iter()
            .map(|(x, _)| *x)
            .fold(f32::INFINITY, f32::min);
        let x_hi = self
            .points
            .iter()
            .map(|(x, _)| *x)
            .fold(f32::NEG_INFINITY, f32::max);
        let mut y_lo = self.baseline;
        let mut y_hi = self.baseline;
        for (_, y) in &self.points {
            y_lo = y_lo.min(*y);
            y_hi = y_hi.max(*y);
        }
        let x_span = (x_hi - x_lo).max(f32::EPSILON);
        let y_span = (y_hi - y_lo).max(f32::EPSILON);
        let map_x = |x: f32| plot_left + (x - x_lo) / x_span * plot_w;
        let map_y = |y: f32| plot_bottom - (y - y_lo) / y_span * plot_h;
        let by = map_y(self.baseline);

        for (seg_idx, pair) in self.points.windows(2).enumerate() {
            let (x0, y0) = pair[0];
            let (x1, y1) = pair[1];
            let px0 = map_x(x0);
            let px1 = map_x(x1);
            let py0 = map_y(y0);
            let py1 = map_y(y1);
            // Colour by average side of baseline.
            let mean = f32::midpoint(y0, y1);
            let color = if mean >= self.baseline {
                self.above_color
            } else {
                self.below_color
            };
            g.fill(Fill::Solid(chart_to_wisp(color)));
            let bottom_left = pixel_to_ndc(Vec2::new(px0, by), viewport_px);
            let bottom_right = pixel_to_ndc(Vec2::new(px1, by), viewport_px);
            let top_right = pixel_to_ndc(Vec2::new(px1, py1), viewport_px);
            let top_left = pixel_to_ndc(Vec2::new(px0, py0), viewport_px);
            // CCW winding — convex by construction because the
            // top and bottom edges share x-extents.
            g.draw_polygon(&[bottom_left, bottom_right, top_right, top_left]);
            elements.push((
                g.primitive_count() - 1,
                crate::interaction::ChartElementId::Bar(seg_idx),
            ));
        }
        crate::interaction::EmittedChart {
            graphics: g,
            elements,
        }
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

    #[test]
    fn baseline_emits_one_quad_per_segment() {
        let bc = BaselineChart::new(vec![(0.0, 10.0), (1.0, 5.0), (2.0, -5.0), (3.0, 15.0)], 0.0);
        let theme = Theme::light();
        let g = bc.emit_graphics(&theme, Vec2::new(400.0, 240.0));
        // 4 points → 3 segments.
        assert_eq!(g.primitive_count(), 3);
    }

    #[test]
    fn baseline_with_one_point_emits_nothing() {
        let bc = BaselineChart::new(vec![(0.0, 5.0)], 0.0);
        let theme = Theme::light();
        let g = bc.emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
