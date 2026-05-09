//! `Filter` trait + built-in filters: blur, drop shadow, motion blur, color matrix.
//!
//! The `Filter` trait and `FilterContext` land in M0.16. Submodules host the
//! built-in filter implementations.

pub mod blur;
pub mod color_matrix;
pub mod drop_shadow;
pub mod motion_blur;
