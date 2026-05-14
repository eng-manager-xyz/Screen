//! Multi-view charts — trellis (small multiples in a grid) and
//! scatterplot matrix (SPLOM).
//!
//! Both lay out a grid of mini sub-plots. v1 ships simplified
//! variants: a trellis that takes pre-built per-cell `Graphics`
//! and tiles them, plus a SPLOM that renders Linear×Linear
//! scatter cells from a multi-dimensional dataset.

pub mod splom;
pub mod trellis;

pub use splom::{Splom, SplomDimension};
pub use trellis::{Trellis, TrellisCell};
