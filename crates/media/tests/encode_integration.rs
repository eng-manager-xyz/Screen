//! End-to-end integration test for [`media::encode::LiveGstreamerEncoder`].
//!
//! Drives the encoder through its full lifecycle and verifies the
//! resulting `.mp4` actually contains both video and audio streams
//! via `gst-discoverer-1.0`. The unit tests in `encode.rs` only
//! assert on the argv shape — they don't invoke gst-launch and so
//! can't catch property-name regressions in the audio leg.
//!
//! macOS-only: the test exercises the `vtenc_h264_hw` hardware
//! encoder. Linux + Windows pipeline strings are still validated by
//! the unit tests in `encode.rs`.

#![cfg(target_os = "macos")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use media::encode::{EncoderConfig, LiveGstreamerEncoder, OutputFormat, VideoEncoder};
use media::gstreamer::is_available;

const WIDTH: u32 = 64;
const HEIGHT: u32 = 64;
const FRAMERATE: u32 = 30;
const SAMPLE_RATE: u32 = 48_000;
const CHANNELS: u8 = 2;

fn gst_discoverer_available() -> bool {
    Command::new("gst-discoverer-1.0")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
}

// ---- M-SAVE.2 — MP4 → WebM transcode round-trip ----

/// Encode a short test MP4 at `path` — 0.5 s of solid red, optionally
/// with a 440 Hz sine audio track. Shared by the transcode tests so
/// they exercise a real (encoder-produced) MP4 rather than a fixture.
fn encode_test_mp4(path: &Path, with_audio: bool) {
    let mut encoder = LiveGstreamerEncoder::new(EncoderConfig {
        output_path: path.to_path_buf(),
        width: WIDTH,
        height: HEIGHT,
        framerate: FRAMERATE,
        sample_rate: SAMPLE_RATE,
        channels: CHANNELS,
        format: OutputFormat::Mp4H264Aac,
    })
    .expect("live encoder constructs + spawns");

    let mut frame = vec![0u8; (WIDTH as usize) * (HEIGHT as usize) * 4];
    for px in frame.chunks_exact_mut(4) {
        px.copy_from_slice(&[0, 0, 255, 255]); // B, G, R, A
    }
    let interval = Duration::from_micros(1_000_000 / u64::from(FRAMERATE));
    for i in 0..(FRAMERATE / 2) {
        encoder
            .push_video_frame(&frame, interval * i)
            .expect("push video frame");
    }
    if with_audio {
        let chunk_frames = SAMPLE_RATE as usize / 10;
        for chunk_idx in 0..5_u64 {
            encoder
                .push_audio_chunk(
                    &sine_chunk_stereo(chunk_idx, chunk_frames),
                    Duration::from_millis(chunk_idx * 100),
                )
                .expect("push audio chunk");
        }
    }
    Box::new(encoder).finalize().expect("finalize mp4");
}

#[test]
fn transcode_video_only_mp4_to_webm_produces_vp9() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    if !gst_discoverer_available() {
        eprintln!("gst-discoverer-1.0 not on PATH — skipping");
        return;
    }
    let mp4 = std::env::temp_dir().join(format!("transcode_vid_{}.mp4", std::process::id()));
    let webm = std::env::temp_dir().join(format!("transcode_vid_{}.webm", std::process::id()));
    cleanup_artifacts(&mp4);
    let _ = std::fs::remove_file(&webm);

    encode_test_mp4(&mp4, false);
    // A video-only scratch must probe as no-audio so the transcode
    // takes the video-only pipeline (no dangling decodebin audio pad).
    assert!(
        !media::encode::scratch_has_audio(&mp4),
        "video-only mp4 should report no audio track"
    );

    media::encode::transcode_to_webm(&mp4, &webm).expect("transcode to webm");
    assert!(webm.exists(), "webm should exist after transcode");

    let probe = Command::new("gst-discoverer-1.0")
        .arg(&webm)
        .output()
        .expect("spawn gst-discoverer-1.0");
    let stdout = String::from_utf8_lossy(&probe.stdout);
    assert!(stdout.contains("video #"), "no video stream:\n{stdout}");
    assert!(
        stdout.to_lowercase().contains("vp9"),
        "expected VP9 video:\n{stdout}"
    );

    cleanup_artifacts(&mp4);
    let _ = std::fs::remove_file(&webm);
}

#[test]
fn transcode_mp4_with_audio_to_webm_has_vp9_and_opus() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    if !gst_discoverer_available() {
        eprintln!("gst-discoverer-1.0 not on PATH — skipping");
        return;
    }
    let mp4 = std::env::temp_dir().join(format!("transcode_av_{}.mp4", std::process::id()));
    let webm = std::env::temp_dir().join(format!("transcode_av_{}.webm", std::process::id()));
    cleanup_artifacts(&mp4);
    let _ = std::fs::remove_file(&webm);

    encode_test_mp4(&mp4, true);
    assert!(
        media::encode::scratch_has_audio(&mp4),
        "audio mp4 should probe as having an audio track"
    );

    media::encode::transcode_to_webm(&mp4, &webm).expect("transcode to webm");

    let probe = Command::new("gst-discoverer-1.0")
        .arg(&webm)
        .output()
        .expect("spawn gst-discoverer-1.0");
    let stdout = String::from_utf8_lossy(&probe.stdout);
    assert!(
        stdout.to_lowercase().contains("vp9"),
        "expected VP9 video:\n{stdout}"
    );
    assert!(
        stdout.to_lowercase().contains("opus"),
        "expected Opus audio (audio-leg regression):\n{stdout}"
    );

    cleanup_artifacts(&mp4);
    let _ = std::fs::remove_file(&webm);
}

