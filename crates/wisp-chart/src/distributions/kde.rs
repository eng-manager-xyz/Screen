//! Kernel-density estimate (KDE) — smoothed continuous
//! distribution. Renders a polyline along the density curve
//! sampled at `resolution` x-positions.

use glam::Vec2;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// Bandwidth selection rule for the KDE.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum BandwidthRule {
    /// Silverman's rule of thumb: `h = 1.06 · σ · n^(-1/5)`.
    #[default]
    Silverman,
    /// Caller-supplied bandwidth — no automatic selection.
    Manual(f32),
}

/// KDE plot value type.
#[derive(Clone, Debug)]
pub struct KdePlot {
    /// Raw samples.
    pub samples: Vec<f32>,
    /// Bandwidth rule.
    pub bandwidth: BandwidthRule,
    /// X-axis sample count.
    pub resolution: usize,
    /// Curve colour.
    pub color: ChartColor,
}

impl KdePlot {
    /// Construct with Silverman bandwidth + 128-px curve resolution.
    #[must_use]
    pub fn new(samples: Vec<f32>) -> Self {
        Self {
            samples,
            bandwidth: BandwidthRule::Silverman,
            resolution: 128,
            color: ChartColor::from_hex("#0072b2").unwrap(),
        }
    }

    /// Override bandwidth rule.
    #[must_use]
    pub const fn bandwidth(mut self, rule: BandwidthRule) -> Self {
        self.bandwidth = rule;
        self
    }

    /// Override curve colour.
    #[must_use]
    pub fn color(mut self, color: ChartColor) -> Self {
        self.color = color;
        self
    }

    /// Compute the bandwidth (`h`) used by the kernel sum.
    /// Exposed for tests + curious callers.
    #[must_use]
    pub fn computed_bandwidth(&self) -> f32 {
        match self.bandwidth {
            BandwidthRule::Manual(h) => h.max(f32::EPSILON),
            BandwidthRule::Silverman => silverman(&self.samples),
        }
    }

    /// Emit the KDE curve as a polyline + a baseline line so
    /// the area under the curve reads as a shape.
    #[must_use]
    #[allow(
        clippy::many_single_char_names,
        reason = "Math conventions: x/h/t/u read more clearly than wordy synonyms in KDE formula."
    )]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        if self.samples.len() < 2 || self.resolution < 2 {
            return g;
        }
        let pad = 16.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let plot_w = plot_right - plot_left;
        let plot_h = plot_bottom - plot_top;

        let (lo, hi) = sample_extent(&self.samples);
        let pad_x = (hi - lo) * 0.05;
        let x_lo = lo - pad_x;
        let x_hi = hi + pad_x;
        let h = self.computed_bandwidth();
        let n_inv = 1.0 / usize_to_f32(self.samples.len());

        // Sample the density at each x; track the max for
        // normalisation.
        let mut ys = Vec::with_capacity(self.resolution);
        let mut max_y = 0.0_f32;
        for i in 0..self.resolution {
            let t = usize_to_f32(i) / usize_to_f32(self.resolution - 1);
            let x = x_lo + t * (x_hi - x_lo);
            let mut density = 0.0_f32;
            for &sample in &self.samples {
                let u = (x - sample) / h;
                density += gaussian_kernel(u);
            }
            density *= n_inv / h;
            max_y = max_y.max(density);
            ys.push((x, density));
        }
        if max_y < f32::EPSILON {
            return g;
        }

        let map_x = |x: f32| plot_left + (x - x_lo) / (x_hi - x_lo) * plot_w;
        let to_screen_y = |y: f32| plot_bottom - (y / max_y) * plot_h;

        g.fill(Fill::Solid(chart_to_wisp(self.color)));
        let stroke = 1.5 / viewport_px.x * 2.0;
        for pair in ys.windows(2) {
            let (x0, y0) = pair[0];
            let (x1, y1) = pair[1];
            let p0 = pixel_to_ndc(Vec2::new(map_x(x0), to_screen_y(y0)), viewport_px);
            let p1 = pixel_to_ndc(Vec2::new(map_x(x1), to_screen_y(y1)), viewport_px);
            g.draw_line(p0, p1, stroke);
        }
        g
    }
}

fn gaussian_kernel(u: f32) -> f32 {
    let coeff = 1.0 / (std::f32::consts::TAU.sqrt());
    coeff * (-0.5 * u * u).exp()
}

fn silverman(samples: &[f32]) -> f32 {
    if samples.len() < 2 {
        return 1.0;
    }
    let n = usize_to_f32(samples.len());
    let mean: f32 = samples.iter().sum::<f32>() / n;
    let var: f32 = samples
        .iter()
        .map(|v| {
            let d = v - mean;
            d * d
        })
        .sum::<f32>()
        / n;
    let sigma = var.sqrt();
    (1.06 * sigma * n.powf(-0.2)).max(f32::EPSILON)
}

fn sample_extent(samples: &[f32]) -> (f32, f32) {
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for &v in samples {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if lo.is_infinite() {
        (0.0, 1.0)
    } else {
        (lo, hi)
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
        reason = "sample counts ≤ ~10^6 fit f32 mantissa"
    )]
    {
        v as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kde_emits_resolution_minus_one_segments() {
        let samples: Vec<f32> = (0_u8..50).map(|i| f32::from(i) * 0.1).collect();
        let kde = KdePlot::new(samples).bandwidth(BandwidthRule::Manual(0.5));
        let kde = KdePlot {
            resolution: 32,
            ..kde
        };
        let theme = Theme::light();
        let g = kde.emit_graphics(&theme, Vec2::new(400.0, 240.0));
        // 32 sample points → 31 line segments.
        assert_eq!(g.primitive_count(), 31);
    }

    #[test]
    fn silverman_returns_positive_bandwidth_for_real_input() {
        let samples = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let kde = KdePlot::new(samples);
        assert!(kde.computed_bandwidth() > 0.0);
    }

    #[test]
    fn manual_bandwidth_overrides_silverman() {
        let samples = vec![1.0_f32, 2.0, 3.0];
        let kde = KdePlot::new(samples).bandwidth(BandwidthRule::Manual(0.42));
        assert!((kde.computed_bandwidth() - 0.42).abs() < 1e-5);
    }

    #[test]
    fn empty_or_singleton_samples_emits_nothing() {
        let theme = Theme::light();
        let g = KdePlot::new(vec![]).emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 0);
        let g = KdePlot::new(vec![1.0]).emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
