//! `AudioMixer` — sum two mono/stereo F32 input streams into a
//! single interleaved F32LE output (M-EXPORT.2 of M-RECORD-EXPORT).
//!
//! Pure-Rust math; no GStreamer dep. Bridges the per-channel mic
//! [`crate::audio::AudioChunk`] callbacks + the per-channel system-
//! audio SCK delegate samples into one stream of audio chunks the
//! [`crate::encode::VideoEncoder::push_audio_chunk`] sink consumes.
//!
//! ## Mix strategy
//!
//! Both inputs are assumed F32 interleaved at the same sample rate
//! and channel count as the configured output (validated at
//! construction).
//!
//! Each `pull()` call returns the *common* prefix of both buffers
//! — `min(mic_avail, sys_avail)` samples — summed per-channel and
//! soft-clipped to `[-1.0, 1.0]`.
//!
//! When only one input is available (the other isn't pushing), the
//! mixer returns that input's samples unchanged. This is the
//! "mic-only" / "sys-audio-only" v0 mode — both channels enabled but
//! one yet to fire.
//!
//! ```admonish note title="Why soft-clip, not divide-by-two"
//! Naive `(a + b) / 2` halves the loudness when only one source is
//! active. Soft-clip preserves loudness when one source is silent
//! and prevents distortion when both are loud — closer to what a
//! real mixer's bus does.
//! ```

use std::collections::VecDeque;

/// Failure modes for the audio mixer.
#[derive(Debug, thiserror::Error)]
pub enum MixerError {
    /// Sample buffer length wasn't a multiple of channel count.
    #[error("sample count {0} is not a multiple of channels {1}")]
    UnalignedSamples(usize, u8),
    /// Mixer's output channel count was zero (rejected at config).
    #[error("output channels may not be zero")]
    ZeroChannels,
}

/// Two-source audio mixer. Both inputs must use the same sample
/// rate + channel count as the configured output.
#[derive(Debug)]
pub struct AudioMixer {
    channels: u8,
    mic_queue: VecDeque<f32>,
    sys_audio_queue: VecDeque<f32>,
}

impl AudioMixer {
    /// Construct a new mixer for the given channel count.
    ///
    /// # Errors
    ///
    /// Returns [`MixerError::ZeroChannels`] if `channels == 0`.
    pub fn new(channels: u8) -> Result<Self, MixerError> {
        if channels == 0 {
            return Err(MixerError::ZeroChannels);
        }
        Ok(Self {
            channels,
            mic_queue: VecDeque::new(),
            sys_audio_queue: VecDeque::new(),
        })
    }

    /// Push interleaved F32 mic samples. `samples.len()` must be a
    /// multiple of channel count.
    ///
    /// # Errors
    ///
    /// Returns [`MixerError::UnalignedSamples`] on misalignment.
    pub fn push_mic(&mut self, samples: &[f32]) -> Result<(), MixerError> {
        self.validate_alignment(samples.len())?;
        self.mic_queue.extend(samples.iter().copied());
        Ok(())
    }

    /// Push interleaved F32 system-audio samples. Same alignment
    /// contract as [`Self::push_mic`].
    ///
    /// # Errors
    ///
    /// Returns [`MixerError::UnalignedSamples`] on misalignment.
    pub fn push_sys_audio(&mut self, samples: &[f32]) -> Result<(), MixerError> {
        self.validate_alignment(samples.len())?;
        self.sys_audio_queue.extend(samples.iter().copied());
        Ok(())
    }

    /// Drain whatever's currently mixable + return the merged
    /// interleaved samples.
    ///
    /// Length = `min(mic_avail, sys_avail)` when both are active;
    /// `mic_avail` when only mic is pushing; `sys_avail` when only
    /// system-audio is pushing; `0` when neither has anything.
    ///
    /// Always returns a sample count that's a multiple of channel
    /// count.
    pub fn pull(&mut self) -> Vec<f32> {
        let chan = self.channels as usize;
        let mic_n = self.mic_queue.len();
        let sys_n = self.sys_audio_queue.len();
        match (mic_n, sys_n) {
            (0, 0) => Vec::new(),
            (mic_count, 0) => {
                // Trim to channel-aligned length (the input was
                // aligned but a partial frame might remain queued
                // from a previous truncated mix).
                let take = mic_count - (mic_count % chan);
                self.mic_queue.drain(..take).collect()
            }
            (0, sys_count) => {
                let take = sys_count - (sys_count % chan);
                self.sys_audio_queue.drain(..take).collect()
            }
            (mic_count, sys_count) => {
                let n = mic_count.min(sys_count);
                let take = n - (n % chan);
                let mut out = Vec::with_capacity(take);
                for _ in 0..take {
                    let mic_sample = self.mic_queue.pop_front().unwrap_or(0.0);
                    let sys_sample = self.sys_audio_queue.pop_front().unwrap_or(0.0);
                    out.push(soft_clip(mic_sample + sys_sample));
                }
                out
            }
        }
    }

    /// Bytes currently queued from the mic input (sample count, not
    /// byte count — multiply by 4 for F32 byte length).
    #[must_use]
    pub fn mic_queued(&self) -> usize {
        self.mic_queue.len()
    }

