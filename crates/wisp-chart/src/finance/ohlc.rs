//! OHLC bar chart — thin vertical range line per period + two
//! small horizontal ticks for open (left) and close (right).

use glam::Vec2;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::finance::candlestick::Period;
use crate::theme::Theme;

/// OHLC chart — periods share `Period` with the candlestick
/// variant. Only the visual encoding differs.
#[derive(Clone, Debug)]
pub struct Ohlc {
    /// Periods in left-to-right order.
    pub periods: Vec<Period>,
    /// Fill colour for up periods (close >= open).
    pub up_color: ChartColor,
    /// Fill colour for down periods.
    pub down_color: ChartColor,
    /// Open / close tick length as fraction of band width
    /// (`0.0..1.0`). Default `0.3`.
    pub tick_length_fraction: f32,
}

impl Ohlc {
    /// Construct with default colours + 30% tick length.
    #[must_use]
    pub fn new(periods: Vec<Period>) -> Self {
        Self {
            periods,
            up_color: ChartColor::from_hex("#27ae60").unwrap(),
            down_color: ChartColor::from_hex("#e74c3c").unwrap(),
            tick_length_fraction: 0.3,
        }
    }

    /// Emit one range line + two ticks per period.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        if self.periods.is_empty() {
            return g;
        }
        let pad = 16.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let plot_w = plot_right - plot_left;
        let plot_h = plot_bottom - plot_top;
        let n = self.periods.len();
        let band_w = plot_w / usize_to_f32(n);
        let tick_w = band_w * self.tick_length_fraction;

        let lo = self
            .periods
            .iter()
            .map(|p| p.low)
            .fold(f32::INFINITY, f32::min);
        let hi = self
            .periods
            .iter()
            .map(|p| p.high)
            .fold(f32::NEG_INFINITY, f32::max);
        let span = (hi - lo).max(f32::EPSILON);
        let map_y = |v: f32| plot_bottom - (v - lo) / span * plot_h;

        let line_w_ndc = 1.5 / viewport_px.x * 2.0;

        for (i, p) in self.periods.iter().enumerate() {
            let centre_x = plot_left + (usize_to_f32(i) + 0.5) * band_w;
            let color = if p.is_up() {
                self.up_color
            } else {
                self.down_color
            };
            g.fill(Fill::Solid(chart_to_wisp(color)));
            // Vertical range line.
            let a = pixel_to_ndc(Vec2::new(centre_x, map_y(p.low)), viewport_px);
            let b = pixel_to_ndc(Vec2::new(centre_x, map_y(p.high)), viewport_px);
            g.draw_line(a, b, line_w_ndc);
            // Open tick — left.
            let oa = pixel_to_ndc(Vec2::new(centre_x - tick_w, map_y(p.open)), viewport_px);
            let ob = pixel_to_ndc(Vec2::new(centre_x, map_y(p.open)), viewport_px);
            g.draw_line(oa, ob, line_w_ndc);
            // Close tick — right.
            let ca = pixel_to_ndc(Vec2::new(centre_x, map_y(p.close)), viewport_px);
            let cb = pixel_to_ndc(Vec2::new(centre_x + tick_w, map_y(p.close)), viewport_px);
            g.draw_line(ca, cb, line_w_ndc);
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
        reason = "period counts ≤ ~250 in practice"
    )]
    {
        v as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Ohlc {
        Ohlc::new(vec![
            Period::new(100.0, 110.0, 95.0, 108.0),
            Period::new(108.0, 115.0, 105.0, 102.0),
            Period::new(102.0, 109.0, 100.0, 107.0),
        ])
    }

    #[test]
    fn ohlc_emits_three_lines_per_period() {
        let theme = Theme::light();
        let g = fixture().emit_graphics(&theme, Vec2::new(400.0, 240.0));
        // 3 periods × (range + open-tick + close-tick) = 9.
        assert_eq!(g.primitive_count(), 9);
    }

    #[test]
    fn empty_ohlc_emits_no_primitives() {
        let theme = Theme::light();
        let g = Ohlc::new(vec![]).emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
