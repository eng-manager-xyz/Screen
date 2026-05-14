//! `wisp-chart` — opinionated chart compositions rendered by `wisp`.
//!
//! Data structs go in, a scene-graph subtree comes out. v1 ships
//! Gantt charts only; bar / line / area follow.
//!
//! ## Scope
//!
//! - `wisp-chart` **depends on `wisp`**, never the reverse.
//! - Chart-specific deps (`jiff`, palettes, contrast utils) live in
//!   this crate's `Cargo.toml` only. `wisp` stays a Pixi-equivalent
//!   primitive renderer.
//! - The crate compiles for native targets AND for
//!   `wasm32-unknown-unknown`, so consumers can render the same
//!   chart in a native window or in a `<canvas>` element via
//!   WebGPU.
//!
//! See the wisp-chart book at `/Screen/wisp-chart/` for the
//! authoritative chapter set.
//!
//! ## Public surface (v1)
//!
//! Today the crate is a skeleton — module declarations only. The
//! data model + theme + layout + render passes land in subsequent
//! M-CHART.0 chunks (AUT-180).

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod color;
pub mod gantt;
pub mod palette;
pub mod scale;
pub mod theme;

pub use color::Color;
pub use gantt::{Bar, DateRange, Gantt, Person, PersonMap, Row};
pub use palette::OwnerPalette;
pub use scale::{BandScale, LinearScale, LogScale, OrdinalScale, TimeScale};
pub use theme::Theme;
