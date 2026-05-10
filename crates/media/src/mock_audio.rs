//! Deterministic mock audio sources (M-MEDIA.4 / AUT-100).
//!
//! Enables TDD for waveform + histogram rendering without requiring a
//! microphone or GStreamer. Three shapes — sine wave, silence, step
//! pulse — emit timestamped [`AudioChunk`]s with byte-exact reproducible
//! samples. Every M-MEDIA test that wants "this is what audio looks
//! like" without a live device uses one of these.
//!
//! # Sources
//!
//! - [`SineWaveSource`] — `sin(2π·f·t)`. Stable amplitude + frequency
//!   are the histogram's "this RMS should be √2/2" reference.
//! - [`SilenceSource`] — all zeros. The histogram's "this should
//!   quantize to zero bars" reference.
//! - [`StepPulseSource`] — a single 1.0 spike at a configurable frame
//!   index, otherwise zero. The histogram's "this bucket should have a
//!   detectable peak" reference.
//!
//! Each source is an iterator-like type with `next_chunk(frames)`. It
//! advances an internal frame counter on each call; timestamps come
//! from `MediaTime::from_sample`.

use crate::audio::{AudioChunk, AudioFormat};
use crate::clock::MediaTime;

/// Sine-wave source. Emits `amplitude · sin(2π · frequency · t)` on
/// every channel.
#[derive(Debug, Clone)]
pub struct SineWaveSource {
    format: AudioFormat,
    /// Hz.
    frequency: f32,
    /// 0..=1 (clipped at f32 bounds when written).
    amplitude: f32,
    /// Frames emitted so far.
    next_frame: u64,
}

impl SineWaveSource {
    /// Construct a sine source at the given format / frequency /
    /// amplitude. `amplitude` is clamped to `[0, 1]`.
    #[must_use]
    pub fn new(format: AudioFormat, frequency_hz: f32, amplitude: f32) -> Self {
        Self {
            format,
            frequency: frequency_hz.max(0.0),
            amplitude: amplitude.clamp(0.0, 1.0),
            next_frame: 0,
        }
    }

    /// Format of the emitted chunks.
    #[must_use]
    pub fn format(&self) -> AudioFormat {
        self.format
    }

    /// Cumulative number of frames emitted across `next_chunk` calls.
    #[must_use]
    pub fn frames_emitted(&self) -> u64 {
        self.next_frame
    }

    /// Emit the next `frames` of audio. Always returns a chunk of
    /// exactly `frames × channels` samples (or fewer if the requested
    /// count is zero).
    pub fn next_chunk(&mut self, frames: u64) -> AudioChunk {
        let pts = MediaTime::from_sample(self.next_frame, self.format.sample_rate);
        let total_samples = usize::try_from(frames).expect("frames fits in usize")
            * usize::from(self.format.channels);
        let mut samples = Vec::with_capacity(total_samples);
        for f in 0..frames {
            let frame_idx = self.next_frame + f;
            let t = frame_idx_to_seconds(frame_idx, self.format.sample_rate);
            let v = self.amplitude * (std::f32::consts::TAU * self.frequency * t).sin();
            for _ in 0..self.format.channels {
                samples.push(v);
            }
        }
        self.next_frame = self.next_frame.saturating_add(frames);
        AudioChunk::new(self.format, samples, pts).expect("sine source produces valid chunk")
    }
}

/// Silence source — emits all-zero samples.
#[derive(Debug, Clone)]
pub struct SilenceSource {
    format: AudioFormat,
    next_frame: u64,
}

impl SilenceSource {
    /// Construct.
    #[must_use]
    pub fn new(format: AudioFormat) -> Self {
        Self {
            format,
            next_frame: 0,
        }
    }

