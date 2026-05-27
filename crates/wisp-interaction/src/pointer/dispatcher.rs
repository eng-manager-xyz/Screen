//! `PointerDispatcher` — the wiring that takes raw pointer events
//! plus a hit-test result and emits typed `Pointer<E>` events to the
//! [`CallbackRegistry`].
//!
//! Bubble walk is type-keyed via the [`Event`] trait, which ties an
//! event marker (`Click`, `Drag`, etc.) to its `EventKind` tag and
//! callback variant. The generic [`bubble`] function is a single
//! impl that works for every event type without `unsafe`.

use glam::Vec2;

use crate::input::{MouseButton, WheelDelta};
use crate::pointer::events::{
    Cancel, Click, Drag, DragDrop, DragEnd, DragEnter, DragLeave, DragOver, DragStart, EventKind,
    Hit, Move, Out, Over, Pointer, PointerId, PointerLocation, Press, Release, Scroll,
};
use crate::pointer::registry::{Callback, CallbackRegistry};
use crate::pointer::state::PointerStateMap;
use wisp::scene::{NodeId, Stage};

/// Distance (in viewport pixels) a press must travel before being
/// reclassified from a click to a drag. Matches `PixiJS` default.
pub const CLICK_THRESHOLD_PX: f32 = 5.0;

/// Type-erased reference to a `Fn(&Pointer<E>)` handler. Used as
/// the return type of [`Event::extract`] so the dispatcher can
/// invoke a registered closure without downcasting.
pub type HandlerRef<'a, E> = &'a dyn Fn(&Pointer<E>);

/// Marker trait tying a concrete event type to its `EventKind` tag
/// and the callback variant that holds the matching closure.
///
/// Allows [`bubble`] to be one generic function over `E: Event`
/// instead of 15 hand-written copies or an `unsafe` transmute hack.
pub trait Event: Sized + 'static {
    /// The `EventKind` enum variant this event maps to.
    fn kind() -> EventKind;

    /// Extract the typed handler from a `Callback`, if the variant
    /// matches. Returns `None` if `cb` is for a different event type.
    fn extract(cb: &Callback) -> Option<HandlerRef<'_, Self>>;
}

macro_rules! impl_event {
    ($ty:ty, $kind:ident, $variant:ident) => {
        impl Event for $ty {
            fn kind() -> EventKind {
                EventKind::$kind
            }
            fn extract(cb: &Callback) -> Option<HandlerRef<'_, Self>> {
                if let Callback::$variant(f) = cb {
                    Some(f.as_ref())
                } else {
                    None
                }
            }
        }
    };
}

impl_event!(Over, Over, Over);
impl_event!(Out, Out, Out);
impl_event!(Move, Move, Move);
impl_event!(Cancel, Cancel, Cancel);
impl_event!(Press, Press, Press);
impl_event!(Release, Release, Release);
impl_event!(Click, Click, Click);
impl_event!(DragStart, DragStart, DragStart);
impl_event!(Drag, Drag, Drag);
impl_event!(DragEnd, DragEnd, DragEnd);
impl_event!(DragEnter, DragEnter, DragEnter);
impl_event!(DragOver, DragOver, DragOver);
impl_event!(DragLeave, DragLeave, DragLeave);
impl_event!(DragDrop, DragDrop, DragDrop);
impl_event!(Scroll, Scroll, Scroll);

/// Main entry point for pointer event dispatch.
#[derive(Default)]
pub struct PointerDispatcher {
    state: PointerStateMap,
}

impl PointerDispatcher {
    /// Construct empty.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow per-pointer state immutably (tests + diagnostics).
    #[must_use]
    pub fn state(&self) -> &PointerStateMap {
        &self.state
    }

    /// Update last-known location for this pointer and, if the
    /// topmost hit changed, emit `Out` to the previous hover target
    /// and `Over` to the new one. Also emits `Move` to the current
    /// target and `Drag` for any in-progress drag.
    ///
    /// `hits` is the back-to-front list from the hit-test backend
    /// (topmost first). Pass empty for "pointer over nothing".
    pub fn on_pointer_move(
        &mut self,
        pointer_id: PointerId,
        location: PointerLocation,
        hits: &[Hit],
        stage: &Stage,
        registry: &CallbackRegistry,
    ) {
        let ptr_state = self.state.entry(pointer_id);
        let prev_hover = ptr_state.current_hover;
        let new_hover = hits.first().map(|h| h.node);

        let delta = ptr_state
            .last_location
            .map_or(Vec2::ZERO, |prev| location.viewport - prev.viewport);
        ptr_state.last_location = Some(location);

        let promoted = ptr_state.accumulate_motion(delta, CLICK_THRESHOLD_PX);

        // Snapshot per-button drag info (release the &mut ptr_state borrow).
        let drag_info: Vec<(MouseButton, NodeId, Vec2)> = ptr_state
            .press_entries
            .iter()
            .filter(|(_, e)| e.is_dragging)
            .map(|(b, e)| (*b, e.press_target, e.distance_moved))
            .collect();

        ptr_state.current_hover = new_hover;

        // DragStart for newly-promoted buttons.
        for button in promoted {
            if let Some(entry) = self.state.entry(pointer_id).press_entries.get(&button) {
                let target = entry.press_target;
                bubble(
                    target,
                    pointer_id,
                    location,
                    DragStart { button },
                    stage,
                    registry,
                );
            }
        }

        // Out / Over.
        if prev_hover != new_hover {
            if let Some(prev) = prev_hover {
                bubble(prev, pointer_id, location, Out, stage, registry);
            }
            if let Some(new) = new_hover {
                bubble(new, pointer_id, location, Over, stage, registry);
            }
        }

        // Move to current hover.
        if let Some(target) = new_hover {
            bubble(
                target,
                pointer_id,
                location,
                Move { delta },
                stage,
                registry,
            );
        }

        // Drag per in-progress button.
        for (button, target, distance) in drag_info {
            bubble(
                target,
                pointer_id,
                location,
                Drag {
                    button,
                    delta,
                    distance,
                },
                stage,
                registry,
            );
        }
    }

