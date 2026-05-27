//! `Pointer<E>` envelope + 15-variant event taxonomy.
//!
//! Shape mirrors Bevy 0.18's `bevy_picking::events::Pointer<E>`
//! (`crates/bevy_picking/src/events.rs:139-340`). We deliberately
//! keep the typed-event-per-kind shape (one struct per Click / Drag /
//! Scroll / etc.) instead of a single enum because handler registry
//! lookup is `(NodeId, EventKind)` → list of `Fn(&Pointer<E>)` and
//! per-kind type safety is the whole point.

use std::cell::Cell;

use glam::Vec2;

use crate::input::{ModifierState, MouseButton, WheelDelta};
use wisp::scene::NodeId;

/// Identifies a single pointer.
///
/// Matches Bevy's `PointerId::{Mouse, Touch(u64), Custom(Uuid)}`
/// shape (`pointer.rs:32-46`) but uses `u128` instead of Uuid to
/// avoid pulling a `uuid` dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerId {
    /// The system mouse / trackpad pointer.
    Mouse,
    /// A touch contact. The `u64` is OS-assigned and stable for the
    /// lifetime of a single finger contact (matches
    /// `winit::event::Touch::id` + DOM `Touch.identifier`).
    Touch(u64),
    /// Custom pointer (synthetic test pointer, future VR-ray, etc.).
    Custom(u128),
}

/// Coordinates + modifiers at the moment of dispatch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerLocation {
    /// Position in CSS / logical pixels relative to the host
    /// surface's top-left.
    pub viewport: Vec2,
    /// Modifier-key snapshot.
    pub modifiers: ModifierState,
}

/// One hit-test result. Produced by WI.3's `HitTestBackend`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// Hit node.
    pub node: NodeId,
    /// Depth from the camera (for 3D) or sort key (for 2D).
    /// Smaller is "in front". Hits get sorted by this before
    /// dispatch.
    pub depth: f32,
    /// Pointer position in the hit node's LOCAL space (after the
    /// node's world transform inverse is applied).
    pub local_pos: Vec2,
}

/// Envelope wrapping a typed pointer event with target + ID +
/// location + propagation flag.
///
/// The `stop_bubble` field uses `Cell<bool>` so handlers with
/// `Fn(&Pointer<E>)` signature (no `&mut`) can still halt bubbling.
/// `Cell<bool>` is `Copy + Send` — but `Pointer<E>` itself is
/// !Sync because of the Cell. That's fine; single-threaded dispatch.
#[derive(Debug)]
pub struct Pointer<E> {
    /// The hit node this event targets.
    pub target: NodeId,
    /// Which pointer.
    pub pointer_id: PointerId,
    /// Coords + modifiers at dispatch time.
    pub location: PointerLocation,
    /// Typed event payload.
    pub event: E,
    /// Flipped by [`stop_bubble`](Pointer::stop_bubble); the
    /// dispatcher reads after each handler call and halts ancestor
    /// dispatch if true.
    bubble_stopped: Cell<bool>,
}

impl<E> Pointer<E> {
    /// Construct a Pointer event. Used by the dispatcher; callers
    /// rarely build these directly.
    pub fn new(target: NodeId, pointer_id: PointerId, location: PointerLocation, event: E) -> Self {
        Self {
            target,
            pointer_id,
            location,
            event,
            bubble_stopped: Cell::new(false),
        }
    }

    /// Halt event bubbling. Subsequent ancestor handlers won't fire.
    /// Equivalent to Bevy's `On::propagate(false)` and Pixi's
    /// `stopPropagation()`.
    pub fn stop_bubble(&self) {
        self.bubble_stopped.set(true);
    }

    /// `true` if [`stop_bubble`](Self::stop_bubble) has been called.
    pub fn is_bubble_stopped(&self) -> bool {
        self.bubble_stopped.get()
    }
}

// ─── 15 event variants ─────────────────────────────────────────────

/// Pointer entered the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Over;

/// Pointer left the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Out;

/// Pointer moved while over the target. `delta` is the change since
/// the previous move event for this pointer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Move {
    /// Motion delta in CSS / logical pixels.
    pub delta: Vec2,
}

/// Pointer was forcibly cancelled (OS interruption, e.g. incoming
/// call on mobile). Consumers should release any in-progress state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cancel;

/// Mouse button went down on the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Press {
    /// Which button.
    pub button: MouseButton,
}

/// Mouse button went up while over the target's PRESS PATH.
/// (May not be the same target as the Press if the pointer moved.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Release {
    /// Which button.
    pub button: MouseButton,
}

/// A press + release pair both landed on the same target with
/// movement under [`CLICK_THRESHOLD_PX`](crate::pointer::dispatcher::CLICK_THRESHOLD_PX).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Click {
    /// Which button.
    pub button: MouseButton,
    /// Hit details at click time.
    pub hit: Hit,
}

/// Drag began (a press moved beyond
/// [`CLICK_THRESHOLD_PX`](crate::pointer::dispatcher::CLICK_THRESHOLD_PX)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DragStart {
    /// Which button initiated.
    pub button: MouseButton,
}

/// Drag in progress. `delta` is the motion since the last `Drag`
/// event; `distance` is the cumulative motion since `DragStart`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drag {
    /// Which button.
    pub button: MouseButton,
    /// Motion delta since the last `Drag`.
    pub delta: Vec2,
    /// Cumulative motion since the originating `DragStart`.
    pub distance: Vec2,
}

/// Drag ended (button released).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DragEnd {
    /// Which button.
    pub button: MouseButton,
}

/// A drag entered the target (pointer moved over while dragging
/// another node).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DragEnter {
    /// The dragged source node.
    pub dragged: NodeId,
}

/// Drag is over the target (per-move event, like `Move` but for
/// drag-over instead of plain hover).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DragOver {
    /// The dragged source node.
    pub dragged: NodeId,
}

/// Drag left the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DragLeave {
    /// The dragged source node.
    pub dragged: NodeId,
}

/// Drag was released over the target (drop completed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DragDrop {
    /// The dropped source node.
    pub dropped: NodeId,
}

/// Wheel rotated while over the target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scroll {
    /// Scroll delta + unit (pixel vs. line).
    pub delta: WheelDelta,
}

// ─── EventKind tag ────────────────────────────────────────────────

/// Stable tag identifying an event variant, used as the secondary
/// key in [`crate::pointer::CallbackRegistry`] alongside `NodeId`.
///
/// (Cannot use `TypeId` of the inner event because the registry
/// stores type-erased boxes by kind; a closure registered for
/// `Click` must not be invoked for `DragStart` even though both are
/// `Pointer<E>`.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// [`Over`] events.
    Over,
    /// [`Out`] events.
    Out,
    /// [`Move`] events.
    Move,
    /// [`Cancel`] events.
    Cancel,
    /// [`Press`] events.
    Press,
    /// [`Release`] events.
    Release,
    /// [`Click`] events.
    Click,
    /// [`DragStart`] events.
    DragStart,
    /// [`Drag`] events.
    Drag,
    /// [`DragEnd`] events.
    DragEnd,
    /// [`DragEnter`] events.
    DragEnter,
    /// [`DragOver`] events.
    DragOver,
    /// [`DragLeave`] events.
    DragLeave,
    /// [`DragDrop`] events.
    DragDrop,
    /// [`Scroll`] events.
    Scroll,
}
