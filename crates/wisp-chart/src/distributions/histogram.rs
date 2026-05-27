//! Histogram — bin a continuous variable and render each bin
//! as a bar. v1 supports caller-specified bin count or auto via
//! Freedman-Diaconis (`auto`), plus optional `extent` override.

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// How many bins the histogram should use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BinCount {
    /// Square-root rule — `sqrt(n).ceil()`. Fast, decent for
    /// most distributions, doesn't need quartile math.
    #[default]
    Auto,
    /// Fixed bin count chosen by the caller.
    Fixed(usize),
}

/// One bin in a histogram — its `[lo, hi)` range and its
/// count.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HistogramBin {
    /// Inclusive lower edge.
    pub lo: f32,
    /// Exclusive upper edge.
    pub hi: f32,
    /// Number of samples falling into the bin.
    pub count: usize,
}

/// Histogram value type.
#[derive(Clone, Debug)]
pub struct Histogram {
    /// Pre-binned data — `[lo, hi, count]` per bin.
    pub bins: Vec<HistogramBin>,
    /// Bar fill colour.
    pub color: ChartColor,
}

impl Histogram {
    /// Build a histogram from raw samples. `bin_count` controls
    /// binning; `extent` optionally clips the bin range
    /// (samples outside the extent are dropped).
    #[must_use]
    pub fn from_samples(samples: &[f32], bin_count: BinCount, extent: Option<(f32, f32)>) -> Self {
        let color = ChartColor::from_hex("#0072b2").unwrap();
        if samples.is_empty() {
            return Self {
                bins: Vec::new(),
                color,
            };
        }
        let (lo, hi) = extent.unwrap_or_else(|| {
            let mut lo = f32::INFINITY;
            let mut hi = f32::NEG_INFINITY;
            for &v in samples {
                lo = lo.min(v);
                hi = hi.max(v);
            }
            (lo, hi)
        });
        if (hi - lo).abs() < f32::EPSILON {
            return Self {
                bins: vec![HistogramBin {
                    lo,
                    hi: lo + 1.0,
                    count: samples.len(),
                }],
                color,
            };
        }
        let n_bins = match bin_count {
            BinCount::Auto => {
                #[allow(
                    clippy::cast_precision_loss,
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "sample count ≤ ~10^6; sqrt result fits usize"
                )]
                let n = (samples.len() as f32).sqrt().ceil() as usize;
                n.max(1)
            }
            BinCount::Fixed(n) => n.max(1),
        };
        let span = hi - lo;
        let bin_w = span / usize_to_f32(n_bins);
        let mut counts = vec![0_usize; n_bins];
        for &v in samples {
            if v < lo || v > hi {
                continue;
            }
            let normalised = ((v - lo) / bin_w).floor().max(0.0);
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "normalised >= 0 and clamped below by min(n_bins-1)"
            )]
            let idx = (normalised as usize).min(n_bins - 1);
            counts[idx] += 1;
        }
        let bins = counts
            .into_iter()
            .enumerate()
            .map(|(i, count)| HistogramBin {
                lo: lo + usize_to_f32(i) * bin_w,
                hi: lo + usize_to_f32(i + 1) * bin_w,
                count,
            })
            .collect();
        Self { bins, color }
    }

    /// Override the bar colour.
    #[must_use]
    pub fn color(mut self, color: ChartColor) -> Self {
        self.color = color;
        self
    }

    /// Emit one bar per bin.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        self.emit_with_interaction(theme, viewport_px).graphics
    }

    /// Like [`emit_graphics`](Self::emit_graphics) but additionally
    /// returns a reverse-lookup table mapping each bin's primitive
    /// index → [`ChartElementId::Bin`](crate::interaction::ChartElementId::Bin).
    #[must_use]
    pub fn emit_with_interaction(
        &self,
        theme: &Theme,
        viewport_px: Vec2,
    ) -> crate::interaction::EmittedChart {
        let _ = theme;
        let mut g = Graphics::new();
        let mut elements: Vec<(usize, crate::interaction::ChartElementId)> = Vec::new();
        if self.bins.is_empty() {
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
        let n = self.bins.len();
        let bar_w = plot_w / usize_to_f32(n);
        let max_count: usize = self.bins.iter().map(|b| b.count).max().unwrap_or(1);
        if max_count == 0 {
            return crate::interaction::EmittedChart {
                graphics: g,
                elements,
            };
        }
        let max_count_f = usize_to_f32(max_count);
        g.fill(Fill::Solid(chart_to_wisp(self.color)));
        for (i, bin) in self.bins.iter().enumerate() {
            let frac = usize_to_f32(bin.count) / max_count_f;
            let bar_h = plot_h * frac;
            let x = plot_left + usize_to_f32(i) * bar_w;
            let y = plot_bottom - bar_h;
            // Inset by 1 px on each side so adjacent bars don't
            // visually merge.
            let rect = px_rect_to_ndc(x + 0.5, y, (bar_w - 1.0).max(0.0), bar_h, viewport_px);
            g.draw_rect(rect);
            elements.push((
                g.primitive_count() - 1,
                crate::interaction::ChartElementId::Bin(i),
            ));
        }
        crate::interaction::EmittedChart {
            graphics: g,
            elements,
        }
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
        reason = "bin counts ≤ ~10^6; well within f32 mantissa for our uses"
    )]
    {
        v as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histogram_bin_count_matches_fixed_request() {
        let samples: Vec<f32> = (0_u8..100).map(|i| f32::from(i % 10)).collect();
        let h = Histogram::from_samples(&samples, BinCount::Fixed(10), None);
        assert_eq!(h.bins.len(), 10);
    }

    #[test]
    fn histogram_auto_bin_count_is_sqrt_n() {
        let samples: Vec<f32> = (1u8..=16).map(f32::from).collect();
        let h = Histogram::from_samples(&samples, BinCount::Auto, None);
        // sqrt(16) = 4.
        assert_eq!(h.bins.len(), 4);
    }

    #[test]
    fn histogram_emits_one_bar_per_bin() {
        let samples: Vec<f32> = (1u8..=10).map(f32::from).collect();
        let h = Histogram::from_samples(&samples, BinCount::Fixed(5), None);
        let theme = Theme::light();
        let g = h.emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 5);
    }

    #[test]
    fn empty_samples_emits_no_bars() {
        let h = Histogram::from_samples(&[], BinCount::Auto, None);
        let theme = Theme::light();
        let g = h.emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 0);
    }

    #[test]
    fn extent_override_drops_out_of_range_samples() {
        let samples = vec![1.0_f32, 2.0, 3.0, 100.0];
        let h = Histogram::from_samples(&samples, BinCount::Fixed(3), Some((0.0, 5.0)));
        // 100.0 is outside the (0, 5) extent.
        let total: usize = h.bins.iter().map(|b| b.count).sum();
        assert_eq!(total, 3);
    }
}
