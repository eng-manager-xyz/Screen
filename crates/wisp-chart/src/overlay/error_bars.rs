//! Error-bar overlay — vertical whisker + caps per data point.
//! Composes alongside a primary mark (bar / point / line) by
//! sharing the underlying chart's Y domain.
//!
//! Symmetric, asymmetric, and confidence-interval inputs are
//! all expressed as a `(mean, lower, upper)` triple after the
//! caller converts whatever raw input they have (standard
//! error, sample size, CI percentile, etc.) into absolute
//! lower / upper Y values.

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// How a point's error is supplied. Variants are inputs to
/// [`ErrorPoint::new`] / [`ErrorPoint::symmetric`] /
/// [`ErrorPoint::asymmetric`] — once stored on `ErrorPoint`
/// everything is reduced to absolute `(lower, upper)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ErrorKind {
    /// Same offset above and below the mean — typical for
    /// standard deviation or `±` measurement uncertainty.
    Symmetric(f32),
    /// Different offsets above and below — typical for
    /// skewed distributions or quantile-based intervals.
    Asymmetric {
        /// Offset below the mean (subtracted to get the lower
        /// bar end).
        lower: f32,
        /// Offset above the mean (added to get the upper bar
        /// end).
        upper: f32,
    },
    /// 95% (or other) confidence interval expressed as a
    /// half-width. Caller is responsible for the math
    /// (multiply standard error by 1.96 etc.).
    ConfidenceInterval(f32),
}

/// One error-bar point in chart-domain Y space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ErrorPoint {
    /// `x` position as a fraction `[0, 1]` of the plot's width.
    /// `0.0` = left edge, `0.5` = centre, `1.0` = right edge.
    /// Caller chooses how this maps to their X scale — for a
    /// 4-band bar chart at `(i + 0.5) / 4` is the centre of
    /// band `i`.
    pub x_fraction: f32,
    /// Mean / central value in Y domain units.
    pub mean: f32,
    /// Lower bar end in Y domain units.
    pub lower: f32,
    /// Upper bar end in Y domain units.
    pub upper: f32,
}

impl ErrorPoint {
    /// Construct directly from `(x_fraction, mean, lower, upper)`.
    #[must_use]
    pub const fn new(x_fraction: f32, mean: f32, lower: f32, upper: f32) -> Self {
        Self {
            x_fraction,
            mean,
            lower,
            upper,
        }
    }

    /// Build from a symmetric half-width: `lower = mean - h`,
    /// `upper = mean + h`.
    #[must_use]
    pub fn symmetric(x_fraction: f32, mean: f32, half_width: f32) -> Self {
        Self {
            x_fraction,
            mean,
            lower: mean - half_width,
            upper: mean + half_width,
        }
    }

    /// Build from asymmetric offsets.
    #[must_use]
    pub fn asymmetric(x_fraction: f32, mean: f32, below: f32, above: f32) -> Self {
        Self {
            x_fraction,
            mean,
            lower: mean - below,
            upper: mean + above,
        }
    }
}

/// Error-bar overlay primitive list.
#[derive(Clone, Debug)]
pub struct ErrorBars {
    /// Per-point error spec.
    pub points: Vec<ErrorPoint>,
    /// Y-domain of the underlying chart — `(min, max)`. Used to
    /// map `lower / mean / upper` from chart units to pixel
    /// space. Must match the underlying chart's domain or the
    /// bars will land at the wrong height.
    pub y_domain: (f32, f32),
    /// Whisker + cap stroke colour.
    pub color: ChartColor,
    /// Horizontal cap length in pixels.
    pub cap_width_px: f32,
    /// Whisker line width in pixels.
    pub stroke_width_px: f32,
}

impl ErrorBars {
    /// Construct with default cap (10 px) + stroke (1.5 px) +
    /// near-black colour.
    #[must_use]
    pub fn new(points: Vec<ErrorPoint>, y_domain: (f32, f32)) -> Self {
        Self {
            points,
            y_domain,
            color: ChartColor::from_hex("#222222").unwrap(),
            cap_width_px: 10.0,
            stroke_width_px: 1.5,
        }
    }

