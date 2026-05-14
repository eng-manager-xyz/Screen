//! Mark types — what each row of the `DataFrame` turns into
//! visually.
//!
//! v1 ships `Mark::Bar`. Future marks (`Line`, `Area`, `Point`,
//! `Cell`, `Box`, `Candlestick`, `Polygon`, `Arc`, etc.) extend
//! this enum.

/// The drawable shape a Plot emits per row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mark {
    /// Rectangular bar. With `value_labels = true` the renderer
    /// draws each bar's numeric value on top of (or inside) the
    /// bar.
    Bar {
        /// Show numeric value labels on each bar.
        value_labels: bool,
    },
}

impl Default for Mark {
    fn default() -> Self {
        Mark::Bar {
            value_labels: false,
        }
    }
}
