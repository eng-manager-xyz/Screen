//! Timeline editing — split + undo/redo (ED.11 / M-EDIT).
//!
//! The razor on the bench and the trim bin. A **split** divides the clip
//! under the playhead into two segments; **undo/redo** walk the trim bin so
//! nothing is ever lost. The heavy lifting is the (proptest-verified)
//! [`edit::History`] from ED.2 — this module is the thin layer that runs an
//! edit against that history and syncs the result into the reactive project
//! signal the timeline reads.
//!
//! Split is duration-preserving, so the playhead clock needs no update.
//! Duration-changing edits (ripple delete, trim) land next and use
//! `EditorPlayer::set_duration`.

use edit::{EditOp, EditProject, History};
use leptos::prelude::*;

use crate::editor_ipc;

/// Reuse the existing edit history if it belongs to `current` (same source
/// clip), otherwise start fresh. A different source path means a different
/// clip was opened, so the old undo stack no longer applies. Pure.
#[must_use]
pub fn resolve_history(existing: Option<History>, current: &EditProject) -> History {
    match existing {
        Some(history) if history.project().source.path == current.source.path => history,
        _ => History::new(current.clone()),
    }
}

/// Run `edit` against the project's persistent history, then sync the
/// project signal to the result. `history` carries the undo/redo stacks
/// across calls.
fn run(
    project: RwSignal<Option<EditProject>>,
    history: StoredValue<Option<History>>,
    edit: impl FnOnce(&mut History),
) {
    let Some(current) = project.get_untracked() else {
        return;
    };
    let mut hist = resolve_history(history.get_value(), &current);
    edit(&mut hist);
    let edited = hist.project().clone();
    let duration = edited.project_duration();
    project.set(Some(edited));
    history.set_value(Some(hist));
    // Keep the backend clock's range in step with the (possibly changed)
    // timeline length — a no-op for split, the point of it for ripple/undo.
    editor_ipc::editor_transport(&editor_ipc::TransportAction::SetDuration { frames: duration });
}

/// Split the clip under the playhead into two (the razor).
pub fn split_at(
    project: RwSignal<Option<EditProject>>,
    history: StoredValue<Option<History>>,
    at: u64,
) {
    run(project, history, |hist| {
        let _ = hist.apply(&EditOp::Split { at });
    });
}

/// The selected clip's `[start, end)` range in **project** frames. Pure.
#[must_use]
pub fn segment_project_range(project: &EditProject, index: usize) -> Option<(u64, u64)> {
    let mut acc = 0u64;
    for (i, seg) in project.segments.iter().enumerate() {
        let len = seg.project_len();
        if i == index {
            return Some((acc, acc + len));
        }
        acc += len;
    }
    None
}

/// Ripple-delete the selected clip — remove it and close the gap (the
/// downstream clips slide left; the timeline shortens).
pub fn ripple_delete_selected(
    project: RwSignal<Option<EditProject>>,
    history: StoredValue<Option<History>>,
    selected: Option<usize>,
) {
    let Some(index) = selected else {
        return;
    };
    let Some(current) = project.get_untracked() else {
        return;
    };
    let Some((start, end)) = segment_project_range(&current, index) else {
        return;
    };
    run(project, history, |hist| {
        let _ = hist.apply(&EditOp::RippleDelete { start, end });
    });
}

/// Add a default ~1.5 s zoom (1.6× centre) starting at project frame `at`,
/// clamped to the timeline. No-op if the window would be empty. `AddZoom`
/// assigns the region a fresh id.
pub fn add_zoom_default(
    project: RwSignal<Option<EditProject>>,
    history: StoredValue<Option<History>>,
    at: u64,
) {
    let Some(current) = project.get_untracked() else {
        return;
    };
    let duration = current.project_duration();
    if duration == 0 {
        return;
    }
    let start = at.min(duration.saturating_sub(1));
    let window = (u64::from(current.project_fps) * 3 / 2).max(1); // ~1.5 s
    let end = (start + window).min(duration);
    if end <= start {
        return;
    }
    let zoom = edit::zoom::ZoomSegment::manual(edit::zoom::ZoomId(0), start, end, 1.6);
    run(project, history, |hist| {
        let _ = hist.apply(&EditOp::AddZoom { zoom });
    });
}

