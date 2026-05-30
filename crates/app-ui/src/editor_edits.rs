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
    project.set(Some(hist.project().clone()));
    history.set_value(Some(hist));
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
}
