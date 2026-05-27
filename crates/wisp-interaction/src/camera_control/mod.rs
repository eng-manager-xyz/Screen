//! Camera controllers — convert pointer / wheel input into camera
//! pose changes. Two flavours:
//!
//! - [`OrbitController`] — 3D orbit around a target point. Port of
//!   Three.js's `OrbitControls.js` (state machine, spherical-coord
//!   math, damping, dolly clamps, auto-rotate).
//! - [`PanZoomController`] — 2D pan + zoom-around-pointer (Figma /
//!   Google-Maps math). Mutates a [`Viewport2D`] you pass in.
//!
//! The 3D controller stays decoupled from `wisp-3d` via a tiny
//! [`Camera3D`] trait — the controller mutates whatever struct
//! impls it. Hosts (wisp-3d-web, the recorder) provide the trivial
//! impl over `wisp_3d::Camera3D`. This keeps `wisp-interaction` free
//! of a `wisp-3d` dependency so the publish dep direction stays:
//! `wisp` → `wisp-interaction` → wisp-3d-web (the consumer joins the
//! two), never `wisp-interaction → wisp-3d`.

pub mod orbit;
pub mod pan_zoom;

pub use orbit::{Camera3D, OrbitController, OrbitState};
pub use pan_zoom::{PanZoomController, Viewport2D};
