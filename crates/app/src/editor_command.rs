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
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use decode::gstreamer_pipe::GstreamerPipeStream;
use edit::{ClipRef, EditProject};
use tauri::{Emitter, Manager, State};

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
pub async fn open_in_editor(app: tauri::AppHandle, path: String) -> Result<EditProject, String> {
    // Probe off the UI worker thread — `gst-discoverer-1.0` can stall on a
    // large or remote clip, and the Record→Edit handoff promise shouldn't
    // block the webview while it does.
    let probe_path = path.clone();
    let meta = tauri::async_runtime::spawn_blocking(move || {
        GstreamerPipeStream::probe(Path::new(&probe_path)).map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| format!("probe task failed to join: {err}"))??;

    let mut project = project_from_metadata(
        PathBuf::from(path),
        meta.width,
        meta.height,
        meta.frame_rate,
        meta.frame_count.unwrap_or(0),
    );
    // ED.17: attach the cursor track captured during the just-finished
    // recording (the Record→Edit handoff). `take` consumes it so it can't
    // re-attach to a later, unrelated clip; `None` for clips opened without a
    // fresh recording (e.g. a re-opened file).
    if let Some(track) = app
        .state::<crate::recording::RecordingState>()
        .take_cursor_track()
    {
        project.cursor_track = Some(track);
    }
    // ED.17 / ISS-16: attach the click log captured during the recording — it
    // feeds auto-zoom + the ED.19 click ripples. Same consume-once handoff.
    if let Some(clicks) = app
        .state::<crate::recording::RecordingState>()
        .take_clicks()
    {
        project.clicks = Some(clicks);
    }
    // Spin up the playhead session for this clip (ED.7 transport drives it).
    // Resolved by handle since the `.await` above rules out holding a
    // `State<'_>` borrow across it.
    let state = app.state::<EditorSessionState>();
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

/// Shared state for the editor export command (ED.22): a cooperative
/// cancel flag plus a single-run guard so a second export can't un-cancel
/// the first and race the same output file.
#[derive(Default)]
pub struct EditorExportState {
    cancel: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
}

/// RAII guard releasing the single-export slot when an export finishes
/// (on any return path, including error/panic).
struct ExportRunGuard(Arc<AtomicBool>);

impl Drop for ExportRunGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl EditorExportState {
    /// Claim the single export slot, resetting the cancel flag for the new
    /// run. Returns a guard that releases the slot on drop, or `None` if an
    /// export is already in flight.
    fn try_begin(&self) -> Option<ExportRunGuard> {
        self.running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()?;
        self.cancel.store(false, Ordering::Relaxed);
        Some(ExportRunGuard(Arc::clone(&self.running)))
    }

    /// Raise the cancel flag for an in-flight export.
    fn request_cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// A clone of the cancel flag for the export job to poll.
    fn cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel)
    }
}

/// `editor-export-progress` event payload.
#[derive(Clone, serde::Serialize)]
struct ExportProgress {
    done: u64,
    total: u64,
}

/// Derive the export output path for `source` + `format`: alongside the
/// recordings folder, named `<source-stem>-edited.<ext>` so it never
/// clobbers the source recording. Pure (no I/O) for testability.
#[must_use]
pub fn edited_export_path(output_dir: &Path, source: &Path, extension: &str) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("export");
    output_dir.join(format!("{stem}-edited.{extension}"))
}

/// Export the edited `project` to an `.mp4`, streaming `editor-export-progress`
/// events and honoring [`editor_export_cancel`]. Returns the output path.
///
/// Runs the compose + encode on the blocking pool so the webview stays
/// responsive; progress is throttled to ~100 events over the export.
///
/// # Errors
///
/// Returns a message if the source can't be opened, the encoder fails, or
/// the export was cancelled.
#[tauri::command]
pub async fn editor_export(
    app: tauri::AppHandle,
    project: EditProject,
    format: Option<String>,
) -> Result<String, String> {
    use media::encode::OutputFormat;

    let format = format
        .as_deref()
        .and_then(OutputFormat::from_slug)
        .unwrap_or_default();
    // Resolve state by handle (an async command can't hold a `State<'_>`
    // borrow across `.await`). Claim the single export slot — `_guard`
    // lives to the end of the command and releases it on drop; a second
    // concurrent export is rejected rather than racing the same file.
    let (cancel, _guard) = {
        let st = app.state::<EditorExportState>();
        let Some(guard) = st.try_begin() else {
            return Err("an export is already in progress".to_owned());
        };
        (st.cancel_flag(), guard)
    };

    let source = project.source.path.clone();
    let dir = crate::recorder_settings::resolved_output_dir(&app);
    let out = edited_export_path(&dir, &source, format.extension());

    let app_emit = app.clone();
    let out_job = out.clone();
    let job = tauri::async_runtime::spawn_blocking(move || -> Result<PathBuf, String> {
        crate::recording_paths::ensure_parent_dir(&out_job)
            .map_err(|err| format!("failed to create output dir: {err}"))?;
        crate::editor_export::export_edited_project(
            project,
            &source,
            out_job,
            format,
            &cancel,
            |done, total| {
                let step = (total / 100).max(1);
                if done % step == 0 || done == total {
                    let _ = app_emit.emit("editor-export-progress", ExportProgress { done, total });
                }
            },
        )
    })
    .await
    .map_err(|err| format!("export task failed to join: {err}"))?;

    let path = job?;
    Ok(path.to_string_lossy().into_owned())
}