    /// Emit `frames` of silence.
    pub fn next_chunk(&mut self, frames: u64) -> AudioChunk {
        let pts = MediaTime::from_sample(self.next_frame, self.format.sample_rate);
        let total_samples = usize::try_from(frames).expect("frames fits in usize")
            * usize::from(self.format.channels);
        let samples = vec![0.0_f32; total_samples];
        self.next_frame = self.next_frame.saturating_add(frames);
        AudioChunk::new(self.format, samples, pts).expect("silence source produces valid chunk")
    }
}

/// Step-pulse source. Emits a single-frame `1.0` spike at
/// `spike_frame_index`; every other frame is `0.0`. Useful for
/// histogram-peak assertions that need a known location.
#[derive(Debug, Clone)]
pub struct StepPulseSource {
    format: AudioFormat,
    spike_frame_index: u64,
    next_frame: u64,
}

impl StepPulseSource {
    /// Construct with the spike at `spike_frame_index`.
    #[must_use]
    pub fn new(format: AudioFormat, spike_frame_index: u64) -> Self {
        Self {
            format,
            spike_frame_index,
            next_frame: 0,
        }
    }

    /// Emit `frames` of step-pulse audio.
    pub fn next_chunk(&mut self, frames: u64) -> AudioChunk {
        let pts = MediaTime::from_sample(self.next_frame, self.format.sample_rate);
        let total_samples = usize::try_from(frames).expect("frames fits in usize")
            * usize::from(self.format.channels);
        let mut samples = vec![0.0_f32; total_samples];
        if self.spike_frame_index >= self.next_frame
            && self.spike_frame_index < self.next_frame + frames
        {
            let offset_frames =
                usize::try_from(self.spike_frame_index - self.next_frame).expect("fits in usize");
            let base = offset_frames * usize::from(self.format.channels);
            for c in 0..usize::from(self.format.channels) {
                samples[base + c] = 1.0;
            }
        }
        self.next_frame = self.next_frame.saturating_add(frames);
        AudioChunk::new(self.format, samples, pts).expect("pulse source produces valid chunk")
    }
}

