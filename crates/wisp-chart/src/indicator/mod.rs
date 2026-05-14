//! Indicator charts — dashboard summary tiles + gauges + bullets.
//!
//! Unlike cartesian charts (Bar / Line / Area / Point) which compose
//! through the [`crate::plot::Plot`] grammar facade, indicators are
//! self-contained: each indicator is a value type with its own
//! `emit_graphics(theme, viewport_px) -> wisp::Graphics` rendering
//! call. Text overlays (big number, label) come back via a separate
//! `emit_text_labels(theme, viewport_px, font) -> Vec<wisp::Text>`
//! method since `Text` needs a Font.

pub mod bullet;
pub mod gauge;
pub mod kpi;

pub use bullet::{Bullet, Orientation};
pub use gauge::{Gauge, Zone};
pub use kpi::{Delta, DeltaKind, Kpi, format_value};
