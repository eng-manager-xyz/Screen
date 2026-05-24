//! End-to-end integration test for [`media::encode::GstreamerEncoder`].
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

use media::encode::{EncoderConfig, GstreamerEncoder, OutputFormat, VideoEncoder};
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

#[test]
fn full_lifecycle_with_audio_produces_mp4_with_both_streams() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    if !gst_discoverer_available() {
        eprintln!("gst-discoverer-1.0 not on PATH — skipping");
        return;
    }

    let output_path = std::env::temp_dir().join(format!(
        "encode_integration_audio_{}.mp4",
        std::process::id()
    ));
    cleanup_artifacts(&output_path);

    let mut encoder = GstreamerEncoder::new(EncoderConfig {
        output_path: output_path.clone(),
        width: WIDTH,
        height: HEIGHT,
        framerate: FRAMERATE,
        sample_rate: SAMPLE_RATE,
        channels: CHANNELS,
        format: OutputFormat::Mp4H264Aac,
    })
    .expect("encoder constructs");

    // 1 second of solid-red BGRA at 30 fps.
    let mut frame = vec![0u8; (WIDTH as usize) * (HEIGHT as usize) * 4];
    for px in frame.chunks_exact_mut(4) {
        px.copy_from_slice(&[0, 0, 255, 255]); // B, G, R, A
    }
    let frame_interval = Duration::from_micros(1_000_000 / u64::from(FRAMERATE));
    for i in 0..FRAMERATE {
        encoder
            .push_video_frame(&frame, frame_interval * i)
            .expect("push video frame");
    }

    // 1 second of 440 Hz sine, interleaved F32 stereo, pushed in
    // 100 ms chunks (matches the M-MIC.1 cadence).
    let chunk_frames = SAMPLE_RATE as usize / 10;
    let chunks_per_second = 10_u64;
    for chunk_idx in 0..chunks_per_second {
        let samples = sine_chunk_stereo(chunk_idx, chunk_frames);
        let pts = Duration::from_millis(chunk_idx * 100);
        encoder
            .push_audio_chunk(&samples, pts)
            .expect("push audio chunk");
    }

    assert_eq!(encoder.frames_pushed(), u64::from(FRAMERATE));
    assert_eq!(encoder.audio_chunks_pushed(), chunks_per_second);

    let final_path = Box::new(encoder).finalize().expect("finalize succeeds");
    assert_eq!(final_path, output_path);
    assert!(output_path.exists(), "encoded mp4 should exist on disk");

    let probe = Command::new("gst-discoverer-1.0")
        .arg(&output_path)
        .output()
        .expect("spawn gst-discoverer-1.0");
    assert!(
        probe.status.success(),
        "gst-discoverer-1.0 exited non-zero: {}",
        String::from_utf8_lossy(&probe.stderr)
    );
    let stdout = String::from_utf8_lossy(&probe.stdout);
    assert!(
        stdout.contains("video #"),
        "no video stream announced:\n{stdout}"
    );
    assert!(
        stdout.contains("audio #"),
        "no audio stream announced (audio-recording regression):\n{stdout}"
    );
    assert!(
        stdout.contains("H.264") || stdout.contains("h264"),
        "expected H.264 video, got:\n{stdout}"
    );
    assert!(
        stdout.contains("AAC") || stdout.contains("aac"),
        "expected AAC audio, got:\n{stdout}"
    );

    cleanup_artifacts(&output_path);
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
    for suffix in [".bgra.scratch", ".f32.scratch"] {
        let mut p = output_path.as_os_str().to_owned();
        p.push(suffix);
        let _ = std::fs::remove_file(PathBuf::from(p));
    }
}