/// Remove the zoom region with the given id.
pub fn remove_zoom(
    project: RwSignal<Option<EditProject>>,
    history: StoredValue<Option<History>>,
    id: edit::zoom::ZoomId,
) {
    run(project, history, |hist| {
        let _ = hist.apply(&EditOp::RemoveZoom { id });
    });
}

/// Set segment `index`'s playback speed (`timescale`). This changes the
/// project length, so `run` re-syncs the backend clock via `SetDuration`.
pub fn set_speed(
    project: RwSignal<Option<EditProject>>,
    history: StoredValue<Option<History>>,
    index: usize,
    timescale: f64,
) {
    run(project, history, |hist| {
        let _ = hist.apply(&EditOp::SetSpeed { index, timescale });
    });
}

/// Set the crop rectangle (a full-frame rect clears the crop). The op
/// sanitizes the rect to a valid in-frame sub-rect.
pub fn set_crop(
    project: RwSignal<Option<EditProject>>,
    history: StoredValue<Option<History>>,
    rect: edit::style::CropRect,
) {
    run(project, history, |hist| {
        let _ = hist.apply(&EditOp::SetCrop { rect });
    });
}

/// Set the output aspect ratio (reframes the export canvas).
pub fn set_aspect(
    project: RwSignal<Option<EditProject>>,
    history: StoredValue<Option<History>>,
    ratio: edit::style::AspectRatio,
) {
    run(project, history, |hist| {
        let _ = hist.apply(&EditOp::SetAspect { ratio });
    });
}

/// Set the background framing config (backdrop + padding / radius / shadow).
pub fn set_background(
    project: RwSignal<Option<EditProject>>,
    history: StoredValue<Option<History>>,
    config: edit::style::BackgroundConfig,
) {
    run(project, history, move |hist| {
        let _ = hist.apply(&EditOp::SetBackground { config });
    });
}

/// Undo the last edit (the trim bin — nothing is lost).
pub fn undo(project: RwSignal<Option<EditProject>>, history: StoredValue<Option<History>>) {
    run(project, history, |hist| {
        hist.undo();
    });
}

/// Redo the last undone edit.
pub fn redo(project: RwSignal<Option<EditProject>>, history: StoredValue<Option<History>>) {
    run(project, history, |hist| {
        hist.redo();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use edit::ClipRef;
    use std::path::PathBuf;

    fn project(path: &str) -> EditProject {
        EditProject::from_recording(ClipRef::new(PathBuf::from(path), 1920, 1080, 30, 900))
    }

    #[test]
    fn resolve_reuses_same_clip_history_preserving_undo() {
        let p = project("/tmp/a.mp4");
        let mut h = History::new(p.clone());
        h.apply(&EditOp::Split { at: 300 }).unwrap();
        assert!(h.can_undo());
        // Same clip → reuse the history, undo stack intact.
        let resolved = resolve_history(Some(h), &p);
        assert!(resolved.can_undo());
    }

    #[test]
    fn resolve_starts_fresh_for_a_different_clip() {
        let mut h = History::new(project("/tmp/a.mp4"));
        h.apply(&EditOp::Split { at: 300 }).unwrap();
        let b = project("/tmp/b.mp4");
        let resolved = resolve_history(Some(h), &b);
        assert!(!resolved.can_undo(), "different clip → fresh history");
        assert_eq!(resolved.project().source.path, b.source.path);
    }

    #[test]
    fn resolve_none_starts_fresh() {
        let p = project("/tmp/a.mp4");
        let resolved = resolve_history(None, &p);
        assert!(!resolved.can_undo());
        assert_eq!(resolved.project().segments.len(), 1);
    }

    #[test]
    fn segment_range_is_cumulative_project_frames() {
        let mut p = project("/tmp/a.mp4");
        p.segments = vec![
            edit::TimelineSegment::new(0, 300),
            edit::TimelineSegment::new(300, 900),
        ];
        assert_eq!(segment_project_range(&p, 0), Some((0, 300)));
        assert_eq!(segment_project_range(&p, 1), Some((300, 900)));
        assert_eq!(segment_project_range(&p, 2), None);
    }
}
