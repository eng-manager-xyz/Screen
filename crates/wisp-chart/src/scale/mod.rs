//! Scale module — domain → range mappings + tick generators.
//!
//! Every cartesian / polar / heatmap chart maps abstract domain
//! values (numbers, categories, dates) into pixel coordinates
//! through a `Scale`. Building this once means every chart family
//! gets consistent tick placement, padding, and edge behaviour for
//! free.
//!
//! ## What ships in v1
//!
//! | Scale | Domain | Range | Used by |
//! |---|---|---|---|
//! | [`LinearScale`] | continuous `f32` | continuous `f32` | bar (y), line, scatter, histogram, area, KPI sparkline |
//! | [`BandScale`] | discrete categories | continuous `f32` | bar (x), grouped bar, box plot |
//! | [`OrdinalScale`] | discrete categories | discrete index | colour encoding lookups |
//! | [`TimeScale`] | `jiff::civil::Date` | continuous `f32` | line / area with time x, Gantt, candlestick |
//! | [`LogScale`] | continuous `f32 > 0` | continuous `f32` | bubble x (GDP), histogram of skewed data |
//!
//! ## Convention
//!
//! All scales map `domain` → `range`, both as `(f32, f32)` tuples
//! (or category lists for band / ordinal). The convention is:
//!
//! * `range.0` corresponds to the LEFT edge of the plot area for X
//!   scales and the BOTTOM edge for Y scales. Callers pre-flip Y
//!   ranges so a `LinearScale::new((0, 100), (plot_bottom_y,
//!   plot_top_y))` puts low values at the bottom.
//! * Tick generators return the values inside the domain that
//!   should get tick marks; the rendering layer projects each
//!   through `map` and draws.
//!
//! ## What doesn't ship in v1
//!
//! * Categorical sorting strategies (currently insertion order).
//! * Non-natural-log bases for `LogScale` (log10 default; arbitrary
//!   base accepted but tick generation tuned for base 10).
//! * Time-axis localisation / week-start configuration — Mondays
//!   are the assumed week start, matching the existing Gantt
//!   convention.

pub mod band;
pub mod linear;
pub mod log;
pub mod ordinal;
pub mod time;

pub use band::BandScale;
pub use linear::LinearScale;
pub use log::LogScale;
pub use ordinal::OrdinalScale;
pub use time::TimeScale;

/// Result of generating ticks at a given density.
///
/// Each entry carries both the domain value (the data point the
/// tick stands for) and the projected range value (the pixel
/// coordinate). Callers use `value` to format the tick label and
/// `position` to place it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tick<T> {
    /// Domain value the tick stands for (e.g. `42.0` for a linear
    /// scale, `Date(2026-04-01)` for a time scale).
    pub value: T,
    /// Range value (the pixel coordinate) for this tick.
    pub position: f32,
}
