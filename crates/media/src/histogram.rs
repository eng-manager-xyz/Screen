//! Audio histogram quantization (M-MEDIA.8 / AUT-104).
//!
//! Turns an [`AudioChunk`] (or stream of chunks) into design-friendly
//! rectangle bars for timeline / dope-sheet visualization. Each
//! [`AudioBar`] carries `start_time` + `duration` (on the media
//! timeline), `peak` (max `|sample|` in the bucket), and `rms`.
//!
//! # Bucket size
//!
//! Default range is **20–50 ms** — dope-sheet readability sweet spot
//! (≈ 20–50 bars per second of audio). 10 ms is supported for
//! fine-grained tests + scroll-through-zoom views.
//!
//! # Math
//!
//! - `peak = max_{samples in bucket}(|s|)`. Always in `[0, 1]`.
//! - `rms  = sqrt(mean(s²))`. Same range. For a pure sine of
//!   amplitude `A`, `rms ≈ A / √2 ≈ 0.7071·A`.
//!
//! Multi-channel chunks collapse to a single bar series — every
//! sample in the interleaved buffer counts toward the same bucket.
//! That matches dope-sheet rendering (one row per audio track, not
//! per channel) and keeps the math + tests simple. M-MEDIA.9
//! (geometry) handles mono vs stereo display modes.

use crate::audio::AudioChunk;
use crate::clock::{MediaDuration, MediaTime};

/// One bar in an [`AudioHistogram`] — a quantized window of audio
/// summarized by its peak + RMS.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioBar {
    /// When this bar's window starts on the media timeline.
    pub start_time: MediaTime,
    /// Window duration.
    pub duration: MediaDuration,
    /// Maximum `|sample|` over the window. `[0, 1]` for normalized audio.
    pub peak: f32,
    /// Root-mean-square over the window. `[0, 1]` for normalized audio.
    pub rms: f32,
}

/// A list of [`AudioBar`]s with a uniform `bucket_duration`.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioHistogram {
    /// Width of each bucket on the media timeline.
    pub bucket_duration: MediaDuration,
    /// The bars, in increasing `start_time` order.
    pub bars: Vec<AudioBar>,
}

impl AudioHistogram {
    /// Convenience: number of bars.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bars.len()
    }

    /// Convenience: true when no bars are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }
}

