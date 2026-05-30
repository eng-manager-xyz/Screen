//! Integration test: ED.1–ED.8 work in concert.
//!
//! Proves the backend editor chain composes end-to-end, not just per-chunk:
//!
//! ```text
//! EditorSession.seek (clock, ED.7/ED.4)
//!   → EditProject::source_time (model, ED.1)
//!     → EditorVideoStream.frame (random-access decode, ED.3)
//!       → EditorPreview.render_frame (wisp compose, ED.6)
//!         → composed BGRA
//! ```
//!
//! Reads the committed decode fixture (`crates/decode/tests/fixtures/sample.mp4`,
//! 480×270 @ 30 fps). gst + wgpu guarded — skips cleanly without `GStreamer`.

use std::path::{Path, PathBuf};

use decode::EditorVideoStream;
use media::gstreamer::is_available as gstreamer_available;
use screen_app::editor_command::project_from_metadata;
use screen_app::editor_preview::EditorPreview;
use screen_app::editor_session::{EditorSession, TransportAction};

const FIXTURE: &str = "../decode/tests/fixtures/sample.mp4";

#[test]
fn editor_pipeline_seeks_decodes_and_composes_in_concert() {
    if !gstreamer_available() {
        eprintln!("gstreamer not on PATH — skipping editor pipeline integration");
        return;
    }

    // ED.3 — open the seekable stream + read metadata.
    let mut stream = EditorVideoStream::open(Path::new(FIXTURE)).expect("open stream");
    let (width, height) = (stream.width(), stream.height());
    let frame_count = stream.frame_count().unwrap_or(0).max(1);

    // ED.1 — a default, full-length, real-time project for the clip
    // (built through the same helper `open_in_editor` uses).
    let project = project_from_metadata(
        PathBuf::from(FIXTURE),
        width,
        height,
        stream.frame_rate(),
        frame_count,
    );
    assert_eq!(project.project_duration(), frame_count);

    // ED.7 + ED.4 — the backend session / clock for the project.
    let mut session = EditorSession::new(project.project_fps, project.project_duration());

    // ED.6 — the compose pump at the clip's dimensions.
    let mut preview = EditorPreview::new(width, height).expect("init preview");

    // Drive the clock to a few playhead positions and verify the full chain
    // produces a correctly-sized composed frame at each.
    let bytes_per_frame = (width as usize) * (height as usize) * 4;
    for &target in &[0u64, frame_count / 2, frame_count.saturating_sub(1)] {
        let status = session.apply(TransportAction::Seek { frame: target });
        let playhead = status.current_frame;
        assert!(playhead < frame_count, "playhead stays within the clip");

        // ED.1 — map the project frame to a source frame. For a single
        // real-time segment this is the identity, which we assert.
        let source_frame = project.source_time(playhead).expect("frame maps to source");
        assert_eq!(source_frame, playhead, "1× project maps project→source 1:1");

        // ED.3 → ED.6 — decode that source frame and compose it.
        let frame = stream.frame(source_frame).expect("decode source frame");
        assert_eq!(frame.frame_index, source_frame);
        let composed = preview.render_frame(frame.bgra).expect("compose");
        assert_eq!((composed.width, composed.height), (width, height));
        assert_eq!(composed.bytes.len(), bytes_per_frame);
    }

    // ED.4 — play + tick advances the clock (the host-injects-dt model the
    // transport UI drives).
    // A short tick (100 ms @ 30 fps project = 3 frames) stays inside the
    // 8-frame fixture, so the clock advances and playback continues. (A 1 s
    // tick would overshoot this tiny clip and — correctly — clamp + pause at
    // the end, which is the ED.4 end-of-range behavior.)
    session.apply(TransportAction::Seek { frame: 0 });
    session.apply(TransportAction::Play);
    let status = session.apply(TransportAction::Tick { dt_ms: 100 });
    assert!(status.playing, "still playing mid-clip");
    assert_eq!(
        status.current_frame, 3,
        "clock advanced 3 frames under a 100ms tick"
    );
}
