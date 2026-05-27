//! `HitTestBackend` trait — pluggable picking strategy.
//!
//! The trait is intentionally tiny: take a viewport-space point, return
//! the back-to-front list of hits (topmost first). Concrete backends:
//!
//! - [`Wisp2dHitTest`](super::Wisp2dHitTest) — wisp 2D scene-graph,
//!   walks the stage tree, applies the `PickableMap` filter, transforms
//!   the pointer into each node's local space, and tests against the
//!   per-node [`HitShape`](super::HitShape).
//! - `Wisp3DHitTest` — perspective camera + mesh raycast. Lands in a
//!   follow-up ticket alongside wisp-3d.
//!
//! The dispatcher calls `pick` once per pointer event and feeds the
//! `Vec<Hit>` into [`PointerDispatcher::on_pointer_*`](crate::pointer::PointerDispatcher).

use glam::Vec2;

use crate::pointer::Hit;

/// Pluggable hit-test strategy.
pub trait HitTestBackend {
    /// Return the back-to-front list of hits under `viewport_pointer`.
    ///
    /// `viewport_pointer` is in the same coordinate space the host
    /// passes to [`crate::pointer::PointerLocation::viewport`] —
    /// typically CSS / logical pixels relative to the host surface's
    /// top-left, no DPR scaling.
    ///
    /// Index 0 is the topmost hit (the one the renderer would draw
    /// last). An empty `Vec` means "pointer over nothing pickable".
    fn pick(&self, viewport_pointer: Vec2) -> Vec<Hit>;
}