/// Quantize a single [`AudioChunk`] into uniform-width bars.
///
/// The chunk's `pts` is the start of the first bar. Each bar covers
/// `bucket_duration` of audio. The final bar may be shorter if the
/// chunk doesn't divide evenly — it carries the actual remainder
/// duration (not `bucket_duration`) so the timeline math stays exact.
///
/// # Panics
///
/// Panics if `bucket_duration` is zero or negative.
#[must_use]
pub fn quantize(chunk: &AudioChunk, bucket_duration: MediaDuration) -> AudioHistogram {
    assert!(
        bucket_duration.as_nanos() > 0,
        "bucket_duration must be positive"
    );

    let fmt = chunk.format();
    let channels = usize::from(fmt.channels);
    let samples = chunk.samples();
    let total_frames = chunk.frame_count();

    // Frames per bucket = bucket_duration * sample_rate.
    let frames_per_bucket = bucket_duration
        .as_seconds()
        .mul_add(f64::from(fmt.sample_rate), 0.0)
        .round();
    assert!(
        frames_per_bucket >= 1.0,
        "bucket_duration is shorter than one sample at sample_rate={}",
        fmt.sample_rate
    );
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "frames_per_bucket guaranteed >= 1.0 above; ≤ realistic frame counts fit usize"
    )]
    let frames_per_bucket = frames_per_bucket as usize;

    if total_frames == 0 {
        return AudioHistogram {
            bucket_duration,
            bars: Vec::new(),
        };
    }

    let bar_count = total_frames.div_ceil(frames_per_bucket);
    let mut bars = Vec::with_capacity(bar_count);
    let chunk_start = chunk.pts();

    for bar_idx in 0..bar_count {
        let frame_lo = bar_idx * frames_per_bucket;
        let frame_hi = ((bar_idx + 1) * frames_per_bucket).min(total_frames);
        let sample_lo = frame_lo * channels;
        let sample_hi = frame_hi * channels;
        let window = &samples[sample_lo..sample_hi];

        let (peak, rms) = if window.is_empty() {
            (0.0_f32, 0.0_f32)
        } else {
            let peak = window
                .iter()
                .copied()
                .fold(0.0_f32, |acc, x| acc.max(x.abs()));
            let sum_sq: f64 = window.iter().map(|&x| f64::from(x).powi(2)).sum();
            #[expect(
                clippy::cast_precision_loss,
                reason = "window length above 2^53 is not realistic for one bar"
            )]
            let mean = sum_sq / (window.len() as f64);
            #[expect(clippy::cast_possible_truncation, reason = "RMS in [0, 1] fits f32")]
            let rms = mean.sqrt() as f32;
            (peak, rms)
        };

        let start_offset =
            MediaTime::from_sample(frame_lo as u64, fmt.sample_rate) - MediaTime::ZERO;
        let actual_duration = MediaTime::from_sample(frame_hi as u64, fmt.sample_rate)
            - MediaTime::from_sample(frame_lo as u64, fmt.sample_rate);

        bars.push(AudioBar {
            start_time: chunk_start + start_offset,
            duration: actual_duration,
            peak,
            rms,
        });
    }

    AudioHistogram {
        bucket_duration,
        bars,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::AudioFormat;
    use crate::mock_audio::{SilenceSource, SineWaveSource, StepPulseSource};

    #[test]
    fn silence_chunk_produces_zero_amplitude_bars() {
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = SilenceSource::new(fmt);
        let chunk = src.next_chunk(48_000); // 1 s
        let h = quantize(&chunk, MediaDuration::from_millis(50));
        assert_eq!(h.len(), 20); // 1 s / 50 ms = 20
        for bar in &h.bars {
            assert!(
                bar.peak.abs() < f32::EPSILON,
                "expected peak ~0, got {}",
                bar.peak
            );
            assert!(
                bar.rms.abs() < f32::EPSILON,
                "expected rms ~0, got {}",
                bar.rms
            );
        }
    }

    #[test]
    fn sine_chunk_produces_stable_rms_near_amp_over_sqrt_2() {
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = SineWaveSource::new(fmt, 1_000.0, 0.6);
        let chunk = src.next_chunk(48_000); // 1 s
        let h = quantize(&chunk, MediaDuration::from_millis(50));
        assert_eq!(h.len(), 20);
        let expected = 0.6 / 2.0_f32.sqrt();
        // Skip the first and last bars (boundary cycles may have less coverage).
        for (i, bar) in h.bars.iter().enumerate().skip(1).take(18) {
            assert!(
                (bar.rms - expected).abs() < 0.05,
                "bar {i} rms {} expected ~{expected}",
                bar.rms,
            );
        }
    }

    #[test]
    fn pulse_chunk_produces_one_bar_with_peak_one() {
        let fmt = AudioFormat::mono_f32(48_000);
        // Spike at frame 1200 → 25 ms in. With 50 ms buckets, that's
        // bar index 0.
        let mut src = StepPulseSource::new(fmt, 1_200);
        let chunk = src.next_chunk(48_000);
        let h = quantize(&chunk, MediaDuration::from_millis(50));
        assert!((h.bars[0].peak - 1.0).abs() < f32::EPSILON);
        for bar in &h.bars[1..] {
            assert!(bar.peak.abs() < f32::EPSILON);
        }
    }

    #[test]
    fn bucket_count_matches_duration_at_20ms() {
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = SilenceSource::new(fmt);
        let chunk = src.next_chunk(48_000); // 1 s
        let h = quantize(&chunk, MediaDuration::from_millis(20));
        assert_eq!(h.len(), 50); // 1 s / 20 ms = 50
    }

    #[test]
    fn bucket_count_matches_duration_at_10ms() {
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = SilenceSource::new(fmt);
        let chunk = src.next_chunk(48_000); // 1 s
        let h = quantize(&chunk, MediaDuration::from_millis(10));
        assert_eq!(h.len(), 100); // 1 s / 10 ms = 100
    }

    #[test]
    fn bucket_count_matches_duration_at_50ms() {
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = SilenceSource::new(fmt);
        let chunk = src.next_chunk(48_000); // 1 s
        let h = quantize(&chunk, MediaDuration::from_millis(50));
        assert_eq!(h.len(), 20);
    }

    #[test]
    fn empty_chunk_produces_zero_bars() {
        let fmt = AudioFormat::mono_f32(48_000);
        let chunk = AudioChunk::new(fmt, vec![], MediaTime::ZERO).unwrap();
        let h = quantize(&chunk, MediaDuration::from_millis(20));
        assert!(h.is_empty());
    }

    #[test]
    fn bar_timestamps_are_contiguous_and_start_at_chunk_pts() {
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = SineWaveSource::new(fmt, 440.0, 0.5);
        let chunk = src.next_chunk(48_000);
        let h = quantize(&chunk, MediaDuration::from_millis(50));
        // First bar starts at the chunk's PTS (0 here).
        assert_eq!(h.bars[0].start_time, chunk.pts());
        // Successive bars are contiguous: bar[i+1].start = bar[i].start + bar[i].duration.
        for i in 0..(h.bars.len() - 1) {
            let next_expected = h.bars[i].start_time + h.bars[i].duration;
            assert_eq!(
                h.bars[i + 1].start_time,
                next_expected,
                "gap or overlap between bar {i} and {}",
                i + 1
            );
        }
    }

    #[test]
    fn stereo_chunk_collapses_channels_into_single_bar_series() {
        let fmt = AudioFormat::stereo_f32(48_000);
        let mut src = SineWaveSource::new(fmt, 440.0, 0.6);
        let chunk = src.next_chunk(48_000);
        let h = quantize(&chunk, MediaDuration::from_millis(50));
        assert_eq!(h.len(), 20);
        // RMS should still be ≈ 0.6 / √2 because L == R from the
        // SineWaveSource — averaging doesn't change it.
        let expected = 0.6 / 2.0_f32.sqrt();
        for (i, bar) in h.bars.iter().enumerate().skip(1).take(18) {
            assert!(
                (bar.rms - expected).abs() < 0.05,
                "stereo bar {i} rms {} expected ~{expected}",
                bar.rms,
            );
        }
    }

    #[test]
    fn types_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AudioBar>();
        assert_send_sync::<AudioHistogram>();
    }
}
