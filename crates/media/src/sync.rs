//! Audio/video sync harness (M-MEDIA.7 / AUT-103).
//!
//! Combines [`GstreamerAudioCapture`](super::gstreamer_audio::GstreamerAudioCapture)
//! and [`GstreamerVideoCapture`](super::gstreamer_video::GstreamerVideoCapture)
//! into one harness that reports per-stream timing and inter-stream
//! drift. Synthetic [`audiotestsrc`] + [`videotestsrc`] sources keep
//! the assertion deterministic — live capture (M-MEDIA.15 / .16) will
//! reuse the same harness shape but with `autoaudiosrc` / `autovideosrc`.
//!
//! # What "drift" means here
//!
//! Each stream stamps its own PTS via [`MediaTime::from_sample`] /
//! [`MediaTime::from_frame`]. For synthetic sources, both PTS values
//! are derived from per-stream counters at construction-time rates, so
//! the per-stream PTS *is* the timeline. We report:
//!
//! - `first_audio_pts` / `first_video_pts` (≈ 0 s for synthetic
//!   sources).
//! - `last_audio_pts` / `last_video_pts` (= sample_count / rate and
//!   frame_count / fps respectively).
//! - `drift` = `|last_audio_pts - last_video_pts|`. For synthetic
//!   sources this is near-zero by construction; for live sources it
//!   reports the underlying clock disagreement.

use crate::audio::AudioFormat;
use crate::clock::{MediaDuration, MediaTime};
use crate::gstreamer_audio::{self, GstreamerAudioCapture};
use crate::gstreamer_video::{self, GstreamerVideoCapture};

/// Failure modes for the sync harness.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Audio-side spawn / read error.
    #[error(transparent)]
    Audio(#[from] gstreamer_audio::Error),
    /// Video-side spawn / read error.
    #[error(transparent)]
    Video(#[from] gstreamer_video::Error),
}

/// Tunable parameters for one harness run.
#[derive(Debug, Clone, Copy)]
pub struct SyncConfig {
    /// Audio sample rate + channel count.
    pub audio_format: AudioFormat,
    /// Sine frequency emitted by the audio test source.
    pub audio_frequency_hz: f32,
    /// Video frame dimensions.
    pub video_width: u32,
    /// Video frame dimensions.
    pub video_height: u32,
    /// Video framerate (fps).
    pub video_framerate: u32,
    /// Per-chunk audio frame count (e.g. 4_800 = 100 ms at 48 kHz).
    pub audio_chunk_frames: u64,
    /// Total wall-clock duration to capture.
    pub duration: MediaDuration,
}

impl SyncConfig {
    /// Default deterministic harness: 48 kHz mono audio + 64×36 30 fps
    /// video for 1 second.
    #[must_use]
    pub fn deterministic_1s() -> Self {
        Self {
            audio_format: AudioFormat::mono_f32(48_000),
            audio_frequency_hz: 440.0,
            video_width: 64,
            video_height: 36,
            video_framerate: 30,
            audio_chunk_frames: 4_800, // 100 ms
            duration: MediaDuration::from_seconds(1.0),
        }
    }
}

/// Result of a single [`run`] capture.
#[derive(Debug, Clone, Copy)]
pub struct SyncReport {
    /// Cumulative audio frames captured.
    pub audio_frames: u64,
    /// Cumulative video frames captured.
    pub video_frames: u64,
    /// PTS of the first audio chunk's first sample.
    pub first_audio_pts: MediaTime,
    /// PTS of the first video frame.
    pub first_video_pts: MediaTime,
    /// PTS of the last audio chunk's first sample.
    pub last_audio_pts: MediaTime,
    /// PTS of the last video frame.
    pub last_video_pts: MediaTime,
    /// `|last_audio_pts - last_video_pts|`. For synthetic test sources
    /// this is near-zero by construction.
    pub drift: MediaDuration,
}

impl SyncReport {
    /// True when the inter-stream drift is within `tolerance`.
    #[must_use]
    pub fn drift_within(&self, tolerance: MediaDuration) -> bool {
        self.drift.as_nanos().abs() <= tolerance.as_nanos().abs()
    }
}

impl std::fmt::Display for SyncReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SyncReport(audio_frames={af} video_frames={vf} \
             first_audio={fa:.6}s first_video={fv:.6}s \
             last_audio={la:.6}s last_video={lv:.6}s \
             drift={d:.6}s)",
            af = self.audio_frames,
            vf = self.video_frames,
            fa = self.first_audio_pts.as_seconds(),
            fv = self.first_video_pts.as_seconds(),
            la = self.last_audio_pts.as_seconds(),
            lv = self.last_video_pts.as_seconds(),
            d = self.drift.as_seconds(),
        )
    }
}

