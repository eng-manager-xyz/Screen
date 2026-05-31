//! Audio waveform lane (ED.10 / M-EDIT).
//!
//! You cannot splice on a sound you cannot see. The cutting room solved
//! this with the **mag track** running beside the picture and a soundhead
//! on the flatbed; we solve it with a waveform — the audio's peak envelope
//! made visible. [`downsample_peaks`] reduces a sea of samples to one
//! min/max pair per horizontal bucket (drawing every sample is impossible
//! and pointless — the envelope is what the eye actually reads); the
//! [`AudioWaveform`] lane draws those buckets beneath the video track.
//!
//! Decoding the recording's audio into samples is gst work that lands with
//! the render-integration pass; this chunk is the (pure, tested) envelope
//! math + the lane that renders it.

use leptos::prelude::*;

/// The min/max amplitude envelope of one horizontal bucket of the waveform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveBucket {
    /// Most-negative sample in the bucket.
    pub min: f32,
    /// Most-positive sample in the bucket.
    pub max: f32,
}

impl WaveBucket {
    /// Peak-to-peak amplitude (`max - min`), the bucket's drawn height.
    #[must_use]
    pub fn amplitude(self) -> f32 {
        (self.max - self.min).max(0.0)
    }
}

/// Reduce `samples` to `buckets` min/max envelopes — the peak-pair
/// representation every scrubbable waveform uses. Pure + deterministic.
#[must_use]
pub fn downsample_peaks(samples: &[f32], buckets: usize) -> Vec<WaveBucket> {
    if buckets == 0 || samples.is_empty() {
        return Vec::new();
    }
    let n = samples.len();
    (0..buckets)
        .map(|bucket| {
            let start = bucket * n / buckets;
            let end = (((bucket + 1) * n / buckets).max(start + 1)).min(n);
            let slice = &samples[start..end];
            let mut min = f32::INFINITY;
            let mut max = f32::NEG_INFINITY;
            for &s in slice {
                min = min.min(s);
                max = max.max(s);
            }
            if slice.is_empty() {
                WaveBucket { min: 0.0, max: 0.0 }
            } else {
                WaveBucket { min, max }
            }
        })
        .collect()
}

#[allow(
    clippy::cast_precision_loss,
    reason = "bucket counts are small; exact in f64 for percent positioning"
)]
fn percent(part: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    part as f64 / total as f64 * 100.0
}

/// The horizontal `(left%, width%)` of bucket `index` of `count`, tiling the
/// lane edge-to-edge (bar `i` spans `[i/count, (i+1)/count]`). Computing the
/// width as the gap to the next bucket avoids cumulative rounding and lands
/// the final bar exactly at 100%. Pure — the regression guard for the
/// "bars render blank because they have no width" bug.
#[must_use]
fn bar_geometry(index: usize, count: usize) -> (f64, f64) {
    let left = percent(index, count);
    let right = percent(index + 1, count);
    (left, (right - left).max(0.0))
}

/// The audio lane: the waveform envelope beneath the video track. Reads the
/// peak buckets from context; renders a quiet baseline until the audio is
/// decoded (render-integration).
#[component]
pub fn AudioWaveform() -> impl IntoView {
    let peaks =
        use_context::<RwSignal<Vec<WaveBucket>>>().unwrap_or_else(|| RwSignal::new(Vec::new()));
    view! {
        <div class="timeline-lane timeline-lane--audio" aria-label="Audio track">
            {move || {
                let buckets = peaks.get();
                if buckets.is_empty() {
                    return view! { <div class="waveform-baseline"></div> }.into_any();
                }
                let count = buckets.len();
                buckets
                    .into_iter()
                    .enumerate()
                    .map(|(index, bucket)| {
                        let (left, width) = bar_geometry(index, count);
                        // Signal is normalized to [-1, 1] → amplitude in [0, 2].
                        let height = (f64::from(bucket.amplitude()) / 2.0 * 100.0).clamp(2.0, 100.0);
                        let style = format!("left:{left:.3}%;width:{width:.3}%;height:{height:.1}%");
                        view! { <span class="waveform-bar" style=style></span> }
                    })
                    .collect_view()
                    .into_any()
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn empty_inputs_yield_no_buckets() {
        assert!(downsample_peaks(&[], 8).is_empty());
        assert!(downsample_peaks(&[0.5, -0.5], 0).is_empty());
    }

    #[test]
    fn buckets_capture_min_max_envelope() {
        let samples = [-1.0, 1.0, -0.5, 0.5];
        let peaks = downsample_peaks(&samples, 2);
        assert_eq!(peaks.len(), 2);
        assert!(approx(peaks[0].min, -1.0) && approx(peaks[0].max, 1.0));
        assert!(approx(peaks[1].min, -0.5) && approx(peaks[1].max, 0.5));
        assert!(approx(peaks[0].amplitude(), 2.0));
        assert!(approx(peaks[1].amplitude(), 1.0));
    }

    #[test]
    fn more_buckets_than_samples_does_not_panic() {
        let peaks = downsample_peaks(&[0.2, -0.3], 8);
        assert_eq!(peaks.len(), 8);
        // Every bucket still has a valid (finite) envelope.
        assert!(peaks.iter().all(|b| b.min.is_finite() && b.max.is_finite()));
    }

    #[test]
    fn bars_tile_the_lane_edge_to_edge() {
        // Each bar's [left, left+width] must abut the next with no gap, and
        // the last bar must reach 100% — a zero/absent width is the blank-
        // lane bug (an inline `left`/`height` on a width-less inline span
        // renders nothing).
        let count = 7;
        let mut cursor = 0.0;
        for index in 0..count {
            let (left, width) = bar_geometry(index, count);
            assert!(
                (left - cursor).abs() < 1e-9,
                "bar {index} abuts the previous"
            );
            assert!(width > 0.0, "bar {index} has a non-zero width");
            cursor = left + width;
        }
        assert!((cursor - 100.0).abs() < 1e-9, "bars fill the lane to 100%");
    }

    #[test]
    fn one_bucket_spans_all_samples() {
        let samples = [0.1, 0.9, -0.4, 0.2, -0.7];
        let peaks = downsample_peaks(&samples, 1);
        assert_eq!(peaks.len(), 1);
        assert!(approx(peaks[0].min, -0.7) && approx(peaks[0].max, 0.9));
    }
}
