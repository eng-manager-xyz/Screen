//! Layout math — pure functions, no rendering, no `wisp` types.
//!
//! Lands in the M-CHART.0 layout chunk. v1 will provide:
//!
//! - `date_to_x(date, range, plot_width) -> f32` — maps any
//!   date inside `range` to a pixel x-coordinate on `[0, plot_width)`.
//! - `row_to_y(row_index, theme) -> f32` — pixel y of the row's
//!   top edge.
//! - `divider_dates(range) -> { weeks, months }` — list of
//!   week-start / month-start dates inside `range` for grid
//!   rendering.
//!
//! Today the module is a placeholder so the crate compiles. The
//! functions land alongside the snapshot tests.
