//! Heatmaps — table grid, calendar (year-in-review), lasagna
//! (per-row time-series stripe), and 2D histogram. All map a
//! numeric value to a colour via a [`SequentialPalette`] and
//! emit one filled rectangle per cell.

pub mod calendar;
pub mod histogram2d;
pub mod lasagna;
pub mod palette;
pub mod table;

pub use calendar::{CalendarHeatmap, CalendarValue};
pub use histogram2d::Histogram2D;
pub use lasagna::LasagnaHeatmap;
pub use palette::SequentialPalette;
pub use table::TableHeatmap;
