//! `open_in_editor` — the Record→Edit handoff command (ED.5 / M-EDIT).
//!
//! Probes a finished recording's metadata and builds a fresh, untouched
//! [`edit::EditProject`] (one full-length real-time segment) for the
//! editor UI to load. The heavy lifting — the edit model itself — lives in
//! the pure `edit` crate; this is the thin Tauri wrapper.

#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects State<'_, T> into #[command] fns by value; it is borrowed, not moved"
)]

use std::path::{Path, PathBuf};

use decode::gstreamer_pipe::GstreamerPipeStream;
use edit::{ClipRef, EditProject};
use tauri::State;

use crate::editor_session::{EditorSession, EditorSessionState};

/// Build a default editor project from a recording's probed metadata.
///
/// Split out from the command so it's unit-testable without spawning
/// `GStreamer`.
#[must_use]
pub fn project_from_metadata(
    path: PathBuf,
    width: u32,
    height: u32,
    frame_rate: f32,
    frame_count: u64,
) -> EditProject {
    EditProject::from_recording(ClipRef::new(
        path,
        width,
        height,
        fps_round(frame_rate),
        frame_count,
    ))
}

/// Round a reported (possibly fractional / NTSC) frame rate to a whole
/// fps, clamped to at least 1. Non-finite / non-positive rates fall back
/// to 30.
fn fps_round(frame_rate: f32) -> u32 {
    if !(frame_rate.is_finite() && frame_rate > 0.0) {
        return 30;
    }
    let rounded = frame_rate.round();
    if rounded < 1.0 {
        1
    } else {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "rounded is finite and >= 1.0; real frame rates fit u32"
        )]
        let fps = rounded as u32;
        fps
    }
}

/// Open a finished recording in the editor: probe it with
/// `gst-discoverer-1.0` and return a default [`EditProject`] (the
/// recording, untouched, ready to edit).
///
/// # Errors
///
/// Returns the probe error string if the file can't be read (missing
/// `GStreamer`, unreadable media).
#[tauri::command]
pub fn open_in_editor(
    path: String,
    state: State<'_, EditorSessionState>,
) -> Result<EditProject, String> {
    let meta = GstreamerPipeStream::probe(Path::new(&path)).map_err(|err| err.to_string())?;
    let project = project_from_metadata(
        PathBuf::from(path),
        meta.width,
        meta.height,
        meta.frame_rate,
        meta.frame_count.unwrap_or(0),
    );
    // Spin up the playhead session for this clip (ED.7 transport drives it).
    let mut guard = state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = Some(EditorSession::new(
        project.project_fps,
        project.project_duration(),
    ));
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_from_metadata_builds_one_full_length_segment() {
        let p = project_from_metadata(PathBuf::from("/tmp/rec.mp4"), 1920, 1080, 29.97, 600);
        assert_eq!(p.source.width, 1920);
        assert_eq!(p.source.height, 1080);
        assert_eq!(p.source.source_fps, 30, "29.97 rounds to 30");
        assert_eq!(p.source.frame_count, 600);
        assert_eq!(p.segments.len(), 1);
        assert_eq!(p.project_duration(), 600);
        assert!(p.zooms.is_empty());
    }

    #[test]
    fn fps_round_handles_edge_cases() {
        assert_eq!(fps_round(29.97), 30);
        assert_eq!(fps_round(30.0), 30);
        assert_eq!(fps_round(59.94), 60);
        assert_eq!(fps_round(0.0), 30, "zero falls back");
        assert_eq!(fps_round(-5.0), 30, "negative falls back");
        assert_eq!(fps_round(f32::NAN), 30, "NaN falls back");
        assert_eq!(fps_round(0.4), 1, "sub-1 clamps to 1");
    }
}
