//! Pointer event taxonomy + dispatcher + multi-pointer state
//! (WI.2 / AUT-305).
//!
//! - [`Pointer<E>`] envelope carries `target` (the hit `NodeId`),
//!   `pointer_id` (mouse / touch / custom), `location` (viewport
//!   coords + modifiers), and the typed `event` payload.
//! - 15-variant event taxonomy: `Over`, `Out`, `Move`, `Cancel`,
//!   `Press`, `Release`, `Click`, `DragStart`, `Drag`, `DragEnd`,
//!   `DragEnter`, `DragOver`, `DragLeave`, `DragDrop`, `Scroll`.
//!   Matches Bevy `crates/bevy_picking/src/events.rs:139-340`.
//! - [`PointerDispatcher`] keeps per-`PointerId` state (last position,
//!   pressed buttons, press-path per button — `PixiJS` pattern from
//!   `EventBoundary.ts:677-708` so drag survives the pointer leaving
//!   the originating target).
//! - [`CallbackRegistry`] stores `Box<dyn Fn(&Pointer<E>)>` closures
//!   keyed by `(NodeId, EventKind)`. Bubble walks up the wisp `Stage`
//!   ancestor chain (`Container::parent()`); `Pointer::stop_bubble()`
//!   halts further ancestors.

pub mod dispatcher;
pub mod events;
pub mod registry;
pub mod state;

pub use dispatcher::PointerDispatcher;
pub use events::{
    Cancel, Click, Drag, DragDrop, DragEnd, DragEnter, DragLeave, DragOver, DragStart, EventKind,
    Hit, Move, Out, Over, Pointer, PointerId, PointerLocation, Press, Release, Scroll,
};
pub use registry::CallbackRegistry;
pub use state::{PointerState, PressEntry};
