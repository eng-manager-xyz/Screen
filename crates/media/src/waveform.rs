//! Waveform bar geometry (M-MEDIA.9 / AUT-105).
//!
//! Maps an [`AudioHistogram`] (M-MEDIA.8) to a list of axis-aligned
//! rectangles that `wisp`'s graphics pipeline can render directly. The
//! point of this module is the [crate-level architecture
//! boundary](crate#three-way-split) — `wisp` should NOT know about
//! audio. `media` produces geometry; `wisp` draws it.
//!
//! # Coordinate convention
//!
//! Rectangles use a `y`-up convention (matching wisp NDC): `x`/`y` is
//! the **bottom-left** corner, `width`/`height` are positive. Callers
//! pass layout values in whatever unit they want — NDC `[-1, +1]`,
//! screen pixels, or normalized `[0, 1]`. The math is unit-agnostic.
//!
//! # Display modes
//!
//! - [`mono_bars`] — one bar per bucket, two arrangement options:
//!   - [`WaveformDisplayMode::Anchored`] — bar's bottom sits on
//!     `baseline_y`, grows upward by `value * max_height`. Common for
//!     dope-sheet rows above a timeline.
//!   - [`WaveformDisplayMode::Mirrored`] — bar centered on
//!     `baseline_y`, extends `value * max_height / 2` in both
//!     directions. Common for centered "VU-style" displays.
//! - [`stereo_bars`] — pairs left + right histograms: left grows up
//!   from `baseline_y`, right grows down. Mode is implicitly anchored;
//!   [`WaveformLayout::mode`] is ignored.
//!
//! # Bar metric
//!
//! [`BarMetric::Peak`] uses each bucket's `peak` (max `|sample|`); the
//! visualization feels punchier. [`BarMetric::Rms`] uses RMS; the
//! visualization feels smoother.

use crate::histogram::AudioHistogram;

/// One waveform bar's rectangle.
///
/// Coordinates use a `y`-up convention: `x`/`y` is the bottom-left
/// corner; `width`/`height` are non-negative.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveformBarRect {
    /// Left edge.
    pub x: f32,
    /// Bottom edge (`y`-up).
    pub y: f32,
    /// Bar width — always `> 0` when `layout.bar_width > 0`.
    pub width: f32,
    /// Bar height — `0` for silence, `layout.max_height` for `|sample| = 1.0`.
    pub height: f32,
    /// RGBA color, each component in `[0, 1]`.
    pub color: [f32; 4],
}

/// Which summary statistic to drive bar height with.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BarMetric {
    /// Bar height ∝ `peak` (max `|sample|`). Punchy.
    #[default]
    Peak,
    /// Bar height ∝ `rms`. Smoother.
    Rms,
}

/// Arrangement of bars relative to the baseline.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WaveformDisplayMode {
    /// Bar's bottom edge sits on `baseline_y`; grows upward.
    #[default]
    Anchored,
    /// Bar is centered on `baseline_y`; extends half up + half down.
    Mirrored,
}

/// Layout parameters shared by every bar in one waveform.
///
/// All values use the caller's chosen unit system (NDC, pixels, …).
#[derive(Debug, Clone, Copy)]
pub struct WaveformLayout {
    /// `x` of the first bar's left edge.
    pub origin_x: f32,
    /// Reference `y` line for [`WaveformDisplayMode`].
    pub baseline_y: f32,
    /// Width of one bar (excluding gap). Must be `> 0`.
    pub bar_width: f32,
    /// Horizontal spacing between adjacent bar left edges, on top of
    /// `bar_width`. May be `0` for touching bars.
    pub bar_gap: f32,
    /// Bar height when the metric is `1.0`. Must be `> 0`.
    pub max_height: f32,
    /// Per-bar RGBA color.
    pub color: [f32; 4],
    /// Whether to read `peak` or `rms` from each bucket.
    pub metric: BarMetric,
    /// Anchored vs mirrored.
    pub mode: WaveformDisplayMode,
}

impl WaveformLayout {
    /// Reasonable defaults for an NDC `[-1, +1]` mono waveform centered
    /// vertically: gray bars `0.02` wide, gap `0.005`, max height
    /// `0.4`, anchored at `y = 0`. Tweak from here.
    #[must_use]
    pub fn ndc_default() -> Self {
        Self {
            origin_x: -0.9,
            baseline_y: 0.0,
            bar_width: 0.02,
            bar_gap: 0.005,
            max_height: 0.4,
            color: [0.65, 0.70, 0.78, 1.0],
            metric: BarMetric::Peak,
            mode: WaveformDisplayMode::Anchored,
        }
    }
}