// ---- M-QUAL.1 — live (streaming) encoder round-trip ----

/// The live encoder streams BGRA into gst-launch's stdin during
/// capture (compressed intermediate on disk) and remuxes the small
/// raw-audio scratch in at finalize. This verifies the full lifecycle
/// produces a valid H.264 + AAC MP4 of the right dimensions — the
/// argv unit tests can't catch a stdin/EOS/remux regression.
#[test]
fn live_encoder_video_and_audio_produces_mp4_with_both_streams() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    if !gst_discoverer_available() {
        eprintln!("gst-discoverer-1.0 not on PATH — skipping");
        return;
    }

    let output_path =
        std::env::temp_dir().join(format!("live_encode_av_{}.mp4", std::process::id()));
    cleanup_artifacts(&output_path);

    let mut encoder = LiveGstreamerEncoder::new(EncoderConfig {
        output_path: output_path.clone(),
        width: WIDTH,
        height: HEIGHT,
        framerate: FRAMERATE,
        sample_rate: SAMPLE_RATE,
        channels: CHANNELS,
        format: OutputFormat::Mp4H264Aac,
    })
    .expect("live encoder constructs + spawns");

    let mut frame = vec![0u8; (WIDTH as usize) * (HEIGHT as usize) * 4];
    for px in frame.chunks_exact_mut(4) {
        px.copy_from_slice(&[0, 0, 255, 255]); // B, G, R, A
    }
    let interval = Duration::from_micros(1_000_000 / u64::from(FRAMERATE));
    for i in 0..FRAMERATE {
        encoder
            .push_video_frame(&frame, interval * i)
            .expect("push video frame to live stdin");
    }
    let chunk_frames = SAMPLE_RATE as usize / 10;
    for chunk_idx in 0..10_u64 {
        encoder
            .push_audio_chunk(
                &sine_chunk_stereo(chunk_idx, chunk_frames),
                Duration::from_millis(chunk_idx * 100),
            )
            .expect("push audio chunk");
    }

    assert_eq!(encoder.frames_pushed(), u64::from(FRAMERATE));
    assert_eq!(encoder.audio_chunks_pushed(), 10);

    let final_path = Box::new(encoder).finalize().expect("finalize succeeds");
    assert_eq!(final_path, output_path);
    assert!(output_path.exists(), "encoded mp4 should exist on disk");

    let stdout = discover(&output_path);
    assert!(stdout.contains("video #"), "no video stream:\n{stdout}");
    assert!(stdout.contains("audio #"), "no audio stream:\n{stdout}");
    assert!(
        stdout.contains("H.264") || stdout.contains("h264"),
        "expected H.264 video:\n{stdout}"
    );
    assert!(
        stdout.contains("AAC") || stdout.contains("aac"),
        "expected AAC audio:\n{stdout}"
    );
    assert!(stdout.contains("64"), "expected 64x64 dims:\n{stdout}");

    // The compressed intermediate + raw audio scratch are cleaned up.
    for suffix in [".live-video.scratch", ".f32.scratch"] {
        let mut p = output_path.as_os_str().to_owned();
        p.push(suffix);
        assert!(
            !PathBuf::from(&p).exists(),
            "scratch {p:?} should be removed after finalize"
        );
    }

    cleanup_artifacts(&output_path);
}

/// A screen-only recording has no audio. The live encoder must produce
/// a valid video-only MP4 by *moving* the intermediate into place (the
/// remux is skipped) — guards the `has_video && !has_audio` finalize arm.
#[test]
fn live_encoder_video_only_moves_intermediate_to_output() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    if !gst_discoverer_available() {
        eprintln!("gst-discoverer-1.0 not on PATH — skipping");
        return;
    }

    let output_path =
        std::env::temp_dir().join(format!("live_encode_vid_{}.mp4", std::process::id()));
    cleanup_artifacts(&output_path);

    let mut encoder = LiveGstreamerEncoder::new(EncoderConfig {
        output_path: output_path.clone(),
        width: WIDTH,
        height: HEIGHT,
        framerate: FRAMERATE,
        sample_rate: SAMPLE_RATE,
        channels: CHANNELS,
        format: OutputFormat::Mp4H264Aac,
    })
    .expect("live encoder constructs + spawns");

    let mut frame = vec![0u8; (WIDTH as usize) * (HEIGHT as usize) * 4];
    for px in frame.chunks_exact_mut(4) {
        px.copy_from_slice(&[255, 0, 0, 255]); // solid blue
    }
    let interval = Duration::from_micros(1_000_000 / u64::from(FRAMERATE));
    for i in 0..(FRAMERATE / 2) {
        encoder
            .push_video_frame(&frame, interval * i)
            .expect("push video frame");
    }

    let final_path = Box::new(encoder).finalize().expect("finalize succeeds");
    assert_eq!(final_path, output_path);
    assert!(output_path.exists(), "video-only mp4 should exist");

    let stdout = discover(&output_path);
    assert!(stdout.contains("video #"), "no video stream:\n{stdout}");
    assert!(
        !stdout.contains("audio #"),
        "video-only recording must have no audio track:\n{stdout}"
    );

    cleanup_artifacts(&output_path);
}

