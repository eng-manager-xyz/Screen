//! `wisp-chart-web` — Trunk-driven WebGPU demo for `wisp-chart`.
//!
//! The same `wisp-chart` crate compiles for native AND
//! `wasm32-unknown-unknown`. This crate is the browser-facing
//! consumer: it owns the HTML page, finds a `<canvas>`, creates
//! a `wgpu::Surface` from it with the `BROWSER_WEBGPU` backend,
//! and draws whatever `wisp-chart` returns.
//!
//! Build with Trunk:
//!
//! ```bash
//! cd crates/wisp-chart-web
//! trunk build --release
//! ```
//!
//! Or run the dev server:
//!
//! ```bash
//! just dev-wisp-chart-demo
//! ```
//!
//! Today the demo clears the canvas to white and prints a
//! diagnostic message to the JS console — enough to prove the
//! WebGPU path is wired up. The actual chart rendering lands in
//! M-CHART.0 chunk 3 (when `Gantt::render(&Theme) -> SceneNode`
//! ships).

#![cfg_attr(docsrs, feature(doc_cfg))]

// Re-export wisp-chart so downstream consumers of this crate
// (rare — almost everyone consumes wisp-chart directly) can grab
// it via one Cargo dep line.
pub use wisp_chart;

// Everything browser-flavoured is scoped to the wasm32 target.
// On native targets this crate is a no-op `rlib` (compiles, no
// public functions). The `cargo check --workspace` gate on
// macOS/Ubuntu/Windows runners stays green without doing any
// browser work.
#[cfg(target_arch = "wasm32")]
mod web;