/// Request cancellation of an in-flight [`editor_export`].
#[tauri::command]
pub fn editor_export_cancel(state: State<'_, EditorExportState>) {
    state.request_cancel();
}

/// The `.screenproj` path for a `source` recording: the source path with its
/// extension swapped, so the project file sits beside the recording. Pure.
#[must_use]
pub fn screenproj_path(source: &Path) -> PathBuf {
    source.with_extension(edit::persist::SCREENPROJ_EXTENSION)
}

/// Save `project` to its `.screenproj` (beside the source recording);
/// returns the written path.
///
/// # Errors
///
/// Returns a message if serialization or the file write fails.
#[tauri::command]
pub fn editor_save_project(project: EditProject) -> Result<String, String> {
    let path = screenproj_path(&project.source.path);
    let json = edit::persist::to_screenproj(&project)?;
    std::fs::write(&path, json).map_err(|err| format!("write {}: {err}", path.display()))?;
    Ok(path.to_string_lossy().into_owned())
}

/// Load an editor project from a `.screenproj` `path`.
///
/// # Errors
///
/// Returns a message if the file can't be read or parsed.
#[tauri::command]
pub fn editor_load_project(path: String) -> Result<EditProject, String> {
    let json = std::fs::read_to_string(&path).map_err(|err| format!("read {path}: {err}"))?;
    edit::persist::from_screenproj(&json)
}

/// A recording in the library: its file path, display name, and whether a
/// saved `.screenproj` sits beside it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct RecordingEntry {
    /// Absolute path to the `.mp4`.
    pub path: String,
    /// Display name (the file stem).
    pub name: String,
    /// Whether a `<stem>.screenproj` exists beside the recording.
    pub has_project: bool,
}

/// Build library entries from a directory listing: each `.mp4` becomes an
/// entry, newest first (lexical-descending on the timestamped filename),
/// flagged if a sibling `.screenproj` is present. Pure (no I/O).
#[must_use]
pub fn recording_entries(filenames: &[String], dir: &Path) -> Vec<RecordingEntry> {
    let mut mp4s: Vec<&String> = filenames
        .iter()
        .filter(|n| {
            Path::new(n.as_str())
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("mp4"))
        })
        .collect();
    mp4s.sort_unstable();
    mp4s.reverse(); // newest (highest timestamp in the name) first
    mp4s.into_iter()
        .map(|name| {
            let stem = Path::new(name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(name);
            let proj = format!("{stem}.{}", edit::persist::SCREENPROJ_EXTENSION);
            RecordingEntry {
                path: dir.join(name).to_string_lossy().into_owned(),
                name: stem.to_owned(),
                has_project: filenames.iter().any(|f| f == &proj),
            }
        })
        .collect()
}

/// List the recordings in the user's output folder (newest first). Returns
/// an empty list if the folder can't be read.
#[tauri::command]
pub fn list_recordings(app: tauri::AppHandle) -> Vec<RecordingEntry> {
    let dir = crate::recorder_settings::resolved_output_dir(&app);
    let Ok(read) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let names: Vec<String> = read
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().to_str().map(ToOwned::to_owned))
        .collect();
    recording_entries(&names, &dir)
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
    fn recording_entries_filters_sorts_and_flags_projects() {
        let names = vec![
            "rec-2026-01-01.mp4".to_owned(),
            "rec-2026-02-01.mp4".to_owned(),
            "rec-2026-02-01.screenproj".to_owned(),
            "notes.txt".to_owned(),
        ];
        let entries = recording_entries(&names, Path::new("/out"));
        assert_eq!(entries.len(), 2, "only .mp4 files become entries");
        assert_eq!(entries[0].name, "rec-2026-02-01", "newest first");
        assert!(entries[0].has_project, "Feb has a sibling .screenproj");
        // Path-separator-agnostic (Windows joins with `\`).
        assert!(entries[0].path.ends_with("rec-2026-02-01.mp4"));
        assert_eq!(entries[1].name, "rec-2026-01-01");
        assert!(!entries[1].has_project);
    }

    #[test]
    fn screenproj_path_swaps_extension() {
        assert_eq!(
            screenproj_path(Path::new("/rec/clip.mp4")),
            PathBuf::from("/rec/clip.screenproj")
        );
    }

    #[test]
    fn edited_export_path_names_alongside_source() {
        let out = edited_export_path(Path::new("/out"), Path::new("/rec/clip.mp4"), "mp4");
        assert_eq!(out, PathBuf::from("/out/clip-edited.mp4"));
        // Missing stem falls back to a stable name.
        let fallback = edited_export_path(Path::new("/out"), Path::new("/"), "mp4");
        assert_eq!(fallback, PathBuf::from("/out/export-edited.mp4"));
    }

    #[test]
    fn editor_export_state_allows_one_run_at_a_time() {
        let st = EditorExportState::default();
        let g1 = st.try_begin();
        assert!(g1.is_some(), "first export claims the slot");
        assert!(
            st.try_begin().is_none(),
            "a second export is rejected while one is in flight"
        );
        drop(g1);
        assert!(
            st.try_begin().is_some(),
            "the slot is released once the first export finishes"
        );
    }

    #[test]
    fn begin_clears_a_stale_cancel_flag() {
        let st = EditorExportState::default();
        st.request_cancel();
        assert!(st.cancel.load(Ordering::Relaxed));
        let _g = st.try_begin().expect("claim slot");
        assert!(
            !st.cancel.load(Ordering::Relaxed),
            "begin resets cancel so a prior cancel doesn't kill the new run"
        );
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
