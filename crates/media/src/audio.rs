//! Audio sample + chunk data model (M-MEDIA.3 / AUT-99).
//!
//! Audio in this crate is **normalized `f32`** end-to-end: that's the
//! shape every downstream visualization (audio-histogram quantization in
//! M-MEDIA.8, waveform geometry in M-MEDIA.9) expects, and the shape
//! GStreamer's `audioconvert ! audio/x-raw,format=F32LE` produces
//! natively.
//!
//! # Types
//!
//! - [`AudioFormat`] — sample-rate + channel-count + sample-format
//!   triple.
//! - [`AudioChunk`] — a `Timestamped<Vec<f32>>` carrying its
//!   [`AudioFormat`] + a derived [`MediaDuration`].
//!
//! Interleave order is **planar-per-frame**: `[L₀, R₀, L₁, R₁, …]` for
//! stereo. Mono is just `[s₀, s₁, …]`. Matches the GStreamer raw audio
//! convention; matches what cpal / coreaudio emit too, so future device
//! capture won't need to re-layout buffers.
//!
//! # Quick start
//!
//! ```rust
//! use media::audio::{AudioChunk, AudioFormat, SampleFormat};
//! use media::clock::MediaTime;
//!
//! let fmt = AudioFormat::mono_f32(48_000);
//! let samples = vec![0.0_f32; 48_000]; // 1.0 s of silence at 48 kHz mono.
//! let chunk = AudioChunk::new(fmt, samples, MediaTime::ZERO).expect("valid");
//! assert!((chunk.duration().as_seconds() - 1.0).abs() < 1e-9);
//! ```

use crate::clock::{MediaDuration, MediaTime};

/// Internal sample format. Today every code path normalizes to
/// `F32`; the enum exists so external GStreamer / cpal capture paths
/// can declare their *input* layout before normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleFormat {
    /// 32-bit float, range `[-1.0, +1.0]` after normalization.
    F32,
    /// 16-bit signed integer.
    I16,
    /// 8-bit unsigned integer.
    U8,
}

/// Sample-rate + channels + sample-format triple. Describes the *layout*
/// of an [`AudioChunk`]'s sample buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioFormat {
    /// Samples per channel per second. 48 kHz is the recorder default;
    /// 44.1 kHz is common for music sources.
    pub sample_rate: u32,
    /// Channel count. 1 = mono, 2 = stereo. Higher counts are valid
    /// (5.1, 7.1) but unused for V1.
    pub channels: u8,
    /// On-disk / on-wire sample format. Normalized to `F32` inside
    /// [`AudioChunk::samples`].
    pub sample_format: SampleFormat,
}

impl AudioFormat {
    /// Common preset: 48 kHz mono F32 — the recorder's default
    /// microphone format.
    #[must_use]
    pub const fn mono_f32(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            channels: 1,
            sample_format: SampleFormat::F32,
        }
    }

    /// Common preset: 48 kHz stereo F32.
    #[must_use]
    pub const fn stereo_f32(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            channels: 2,
            sample_format: SampleFormat::F32,
        }
    }
}

/// Validation failure for [`AudioChunk::new`].
#[derive(Debug, thiserror::Error)]
pub enum AudioChunkError {
    /// Sample buffer length is not a multiple of the channel count.
    /// Each *frame* must carry one sample per channel.
    #[error(
        "audio chunk sample length {len} is not a multiple of channel count {channels} \
         (each frame must carry one sample per channel)"
    )]
    UnalignedSamples {
        /// `samples.len()`.
        len: usize,
        /// `format.channels`.
        channels: u8,
    },
    /// Channel count is zero — not a meaningful format.
    #[error("audio format channel count must be > 0")]
    ZeroChannels,
    /// Sample rate is zero — not a meaningful format.
    #[error("audio format sample rate must be > 0")]
    ZeroSampleRate,
}

