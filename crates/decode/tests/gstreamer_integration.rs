//! Integration test for [`GstreamerPipeStream`].
//!
//! Reads back the committed `tests/fixtures/sample.mp4` (480×270 @ 30 fps,
//! 8 frames) via the `GStreamer` CLI pipeline. Skips gracefully if
//! `gst-launch-1.0` / `gst-discoverer-1.0` aren't on `PATH` so machines
//! without `GStreamer` don't fail the gate.

use std::path::Path;

use decode::VideoStream;
use decode::gstreamer_pipe::GstreamerPipeStream;
use media::gstreamer::is_available as gstreamer_available;

const FIXTURE: &str = "tests/fixtures/sample.mp4";

#[test]
fn probe_reports_expected_dimensions() {
    if !gstreamer_available() {
        eprintln!("gst-launch-1.0/gst-discoverer-1.0 not on PATH — skipping");
        return;
    }
    let meta = GstreamerPipeStream::probe(Path::new(FIXTURE)).expect("probe");
    assert_eq!(meta.width, 480);
    assert_eq!(meta.height, 270);
    assert!(
        (meta.frame_rate - 30.0).abs() < 0.01,
        "expected ~30 fps, got {}",
        meta.frame_rate
    );
}

#[test]
fn streams_all_frames_then_terminates() {
    if !gstreamer_available() {
        eprintln!("gstreamer not on PATH — skipping");
        return;
    }
    let mut stream = GstreamerPipeStream::open(Path::new(FIXTURE)).expect("open");
    assert_eq!(stream.width(), 480);
    assert_eq!(stream.height(), 270);

    let expected_frame_size = (stream.width() * stream.height() * 4) as usize;
    let mut count = 0u64;
    while let Some(frame) = stream.next_frame() {
        assert_eq!(frame.bgra.len(), expected_frame_size);
        assert_eq!(frame.frame_index, count);
        count += 1;
        assert!(count < 1000, "ran past sane frame limit");
    }
    // Source has 8 frames; codec re-encode round-trip can produce ±1 due
    // to GOP boundaries / decodebin's edge cases. Tolerate the small wobble.
    assert!((7..=9).contains(&count), "expected ~8 frames, got {count}");
}

#[test]
fn frames_are_not_all_identical() {
    if !gstreamer_available() {
        eprintln!("gstreamer not on PATH — skipping");
        return;
    }
    let mut stream = GstreamerPipeStream::open(Path::new(FIXTURE)).expect("open");
    let f0 = stream.next_frame().expect("frame 0");
    let f_last = std::iter::from_fn(|| stream.next_frame())
        .last()
        .expect("at least one more frame");
    assert_ne!(
        f0.bgra, f_last.bgra,
        "first and last decoded frames should differ"
    );
}
