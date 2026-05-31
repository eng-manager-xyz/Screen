//! Undo / redo for the edit model.

use crate::ops::{EditError, EditOp};
use crate::project::EditProject;

/// Default maximum number of undo steps retained.
pub const DEFAULT_HISTORY_LIMIT: usize = 200;

/// A bounded undo / redo stack wrapping a live [`EditProject`].
///
/// Edits go through [`History::apply`], which snapshots the project
/// before each change. Because `apply` runs the operation on a clone and
/// only commits when the project actually changed, `apply` then `undo` is
/// guaranteed to restore the exact prior state — no fragile per-operation
/// inverse derivation, and no-op operations never pollute the undo stack.
#[derive(Clone, Debug)]
pub struct History {
    current: EditProject,
    undo: Vec<EditProject>,
    redo: Vec<EditProject>,
    limit: usize,
}

impl History {
    /// Wrap a project with an undo stack of the default depth.
    #[must_use]
    pub fn new(project: EditProject) -> Self {
        Self::with_limit(project, DEFAULT_HISTORY_LIMIT)
    }

    /// Wrap a project with an undo stack of at most `limit` steps
    /// (clamped to at least 1).
    #[must_use]
    pub fn with_limit(project: EditProject, limit: usize) -> Self {
        Self {
            current: project,
            undo: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// The live project.
    #[must_use]
    pub fn project(&self) -> &EditProject {
        &self.current
    }

    /// Whether there is anything to undo.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether there is anything to redo.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Apply an edit, recording it for undo if it changed anything.
    ///
    /// # Errors
    ///
    /// Propagates [`EditError`] from [`EditProject::apply`]. On error the
    /// project is left unchanged (the operation runs on a clone that is
    /// discarded).
    pub fn apply(&mut self, op: &EditOp) -> Result<(), EditError> {
        let mut next = self.current.clone();
        next.apply(op)?;
        if next != self.current {
            self.undo.push(std::mem::replace(&mut self.current, next));
            if self.undo.len() > self.limit {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
        Ok(())
    }

    /// Undo the last recorded change. Returns whether anything was undone
    /// (the boolean is an optional signal — `undo` is usually called for
    /// effect).
    #[allow(
        clippy::must_use_candidate,
        reason = "undo is invoked for its side effect; the bool is an optional 'did something' signal callers may ignore"
    )]
    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.current, prev));
            true
        } else {
            false
        }
    }

    /// Redo the last undone change. Returns whether anything was redone.
    #[allow(
        clippy::must_use_candidate,
        reason = "redo is invoked for its side effect; the bool is an optional 'did something' signal callers may ignore"
    )]
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.current, next));
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::ClipRef;
    use crate::ops::TrimEdge;
    use crate::segment::TimelineSegment;
    use crate::zoom::{ZoomId, ZoomSegment};
    use proptest::prelude::*;
    use std::path::PathBuf;

    fn project() -> EditProject {
        EditProject::from_recording(ClipRef::new(
            PathBuf::from("/tmp/rec.mp4"),
            1920,
            1080,
            30,
            900,
        ))
    }

    #[test]
    fn apply_records_and_undo_redo_round_trip() {
        let mut h = History::new(project());
        assert!(!h.can_undo());
        h.apply(&EditOp::Split { at: 300 }).unwrap();
        assert!(h.can_undo());
        assert_eq!(h.project().segments.len(), 2);

        assert!(h.undo());
        assert_eq!(h.project().segments.len(), 1);
        assert!(h.can_redo());

        assert!(h.redo());
        assert_eq!(h.project().segments.len(), 2);
        assert_eq!(h.project().segments[1], TimelineSegment::new(300, 900));
    }

    #[test]
    fn noop_apply_does_not_record() {
        let mut h = History::new(project());
        h.apply(&EditOp::Split { at: 0 }).unwrap(); // boundary → no change
        assert!(!h.can_undo());
    }

    #[test]
    fn new_apply_clears_redo() {
        let mut h = History::new(project());
        h.apply(&EditOp::Split { at: 300 }).unwrap();
        h.undo();
        assert!(h.can_redo());
        h.apply(&EditOp::SetSpeed {
            index: 0,
            timescale: 2.0,
        })
        .unwrap();
        assert!(!h.can_redo());
    }

    #[test]
    fn history_limit_bounds_undo_depth() {
        let mut h = History::with_limit(project(), 3);
        for ts in [1.5, 2.0, 2.5, 3.0, 0.5] {
            h.apply(&EditOp::SetSpeed {
                index: 0,
                timescale: ts,
            })
            .unwrap();
        }
        let mut undos = 0;
        while h.undo() {
            undos += 1;
        }
        assert_eq!(undos, 3, "only the last `limit` changes are undoable");
    }

    fn any_op() -> impl Strategy<Value = EditOp> {
        prop_oneof![
            (0u64..1000).prop_map(|at| EditOp::Split { at }),
            (
                0usize..4,
                prop_oneof![Just(TrimEdge::Start), Just(TrimEdge::End)],
                0u64..1000
            )
                .prop_map(|(index, edge, to)| EditOp::Trim { index, edge, to }),
            (0u64..1000, 0u64..1000).prop_map(|(a, b)| EditOp::RippleDelete {
                start: a.min(b),
                end: a.max(b)
            }),
            (0usize..4, 0.2f64..5.0)
                .prop_map(|(index, timescale)| EditOp::SetSpeed { index, timescale }),
            (0u64..880, 1u64..40, 1.0f64..3.0).prop_map(|(s, d, amount)| EditOp::AddZoom {
                zoom: ZoomSegment::manual(ZoomId(0), s, s + d, amount)
            }),
            (0u32..8).prop_map(|id| EditOp::RemoveZoom { id: ZoomId(id) }),
            (0u32..8, 0u64..880, 1u64..40).prop_map(|(id, s, d)| EditOp::MoveZoom {
                id: ZoomId(id),
                start: s,
                end: s + d
            }),
        ]
    }

    proptest! {
        #[test]
        fn single_op_apply_then_undo_is_identity(op in any_op()) {
            let mut h = History::new(project());
            let before = h.project().clone();
            let _ = h.apply(&op);
            h.undo();
            prop_assert_eq!(h.project(), &before);
        }

        #[test]
        fn op_sequence_preserves_invariants(ops in prop::collection::vec(any_op(), 0..25)) {
            let mut h = History::new(project());
            for op in &ops {
                let _ = h.apply(op);
                prop_assert!(
                    h.project().check_invariants().is_ok(),
                    "invariant violated after {:?}: {:?}",
                    op,
                    h.project().check_invariants()
                );
            }
        }

        #[test]
        fn undo_all_returns_to_start(ops in prop::collection::vec(any_op(), 0..25)) {
            let start = project();
            let mut h = History::new(start.clone());
            for op in &ops {
                let _ = h.apply(op);
            }
            while h.undo() {}
            prop_assert_eq!(h.project(), &start);
        }
    }
}
