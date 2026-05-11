//! Integration tests for `media::gstreamer_audio` (M-MEDIA.5 / AUT-101).
//!
//! Runs the actual `gst-launch-1.0` pipeline. Skips gracefully when
//! `GStreamer` isn't on `PATH` so machines without it don't fail the
//! gate.

use std::path::Path;

use media::audio::AudioFormat;
use media::gstreamer::is_available;
use media::gstreamer_audio::GstreamerAudioCapture;

const FIXTURE_MP3: &str = "tests/fixtures/sample-audio.mp3";

#[test]
fn test_source_emits_chunks_with_expected_format_and_pts() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    let fmt = AudioFormat::mono_f32(48_000);
    let mut cap = GstreamerAudioCapture::test_source(fmt, 440.0).expect("spawn");

    // Read three chunks of 4800 frames (100 ms each).
    for i in 0..3 {
        let chunk = cap.next_chunk(4_800).expect("read chunk");
        assert_eq!(chunk.format(), fmt);
        assert_eq!(chunk.frame_count(), 4_800);
        assert!((chunk.duration().as_seconds() - 0.1).abs() < 1e-9);
        let expected_pts = f64::from(i) * 0.1;
        assert!(
            (chunk.pts().as_seconds() - expected_pts).abs() < 1e-9,
            "chunk {i} pts {} expected ~{expected_pts}",
            chunk.pts().as_seconds()
        );
    }
    assert_eq!(cap.frames_emitted(), 3 * 4_800);
}

#[test]
fn test_source_sine_440hz_has_rms_near_amp_over_sqrt_2() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    let fmt = AudioFormat::mono_f32(48_000);
    let mut cap = GstreamerAudioCapture::test_source(fmt, 440.0).expect("spawn");

    // 1 second of audio. audiotestsrc's default volume is 0.8.
    let chunk = cap.next_chunk(48_000).expect("read 1 s");
    let rms = chunk.rms();
    // audiotestsrc emits 0.8 * sin(...) by default → RMS = 0.8 / √2 ≈ 0.566.
    let expected = 0.8 / 2.0_f32.sqrt();
    assert!(
        (rms - expected).abs() < 0.02,
        "expected RMS ≈ {expected}, got {rms}"
    );
}

#[test]
fn test_source_stereo_interleaves_correctly() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    let fmt = AudioFormat::stereo_f32(48_000);
    let mut cap = GstreamerAudioCapture::test_source(fmt, 440.0).expect("spawn");
    let chunk = cap.next_chunk(48).expect("read chunk");
    assert_eq!(chunk.format().channels, 2);
    assert_eq!(chunk.frame_count(), 48);
    assert_eq!(chunk.samples().len(), 96);
    // audiotestsrc emits the same waveform on every channel — L ≈ R.
    for pair in chunk.samples().chunks_exact(2) {
        assert!(
            (pair[0] - pair[1]).abs() < 1e-6,
            "stereo channels should match"
        );
    }
}

#[test]
fn from_file_decodes_real_mp3_fixture() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    let path = Path::new(FIXTURE_MP3);
    if !path.exists() {
        eprintln!("fixture missing: {path:?} — skipping");
        return;
    }
    let fmt = AudioFormat::stereo_f32(44_100);
    let mut cap = GstreamerAudioCapture::from_file(path, fmt).expect("spawn");

    // The fixture is a pure 440 Hz sine. Read 1 s and verify RMS is
    // close to 0.707 (amplitude near 1.0 after MP3 round-trip; allow
    // a wider tolerance for codec artefacts).
    let chunk = cap.next_chunk(44_100).expect("read 1 s of fixture");
    assert_eq!(chunk.format().sample_rate, 44_100);
    assert_eq!(chunk.format().channels, 2);
    assert_eq!(chunk.frame_count(), 44_100);
    let rms = chunk.rms();
    assert!(
        rms > 0.4 && rms < 0.95,
        "fixture sine RMS out of range: {rms}"
    );
    let peak = chunk.peak();
    assert!(peak > 0.5, "fixture sine peak too low: {peak}");
}
