//! `Stagger` — compute per-index time offsets so a group of
//! animations starts in a wave instead of all at once.
//!
//! Modelled on anime.js v4's `stagger(value, options)`. The same
//! builder shape works for linear lists (`Stagger::each(...)`) and
//! 2-D grids (`Stagger::each(...).grid(rows, cols)`).

use std::time::Duration;

/// Origin point for stagger ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StaggerFrom {
    /// First index gets offset 0; offsets grow with index.
    #[default]
    Start,
    /// Last index gets offset 0; offsets grow toward the front.
    End,
    /// Middle index gets offset 0; offsets grow outward.
    Center,
    /// Caller-specified pivot index.
    Index(usize),
}

/// Per-index stagger offset generator.
#[derive(Clone, Copy, Debug)]
pub struct Stagger {
    each: Duration,
    from: StaggerFrom,
    grid: Option<(usize, usize)>,
}

impl Stagger {
    /// Construct with a per-step gap; defaults to `StaggerFrom::Start`,
    /// no grid.
    #[must_use]
    pub const fn each(each: Duration) -> Self {
        Self {
            each,
            from: StaggerFrom::Start,
            grid: None,
        }
    }

    /// Override the origin.
    #[must_use]
    pub const fn from(mut self, from: StaggerFrom) -> Self {
        self.from = from;
        self
    }

    /// Treat the index set as a 2-D `rows × cols` grid. Distance
    /// from the origin is L1 (Manhattan) on the grid.
    #[must_use]
    pub const fn grid(mut self, rows: usize, cols: usize) -> Self {
        self.grid = Some((rows, cols));
        self
    }

    /// Offset for a given index in a flat list of `count` items.
    #[must_use]
    pub fn offset_for(&self, index: usize, count: usize) -> Duration {
        if count == 0 {
            return Duration::ZERO;
        }
        if let Some((rows, cols)) = self.grid
            && cols > 0
        {
            let row = index / cols;
            let col = index % cols;
            let (origin_row, origin_col) = self.grid_origin(rows, cols);
            let dist = row.abs_diff(origin_row) + col.abs_diff(origin_col);
            return self
                .each
                .saturating_mul(u32::try_from(dist).unwrap_or(u32::MAX));
        }
        let pivot = match self.from {
            StaggerFrom::Start => 0,
            StaggerFrom::End => count.saturating_sub(1),
            StaggerFrom::Center => count / 2,
            StaggerFrom::Index(i) => i.min(count.saturating_sub(1)),
        };
        let dist = index.abs_diff(pivot);
        self.each
            .saturating_mul(u32::try_from(dist).unwrap_or(u32::MAX))
    }

    fn grid_origin(&self, rows: usize, cols: usize) -> (usize, usize) {
        match self.from {
            StaggerFrom::Start => (0, 0),
            StaggerFrom::End => (rows.saturating_sub(1), cols.saturating_sub(1)),
            StaggerFrom::Center => (rows / 2, cols / 2),
            StaggerFrom::Index(i) => {
                if cols == 0 {
                    return (0, 0);
                }
                (i / cols, i % cols)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_grows_with_index() {
        let s = Stagger::each(Duration::from_millis(100));
        assert_eq!(s.offset_for(0, 4), Duration::ZERO);
        assert_eq!(s.offset_for(1, 4), Duration::from_millis(100));
        assert_eq!(s.offset_for(3, 4), Duration::from_millis(300));
    }

    #[test]
    fn end_grows_backwards() {
        let s = Stagger::each(Duration::from_millis(50)).from(StaggerFrom::End);
        assert_eq!(s.offset_for(3, 4), Duration::ZERO);
        assert_eq!(s.offset_for(0, 4), Duration::from_millis(150));
    }

    #[test]
    fn center_pivots_middle() {
        let s = Stagger::each(Duration::from_millis(40)).from(StaggerFrom::Center);
        // 5 items, pivot = 2.
        assert_eq!(s.offset_for(2, 5), Duration::ZERO);
        assert_eq!(s.offset_for(1, 5), Duration::from_millis(40));
        assert_eq!(s.offset_for(3, 5), Duration::from_millis(40));
        assert_eq!(s.offset_for(0, 5), Duration::from_millis(80));
        assert_eq!(s.offset_for(4, 5), Duration::from_millis(80));
    }

    #[test]
    fn index_pivot_picks_specific() {
        let s = Stagger::each(Duration::from_millis(20)).from(StaggerFrom::Index(2));
        assert_eq!(s.offset_for(2, 5), Duration::ZERO);
        assert_eq!(s.offset_for(0, 5), Duration::from_millis(40));
    }

    #[test]
    fn grid_uses_l1_distance() {
        // 3×3 grid, center origin.
        let s = Stagger::each(Duration::from_millis(10))
            .from(StaggerFrom::Center)
            .grid(3, 3);
        // Centre cell (row=1, col=1, index=4) → 0.
        assert_eq!(s.offset_for(4, 9), Duration::ZERO);
        // Corner (row=0, col=0, index=0) → L1 distance 2 → 20ms.
        assert_eq!(s.offset_for(0, 9), Duration::from_millis(20));
        // Adjacent edge (row=0, col=1, index=1) → L1 distance 1 → 10ms.
        assert_eq!(s.offset_for(1, 9), Duration::from_millis(10));
    }

    #[test]
    fn empty_returns_zero() {
        let s = Stagger::each(Duration::from_millis(100));
        assert_eq!(s.offset_for(0, 0), Duration::ZERO);
    }
}
