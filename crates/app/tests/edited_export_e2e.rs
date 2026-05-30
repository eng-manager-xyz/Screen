//! Integration test: ED.21 end-to-end edited export.
//!
//! Exports a sped-up project to a real `.mp4`, then decodes the result back
//! to prove it's a valid container at the right dimensions and the retimed
//! length (a 2× segment halves the duration). gst + wgpu guarded — skips
//! cleanly without `GStreamer`.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use decode::EditorVideoStream;
use edit::EditOp;
use media::encode::OutputFormat;
use media::gstreamer::is_available as gstreamer_available;
use screen_app::editor_command::project_from_metadata;
use screen_app::editor_export::export_edited_project;

const FIXTURE: &str = "../decode/tests/fixtures/sample.mp4";

#[test]
fn edited_project_exports_a_valid_retimed_mp4() {
    if !gstreamer_available() {
        eprintln!("gstreamer not on PATH — skipping edited export e2e");
        return;
    }

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
    project
        .apply(&EditOp::SetSpeed {
            index: 0,
            timescale: 2.0,
        })
        .expect("set 2x speed");
    let expected_len = project.project_duration();

    let out = std::env::temp_dir().join("screen_ed21_edited_export.mp4");
    let _ = std::fs::remove_file(&out);
    let cancel = AtomicBool::new(false);
    let mut last_progress = 0u64;
    let written = export_edited_project(
        project,
        Path::new(FIXTURE),
        out.clone(),
        OutputFormat::Mp4H264Aac,
        &cancel,
        |done, _total| last_progress = done,
    )
    .expect("export succeeds");

    assert_eq!(written, out);
    let meta = std::fs::metadata(&out).expect("output file exists");
    assert!(meta.len() > 0, "exported mp4 is non-empty");
    assert_eq!(
        last_progress, expected_len,
        "progress callback reached every frame"
    );

    // Decode the export back: a valid container at the source dimensions and
    // the retimed length (the 2× segment halved it).
    let back = EditorVideoStream::open(&out).expect("reopen export");
    assert_eq!((back.width(), back.height()), (width, height));
    let out_frames = back.frame_count().unwrap_or(0);
    // Allow ±2 for encoder GOP / container rounding.
    assert!(
        out_frames.abs_diff(expected_len) <= 2,
        "exported length {out_frames} ≈ retimed {expected_len}"
    );
    assert!(
        out_frames < frame_count,
        "2× export ({out_frames}) is shorter than the source ({frame_count})"
    );

    let _ = std::fs::remove_file(&out);
}