    /// Handle a mouse-button press.
    pub fn on_pointer_press(
        &mut self,
        pointer_id: PointerId,
        location: PointerLocation,
        button: MouseButton,
        hits: &[Hit],
        stage: &Stage,
        registry: &CallbackRegistry,
    ) {
        let Some(hit) = hits.first() else {
            return;
        };
        let path = ancestor_path(hit.node, stage);
        let ptr_state = self.state.entry(pointer_id);
        ptr_state.record_press(button, hit.node, path, location.viewport);
        // Seed last_location so the next move's `delta` is the
        // press-to-first-move distance (not zero). Otherwise the drag
        // threshold can only fire after at least two move events,
        // which surprises consumers.
        ptr_state.last_location = Some(location);

        bubble(
            hit.node,
            pointer_id,
            location,
            Press { button },
            stage,
            registry,
        );
    }

    /// Handle a mouse-button release.
    pub fn on_pointer_release(
        &mut self,
        pointer_id: PointerId,
        location: PointerLocation,
        button: MouseButton,
        hits: &[Hit],
        stage: &Stage,
        registry: &CallbackRegistry,
    ) {
        let ptr_state = self.state.entry(pointer_id);
        let Some(entry) = ptr_state.consume_press(button) else {
            return;
        };

        let release_target = entry.press_target;
        bubble(
            release_target,
            pointer_id,
            location,
            Release { button },
            stage,
            registry,
        );

        if entry.is_dragging {
            bubble(
                release_target,
                pointer_id,
                location,
                DragEnd { button },
                stage,
                registry,
            );
        } else {
            // Click iff the release-time hit shares the press path.
            let click_target = hits
                .iter()
                .find(|h| h.node == release_target || entry.path.contains(&h.node))
                .copied();
            if let Some(hit) = click_target {
                bubble(
                    hit.node,
                    pointer_id,
                    location,
                    Click { button, hit },
                    stage,
                    registry,
                );
            }
        }
    }

    /// Handle a wheel / scroll event.
    pub fn on_pointer_scroll(
        &mut self,
        pointer_id: PointerId,
        location: PointerLocation,
        delta: WheelDelta,
        hits: &[Hit],
        stage: &Stage,
        registry: &CallbackRegistry,
    ) {
        let Some(hit) = hits.first() else {
            return;
        };
        bubble(
            hit.node,
            pointer_id,
            location,
            Scroll { delta },
            stage,
            registry,
        );
    }

    /// Cancel an in-progress pointer interaction.
    pub fn on_pointer_cancel(
        &mut self,
        pointer_id: PointerId,
        location: PointerLocation,
        stage: &Stage,
        registry: &CallbackRegistry,
    ) {
        if let Some(state) = self.state.get(pointer_id) {
            // Snapshot before the forget below.
            let targets: Vec<NodeId> = state
                .press_entries
                .values()
                .map(|e| e.press_target)
                .collect();
            for target in targets {
                bubble(target, pointer_id, location, Cancel, stage, registry);
            }
        }
        self.state.forget(pointer_id);
    }
}

/// Bubble walk: emit to target, then walk up `Container::parent()`
/// until root or `stop_bubble()` is called. Generic over any
/// implementor of [`Event`] — registry lookup is type-keyed.
fn bubble<E: Event>(
    target: NodeId,
    pointer_id: PointerId,
    location: PointerLocation,
    event: E,
    stage: &Stage,
    registry: &CallbackRegistry,
) {
    let evt = Pointer::new(target, pointer_id, location, event);
    let mut current = Some(target);
    while let Some(node) = current {
        if let Some(cb) = registry.get(node, E::kind())
            && let Some(handler) = E::extract(cb)
        {
            handler(&evt);
            if evt.is_bubble_stopped() {
                return;
            }
        }
        current = stage.get(node).and_then(|n| n.container().parent());
    }
}

/// Walk a node's ancestor chain including the node itself.
fn ancestor_path(node: NodeId, stage: &Stage) -> Vec<NodeId> {
    let mut path = vec![node];
    let mut current = stage.get(node).and_then(|n| n.container().parent());
    while let Some(n) = current {
        path.push(n);
        current = stage.get(n).and_then(|node| node.container().parent());
    }
    path
}