/// One timestamped chunk of normalized `f32` audio.
///
/// Sample buffer is interleaved (planar-per-frame): `[L₀, R₀, L₁, R₁,
/// …]` for stereo. Mono is `[s₀, s₁, …]`. Duration is derived from
/// `samples.len() / channels / sample_rate`.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioChunk {
    format: AudioFormat,
    samples: Vec<f32>,
    pts: MediaTime,
    duration: MediaDuration,
}

impl AudioChunk {
    /// Validate + construct. Fails when channel count is zero, sample
    /// rate is zero, or `samples.len()` isn't a multiple of `channels`.
    pub fn new(
        format: AudioFormat,
        samples: Vec<f32>,
        pts: MediaTime,
    ) -> Result<Self, AudioChunkError> {
        if format.channels == 0 {
            return Err(AudioChunkError::ZeroChannels);
        }
        if format.sample_rate == 0 {
            return Err(AudioChunkError::ZeroSampleRate);
        }
        if !samples.len().is_multiple_of(usize::from(format.channels)) {
            return Err(AudioChunkError::UnalignedSamples {
                len: samples.len(),
                channels: format.channels,
            });
        }
        let frames = (samples.len() / usize::from(format.channels)) as u64;
        let duration = MediaTime::from_sample(frames, format.sample_rate) - MediaTime::ZERO;
        Ok(Self {
            format,
            samples,
            pts,
            duration,
        })
    }

    /// Format of the carried samples.
    #[must_use]
    pub fn format(&self) -> AudioFormat {
        self.format
    }

    /// Borrow the normalized `f32` sample buffer.
    #[must_use]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Number of *frames* (samples-per-channel) in the buffer.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.samples.len() / usize::from(self.format.channels)
    }

    /// Presentation timestamp — when this chunk's first sample
    /// occurs on the media timeline.
    #[must_use]
    pub fn pts(&self) -> MediaTime {
        self.pts
    }

    /// Derived duration of the chunk.
    #[must_use]
    pub fn duration(&self) -> MediaDuration {
        self.duration
    }

    /// Convenience: peak |sample| over the buffer. Used by histogram
    /// quantization (M-MEDIA.8) and capture-side regression checks.
    #[must_use]
    pub fn peak(&self) -> f32 {
        self.samples
            .iter()
            .copied()
            .fold(0.0_f32, |acc, x| acc.max(x.abs()))
    }

    /// Convenience: root-mean-square over the buffer. Used by
    /// histogram quantization.
    #[must_use]
    pub fn rms(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f64 = self.samples.iter().map(|&x| f64::from(x).powi(2)).sum();
        #[expect(
            clippy::cast_precision_loss,
            reason = "sample count above 2^53 is not realistic for one chunk"
        )]
        let mean = sum_sq / (self.samples.len() as f64);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "RMS in [0, 1] easily fits f32"
        )]
        let rms = mean.sqrt() as f32;
        rms
    }
}

/// Floor of the meter scale, in dBFS. Levels at or below this map to
/// 0.0; full-scale (`rms = 1.0`, i.e. `0 dBFS`) maps to 1.0. -60 dBFS
/// is a standard digital audio meter floor — anything below is barely
/// audible.
pub const METER_FLOOR_DBFS: f32 = -60.0;

