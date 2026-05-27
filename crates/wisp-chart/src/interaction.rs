//! Chart-element interaction shape — `ChartElementId` enum +
//! `EmittedChart` return type for the per-kind `emit_with_interaction`
//! methods.
//!
//! ## How it fits with `wisp-interaction`
//!
//! `wisp-interaction` owns the input vocabulary (hit-testing, pointer
//! events, controllers); `wisp-chart` owns the visual output. The
//! glue is this module: each chart kind that opts in returns an
//! [`EmittedChart`] carrying both the `Graphics` AND a vector mapping
//! primitive index → semantic [`ChartElementId`]. The host:
//!
//! 1. Adds the `Graphics` to its `Stage` and remembers the resulting
//!    `NodeId`.
//! 2. Registers a `Pickable` covering the chart's bounding rect, so
//!    a click on the chart fires a `Pointer<Click>` on its node.
//! 3. Inside the click handler, converts the click's local position
//!    back to a primitive index (per-chart geometry math), then looks
//!    up the `ChartElementId` for that index.
//!
//! Charts that have a clear "one primitive == one element" mapping
//! (pie slices, histogram bins, treemap leaves) are trivial; charts
//! that emit several primitives per element (candlesticks with body
//! plus wicks) emit ONE entry per primitive — the same element id
//! may repeat across consecutive primitives.
//!
//! ## Status
//!
//! WI.9 ships the type shape + per-kind `emit_with_interaction` for
//! five representative chart kinds (Pie, Histogram, Treemap,
//! `Histogram2D`, Baseline). The remaining 23 chart kinds get the same
//! treatment in a follow-up sweep (see AUT-315 in the per-PR Linear
//! board) — the API surface is stable, the per-kind implementations
//! are mechanical.

use wisp::Graphics;

/// One emitted chart's payload: rendered primitives + reverse-lookup
/// table.
#[derive(Debug, Clone)]
pub struct EmittedChart {
    /// The drawable primitives. Add this to your `Stage` exactly as
    /// you would the output of `emit_graphics`.
    pub graphics: Graphics,
    /// Maps primitive index (`0..graphics.primitive_count()`) →
    /// semantic [`ChartElementId`]. Entries are guaranteed to be in
    /// ascending primitive-index order so a binary search is OK for
    /// lookups inside hot loops.
    pub elements: Vec<(usize, ChartElementId)>,
}

impl EmittedChart {
    /// Convenience: lookup the `ChartElementId` for a given primitive
    /// index. Returns `None` if the index is past the last recorded
    /// element (defensive — should only happen for primitives the
    /// chart treats as cosmetic, like gridlines).
    #[must_use]
    pub fn element_for_primitive(&self, primitive_idx: usize) -> Option<&ChartElementId> {
        // Linear scan is fine: even the busiest charts have only a
        // few hundred elements, and `elements` is monotone so the
        // first match is the only match.
        self.elements
            .iter()
            .find(|(idx, _)| *idx == primitive_idx)
            .map(|(_, id)| id)
    }
}

/// Semantic identifier for one chart element. The variant tells the
/// host what KIND of thing the user clicked; the payload pinpoints
/// which one.
///
/// Variants cover the 28 chart kinds wisp-chart ships. Unknown /
/// custom elements use `Other(u32)` as an escape hatch.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChartElementId {
    /// Pie / sunburst slice at index N (0 = first slice, CCW).
    Slice(usize),
    /// Histogram bin at index N.
    Bin(usize),
    /// Baseline-chart bar at index N.
    Bar(usize),
    /// Heatmap cell at (row, col).
    Cell {
        /// Row index (0 = top, by chart convention).
        row: usize,
        /// Column index (0 = left).
        col: usize,
    },
    /// Scatter / line point at index N.
    Point(usize),
    /// `wisp-chart::Plot` mark at (`row_idx`, `mark_idx`).
    Mark {
        /// Row index in the encoded data frame.
        row_idx: usize,
        /// Mark index within the row.
        mark_idx: usize,
    },
    /// OHLC / candlestick / waterfall candle at index N.
    Candle(usize),
    /// Gantt bar at index N (matches `Bar` array order).
    GanttBar(usize),
    /// Box-plot box at index N.
    Box(usize),
    /// KDE-plot curve point at index N.
    Kde(usize),
    /// Parallel-coordinates polyline at index N.
    Coord(usize),
    /// Treemap leaf at index N (depth-first traversal order).
    Leaf(usize),
    /// Funnel step at index N (top → bottom).
    Step(usize),
    /// Sankey link at index N.
    Link(usize),
    /// Indicator (gauge / bullet / kpi) sub-element index.
    Indicator(usize),
    /// Radar spoke at index N.
    Spoke(usize),
    /// Contour level at index N.
    Contour(usize),
    /// Ternary-plot region at index N.
    Region(usize),
    /// Trellis cell at (row, col).
    Trellis {
        /// Trellis row.
        row: usize,
        /// Trellis column.
        col: usize,
    },
    /// SPLOM (scatter-plot matrix) cell at (row, col).
    SplomCell {
        /// SPLOM row.
        row: usize,
        /// SPLOM column.
        col: usize,
    },
    /// Error-bar / overlay whisker at index N.
    Whisker(usize),
    /// Legend label at index N.
    Label(usize),
    /// Escape hatch for chart kinds that need a custom id space.
    Other(u32),
}

#[cfg(test)]
mod tests {
    use super::*;
    use wisp::Graphics;

    #[test]
    fn element_for_primitive_round_trip() {
        let mut emitted = EmittedChart {
            graphics: Graphics::new(),
            elements: vec![
                (0, ChartElementId::Slice(0)),
                (1, ChartElementId::Slice(1)),
                (2, ChartElementId::Slice(2)),
            ],
        };
        assert_eq!(
            emitted.element_for_primitive(1),
            Some(&ChartElementId::Slice(1))
        );
        assert_eq!(emitted.element_for_primitive(99), None);

        // Clear elements: lookup returns None for everything.
        emitted.elements.clear();
        assert_eq!(emitted.element_for_primitive(0), None);
    }

    #[test]
    fn id_variants_implement_copy_eq_hash() {
        // Compile-time witness: the enum is Copy + Eq + Hash so it
        // works as a HashMap key inside host code that wants to
        // associate state per chart element (selection, hover, etc.).
        fn assert_traits<T: Copy + Eq + std::hash::Hash>() {}
        assert_traits::<ChartElementId>();
    }
}