/// Run one capture cycle. Spawns both audio + video sources, pulls
/// audio chunks at `audio_chunk_frames` granularity and one video
/// frame per video tick until `duration` is covered. Returns the
/// summary [`SyncReport`].
///
/// # Errors
///
/// - [`Error::Audio`] / [`Error::Video`] on spawn or read failures.
pub fn run(config: SyncConfig) -> Result<SyncReport, Error> {
    let mut audio =
        GstreamerAudioCapture::test_source(config.audio_format, config.audio_frequency_hz)?;
    let mut video = GstreamerVideoCapture::test_source(
        config.video_width,
        config.video_height,
        config.video_framerate,
    )?;

    let duration_seconds = config.duration.as_seconds();
    let target_audio_frames =
        seconds_to_count(duration_seconds, f64::from(config.audio_format.sample_rate));
    let target_video_frames = seconds_to_count(duration_seconds, f64::from(config.video_framerate));

    let mut first_audio_pts: Option<MediaTime> = None;
    let mut last_audio_pts = MediaTime::ZERO;
    let mut audio_frames_total: u64 = 0;
    while audio_frames_total < target_audio_frames {
        let want = config
            .audio_chunk_frames
            .min(target_audio_frames - audio_frames_total);
        let chunk = audio.next_chunk(want)?;
        if first_audio_pts.is_none() {
            first_audio_pts = Some(chunk.pts());
        }
        last_audio_pts = chunk.pts();
        audio_frames_total += want;
    }

    let mut first_video_pts: Option<MediaTime> = None;
    let mut last_video_pts = MediaTime::ZERO;
    let mut video_frames_total: u64 = 0;
    while video_frames_total < target_video_frames {
        let frame = video.next_frame()?;
        let pts = MediaTime::from_seconds(frame.pts_seconds);
        if first_video_pts.is_none() {
            first_video_pts = Some(pts);
        }
        last_video_pts = pts;
        video_frames_total += 1;
    }

    let drift = if last_audio_pts > last_video_pts {
        last_audio_pts - last_video_pts
    } else {
        last_video_pts - last_audio_pts
    };

    Ok(SyncReport {
        audio_frames: audio_frames_total,
        video_frames: video_frames_total,
        first_audio_pts: first_audio_pts.unwrap_or(MediaTime::ZERO),
        first_video_pts: first_video_pts.unwrap_or(MediaTime::ZERO),
        last_audio_pts,
        last_video_pts,
        drift,
    })
}

/// Convert `seconds × rate` to a `u64` count, rounding to nearest.
/// Both inputs are finite + non-negative in normal use; clamps to 0 on
/// anything else.
fn seconds_to_count(seconds: f64, rate: f64) -> u64 {
    let n = (seconds * rate).round();
    if !n.is_finite() || n < 0.0 {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "seconds×rate values for a realistic capture stay well below 2^53 (~9e15) and are non-negative — pre-checked above"
    )]
    let v = n as u64;
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_drift_within_compares_absolute_nanos() {
        let r = SyncReport {
            audio_frames: 0,
            video_frames: 0,
            first_audio_pts: MediaTime::ZERO,
            first_video_pts: MediaTime::ZERO,
            last_audio_pts: MediaTime::ZERO,
            last_video_pts: MediaTime::ZERO,
            drift: MediaDuration::from_millis(5),
        };
        assert!(r.drift_within(MediaDuration::from_millis(10)));
        assert!(!r.drift_within(MediaDuration::from_millis(1)));
    }

    #[test]
    fn deterministic_1s_targets_match_expected_counts() {
        let cfg = SyncConfig::deterministic_1s();
        let target_a = seconds_to_count(
            cfg.duration.as_seconds(),
            f64::from(cfg.audio_format.sample_rate),
        );
        let target_v = seconds_to_count(cfg.duration.as_seconds(), f64::from(cfg.video_framerate));
        assert_eq!(target_a, 48_000);
        assert_eq!(target_v, 30);
    }
}