/// Convert a linear RMS value (`[0, 1]`) into a UI-meter level
/// (`[0, 1]`) on a logarithmic dBFS scale, with `0 dBFS` mapping to
/// `1.0` and [`METER_FLOOR_DBFS`] mapping to `0.0`.
///
/// Linear RMS is a useless meter input because human loudness
/// perception (and microphone signal) is logarithmic — speech sits
/// around `-30 dBFS` (RMS ≈ 0.03), while the linear scale would map
/// the same speech to ~3 % of a 10-bar meter. The dBFS-mapped output
/// puts conversational speech around 50 % of the bar, leaving
/// headroom at both ends for whispers + shouts.
///
/// Indicative outputs (for the default `-60 dBFS` floor):
///
/// | Input RMS | dBFS    | Meter level |
/// | --------- | ------- | ----------- |
/// | 0.0       | -∞      | 0.00        |
/// | 0.001     | -60     | 0.00        |
/// | 0.003     | -50.5   | 0.16        |
/// | 0.01      | -40     | 0.33        |
/// | 0.03      | -30.5   | 0.49        |
/// | 0.1       | -20     | 0.67        |
/// | 0.3       | -10.5   | 0.83        |
/// | 1.0       | 0       | 1.00        |
#[must_use]
pub fn rms_to_meter_level(rms: f32) -> f32 {
    if !rms.is_finite() || rms <= 0.0 {
        return 0.0;
    }
    let db = 20.0 * rms.log10();
    if db <= METER_FLOOR_DBFS {
        return 0.0;
    }
    let level = (db - METER_FLOOR_DBFS) / -METER_FLOOR_DBFS;
    level.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_pts() -> MediaTime {
        MediaTime::from_seconds(0.5)
    }

    #[test]
    fn mono_48khz_one_second_round_trips() {
        let fmt = AudioFormat::mono_f32(48_000);
        let samples = vec![0.0_f32; 48_000]; // 1.0 s
        let chunk = AudioChunk::new(fmt, samples, dummy_pts()).unwrap();
        assert_eq!(chunk.frame_count(), 48_000);
        assert!((chunk.duration().as_seconds() - 1.0).abs() < 1e-9);
        assert_eq!(chunk.format().channels, 1);
        assert!((chunk.pts().as_seconds() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn stereo_48khz_half_second_round_trips() {
        let fmt = AudioFormat::stereo_f32(48_000);
        // 0.5 s at 48 kHz stereo = 24 000 frames × 2 channels = 48 000 samples.
        let samples = vec![0.0_f32; 48_000];
        let chunk = AudioChunk::new(fmt, samples, dummy_pts()).unwrap();
        assert_eq!(chunk.frame_count(), 24_000);
        assert!((chunk.duration().as_seconds() - 0.5).abs() < 1e-9);
        assert_eq!(chunk.format().channels, 2);
    }

    #[test]
    fn unaligned_samples_rejected_for_stereo() {
        let fmt = AudioFormat::stereo_f32(48_000);
        let samples = vec![0.0_f32; 481]; // odd → not multiple of 2
        let err = AudioChunk::new(fmt, samples, dummy_pts()).unwrap_err();
        assert!(
            matches!(
                err,
                AudioChunkError::UnalignedSamples {
                    len: 481,
                    channels: 2
                }
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn zero_channels_rejected() {
        let fmt = AudioFormat {
            sample_rate: 48_000,
            channels: 0,
            sample_format: SampleFormat::F32,
        };
        let err = AudioChunk::new(fmt, vec![], dummy_pts()).unwrap_err();
        assert!(matches!(err, AudioChunkError::ZeroChannels));
    }

    #[test]
    fn zero_sample_rate_rejected() {
        let fmt = AudioFormat {
            sample_rate: 0,
            channels: 1,
            sample_format: SampleFormat::F32,
        };
        let err = AudioChunk::new(fmt, vec![0.0; 100], dummy_pts()).unwrap_err();
        assert!(matches!(err, AudioChunkError::ZeroSampleRate));
    }

    #[test]
    fn empty_buffer_is_valid_zero_duration() {
        let fmt = AudioFormat::mono_f32(48_000);
        let chunk = AudioChunk::new(fmt, vec![], dummy_pts()).unwrap();
        assert_eq!(chunk.frame_count(), 0);
        assert_eq!(chunk.duration(), MediaDuration::ZERO);
    }

    #[test]
    fn peak_returns_max_abs_sample() {
        let fmt = AudioFormat::mono_f32(48_000);
        let samples = vec![0.1, -0.7, 0.3, -0.2];
        let chunk = AudioChunk::new(fmt, samples, dummy_pts()).unwrap();
        assert!((chunk.peak() - 0.7).abs() < 1e-6);
    }

    #[test]
    fn rms_is_zero_for_silence() {
        let fmt = AudioFormat::mono_f32(48_000);
        let chunk = AudioChunk::new(fmt, vec![0.0; 100], dummy_pts()).unwrap();
        assert!(chunk.rms().abs() < 1e-12);
    }

    #[test]
    fn rms_for_unit_constant_signal_is_unit() {
        // For a constant 1.0 signal, RMS = 1.0.
        let fmt = AudioFormat::mono_f32(48_000);
        let chunk = AudioChunk::new(fmt, vec![1.0; 100], dummy_pts()).unwrap();
        assert!((chunk.rms() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn presets_construct_correct_channels() {
        assert_eq!(AudioFormat::mono_f32(48_000).channels, 1);
        assert_eq!(AudioFormat::stereo_f32(48_000).channels, 2);
        assert_eq!(
            AudioFormat::mono_f32(48_000).sample_format,
            SampleFormat::F32
        );
    }

    #[test]
    fn types_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AudioFormat>();
        assert_send_sync::<AudioChunk>();
        assert_send_sync::<AudioChunkError>();
    }

    // ---- rms_to_meter_level ----

    /// `assert!`-friendly "near zero" check that satisfies clippy's
    /// `float_cmp` lint (which rejects `assert_eq!(x, 0.0)`).
    fn approx_zero(x: f32) -> bool {
        x.abs() < 1e-12
    }

    #[test]
    fn meter_level_zero_rms_is_zero() {
        assert!(approx_zero(rms_to_meter_level(0.0)));
    }

    #[test]
    fn meter_level_full_scale_rms_is_one() {
        // 0 dBFS → top of meter.
        assert!((rms_to_meter_level(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn meter_level_below_floor_clamps_to_zero() {
        // Anything quieter than -60 dBFS reads as 0 — far below
        // audibility for a typical recording.
        assert!(approx_zero(rms_to_meter_level(0.0001))); // -80 dBFS
        assert!(approx_zero(rms_to_meter_level(0.0009))); // ~-61 dBFS
    }

    #[test]
    fn meter_level_typical_speech_is_in_useful_mid_range() {
        // Conversational speech RMS sits around 0.03 (-30 dBFS),
        // which must land in the meaningful middle of the meter so
        // the user sees it move.
        let level = rms_to_meter_level(0.03);
        assert!(
            (0.4..=0.6).contains(&level),
            "expected speech RMS to map into 0.4..=0.6, got {level}"
        );
    }

    #[test]
    fn meter_level_monotonically_increases_with_rms() {
        let inputs = [0.0, 0.001, 0.005, 0.01, 0.05, 0.1, 0.3, 0.7, 1.0];
        let mut prev = -1.0;
        for r in inputs {
            let l = rms_to_meter_level(r);
            assert!(l >= prev, "non-monotonic at rms={r}: {l} < {prev}");
            prev = l;
        }
    }

    #[test]
    fn meter_level_rejects_nan_and_negative() {
        assert!(approx_zero(rms_to_meter_level(f32::NAN)));
        assert!(approx_zero(rms_to_meter_level(-0.5)));
        assert!(approx_zero(rms_to_meter_level(f32::NEG_INFINITY)));
    }

    #[test]
    fn meter_level_clamps_above_full_scale() {
        // Sustained-overdrive case: RMS theoretically capped at 1.0
        // for samples in [-1, 1], but EMA / mixer math could in
        // pathological cases give a slightly larger value. Verify
        // we clamp at 1.0 rather than running off the top of the
        // meter.
        assert!((rms_to_meter_level(2.0) - 1.0).abs() < 1e-6);
    }
}