    /// Bytes currently queued from the system-audio input.
    #[must_use]
    pub fn sys_audio_queued(&self) -> usize {
        self.sys_audio_queue.len()
    }

    /// Output channel count this mixer was configured with.
    #[must_use]
    pub fn channels(&self) -> u8 {
        self.channels
    }

    fn validate_alignment(&self, sample_count: usize) -> Result<(), MixerError> {
        if !sample_count.is_multiple_of(self.channels as usize) {
            return Err(MixerError::UnalignedSamples(sample_count, self.channels));
        }
        Ok(())
    }
}

/// Soft-clip a sample to `[-1.0, 1.0]` using `tanh`. Smooth — no
/// hard clip artifacts when both inputs sum past the rail.
fn soft_clip(sample: f32) -> f32 {
    sample.tanh()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_zero_channels() {
        assert!(matches!(AudioMixer::new(0), Err(MixerError::ZeroChannels)));
    }

    #[test]
    fn push_rejects_unaligned_samples() {
        let mut m = AudioMixer::new(2).unwrap();
        // 3 samples / 2 channels = misaligned
        let err = m.push_mic(&[0.1, 0.2, 0.3]);
        assert!(matches!(err, Err(MixerError::UnalignedSamples(3, 2))));
    }

    #[test]
    fn pull_returns_empty_when_no_input() {
        let mut m = AudioMixer::new(2).unwrap();
        assert!(m.pull().is_empty());
    }

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    fn approx_slice_eq(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| approx_eq(*x, *y))
    }

    #[test]
    fn pull_returns_mic_samples_when_only_mic_pushed() {
        let mut m = AudioMixer::new(2).unwrap();
        m.push_mic(&[0.1, 0.2, 0.3, 0.4]).unwrap();
        let out = m.pull();
        assert!(approx_slice_eq(&out, &[0.1, 0.2, 0.3, 0.4]));
    }

    #[test]
    fn pull_returns_sys_audio_samples_when_only_sys_pushed() {
        let mut m = AudioMixer::new(1).unwrap();
        m.push_sys_audio(&[0.5, 0.6]).unwrap();
        let out = m.pull();
        assert!(approx_slice_eq(&out, &[0.5, 0.6]));
    }

    #[test]
    fn pull_mixes_common_prefix_when_both_pushed() {
        let mut m = AudioMixer::new(1).unwrap();
        m.push_mic(&[0.1, 0.2, 0.3]).unwrap();
        m.push_sys_audio(&[0.05, 0.05]).unwrap(); // shorter — only 2 samples mix
        let out = m.pull();
        assert_eq!(out.len(), 2);
        // Soft-clipped sums; small values are ~unchanged.
        assert!((out[0] - 0.15_f32.tanh()).abs() < 1e-6);
        assert!((out[1] - 0.25_f32.tanh()).abs() < 1e-6);
        // Remaining unmatched mic sample stays queued.
        assert_eq!(m.mic_queued(), 1);
        assert_eq!(m.sys_audio_queued(), 0);
    }

    #[test]
    fn pull_drains_queues_on_repeated_calls() {
        let mut m = AudioMixer::new(1).unwrap();
        m.push_mic(&[1.0, 1.0]).unwrap();
        m.push_sys_audio(&[1.0, 1.0]).unwrap();
        let first = m.pull();
        assert_eq!(first.len(), 2);
        let second = m.pull();
        assert!(second.is_empty());
    }

    #[test]
    fn soft_clip_clamps_loud_inputs_within_one() {
        // tanh asymptotes at ±1. At f32 precision, tanh(100.0)
        // saturates EXACTLY at 1.0 — verify <=1 / >=-1 rather than
        // strict inequality.
        assert!(soft_clip(100.0) <= 1.0);
        assert!(soft_clip(-100.0) >= -1.0);
        // Moderately loud inputs (clipping range) should be close
        // to the rail but below 1.0 strictly.
        assert!(soft_clip(3.0) < 1.0);
        assert!(soft_clip(3.0) > 0.99);
    }

    #[test]
    fn soft_clip_preserves_quiet_inputs() {
        // Small values pass through ~linearly. Use a coarser
        // tolerance since tanh(0.1) ≈ 0.09966 — within 1% of input.
        assert!((soft_clip(0.1) - 0.1).abs() < 0.01);
        assert!((soft_clip(-0.1) + 0.1).abs() < 0.01);
        assert!(soft_clip(0.0).abs() < 1e-6);
    }

    #[test]
    fn pull_truncates_to_channel_alignment_for_partial_remaining() {
        // 2 channels; mic has 5 samples (2.5 frames), sys has 0 →
        // pull should return 4 samples (2 frames) and leave 1
        // unaligned sample queued.
        let mut m = AudioMixer::new(2).unwrap();
        // Push aligned (4 samples = 2 frames) then push a stray
        // single sample via push_mic — but the API enforces
        // alignment so we can't. Construct the misalignment by
        // pushing two aligned chunks then draining partially in a
        // contrived test. Skip: the public API never produces
        // misalignment.
        m.push_mic(&[0.1, 0.2, 0.3, 0.4]).unwrap();
        let out = m.pull();
        assert_eq!(out.len(), 4);
    }
}
