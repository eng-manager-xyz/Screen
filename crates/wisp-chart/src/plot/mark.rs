//! Mark types — what each row of the `DataFrame` turns into
//! visually.
//!
//! v1 ships `Mark::Bar` + `Mark::Line`. Future marks (`Area`,
//! `Point`, `Cell`, `Box`, `Candlestick`, `Polygon`, `Arc`, etc.)
//! extend this enum.

/// Line-segment interpolation between adjacent data points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Interpolation {
    /// Straight segments between each (x, y).
    #[default]
    Linear,
    /// Horizontal-then-vertical step. Best for monotonic
    /// step series (e.g. quarterly milestones, billing
    /// step changes).
    Step,
}

/// Optional point-marker style drawn at each data point on a
/// line mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointStyle {
    /// Filled circle marker, radius from `PlotTheme::line_marker_radius_px`.
    Circle,
}

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
    /// Connected line through the rows in `DataFrame` order.
    /// Multi-series via a `Color` encoding — each distinct
    /// category becomes its own line.
    Line {
        /// Segment interpolation between adjacent points.
        interpolation: Interpolation,
        /// Optional per-point marker style. `None` skips
        /// markers entirely.
        marker: Option<PointStyle>,
    },
}

impl Default for Mark {
    fn default() -> Self {
        Mark::Bar {
            value_labels: false,
        }
    }
}
