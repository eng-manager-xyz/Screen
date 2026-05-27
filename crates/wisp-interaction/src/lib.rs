//! `wisp-interaction` — input + hit-testing + camera controllers for
//! the wisp family.
//!
//! ## Why a separate crate
//!
//! Today every wisp-family crate is input-blind. `wisp` ships a 2D
//! scene graph, `wisp-3d` a perspective camera + meshes, `wisp-chart`
//! 28 chart kinds, `wisp-animation` a Driver + tweens — none of them
//! know what a click is. Adding input handling to each individually
//! would produce N inconsistent APIs; this crate owns the vocabulary
//! once and consumers take a dependency for the bits they need:
//!
//! - **hit-test backend** for `wisp` (2D scene-graph picking)
//! - **raycast backend** for `wisp-3d` (mesh picking — follow-up)
//! - **camera controllers** ([`Camera3D`] orbit for `wisp-3d`,
//!   pan/zoom for `wisp`-based scenes)
//! - **event triggers** that wire pointer events into
//!   `wisp-animation` tweens
//! - **reverse-lookup maps** for `wisp-chart` (`NodeId` → `ChartElementId`)
//!
//! ## Reference engines
//!
//! The public API cross-references three precedents (deep-research
//! memos captured in the AUT-303 Linear ticket):
//!
//! - **Bevy 0.18** — adopt the [`ButtonInput<T>`] three-set state
//!   shape, the `Pointer<E>` typed event taxonomy, the per-`PointerId`
//!   multi-touch state map, and the backend-trait pattern.
//! - **`PixiJS` v8** — adopt the press-path bookkeeping for drag-
//!   without-OS-capture and the cursor-style indirection callback.
//! - **Three.js r170+** — port `OrbitControls.js`'s state machine,
//!   spherical-coords math, damping, and touch handlers.
//!
//! We deliberately **skip** ECS, 3-phase DOM event propagation, the
//! 5-state `eventMode` enum, and brute-force per-triangle ray scans.
//!
//! ## Stage of work
//!
//! WI.0 ships the crate skeleton. The remaining surface lands across
//! the AUT-303..AUT-314 chunk sequence. Each module exposes its types
//! once it's filled in; the empty modules below are placeholders so
//! the public path conventions are pinned from day one.
//!
//! See `_docs/book/src/wisp-interaction/overview.md` for the in-depth
//! tour with embedded live examples.
//!
//! [`Camera3D`]: https://docs.rs/wisp-3d/latest/wisp_3d/struct.Camera3D.html

#![doc(html_root_url = "https://docs.rs/screen-wisp-interaction/0.1.0")]

pub use wisp::application::Application;
pub use wisp::scene::{NodeId, Stage};

pub mod hit_test;
pub mod input;
pub mod pointer;

pub use hit_test::{HitShape, HitTestBackend, Pickable, PickableMap, Wisp2dHitTest};
pub use input::{
    AccumulatedMouseMotion, ButtonInput, InputEvent, KeyCode, KeyboardEvent, KeyboardInput,
    ModifierState, MouseButton, MouseButtonEvent, MouseButtonInput, MouseMotionEvent,
    MouseWheelEvent, TouchEvent, TouchPhase, WheelDelta,
};
pub use pointer::{
    CallbackRegistry, Cancel, Click, Drag, DragDrop, DragEnd, DragEnter, DragLeave, DragOver,
    DragStart, EventKind, Hit, Move, Out, Over, Pointer, PointerDispatcher, PointerId,
    PointerLocation, Press, Release, Scroll,
};

// W I.4+ modules land in subsequent commits.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_reexport_resolves_to_wisp_application() {
        // Anchor: prevents accidental re-export shadowing during
        // future refactors. The `wisp::application::Application` path
        // is the canonical source.
        fn _accepts(_: Application) {}
        fn _accepts_native(_: wisp::application::Application) {}
    }

    #[test]
    fn scene_types_reexport_resolves() {
        // NodeId + Stage are the load-bearing types every consumer
        // touches. If wisp ever moves them, this test breaks at
        // compile time, not at runtime.
        fn _accepts_node(_: NodeId) {}
        fn _accepts_stage(_: &Stage) {}
    }
}