/// Layout one mono histogram into a list of bar rectangles.
///
/// # Panics
///
/// Panics if `layout.bar_width <= 0` or `layout.max_height <= 0`. Both
/// are caller errors that would produce degenerate geometry.
#[must_use]
pub fn mono_bars(hist: &AudioHistogram, layout: &WaveformLayout) -> Vec<WaveformBarRect> {
    assert!(layout.bar_width > 0.0, "bar_width must be > 0");
    assert!(layout.max_height > 0.0, "max_height must be > 0");

    let stride = layout.bar_width + layout.bar_gap;
    let mut out = Vec::with_capacity(hist.bars.len());

    for (i, bar) in hist.bars.iter().enumerate() {
        let metric_value = match layout.metric {
            BarMetric::Peak => bar.peak,
            BarMetric::Rms => bar.rms,
        }
        .clamp(0.0, 1.0);

        let height = metric_value * layout.max_height;
        let x = layout.origin_x + i_as_f32(i) * stride;
        let y = match layout.mode {
            WaveformDisplayMode::Anchored => layout.baseline_y,
            WaveformDisplayMode::Mirrored => layout.baseline_y - height * 0.5,
        };

        out.push(WaveformBarRect {
            x,
            y,
            width: layout.bar_width,
            height,
            color: layout.color,
        });
    }

    out
}

/// Layout stereo histograms: `left` bars extend up from `baseline_y`,
/// `right` bars extend down. Always anchored; `layout.mode` is ignored.
///
/// If the two histograms have different lengths, the shorter one wins
/// — the surplus bars are dropped (their data is meaningless without
/// a paired channel).
///
/// # Panics
///
/// Same conditions as [`mono_bars`].
#[must_use]
pub fn stereo_bars(
    left: &AudioHistogram,
    right: &AudioHistogram,
    layout: &WaveformLayout,
) -> Vec<WaveformBarRect> {
    assert!(layout.bar_width > 0.0, "bar_width must be > 0");
    assert!(layout.max_height > 0.0, "max_height must be > 0");

    let n = left.bars.len().min(right.bars.len());
    let stride = layout.bar_width + layout.bar_gap;
    let mut out = Vec::with_capacity(n * 2);

    for i in 0..n {
        let lv = match layout.metric {
            BarMetric::Peak => left.bars[i].peak,
            BarMetric::Rms => left.bars[i].rms,
        }
        .clamp(0.0, 1.0);
        let rv = match layout.metric {
            BarMetric::Peak => right.bars[i].peak,
            BarMetric::Rms => right.bars[i].rms,
        }
        .clamp(0.0, 1.0);

        let lh = lv * layout.max_height;
        let rh = rv * layout.max_height;
        let x = layout.origin_x + i_as_f32(i) * stride;

        out.push(WaveformBarRect {
            x,
            y: layout.baseline_y,
            width: layout.bar_width,
            height: lh,
            color: layout.color,
        });
        out.push(WaveformBarRect {
            x,
            y: layout.baseline_y - rh,
            width: layout.bar_width,
            height: rh,
            color: layout.color,
        });
    }

    out
}