    /// Emit the whisker + caps as a `wisp::Graphics`. Uses a
    /// `16 px` padded viewport as the plot rect; see
    /// [`Self::emit_graphics_in_rect`] when overlaying on a
    /// chart with a different plot area (e.g. a `Plot::Bar`
    /// whose gutter + header dimensions don't match this
    /// default).
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let pad = 16.0_f32;
        let rect = Rect::new(
            pad,
            pad,
            viewport_px.x - pad * 2.0,
            viewport_px.y - pad * 2.0,
        );
        self.emit_graphics_in_rect(theme, viewport_px, rect)
    }

    /// Emit the whisker + caps using the supplied plot
    /// rectangle as the layout reference. The caller's primary
    /// chart computes this rectangle (e.g. `Plot`'s internal
    /// `cartesian_layout` returns one); passing it here keeps
    /// the error bars in lockstep with the bars / points they
    /// annotate.
    #[must_use]
    pub fn emit_graphics_in_rect(
        &self,
        theme: &Theme,
        viewport_px: Vec2,
        plot_rect: Rect,
    ) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        if self.points.is_empty() {
            return g;
        }
        let plot_left = plot_rect.min.x;
        let plot_top = plot_rect.min.y;
        let plot_w = plot_rect.size.x;
        let plot_h = plot_rect.size.y;
        let plot_bottom = plot_top + plot_h;
        let (y_lo, y_hi) = self.y_domain;
        let span = (y_hi - y_lo).max(f32::EPSILON);
        let map_y = |value: f32| plot_bottom - (value - y_lo) / span * plot_h;
        let stroke_ndc = self.stroke_width_px / viewport_px.x * 2.0;

        g.fill(Fill::Solid(chart_to_wisp(self.color)));
        for point in &self.points {
            let px = plot_left + point.x_fraction.clamp(0.0, 1.0) * plot_w;
            let lower_y = map_y(point.lower);
            let upper_y = map_y(point.upper);
            // Whisker.
            let top = pixel_to_ndc(Vec2::new(px, upper_y), viewport_px);
            let bottom = pixel_to_ndc(Vec2::new(px, lower_y), viewport_px);
            g.draw_line(top, bottom, stroke_ndc);
            // Upper cap.
            let cap_half = self.cap_width_px * 0.5;
            let upper_left = pixel_to_ndc(Vec2::new(px - cap_half, upper_y), viewport_px);
            let upper_right = pixel_to_ndc(Vec2::new(px + cap_half, upper_y), viewport_px);
            g.draw_line(upper_left, upper_right, stroke_ndc);
            // Lower cap.
            let lower_left = pixel_to_ndc(Vec2::new(px - cap_half, lower_y), viewport_px);
            let lower_right = pixel_to_ndc(Vec2::new(px + cap_half, lower_y), viewport_px);
            g.draw_line(lower_left, lower_right, stroke_ndc);
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

    #[test]
    fn emit_graphics_3_lines_per_point() {
        let bars = ErrorBars::new(
            vec![
                ErrorPoint::symmetric(0.125, 30.0, 5.0),
                ErrorPoint::symmetric(0.375, 45.0, 8.0),
                ErrorPoint::symmetric(0.625, 38.0, 6.0),
                ErrorPoint::symmetric(0.875, 60.0, 9.0),
            ],
            (0.0, 100.0),
        );
        let theme = Theme::light();
        let g = bars.emit_graphics(&theme, Vec2::new(400.0, 240.0));
        // Per point: 1 whisker + 2 caps = 3. Times 4 points = 12.
        assert_eq!(g.primitive_count(), 12);
    }

    #[test]
    fn symmetric_helper_centres_lower_upper_around_mean() {
        let p = ErrorPoint::symmetric(0.5, 10.0, 2.0);
        assert!((p.lower - 8.0).abs() < 1e-5);
        assert!((p.upper - 12.0).abs() < 1e-5);
    }

    #[test]
    fn asymmetric_helper_uses_separate_offsets() {
        let p = ErrorPoint::asymmetric(0.5, 10.0, 1.0, 3.0);
        assert!((p.lower - 9.0).abs() < 1e-5);
        assert!((p.upper - 13.0).abs() < 1e-5);
    }

    #[test]
    fn ci_95_from_standard_error_matches_1_96_multiplier() {
        // 95% CI half-width = 1.96 × SE.
        let se: f32 = 2.0;
        let ErrorKind::ConfidenceInterval(ci) = ErrorKind::ConfidenceInterval(1.96 * se) else {
            unreachable!()
        };
        assert!((ci - 3.92).abs() < 1e-4);
    }

    #[test]
    fn empty_bars_emit_no_primitives() {
        let bars = ErrorBars::new(vec![], (0.0, 1.0));
        let theme = Theme::light();
        let g = bars.emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
