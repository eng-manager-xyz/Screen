//! Live editor preview session (AUT-510 / ED.6b).
//!
//! Holds the wgpu compose pipeline + a seekable decode stream for the
//! currently-open editor clip, so the webview can render the composed frame
//! (zoom / crop / background / cursor) at any playhead position — through the
//! **same** [`compose_project_frame`](crate::editor_export::compose_project_frame)
//! the export uses, so what you scrub is what you ship.
//!
//! **`EditorPreview` is `Send` but `!Sync`.** Its wgpu `Renderer` holds a
//! `RefCell` (the mask cache), so it is `Send` (proven by the export path
//! running it inside `spawn_blocking`) but `!Sync`. The session therefore
//! lives behind a `Mutex`: a `Mutex<T>` is `Sync` when `T: Send`, which lifts
//! it to the `Send + Sync` that Tauri's `State` requires. Never hand a bare
//! `EditorPreview` to `State`.
//!
//! The two commands mirror the recorder's preview split: `editor_preview_open`
//! (like `start_preview`) opens/updates the session; `editor_preview_frame`
//! (like `latest_camera_frame_bgra`) returns the composed BGRA bytes for one
//! frame as a raw [`tauri::ipc::Response`] (no JSON).

use std::path::PathBuf;
use std::sync::Mutex;

use decode::EditorVideoStream;
use edit::EditProject;
use tauri::State;

use crate::editor_preview::EditorPreview;

/// Decoded-frame cache the seekable stream keeps for scrub locality — enough
/// that a short back-and-forth scrub stays in cache without re-spawning the
/// decoder, far below the 300-frame default (which would pin ~2.5 GB at 1080p
/// for no preview benefit).
const PREVIEW_CACHE_FRAMES: usize = 24;

/// Open compose pipeline + seekable decode stream for one editor clip.
struct EditorPreviewSession {
    preview: EditorPreview,
    stream: EditorVideoStream,
    project: EditProject,
    source_path: PathBuf,
}

impl EditorPreviewSession {
    /// Open the decode stream + wgpu compose pipeline for `project`'s source.
    fn open(project: EditProject) -> Result<Self, String> {
        let source_path = project.source.path.clone();
        let stream = EditorVideoStream::open_with_cache(&source_path, PREVIEW_CACHE_FRAMES)
            .map_err(|e| format!("open source: {e}"))?;
        let mut preview = EditorPreview::new(project.source.width, project.source.height)
            .map_err(|e| format!("init compose: {e}"))?;
        // Background framing is applied once (not per frame), like the export.
        preview.set_background(&project.background);
        Ok(Self {
            preview,
            stream,
            project,
            source_path,
        })
    }
}

/// Tauri-managed state for the live editor preview (AUT-510).
#[derive(Default)]
pub struct EditorPreviewState(Mutex<Option<EditorPreviewSession>>);

impl std::fmt::Debug for EditorPreviewState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `EditorPreview` isn't `Debug` (wgpu handles); summarize.
        f.debug_struct("EditorPreviewState").finish_non_exhaustive()
    }
}

impl EditorPreviewState {
    /// Open or update the preview for `project`.
    ///
    /// Re-opens the decode stream + compose pipeline only when the source clip
    /// (or its dimensions) changed; otherwise it just swaps the stored project
    /// — re-applying the background only when it changed — so edits
    /// (zoom / speed / crop / cursor) take effect on the next composed frame
    /// without rebuilding wgpu on every keystroke.
    ///
    /// # Errors
    ///
    /// Returns a message if the source can't be opened or the wgpu compose
    /// pipeline can't be created.
    pub fn open(&self, project: EditProject) -> Result<(), String> {
        let mut guard = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let same_clip = guard.as_ref().is_some_and(|s| {
            s.source_path == project.source.path
                && s.preview.dimensions() == (project.source.width, project.source.height)
        });
        if same_clip {
            let s = guard
                .as_mut()
                .expect("same_clip implies an open session is present");
            if s.project.background != project.background {
                s.preview.set_background(&project.background);
            }
            s.project = project;
        } else {
            *guard = Some(EditorPreviewSession::open(project)?);
        }
        Ok(())
    }

    /// Compose the stored project's frame `frame` to BGRA bytes, or an empty
    /// `Vec` when there is no open session or the frame is past the end (the
    /// webview skips an empty buffer, exactly like the camera preview poll).
    #[must_use]
    pub fn frame_bytes(&self, frame: u64) -> Vec<u8> {
        let mut guard = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(s) = guard.as_mut() else {
            return Vec::new();
        };
        crate::editor_export::compose_project_frame(
            &mut s.preview,
            &mut s.stream,
            &s.project,
            frame,
        )
        .map(|c| c.bytes)
        .unwrap_or_default()
    }
}

/// Open / update the live editor preview for `project` (AUT-510). Builds the
/// wgpu compose pipeline + decode stream the first time (and when the source
/// clip changes); otherwise updates the project so edits show on the next
/// frame. Synchronous on purpose — the wgpu device is created on the Tauri
/// command worker thread, never the webview/main thread.
///
/// # Errors
///
/// Returns a message if the source can't be opened or wgpu init fails.
#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri deserializes `project` + injects `State` by value into #[command] fns"
)]
pub fn editor_preview_open(
    project: EditProject,
    state: State<'_, EditorPreviewState>,
) -> Result<(), String> {
    state.open(project)
}

/// Composed BGRA bytes for the editor preview at project `frame` (AUT-510),
/// as a raw [`tauri::ipc::Response`] (an `ArrayBuffer` in JS — no JSON). Empty
/// when no clip is open or the frame is past the end.
#[tauri::command]
#[must_use]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects `State` by value into #[command] fns"
)]
pub fn editor_preview_frame(
    frame: u64,
    state: State<'_, EditorPreviewState>,
) -> tauri::ipc::Response {
    tauri::ipc::Response::new(state.frame_bytes(frame))
}
