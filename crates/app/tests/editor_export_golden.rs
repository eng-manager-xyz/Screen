//! Integration test: ED.20 deferred export frame generator.
//!
//! Proves the generator walks the *edited* timeline frame-accurately — a
//! sped-up project emits exactly `project_duration` frames, each mapping to
//! the correct source frame (trim / split / speed honored via
//! `source_time`), composed at the clip's dimensions, with monotonic PTS —
//! in a single forward decode pass (the stream never re-spawns).
//!
//! Reads the committed decode fixture (`crates/decode/tests/fixtures/sample.mp4`).
//! gst + wgpu guarded — skips cleanly without `GStreamer`.

use std::path::{Path, PathBuf};

use decode::EditorVideoStream;
use edit::EditOp;
use media::gstreamer::is_available as gstreamer_available;
use screen_app::editor_command::project_from_metadata;
use screen_app::editor_export::ExportFrameGenerator;

const FIXTURE: &str = "../decode/tests/fixtures/sample.mp4";

#[test]
fn generator_walks_edited_timeline_forward_only() {
    if !gstreamer_available() {
        eprintln!("gstreamer not on PATH — skipping export generator integration");
        return;
    }

    // Probe the fixture for metadata, then build a default full-length project.
    let probe = EditorVideoStream::open(Path::new(FIXTURE)).expect("open probe");
    let (width, height) = (probe.width(), probe.height());
    let frame_rate = probe.frame_rate();
    let frame_count = probe.frame_count().unwrap_or(0).max(2);
    drop(probe);

    let mut project = project_from_metadata(
        PathBuf::from(FIXTURE),
        width,
        height,
        frame_rate,
        frame_count,
    );
    // Speed the single segment up 2× — project_duration halves and each
    // project frame maps to a source frame ~2 apart (still forward).
    project
        .apply(&EditOp::SetSpeed {
            index: 0,
            timescale: 2.0,
        })
        .expect("set speed");
    let expected_len = project.project_duration();
    assert!(
        expected_len >= 1 && expected_len < frame_count,
        "2× speed must shorten the timeline ({expected_len} vs {frame_count})"
    );

    // An independent copy to check the source mapping against the generator.
    let oracle = project.clone();
    let mut generator =
        ExportFrameGenerator::new(project, Path::new(FIXTURE)).expect("init generator");
    assert_eq!(generator.frame_count(), expected_len);

    let mut emitted = 0u64;
    let mut prev_pts = None;
    while let Some(ef) = generator.next_frame() {
        // The generator picked the same source frame the model maps to.
        assert_eq!(
            ef.source_frame,
            oracle.source_time(emitted).expect("maps to source")
        );
        // Composed at the clip's dimensions, full BGRA buffer.
        assert_eq!((ef.frame.width, ef.frame.height), (width, height));
        assert_eq!(
            ef.frame.bytes.len(),
            (width as usize) * (height as usize) * 4
        );
        // PTS strictly increases frame to frame.
        if let Some(p) = prev_pts {
            assert!(ef.pts > p, "pts must increase");
        }
        prev_pts = Some(ef.pts);
        emitted += 1;
    }

    assert_eq!(
        emitted, expected_len,
        "emits exactly project_duration frames"
    );
    // Forward-only walk ⇒ the decode stream spawned exactly once.
    assert_eq!(
        generator.spawn_count(),
        1,
        "forward walk must not re-spawn the decoder"
    );
}

/// AUT-510: the shared `compose_project_frame` (used by BOTH the export
/// generator and the live editor preview) composes a real source frame into a
/// full BGRA buffer of the clip's dimensions. This is the preview/export
/// parity guarantee at the shared-function level — the live preview renders
/// the exact same composed pixels the export bakes, without a webview.
#[test]
fn compose_project_frame_produces_full_bgra() {
    if !gstreamer_available() {
        eprintln!("gstreamer not on PATH — skipping compose_project_frame parity test");
        return;
    }

    let probe = EditorVideoStream::open(Path::new(FIXTURE)).expect("open probe");
    let (width, height) = (probe.width(), probe.height());
    let frame_rate = probe.frame_rate();
    let frame_count = probe.frame_count().unwrap_or(0).max(2);
    drop(probe);

    let project = project_from_metadata(
        PathBuf::from(FIXTURE),
        width,
        height,
        frame_rate,
        frame_count,
    );
    let mut stream =
        EditorVideoStream::open_with_cache(Path::new(FIXTURE), 8).expect("open stream");
    let mut preview =
        screen_app::editor_preview::EditorPreview::new(width, height).expect("init wgpu compose");
    preview.set_background(&project.background);

    let composed =
        screen_app::editor_export::compose_project_frame(&mut preview, &mut stream, &project, 0)
            .expect("frame 0 composes");
    assert_eq!(
        (composed.width, composed.height),
        (width, height),
        "composed at the clip's dimensions"
    );
    assert_eq!(
        composed.bytes.len(),
        (width as usize) * (height as usize) * 4,
        "full BGRA buffer, no padding"
    );
    assert!(
        composed.bytes.iter().any(|&b| b != 0),
        "composed frame is non-empty (real decoded pixels, not a black frame)"
    );
}
