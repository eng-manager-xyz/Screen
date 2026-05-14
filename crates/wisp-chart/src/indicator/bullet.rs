//! Bullet chart — Stephen Few's compact performance-vs-target
//! visualisation. A horizontal bar with three banded qualitative
//! ranges (poor / OK / good) behind it, a target line marker,
//! and the current value as a thinner foreground bar.

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// Orientation of the bullet's primary axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    /// Bar runs left-to-right; target marker is a vertical line.
    #[default]
    Horizontal,
    /// Bar runs bottom-to-top; target marker is a horizontal line.
    Vertical,
}

/// A bullet chart instance.
#[derive(Clone, Debug)]
pub struct Bullet {
    /// Current value to display as the foreground bar.
    pub value: f32,
    /// Target value to display as a contrasting marker line.
    pub target: f32,
    /// Three threshold values defining the
    /// `[poor, satisfactory, good]` qualitative ranges in
    /// ascending order. The total domain runs `0..ranges[2]`.
    pub ranges: [f32; 3],
    /// Horizontal or vertical layout.
    pub orientation: Orientation,
}

impl Bullet {
    /// Emit the qualitative ranges + value bar + target marker
    /// as a `wisp::Graphics`. The chart fills the supplied
    /// `viewport_px` with `8 px` padding.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let mut g = Graphics::new();
        let pad = 8.0_f32;
        let max = self.ranges[2].max(1e-3);
        let value_thickness_ratio = 0.4_f32; // value bar is 40% of band thickness

        match self.orientation {
            Orientation::Horizontal => {
                let plot_left = pad;
                let plot_right = viewport_px.x - pad;
                let plot_top = pad;
                let plot_bottom = viewport_px.y - pad;
                let band_h = plot_bottom - plot_top;
                let value_h = band_h * value_thickness_ratio;
                let value_top = plot_top + (band_h - value_h) * 0.5;

                let map_x = |v: f32| plot_left + (v / max) * (plot_right - plot_left);

                // Three qualitative bands, painted left → right.
                // Each band stretches from 0 to its range[i] top.
                let bands = [
                    (self.ranges[0], theme.indicator.bullet_poor_color),
                    (self.ranges[1], theme.indicator.bullet_ok_color),
                    (self.ranges[2], theme.indicator.bullet_good_color),
                ];
                let mut prev_x = plot_left;
                for (hi, color) in bands {
                    let hi_x = map_x(hi);
                    if hi_x > prev_x {
                        g.fill(Fill::Solid(chart_to_wisp(color)));
                        let r =
                            px_rect_to_ndc(prev_x, plot_top, hi_x - prev_x, band_h, viewport_px);
                        g.draw_rect(r);
                        prev_x = hi_x;
                    }
                }

                // Value bar (foreground, thinner).
                g.fill(Fill::Solid(chart_to_wisp(
                    theme.indicator.bullet_value_color,
                )));
                let value_w = map_x(self.value) - plot_left;
                let r = px_rect_to_ndc(plot_left, value_top, value_w, value_h, viewport_px);
                g.draw_rect(r);

                // Target marker — vertical line.
                g.fill(Fill::Solid(chart_to_wisp(
                    theme.indicator.bullet_target_color,
                )));
                let tx = map_x(self.target);
                let marker_w_ndc = 3.0 / viewport_px.x * 2.0;
                let a = pixel_to_ndc(Vec2::new(tx, plot_top + band_h * 0.15), viewport_px);
                let b = pixel_to_ndc(Vec2::new(tx, plot_top + band_h * 0.85), viewport_px);
                g.draw_line(a, b, marker_w_ndc);
            }
            Orientation::Vertical => {
                let plot_left = pad;
                let plot_right = viewport_px.x - pad;
                let plot_top = pad;
                let plot_bottom = viewport_px.y - pad;
                let band_w = plot_right - plot_left;
                let value_w = band_w * value_thickness_ratio;
                let value_left = plot_left + (band_w - value_w) * 0.5;

                let map_y = |v: f32| plot_bottom - (v / max) * (plot_bottom - plot_top);

                let bands = [
                    (self.ranges[0], theme.indicator.bullet_poor_color),
                    (self.ranges[1], theme.indicator.bullet_ok_color),
                    (self.ranges[2], theme.indicator.bullet_good_color),
                ];
                let mut prev_y = plot_bottom;
                for (hi, color) in bands {
                    let hi_y = map_y(hi);
                    if hi_y < prev_y {
                        g.fill(Fill::Solid(chart_to_wisp(color)));
                        let r = px_rect_to_ndc(plot_left, hi_y, band_w, prev_y - hi_y, viewport_px);
                        g.draw_rect(r);
                        prev_y = hi_y;
                    }
                }

                g.fill(Fill::Solid(chart_to_wisp(
                    theme.indicator.bullet_value_color,
                )));
                let value_top = map_y(self.value);
                let r = px_rect_to_ndc(
                    value_left,
                    value_top,
                    value_w,
                    plot_bottom - value_top,
                    viewport_px,
                );
                g.draw_rect(r);

                g.fill(Fill::Solid(chart_to_wisp(
                    theme.indicator.bullet_target_color,
                )));
                let ty = map_y(self.target);
                let marker_w_ndc = 3.0 / viewport_px.y * 2.0;
                let a = pixel_to_ndc(Vec2::new(plot_left + band_w * 0.15, ty), viewport_px);
                let b = pixel_to_ndc(Vec2::new(plot_left + band_w * 0.85, ty), viewport_px);
                g.draw_line(a, b, marker_w_ndc);
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

fn px_rect_to_ndc(x: f32, y: f32, w: f32, h: f32, viewport_px: Vec2) -> Rect {
    let nx = x / viewport_px.x * 2.0 - 1.0;
    let ny = 1.0 - (y + h) / viewport_px.y * 2.0;
    let nw = w / viewport_px.x * 2.0;
    let nh = h / viewport_px.y * 2.0;
    Rect::new(nx, ny, nw, nh)
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

    fn fixture() -> Bullet {
        Bullet {
            value: 270.0,
            target: 250.0,
            ranges: [150.0, 225.0, 300.0],
            orientation: Orientation::Horizontal,
        }
    }

    #[test]
    fn horizontal_emits_3_bands_plus_value_plus_target() {
        let theme = Theme::light();
        let g = fixture().emit_graphics(&theme, Vec2::new(400.0, 80.0));
        // 3 qualitative bands + 1 value bar + 1 target line = 5.
        assert_eq!(g.primitive_count(), 5);
    }

    #[test]
    fn vertical_orientation_emits_5_primitives() {
        let mut b = fixture();
        b.orientation = Orientation::Vertical;
        let theme = Theme::light();
        let g = b.emit_graphics(&theme, Vec2::new(80.0, 400.0));
        assert_eq!(g.primitive_count(), 5);
    }

    #[test]
    fn zero_value_still_emits_3_bands_and_marker() {
        let mut b = fixture();
        b.value = 0.0;
        let theme = Theme::light();
        let g = b.emit_graphics(&theme, Vec2::new(400.0, 80.0));
        // Zero-width value rect still counts as one primitive
        // (renderer's choice; some bullet libs skip it).
        assert_eq!(g.primitive_count(), 5);
    }
}
