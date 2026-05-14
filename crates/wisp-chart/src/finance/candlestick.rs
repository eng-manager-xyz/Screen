//! Candlestick chart — OHLC price per period as a body
//! (open → close) + wick (low → high).

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// One OHLC period.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Period {
    /// Opening price.
    pub open: f32,
    /// Period high.
    pub high: f32,
    /// Period low.
    pub low: f32,
    /// Closing price.
    pub close: f32,
}

impl Period {
    /// Construct from `(open, high, low, close)`.
    #[must_use]
    pub const fn new(open: f32, high: f32, low: f32, close: f32) -> Self {
        Self {
            open,
            high,
            low,
            close,
        }
    }

    /// `true` when `close >= open` (typically green).
    #[must_use]
    pub fn is_up(&self) -> bool {
        self.close >= self.open
    }
}

/// Candlestick chart — N periods rendered side-by-side.
#[derive(Clone, Debug)]
pub struct Candlestick {
    /// Periods in left-to-right order.
    pub periods: Vec<Period>,
    /// Fill colour for up periods (`close >= open`).
    pub up_color: ChartColor,
    /// Fill colour for down periods.
    pub down_color: ChartColor,
}

impl Candlestick {
    /// Construct with default green-up / red-down colours.
    #[must_use]
    pub fn new(periods: Vec<Period>) -> Self {
        Self {
            periods,
            up_color: ChartColor::from_hex("#27ae60").unwrap(),
            down_color: ChartColor::from_hex("#e74c3c").unwrap(),
        }
    }

    /// Emit body rects + wick lines as `wisp::Graphics`.
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
        let body_w = band_w * 0.65;

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

        let wick_w_ndc = 1.5 / viewport_px.x * 2.0;

        for (i, p) in self.periods.iter().enumerate() {
            let centre_x = plot_left + (usize_to_f32(i) + 0.5) * band_w;
            let body_left = centre_x - body_w * 0.5;
            let color = if p.is_up() {
                self.up_color
            } else {
                self.down_color
            };
            let body_top = map_y(p.open.max(p.close));
            let body_bot = map_y(p.open.min(p.close));
            // Wick (low → high).
            g.fill(Fill::Solid(chart_to_wisp(color)));
            let a = pixel_to_ndc(Vec2::new(centre_x, map_y(p.low)), viewport_px);
            let b = pixel_to_ndc(Vec2::new(centre_x, map_y(p.high)), viewport_px);
            g.draw_line(a, b, wick_w_ndc);
            // Body (open → close).
            let rect = px_rect_to_ndc(
                body_left,
                body_top,
                body_w,
                body_bot - body_top,
                viewport_px,
            );
            g.draw_rect(rect);
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

fn usize_to_f32(v: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "period counts ≤ ~250 in practice; well within f32 mantissa"
    )]
    {
        v as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Candlestick {
        Candlestick::new(vec![
            Period::new(100.0, 110.0, 95.0, 108.0),
            Period::new(108.0, 115.0, 105.0, 102.0),
            Period::new(102.0, 109.0, 100.0, 107.0),
            Period::new(107.0, 112.0, 103.0, 111.0),
        ])
    }

    #[test]
    fn candlestick_emits_one_wick_plus_one_body_per_period() {
        let theme = Theme::light();
        let g = fixture().emit_graphics(&theme, Vec2::new(400.0, 240.0));
        // 4 wicks (lines) + 4 bodies (rects) = 8.
        assert_eq!(g.primitive_count(), 8);
    }

    #[test]
    fn period_is_up_distinguishes_close_above_open() {
        assert!(Period::new(100.0, 110.0, 95.0, 108.0).is_up());
        assert!(!Period::new(100.0, 110.0, 95.0, 92.0).is_up());
    }

    #[test]
    fn empty_candlestick_emits_no_primitives() {
        let theme = Theme::light();
        let g = Candlestick::new(vec![]).emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