#[expect(
    clippy::cast_precision_loss,
    reason = "histogram bar indices below 2^24 fit f32 exactly; realistic dope-sheet bar counts stay there"
)]
fn i_as_f32(i: usize) -> f32 {
    i as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::AudioFormat;
    use crate::clock::MediaDuration;
    use crate::histogram::quantize;
    use crate::mock_audio::{SilenceSource, SineWaveSource};

    fn sine_hist() -> AudioHistogram {
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = SineWaveSource::new(fmt, 440.0, 0.6);
        let chunk = src.next_chunk(48_000);
        quantize(&chunk, MediaDuration::from_millis(50)) // 20 bars
    }

    #[test]
    fn anchored_bars_progress_left_to_right_with_stride() {
        let h = sine_hist();
        let layout = WaveformLayout {
            origin_x: 0.0,
            baseline_y: 0.0,
            bar_width: 0.1,
            bar_gap: 0.02,
            max_height: 1.0,
            color: [1.0; 4],
            metric: BarMetric::Peak,
            mode: WaveformDisplayMode::Anchored,
        };
        let rects = mono_bars(&h, &layout);
        assert_eq!(rects.len(), h.len());
        for (i, r) in rects.iter().enumerate() {
            let expected_x = i_as_f32(i) * 0.12;
            assert!(
                (r.x - expected_x).abs() < 1e-6,
                "bar {i}: x={} expected {expected_x}",
                r.x
            );
            assert!((r.width - 0.1).abs() < 1e-6);
            assert!((r.y - 0.0).abs() < 1e-6, "anchored y == baseline_y");
        }
    }

    #[test]
    fn anchored_bar_height_equals_peak_times_max_height() {
        let h = sine_hist();
        let layout = WaveformLayout {
            origin_x: 0.0,
            baseline_y: 0.0,
            bar_width: 0.1,
            bar_gap: 0.0,
            max_height: 2.0,
            color: [1.0; 4],
            metric: BarMetric::Peak,
            mode: WaveformDisplayMode::Anchored,
        };
        let rects = mono_bars(&h, &layout);
        for (r, bar) in rects.iter().zip(h.bars.iter()) {
            let expected = bar.peak * 2.0;
            assert!(
                (r.height - expected).abs() < 1e-6,
                "got {} expected {expected}",
                r.height
            );
        }
    }

    #[test]
    fn mirrored_bars_are_centered_on_baseline() {
        let h = sine_hist();
        let layout = WaveformLayout {
            origin_x: 0.0,
            baseline_y: 0.5,
            bar_width: 0.1,
            bar_gap: 0.0,
            max_height: 1.0,
            color: [1.0; 4],
            metric: BarMetric::Peak,
            mode: WaveformDisplayMode::Mirrored,
        };
        let rects = mono_bars(&h, &layout);
        for r in &rects {
            let center = r.y + r.height * 0.5;
            assert!(
                (center - 0.5).abs() < 1e-6,
                "expected center=0.5, got {center}"
            );
        }
    }

    #[test]
    fn rms_metric_uses_rms_field() {
        let h = sine_hist();
        let layout = WaveformLayout {
            origin_x: 0.0,
            baseline_y: 0.0,
            bar_width: 0.1,
            bar_gap: 0.0,
            max_height: 1.0,
            color: [1.0; 4],
            metric: BarMetric::Rms,
            mode: WaveformDisplayMode::Anchored,
        };
        let rects = mono_bars(&h, &layout);
        for (r, bar) in rects.iter().zip(h.bars.iter()) {
            assert!(
                (r.height - bar.rms).abs() < 1e-6,
                "rms metric got {} expected {}",
                r.height,
                bar.rms
            );
        }
    }

    #[test]
    fn silent_histogram_produces_zero_height_bars() {
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = SilenceSource::new(fmt);
        let chunk = src.next_chunk(48_000);
        let h = quantize(&chunk, MediaDuration::from_millis(50));
        let rects = mono_bars(&h, &WaveformLayout::ndc_default());
        assert_eq!(rects.len(), 20);
        for r in &rects {
            assert!(r.height.abs() < f32::EPSILON);
        }
    }

    #[test]
    fn empty_histogram_produces_empty_geometry() {
        let h = AudioHistogram {
            bucket_duration: MediaDuration::from_millis(20),
            bars: Vec::new(),
        };
        let rects = mono_bars(&h, &WaveformLayout::ndc_default());
        assert!(rects.is_empty());
    }

    #[test]
    fn stereo_pairs_left_above_right_below_baseline() {
        let fmt = AudioFormat::mono_f32(48_000);
        let mut sl = SineWaveSource::new(fmt, 440.0, 0.6);
        let mut sr = SineWaveSource::new(fmt, 440.0, 0.3);
        let chunk_l = sl.next_chunk(48_000);
        let chunk_r = sr.next_chunk(48_000);
        let hl = quantize(&chunk_l, MediaDuration::from_millis(50));
        let hr = quantize(&chunk_r, MediaDuration::from_millis(50));
        let layout = WaveformLayout {
            origin_x: 0.0,
            baseline_y: 0.5,
            bar_width: 0.1,
            bar_gap: 0.0,
            max_height: 0.4,
            color: [1.0; 4],
            metric: BarMetric::Peak,
            mode: WaveformDisplayMode::Anchored,
        };
        let rects = stereo_bars(&hl, &hr, &layout);
        assert_eq!(rects.len(), hl.len() * 2);

        for pair in rects.chunks_exact(2) {
            let (l, r) = (pair[0], pair[1]);
            // Same x — they share a column.
            assert!((l.x - r.x).abs() < 1e-6);
            // L sits on the baseline going up; R's TOP edge is the baseline going down.
            assert!(
                (l.y - 0.5).abs() < 1e-6,
                "left bar y={} expected baseline 0.5",
                l.y
            );
            assert!(
                ((r.y + r.height) - 0.5).abs() < 1e-6,
                "right top {} expected baseline 0.5",
                r.y + r.height
            );
        }
    }

    #[test]
    fn stereo_truncates_to_shorter_input() {
        let fmt = AudioFormat::mono_f32(48_000);
        let mut sl = SineWaveSource::new(fmt, 440.0, 0.5);
        let mut sr = SineWaveSource::new(fmt, 440.0, 0.5);
        let chunk_l = sl.next_chunk(48_000); // 1s → 20 bars
        let chunk_r = sr.next_chunk(24_000); // 0.5s → 10 bars
        let hl = quantize(&chunk_l, MediaDuration::from_millis(50));
        let hr = quantize(&chunk_r, MediaDuration::from_millis(50));
        let rects = stereo_bars(&hl, &hr, &WaveformLayout::ndc_default());
        assert_eq!(rects.len(), 10 * 2, "truncates to min(20, 10) = 10 pairs");
    }

    #[test]
    fn color_is_propagated_unchanged() {
        let h = sine_hist();
        let layout = WaveformLayout {
            color: [0.1, 0.2, 0.3, 0.5],
            ..WaveformLayout::ndc_default()
        };
        let rects = mono_bars(&h, &layout);
        let expected = [0.1_f32, 0.2, 0.3, 0.5];
        for r in &rects {
            for (a, b) in r.color.iter().zip(expected.iter()) {
                assert!((a - b).abs() < 1e-6, "color channel {a} expected {b}");
            }
        }
    }

    #[test]
    fn manual_regression_four_bar_table() {
        // Four-bar histogram, peaks 1.0, 0.5, 0.25, 0.0, anchored at
        // y=0, bar_width=0.1, gap=0.02, max_height=1.0, origin=0.0.
        // Expected: bars at x = 0.0, 0.12, 0.24, 0.36; height = peak.
        // Manually craft a 4-bucket histogram with peaks 1.0, 0.5, 0.25, 0.0
        // (StepPulseSource isn't expressive enough for arbitrary
        // per-bucket peaks; the bar table is the contract.)
        let bars = vec![
            crate::histogram::AudioBar {
                start_time: crate::clock::MediaTime::ZERO,
                duration: MediaDuration::from_millis(50),
                peak: 1.0,
                rms: 1.0 / std::f32::consts::SQRT_2,
            },
            crate::histogram::AudioBar {
                start_time: crate::clock::MediaTime::from_seconds(0.05),
                duration: MediaDuration::from_millis(50),
                peak: 0.5,
                rms: 0.5 / std::f32::consts::SQRT_2,
            },
            crate::histogram::AudioBar {
                start_time: crate::clock::MediaTime::from_seconds(0.10),
                duration: MediaDuration::from_millis(50),
                peak: 0.25,
                rms: 0.25 / std::f32::consts::SQRT_2,
            },
            crate::histogram::AudioBar {
                start_time: crate::clock::MediaTime::from_seconds(0.15),
                duration: MediaDuration::from_millis(50),
                peak: 0.0,
                rms: 0.0,
            },
        ];
        let h = AudioHistogram {
            bucket_duration: MediaDuration::from_millis(50),
            bars,
        };
        let layout = WaveformLayout {
            origin_x: 0.0,
            baseline_y: 0.0,
            bar_width: 0.1,
            bar_gap: 0.02,
            max_height: 1.0,
            color: [1.0; 4],
            metric: BarMetric::Peak,
            mode: WaveformDisplayMode::Anchored,
        };
        let rects = mono_bars(&h, &layout);

        let expected = [(0.0_f32, 1.0_f32), (0.12, 0.5), (0.24, 0.25), (0.36, 0.0)];
        for (i, (xe, he)) in expected.iter().enumerate() {
            assert!(
                (rects[i].x - xe).abs() < 1e-6,
                "bar {i}: x={} expected {xe}",
                rects[i].x
            );
            assert!(
                (rects[i].height - he).abs() < 1e-6,
                "bar {i}: height={} expected {he}",
                rects[i].height
            );
        }
    }

    #[test]
    fn types_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WaveformBarRect>();
        assert_send_sync::<WaveformLayout>();
    }
}
