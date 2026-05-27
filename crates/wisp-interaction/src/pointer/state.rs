//! Per-`PointerId` state — last position, pressed buttons, press-
//! path bookkeeping for drag-without-OS-capture (Pixi pattern).

use std::collections::HashMap;

use glam::Vec2;

use crate::input::MouseButton;
use crate::pointer::events::{PointerId, PointerLocation};
use wisp::scene::NodeId;

/// Snapshot of one button's press path. When the pointer leaves the
/// original target before release, the dispatcher still walks this
/// recorded ancestor chain for the release / drag-end / drop event.
///
/// Mirrors `PixiJS`'s `EventBoundary.ts:677-708` (`pressTargetsByButton`)
/// → `EventBoundary.ts:1092-1133` (`mapPointerUpOutside`) replay
/// pattern. The chain is the ancestor walk at press time (target +
/// all parents up to the scene root); the dispatcher checks
/// `is_dragging` to decide whether to emit a `Click` or a `DragEnd`
/// on release.
#[derive(Debug, Clone)]
pub struct PressEntry {
    /// Node where the press landed.
    pub press_target: NodeId,
    /// Ancestor chain at press time. Index 0 = `press_target`, index
    /// N = root.
    pub path: Vec<NodeId>,
    /// Viewport position of the press.
    pub press_pos: Vec2,
    /// Cumulative motion since press. Used to distinguish click
    /// (< threshold) from drag (≥ threshold).
    pub distance_moved: Vec2,
    /// Once `distance_moved` exceeds `CLICK_THRESHOLD_PX`, the press
    /// is reclassified as a drag and this flag flips. Stays true
    /// until release.
    pub is_dragging: bool,
}

/// Per-`PointerId` state owned by the dispatcher.
#[derive(Debug, Clone, Default)]
pub struct PointerState {
    /// Last known viewport location for this pointer. `None` until
    /// the first move event.
    pub last_location: Option<PointerLocation>,
    /// Currently-hovered target (the topmost hit at `last_location`),
    /// updated on every move. Used to emit `Over` / `Out` pairs
    /// when it changes.
    pub current_hover: Option<NodeId>,
    /// Press path per held button. Keyed by `MouseButton`, since one
    /// pointer can have multiple buttons held simultaneously
    /// (left + right). Each entry's lifetime spans press → release.
    pub press_entries: HashMap<MouseButton, PressEntry>,
}

impl PointerState {
    /// Construct empty state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a press: insert a fresh `PressEntry` keyed by button.
    pub fn record_press(
        &mut self,
        button: MouseButton,
        press_target: NodeId,
        path: Vec<NodeId>,
        press_pos: Vec2,
    ) {
        self.press_entries.insert(
            button,
            PressEntry {
                press_target,
                path,
                press_pos,
                distance_moved: Vec2::ZERO,
                is_dragging: false,
            },
        );
    }

    /// Remove and return the press entry on release. `None` if no
    /// prior press for this button (out-of-order release).
    pub fn consume_press(&mut self, button: MouseButton) -> Option<PressEntry> {
        self.press_entries.remove(&button)
    }

    /// Update cumulative motion for every held button. Called from
    /// the dispatcher on every motion event. Returns the set of
    /// buttons that just crossed the drag threshold this update.
    pub fn accumulate_motion(&mut self, delta: Vec2, threshold: f32) -> Vec<MouseButton> {
        let mut promoted = Vec::new();
        for (button, entry) in &mut self.press_entries {
            entry.distance_moved += delta;
            if !entry.is_dragging && entry.distance_moved.length() >= threshold {
                entry.is_dragging = true;
                promoted.push(*button);
            }
        }
        promoted
    }
}

/// Per-pointer state map. Keyed by `PointerId`. Lookup is one
/// `HashMap` probe per event — for the realistic case of 1 mouse +
/// up-to-10 touches that's fine.
#[derive(Debug, Default)]
pub struct PointerStateMap {
    inner: HashMap<PointerId, PointerState>,
}

impl PointerStateMap {
    /// Construct empty.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get-or-insert state for this pointer.
    pub fn entry(&mut self, id: PointerId) -> &mut PointerState {
        self.inner.entry(id).or_default()
    }

    /// Borrow state immutably; returns `None` if no prior events
    /// for this pointer.
    #[must_use]
    pub fn get(&self, id: PointerId) -> Option<&PointerState> {
        self.inner.get(&id)
    }

    /// Forget a pointer entirely. Called on `Cancel` events to
    /// clean up touch lifecycles.
    pub fn forget(&mut self, id: PointerId) {
        self.inner.remove(&id);
    }
}
