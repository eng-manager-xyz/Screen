//! Overlay primitives — emitted alongside a primary chart to
//! enrich it with uncertainty / annotations. Today: error bars.

pub mod error_bars;

pub use error_bars::{ErrorBars, ErrorKind, ErrorPoint};
