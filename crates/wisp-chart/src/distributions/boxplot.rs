//! Box plot — five-number summary of a distribution per
//! category. Min / Q1 / median / Q3 / max rendered as box +
//! whiskers + median line.

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// Five-number summary for one category.
#[derive(Clone, Debug, PartialEq)]
pub struct Box {
    /// Display label.
    pub label: String,
    /// Whisker minimum.
    pub min: f32,
    /// First quartile.
    pub q1: f32,
    /// Median.
    pub median: f32,
    /// Third quartile.
    pub q3: f32,
    /// Whisker maximum.
    pub max: f32,
    /// Box fill colour.
    pub color: ChartColor,
}

impl Box {
    /// Construct directly from precomputed quartiles.
    #[must_use]
    pub fn from_summary(
        label: impl Into<String>,
        min: f32,
        q1: f32,
        median: f32,
        q3: f32,
        max: f32,
        color: ChartColor,
    ) -> Self {
        Self {
            label: label.into(),
            min,
            q1,
            median,
            q3,
            max,
            color,
        }
    }

    /// Construct by computing quartiles from a sample slice.
    /// `samples` must be non-empty; quartiles use the
    /// inclusive-median method.
    #[must_use]
    pub fn from_samples(
        label: impl Into<String>,
        samples: &[f32],
        color: ChartColor,
    ) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }
        let mut sorted: Vec<f32> = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let last_idx = sorted.len() - 1;
        let percentile = |p: f32| -> f32 {
            #[allow(
                clippy::cast_precision_loss,
                reason = "last_idx ≤ sample count — practical limit ≤ 10^6 fits f32 mantissa"
            )]
            let scaled = last_idx as f32 * p;
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "scaled ∈ [0, last_idx] after clamp; falls in usize range"
            )]
            let idx = scaled.round() as usize;
            sorted[idx.min(last_idx)]
        };
        let min = sorted[0];
        let max = sorted[last_idx];
        Some(Self {
            label: label.into(),
            min,
            q1: percentile(0.25),
            median: percentile(0.5),
            q3: percentile(0.75),
            max,
            color,
        })
    }
}

/// Box plot — N categories side-by-side.
#[derive(Clone, Debug)]
pub struct BoxPlot {
    /// Categories in left-to-right order.
    pub boxes: Vec<Box>,
}

impl BoxPlot {
    /// Construct from a list of boxes.
    #[must_use]
    pub const fn new(boxes: Vec<Box>) -> Self {
        Self { boxes }
    }

    /// Emit one box + whiskers + median per category.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        if self.boxes.is_empty() {
            return g;
        }
        let pad = 16.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let plot_w = plot_right - plot_left;
        let plot_h = plot_bottom - plot_top;
        let n = self.boxes.len();
        let band_w = plot_w / usize_to_f32(n);
        let box_w = band_w * 0.5;

        let lo = self
            .boxes
            .iter()
            .map(|b| b.min)
            .fold(f32::INFINITY, f32::min);
        let hi = self
            .boxes
            .iter()
            .map(|b| b.max)
            .fold(f32::NEG_INFINITY, f32::max);
        let span = (hi - lo).max(f32::EPSILON);
        let map_y = |v: f32| plot_bottom - (v - lo) / span * plot_h;

        let line_w_ndc = 1.5 / viewport_px.x * 2.0;

        for (i, item) in self.boxes.iter().enumerate() {
            let centre_x = plot_left + (usize_to_f32(i) + 0.5) * band_w;
            let box_left = centre_x - box_w * 0.5;
            let q1_y = map_y(item.q1);
            let q3_y = map_y(item.q3);
            let median_y = map_y(item.median);
            let min_y = map_y(item.min);
            let upper_y = map_y(item.max);
            // Box (Q1 → Q3).
            g.fill(Fill::Solid(chart_to_wisp(item.color)));
            let rect = px_rect_to_ndc(
                box_left,
                q3_y.min(q1_y),
                box_w,
                (q3_y - q1_y).abs(),
                viewport_px,
            );
            g.draw_rect(rect);
            // Median line.
            g.fill(Fill::Solid(chart_to_wisp(
                ChartColor::from_hex("#222222").unwrap(),
            )));
            let p0 = pixel_to_ndc(Vec2::new(box_left, median_y), viewport_px);
            let p1 = pixel_to_ndc(Vec2::new(box_left + box_w, median_y), viewport_px);
            g.draw_line(p0, p1, line_w_ndc);
            // Lower whisker (min → Q1).
            let p0 = pixel_to_ndc(Vec2::new(centre_x, min_y), viewport_px);
            let p1 = pixel_to_ndc(Vec2::new(centre_x, q1_y), viewport_px);
            g.draw_line(p0, p1, line_w_ndc);
            // Upper whisker (Q3 → max).
            let p0 = pixel_to_ndc(Vec2::new(centre_x, q3_y), viewport_px);
            let p1 = pixel_to_ndc(Vec2::new(centre_x, upper_y), viewport_px);
            g.draw_line(p0, p1, line_w_ndc);
            // Whisker caps.
            let cap_w = box_w * 0.4;
            let p0 = pixel_to_ndc(Vec2::new(centre_x - cap_w * 0.5, min_y), viewport_px);
            let p1 = pixel_to_ndc(Vec2::new(centre_x + cap_w * 0.5, min_y), viewport_px);
            g.draw_line(p0, p1, line_w_ndc);
            let p0 = pixel_to_ndc(Vec2::new(centre_x - cap_w * 0.5, upper_y), viewport_px);
            let p1 = pixel_to_ndc(Vec2::new(centre_x + cap_w * 0.5, upper_y), viewport_px);
            g.draw_line(p0, p1, line_w_ndc);
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
        reason = "category counts ≤ ~50 in practice"
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

    #[test]
    fn boxplot_emits_six_primitives_per_box() {
        let bp = BoxPlot::new(vec![
            Box::from_summary("A", 0.0, 10.0, 20.0, 30.0, 40.0, red()),
            Box::from_summary("B", 5.0, 15.0, 25.0, 35.0, 50.0, red()),
        ]);
        let theme = Theme::light();
        let g = bp.emit_graphics(&theme, Vec2::new(400.0, 240.0));
        // Per box: 1 rect + 5 lines = 6. 2 boxes → 12.
        assert_eq!(g.primitive_count(), 12);
    }

    #[test]
    fn box_from_samples_computes_quartiles() {
        let samples: Vec<f32> = (1u8..=9).map(f32::from).collect();
        let bx = Box::from_samples("ints", &samples, red()).unwrap();
        assert!((bx.min - 1.0).abs() < 1e-5);
        assert!((bx.max - 9.0).abs() < 1e-5);
        assert!((bx.median - 5.0).abs() < 1e-5);
    }

    #[test]
    fn box_from_empty_samples_returns_none() {
        assert!(Box::from_samples("empty", &[], red()).is_none());
    }
}
