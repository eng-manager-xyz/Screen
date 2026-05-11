//! Integration test for `media::sync` — runs both `audiotestsrc` +
//! `videotestsrc` `GStreamer` captures for a fixed wall-clock duration
//! and verifies the per-stream PTS timeline (M-MEDIA.7 / AUT-103).
//!
//! Skip-guarded via `media::gstreamer::is_available` so machines
//! without `GStreamer` don't fail the gate.

use media::clock::{MediaDuration, MediaTime};
use media::gstreamer::is_available;
use media::sync::{SyncConfig, run};

#[test]
fn deterministic_1s_capture_yields_expected_frame_counts() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    let cfg = SyncConfig::deterministic_1s();
    let report = run(cfg).expect("sync harness");
    eprintln!("{report}");
    assert_eq!(report.audio_frames, 48_000);
    assert_eq!(report.video_frames, 30);
}

#[test]
fn deterministic_1s_first_pts_are_aligned_within_one_audio_chunk() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    let cfg = SyncConfig::deterministic_1s();
    let report = run(cfg).expect("sync harness");
    // First audio PTS is 0 (first chunk). First video PTS is 0 (first
    // frame). They should be within one audio-chunk duration of each
    // other (100 ms here).
    let delta = if report.first_audio_pts > report.first_video_pts {
        report.first_audio_pts - report.first_video_pts
    } else {
        report.first_video_pts - report.first_audio_pts
    };
    assert!(
        delta.as_nanos().abs() < MediaDuration::from_millis(100).as_nanos(),
        "first PTS not aligned: audio={}, video={}",
        report.first_audio_pts.as_seconds(),
        report.first_video_pts.as_seconds(),
    );
}

#[test]
fn deterministic_1s_drift_is_below_one_frame() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    let cfg = SyncConfig::deterministic_1s();
    let report = run(cfg).expect("sync harness");
    // Both streams stamp PTS from their own counter. last_audio_pts =
    // (48_000 - audio_chunk_frames) / 48_000 (PTS is first sample of
    // last chunk). last_video_pts = 29 / 30 ≈ 0.967. Drift should be
    // bounded.
    let tolerance = MediaDuration::from_seconds(1.0 / 25.0); // < one 25 fps frame
    assert!(
        report.drift_within(tolerance),
        "drift {} exceeds tolerance: {report}",
        report.drift.as_seconds(),
    );
}

#[test]
fn last_pts_values_are_below_capture_duration() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    let cfg = SyncConfig::deterministic_1s();
    let report = run(cfg).expect("sync harness");
    // We captured 1 s of data, so the last PTS should be <= 1 s.
    let one_second = MediaTime::from_seconds(1.0);
    assert!(report.last_audio_pts <= one_second);
    assert!(report.last_video_pts <= one_second);
}