/// AUT-334 regression — on a 5K display (5120×2880) the recorder used
/// to feed `vtenc_h264_hw` a 5120-wide frame, which fails caps
/// negotiation (`not-negotiated (-4)`), breaks the feed pipe after the
/// first frame, and discards the entire recording (empty
/// `~/Movies/Screen/`). The app now clamps capture dims via
/// [`media::encode::fit_within_encoder_limits`]; this drives the
/// *clamped* config through the real `vtenc_h264_hw` encoder and
/// asserts a non-empty, discoverable mp4 at the clamped resolution.
#[test]
fn live_encoder_encodes_clamped_5k_to_nonempty_mp4() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    if !gst_discoverer_available() {
        eprintln!("gst-discoverer-1.0 not on PATH — skipping");
        return;
    }

    // The exact transform the app applies to a 5K display's native dims.
    let (w, h) = media::encode::fit_within_encoder_limits(5120, 2880, OutputFormat::Mp4H264Aac);
    assert!(
        w <= 4096 && h <= 4096,
        "clamp must bring a 5K display within the vtenc_h264_hw 4096 cap, got {w}x{h}"
    );
    assert_eq!((w, h), (4096, 2304), "5120x2880 (16:9) clamps to 4096x2304");

    let output_path =
        std::env::temp_dir().join(format!("live_encode_5k_{}.mp4", std::process::id()));
    cleanup_artifacts(&output_path);

    let mut encoder = LiveGstreamerEncoder::new(EncoderConfig {
        output_path: output_path.clone(),
        width: w,
        height: h,
        framerate: FRAMERATE,
        sample_rate: SAMPLE_RATE,
        channels: CHANNELS,
        format: OutputFormat::Mp4H264Aac,
    })
    .expect("live encoder constructs + spawns at the clamped 4K dims");

    // A few solid-grey frames at the clamped resolution — the exact
    // path that produced `not-negotiated` + a 0-byte file before the fix
    // (at 5120 wide). One reused buffer keeps the allocation bounded.
    let frame = vec![0x80u8; (w as usize) * (h as usize) * 4];
    let interval = Duration::from_micros(1_000_000 / u64::from(FRAMERATE));
    for i in 0..3 {
        encoder
            .push_video_frame(&frame, interval * i)
            .expect("push 4K frame to live stdin (would fail at 5120 wide)");
    }

    let final_path = Box::new(encoder).finalize().expect("finalize succeeds");
    assert_eq!(final_path, output_path);
    assert!(output_path.exists(), "clamped mp4 should exist on disk");
    let len = std::fs::metadata(&output_path)
        .expect("stat clamped mp4")
        .len();
    assert!(
        len > 0,
        "clamped mp4 must be non-empty (was 0 bytes pre-fix)"
    );

    let stdout = discover(&output_path);
    assert!(stdout.contains("video #"), "no video stream:\n{stdout}");
    assert!(
        stdout.contains("4096"),
        "expected 4096-wide video:\n{stdout}"
    );

    cleanup_artifacts(&output_path);
}

/// Run `gst-discoverer-1.0` and return its stdout (asserting success).
fn discover(path: &Path) -> String {
    let probe = Command::new("gst-discoverer-1.0")
        .arg(path)
        .output()
        .expect("spawn gst-discoverer-1.0");
    assert!(
        probe.status.success(),
        "gst-discoverer-1.0 exited non-zero: {}",
        String::from_utf8_lossy(&probe.stderr)
    );
    String::from_utf8_lossy(&probe.stdout).into_owned()
}

#[allow(
    clippy::cast_precision_loss,
    reason = "sample index < 48 000 fits f32 precision for a one-second buffer"
)]
fn sine_chunk_stereo(chunk_idx: u64, frames_per_chunk: usize) -> Vec<f32> {
    let chunk_idx = usize::try_from(chunk_idx).expect("chunk_idx fits usize");
    let mut samples = Vec::with_capacity(frames_per_chunk * usize::from(CHANNELS));
    for n in 0..frames_per_chunk {
        let global = chunk_idx * frames_per_chunk + n;
        let t = global as f32 / SAMPLE_RATE as f32;
        let v = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.3;
        samples.push(v);
        samples.push(v);
    }
    samples
}

fn cleanup_artifacts(output_path: &Path) {
    let _ = std::fs::remove_file(output_path);
    for suffix in [".f32.scratch", ".live-video.scratch"] {
        let mut p = output_path.as_os_str().to_owned();
        p.push(suffix);
        let _ = std::fs::remove_file(PathBuf::from(p));
    }
}