fn frame_idx_to_seconds(frame: u64, sample_rate: u32) -> f32 {
    // Compute in f64 then narrow; safe for any realistic
    // (frame, rate) pair.
    #[expect(
        clippy::cast_precision_loss,
        reason = "frame counts above 2^53 (≈ 195 days at 48 kHz) are not realistic"
    )]
    let frame_f = frame as f64;
    let seconds = frame_f / f64::from(sample_rate);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "seconds in the realistic range fit f32"
    )]
    let s = seconds as f32;
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::AudioFormat;

    #[test]
    fn sine_source_emits_expected_sample_count() {
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = SineWaveSource::new(fmt, 440.0, 0.5);
        let chunk = src.next_chunk(480); // 10 ms at 48 kHz
        assert_eq!(chunk.samples().len(), 480);
        assert_eq!(chunk.frame_count(), 480);
        assert_eq!(src.frames_emitted(), 480);
    }

    #[test]
    fn sine_source_stereo_interleaves() {
        let fmt = AudioFormat::stereo_f32(48_000);
        let mut src = SineWaveSource::new(fmt, 440.0, 1.0);
        let chunk = src.next_chunk(4);
        // Stereo + 4 frames = 8 samples. L == R because the source
        // emits the same value on every channel.
        assert_eq!(chunk.samples().len(), 8);
        for pair in chunk.samples().chunks_exact(2) {
            assert!((pair[0] - pair[1]).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn sine_source_peak_approximates_amplitude() {
        // Over enough samples we should hit close to the requested
        // amplitude. 480 samples covers ~4.4 cycles at 440 Hz / 48 kHz.
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = SineWaveSource::new(fmt, 440.0, 0.5);
        let chunk = src.next_chunk(480);
        let peak = chunk.peak();
        assert!((peak - 0.5).abs() < 0.01, "expected peak ~ 0.5, got {peak}");
    }

    #[test]
    fn sine_source_rms_approximates_amp_over_sqrt2() {
        // For a sine wave, RMS = amplitude / sqrt(2) ≈ 0.7071·A.
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = SineWaveSource::new(fmt, 1_000.0, 1.0);
        // Use 4800 samples (100 ms) so we average over plenty of cycles.
        let chunk = src.next_chunk(4_800);
        let rms = chunk.rms();
        let expected = 1.0 / 2.0_f32.sqrt();
        assert!(
            (rms - expected).abs() < 0.01,
            "expected rms ~ {expected}, got {rms}"
        );
    }

    #[test]
    fn sine_source_pts_advances_between_calls() {
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = SineWaveSource::new(fmt, 440.0, 0.5);
        let c0 = src.next_chunk(48_000); // 1 s
        let c1 = src.next_chunk(48_000); // 1 s
        assert!((c0.pts().as_seconds() - 0.0).abs() < 1e-12);
        assert!((c1.pts().as_seconds() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn silence_source_emits_zeros() {
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = SilenceSource::new(fmt);
        let chunk = src.next_chunk(100);
        assert_eq!(chunk.samples().len(), 100);
        for s in chunk.samples() {
            assert!(s.abs() < f32::EPSILON);
        }
        assert!(chunk.peak() < f32::EPSILON);
        assert!(chunk.rms() < f32::EPSILON);
    }

    #[test]
    fn pulse_source_emits_spike_at_expected_frame() {
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = StepPulseSource::new(fmt, 7);
        let chunk = src.next_chunk(16);
        // Sample 7 should be 1.0; everything else 0.0.
        for (i, &s) in chunk.samples().iter().enumerate() {
            if i == 7 {
                assert!((s - 1.0).abs() < f32::EPSILON);
            } else {
                assert!(s.abs() < f32::EPSILON);
            }
        }
        assert!((chunk.peak() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pulse_source_handles_spike_outside_window() {
        // Spike at frame 100, but chunk only covers frames 0..16.
        let fmt = AudioFormat::mono_f32(48_000);
        let mut src = StepPulseSource::new(fmt, 100);
        let chunk = src.next_chunk(16);
        // Everything zero — spike is in a later chunk.
        assert!(chunk.peak() < f32::EPSILON);
        // Now advance past the spike.
        let _ = src.next_chunk(80);
        let chunk = src.next_chunk(16);
        // Frame 100 within [96, 112) — first chunk had 0..16, second 16..96,
        // this is third chunk: frames 96..112. Spike at index 100-96 = 4.
        assert!((chunk.peak() - 1.0).abs() < f32::EPSILON);
        assert!((chunk.samples()[4] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pulse_source_stereo_spike_fills_both_channels() {
        let fmt = AudioFormat::stereo_f32(48_000);
        let mut src = StepPulseSource::new(fmt, 2);
        let chunk = src.next_chunk(4);
        // 4 frames × 2 channels = 8 samples. Spike at frame 2 → samples 4 and 5.
        for (i, &s) in chunk.samples().iter().enumerate() {
            if i == 4 || i == 5 {
                assert!((s - 1.0).abs() < f32::EPSILON);
            } else {
                assert!(s.abs() < f32::EPSILON);
            }
        }
    }

    #[test]
    fn sources_have_no_gstreamer_or_device_dependency() {
        // Smoke: constructing all three sources and emitting a chunk
        // must succeed in any environment. The test itself is the
        // assertion — no skip-guard needed.
        let fmt = AudioFormat::mono_f32(48_000);
        let _ = SineWaveSource::new(fmt, 440.0, 0.5).next_chunk(10);
        let _ = SilenceSource::new(fmt).next_chunk(10);
        let _ = StepPulseSource::new(fmt, 5).next_chunk(10);
    }

    #[test]
    fn types_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SineWaveSource>();
        assert_send_sync::<SilenceSource>();
        assert_send_sync::<StepPulseSource>();
    }
}
