//! Adapters — translate platform input events into `wisp-interaction`
//! vocabulary.
//!
//! Each adapter is feature-gated so the base crate stays platform-
//! agnostic. Native hosts enable `winit`; browser bundles enable
//! `web`. Adapters expose pure translation functions (winit event →
//! `InputEvent`, `web_sys::PointerEvent` → `Pointer<E>` call) so the
//! integration is testable without ever opening a window.

#[cfg(feature = "winit")]
pub mod winit;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub mod web;
