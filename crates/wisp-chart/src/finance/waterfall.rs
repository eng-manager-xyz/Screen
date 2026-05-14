//! Waterfall chart — cumulative deltas with start / end / pos /
//! neg colour rules. Each non-summary row carries a signed
//! delta; summary rows ("Start", "End") show the absolute
//! running total as a single full-height bar.

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// One row of a waterfall.
#[derive(Clone, Debug, PartialEq)]
pub struct WaterfallRow {
    /// Display label.
    pub label: String,
    /// Signed delta applied to the running total. For
    /// `is_summary = true` rows, this is the absolute height of
    /// the summary bar.
    pub delta: f32,
    /// If `true`, this row resets the cumulative offset — used
    /// for "Start" and "End" totals which are drawn as a single
    /// bar from `0` to the running total at that point.
    pub is_summary: bool,
}

impl WaterfallRow {
    /// Construct a contribution row (positive or negative delta).
    #[must_use]
    pub fn contribution(label: impl Into<String>, delta: f32) -> Self {
        Self {
            label: label.into(),
            delta,
            is_summary: false,
        }
    }

    /// Construct a summary (start / end) row with an absolute
    /// height equal to `value`.
    #[must_use]
    pub fn summary(label: impl Into<String>, value: f32) -> Self {
        Self {
            label: label.into(),
            delta: value,
            is_summary: true,
        }
    }
}

/// Waterfall chart.
#[derive(Clone, Debug)]
pub struct Waterfall {
    /// Rows in left-to-right order.
    pub rows: Vec<WaterfallRow>,
    /// Bar fill for positive deltas.
    pub positive_color: ChartColor,
    /// Bar fill for negative deltas.
    pub negative_color: ChartColor,
    /// Bar fill for summary (start/end) rows.
    pub summary_color: ChartColor,
}

impl Waterfall {
    /// Construct with sensible defaults — green positive, red
    /// negative, blue summary.
    #[must_use]
    pub fn new(rows: Vec<WaterfallRow>) -> Self {
        Self {
            rows,
            positive_color: ChartColor::from_hex("#27ae60").unwrap(),
            negative_color: ChartColor::from_hex("#e74c3c").unwrap(),
            summary_color: ChartColor::from_hex("#0072b2").unwrap(),
        }
    }

    /// Emit one rect per row as `wisp::Graphics`.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        if self.rows.is_empty() {
            return g;
        }

        let pad = 16.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let plot_w = plot_right - plot_left;
        let plot_h = plot_bottom - plot_top;
        let n = self.rows.len();
        let band_w = plot_w / usize_to_f32(n);
        let bar_w = band_w * 0.7;

        // Walk rows once to compute cumulative + max for scale.
        let mut cum = 0.0_f32;
        let mut lo = 0.0_f32;
        let mut hi = 0.0_f32;
        let mut prev = Vec::with_capacity(n);
        let mut next = Vec::with_capacity(n);
        for row in &self.rows {
            let (a, b) = if row.is_summary {
                (0.0, row.delta)
            } else {
                let start = cum;
                let end = cum + row.delta;
                cum = end;
                (start, end)
            };
            prev.push(a);
            next.push(b);
            lo = lo.min(a).min(b);
            hi = hi.max(a).max(b);
        }
        let span = (hi - lo).max(f32::EPSILON);
        let map_y = |v: f32| plot_bottom - (v - lo) / span * plot_h;

        for (i, row) in self.rows.iter().enumerate() {
            let centre_x = plot_left + (usize_to_f32(i) + 0.5) * band_w;
            let bar_left = centre_x - bar_w * 0.5;
            let top = map_y(prev[i].max(next[i]));
            let bot = map_y(prev[i].min(next[i]));
            let color = if row.is_summary {
                self.summary_color
            } else if row.delta >= 0.0 {
                self.positive_color
            } else {
                self.negative_color
            };
            g.fill(Fill::Solid(chart_to_wisp(color)));
            let h = (bot - top).max(2.0);
            let rect = px_rect_to_ndc(bar_left, top, bar_w, h, viewport_px);
            g.draw_rect(rect);
        }
        g
    }
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
        reason = "waterfall row counts are small (~10s in practice)"
    )]
    {
        v as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Waterfall {
        Waterfall::new(vec![
            WaterfallRow::summary("Start", 100.0),
            WaterfallRow::contribution("Revenue", 50.0),
            WaterfallRow::contribution("Costs", -20.0),
            WaterfallRow::contribution("Tax", -10.0),
            WaterfallRow::summary("End", 120.0),
        ])
    }

    #[test]
    fn waterfall_emits_one_bar_per_row() {
        let theme = Theme::light();
        let g = fixture().emit_graphics(&theme, Vec2::new(480.0, 240.0));
        assert_eq!(g.primitive_count(), 5);
    }

    #[test]
    fn empty_waterfall_emits_no_bars() {
        let theme = Theme::light();
        let g = Waterfall::new(vec![]).emit_graphics(&theme, Vec2::new(480.0, 240.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
