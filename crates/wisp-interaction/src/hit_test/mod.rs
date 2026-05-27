//! Hit-testing — pluggable backend that maps a viewport pointer to a
//! `Vec<Hit>` of pickable nodes (WI.3 / AUT-306).
//!
//! Three pieces:
//!
//! - [`HitShape`] — per-node pickable geometry in LOCAL coordinates
//!   (`Rect`, `Circle`, `Ellipse`, `Polygon`, `None`).
//! - [`Pickable`] + [`PickableMap`] — side-table registering which
//!   `NodeId`s are pickable + their shape. **Not** stored on
//!   `wisp::Node` (keeps `wisp` interaction-free).
//! - [`HitTestBackend`] trait + [`Wisp2dHitTest`] impl — walks the
//!   stage, composes world matrices, optionally builds an R-tree
//!   spatial index, and returns hits sorted topmost-first.
//!
//! Lifecycle: build the backend ONCE per scene-graph change and
//! reuse it for every pointer event in that frame. The cost of
//! rebuilding is `O(N)` in stage size; the `pick` call is `O(P)`
//! linear or `O(log P)` indexed, where `P` is the pickable count.

pub mod backend;
pub mod pickable;
pub mod shape;
pub mod wisp_2d;

pub use backend::HitTestBackend;
pub use pickable::{Pickable, PickableMap};
pub use shape::HitShape;
pub use wisp_2d::Wisp2dHitTest;
