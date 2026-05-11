//! Integration tests for `media::gstreamer_video` (M-MEDIA.6 / AUT-102).
//!
//! Runs the actual `gst-launch-1.0` pipeline against `videotestsrc`.
//! Skips gracefully when `GStreamer` isn't on `PATH`.

use media::gstreamer::is_available;
use media::gstreamer_video::GstreamerVideoCapture;

const WIDTH: u32 = 64;
const HEIGHT: u32 = 36;
const FPS: u32 = 30;

#[test]
fn emits_frames_with_expected_dimensions_and_pts() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    let mut cap = GstreamerVideoCapture::test_source(WIDTH, HEIGHT, FPS).expect("spawn");

    for expected_idx in 0..5 {
        let frame = cap.next_frame().expect("frame");
        assert_eq!(frame.width, WIDTH);
        assert_eq!(frame.height, HEIGHT);
        assert_eq!(frame.bgra.len(), (WIDTH * HEIGHT * 4) as usize);
        assert_eq!(frame.frame_index, expected_idx);
        let expected_pts = f64::from(u32::try_from(expected_idx).unwrap()) / f64::from(FPS);
        assert!(
            (frame.pts_seconds - expected_pts).abs() < 1e-6,
            "frame {expected_idx} pts {:.6} expected ~{:.6}",
            frame.pts_seconds,
            expected_pts
        );
    }
    assert_eq!(cap.frames_emitted(), 5);
}

#[test]
fn frames_have_distinct_content_smpte_colorbars() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    let mut cap = GstreamerVideoCapture::test_source(64, 36, 30).expect("spawn");
    // videotestsrc's default pattern is SMPTE colorbars with a tiny
    // animated ball — successive frames have different bytes.
    let first = cap.next_frame().expect("first frame").bgra;
    // Skip ahead a few frames to ensure the ball has moved.
    for _ in 0..15 {
        let _ = cap.next_frame().expect("intermediate frame");
    }
    let later = cap.next_frame().expect("later frame").bgra;
    assert_eq!(first.len(), later.len());
    let diff = first
        .iter()
        .zip(later.iter())
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        diff > 0,
        "consecutive frames should differ (SMPTE colorbars include animated content)"
    );
}

#[test]
fn dimensions_and_framerate_round_trip_through_capture() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping");
        return;
    }
    let cap = GstreamerVideoCapture::test_source(128, 72, 60).expect("spawn");
    assert_eq!(cap.dimensions(), (128, 72));
    assert!((cap.framerate() - 60.0).abs() < 1e-9);
}
