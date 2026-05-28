//! Pickable hit regions for the Gantt chart.
//!
//! `emit_with_interaction` returns one `ChartElementId::GanttBar`
//! per bar, but the H2 planning host also needs hover tooltips on
//! row bands, timeline cells, and the frozen header (project,
//! cell, date, holiday, quarter). Rather than turning every
//! invisible region into a rendered primitive, we expose them as a
//! flat list of `GanttHitRegion`s the host registers into
//! `wisp_interaction::PickableMap`.
//!
//! ## Why this lives outside `render.rs`
//!
//! `emit_graphics` / `emit_with_interaction` emit primitives that
//! actually draw something. Hit regions are invisible by design —
//! a hover over an empty Friday cell should still fire a tooltip
//! even though the cell has no visible fill. Keeping these
//! decoupled lets hosts opt in to the hit-region set they want
//! without paying for it visually or, for tooltip-free embeds,
//! at all.
//!
//! ## Ticket map
//!
//! - `row_hit_regions` (AUT-317) — one region per row band.
//! - `cell_hit_regions` (AUT-318) — one region per timeline week
//!   cell.
//! - `header_hit_regions` (AUT-319) — week headers, holiday pips,
//!   quarter markers.
//!
//! Each helper returns regions in PIXEL space, top-left origin,
//! +Y down (matches `gantt::layout::PixelRect`). Pan-offset
//! application is the host's job — the regions are anchored to
//! the unpanned chart layout.

use glam::Vec2;
use wisp::math::Rect;

use crate::gantt::Gantt;
use crate::interaction::ChartElementId;
use crate::theme::Theme;

/// One pickable region inside a Gantt chart. Pixel-space rect
/// (top-left origin) plus the semantic element id the host should
/// resolve a hover/click event into.
#[derive(Debug, Clone, PartialEq)]
pub struct GanttHitRegion {
    /// Pixel-space rect, top-left origin, `+Y` down. Matches
    /// `gantt::layout::PixelRect` semantics.
    pub rect: Rect,
    /// Semantic element this region resolves to.
    pub element: ChartElementId,
}

impl GanttHitRegion {
    /// Convenience constructor.
    #[must_use]
    pub fn new(rect: Rect, element: ChartElementId) -> Self {
        Self { rect, element }
    }
}

impl Gantt {
    /// Pickable row bands. One region per row, full viewport width,
    /// row-height tall, anchored under the frozen header band.
    ///
    /// Mapped element: [`ChartElementId::GanttRow`] with the row's
    /// index in [`Gantt::rows`].
    ///
    /// The region is wider than the gutter alone so hover anywhere
    /// over the row (label OR timeline cells) resolves to the same
    /// row-level tooltip. Host code that wants the gutter-only band
    /// can clamp the rect's width before registration.
    #[must_use]
    pub fn row_hit_regions(&self, theme: &Theme, viewport_px: Vec2) -> Vec<GanttHitRegion> {
        let row_h = theme.gantt.row_height;
        let mut out = Vec::with_capacity(self.rows.len());
        for (idx, _row) in self.rows.iter().enumerate() {
            let y = crate::gantt::layout::row_top_y(idx, theme);
            out.push(GanttHitRegion::new(
                Rect::new(0.0, y, viewport_px.x, row_h),
                ChartElementId::GanttRow(idx),
            ));
        }
        out
    }

    /// Pickable timeline cells — one region per (row, week) pair.
    ///
    /// Weeks are 7-day buckets anchored at `gantt.range.start`
    /// (see [`crate::gantt::layout::weeks_in_range`]). Cells sit
    /// inside the body pane: their x range is `[gutter_width,
    /// viewport_px.x)` and their y range matches the row layout.
    ///
    /// Mapped element: [`ChartElementId::GanttCell`] with the
    /// row index in `Gantt::rows` and the week index in
    /// `weeks_in_range(gantt.range)`. The host recovers the cell's
    /// `DateRange` by re-running `weeks_in_range`.
    ///
    /// Cells fire INDEPENDENTLY of bar hit regions — a host that
    /// wants "bar takes priority" registers bars with a higher
    /// `depth` (topmost) than cells. The dispatcher sorts hits
    /// topmost-first so the click resolves to whichever
    /// `ChartElementId` is in front.
    #[must_use]
    pub fn cell_hit_regions(&self, theme: &Theme, viewport_px: Vec2) -> Vec<GanttHitRegion> {
        let weeks = crate::gantt::layout::weeks_in_range(self.range);
        if weeks.is_empty() || self.rows.is_empty() {
            return Vec::new();
        }
        let row_h = theme.gantt.row_height;
        let mut out = Vec::with_capacity(weeks.len() * self.rows.len());
        for (row_idx, _row) in self.rows.iter().enumerate() {
            let y = crate::gantt::layout::row_top_y(row_idx, theme);
            for (week_idx, week) in weeks.iter().enumerate() {
                let x_start = crate::gantt::layout::date_to_x(
                    week.start,
                    self.range,
                    theme.gantt.gutter_width,
                    viewport_px.x,
                );
                let x_end = crate::gantt::layout::date_to_x(
                    week.end,
                    self.range,
                    theme.gantt.gutter_width,
                    viewport_px.x,
                );
                out.push(GanttHitRegion::new(
                    Rect::new(x_start, y, (x_end - x_start).max(0.0), row_h),
                    ChartElementId::GanttCell { row_idx, week_idx },
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gantt::{Bar, DateRange, Row};
    use crate::{Person, PersonMap};
    use jiff::civil::date;

    fn fixture() -> Gantt {
        let mut people = PersonMap::default();
        people.insert(Person {
            name: "Matt".into(),
            color: crate::color::Color::from_hex("#0072b2").unwrap(),
        });
        Gantt {
            range: DateRange::year(2026),
            rows: vec![Row::new("vec", "M-VEC"), Row::new("dyn", "M-DYN")],
            bars: vec![Bar::new("vec", date(2026, 1, 1)..date(2026, 6, 1), "Matt")],
            people,
            markers: Vec::new(),
        }
    }

    #[test]
    fn row_hit_regions_emits_one_per_row() {
        let g = fixture();
        let theme = Theme::light();
        let regions = g.row_hit_regions(&theme, Vec2::new(1920.0, 800.0));
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].element, ChartElementId::GanttRow(0));
        assert_eq!(regions[1].element, ChartElementId::GanttRow(1));
    }

    #[test]
    fn row_hit_regions_anchor_under_frozen_header() {
        let g = fixture();
        let theme = Theme::light();
        let regions = g.row_hit_regions(&theme, Vec2::new(1920.0, 800.0));
        // First row's top y must equal header_height.
        assert!((regions[0].rect.min.y - theme.gantt.header_height).abs() < 1e-4);
        // Second row sits one row_height below.
        assert!(
            (regions[1].rect.min.y - (theme.gantt.header_height + theme.gantt.row_height)).abs()
                < 1e-4
        );
    }

    #[test]
    fn row_hit_regions_span_full_viewport_width() {
        let g = fixture();
        let theme = Theme::light();
        let regions = g.row_hit_regions(&theme, Vec2::new(1920.0, 800.0));
        assert!((regions[0].rect.size.x - 1920.0).abs() < 1e-4);
        assert!((regions[0].rect.size.y - theme.gantt.row_height).abs() < 1e-4);
    }

    #[test]
    fn row_hit_regions_empty_gantt_emits_no_regions() {
        let g = Gantt::default();
        let regions = g.row_hit_regions(&Theme::light(), Vec2::new(1920.0, 800.0));
        assert!(regions.is_empty());
    }

    #[test]
    fn cell_hit_regions_emit_rows_times_weeks_grid() {
        let g = fixture(); // 2 rows
        let theme = Theme::light();
        let weeks = crate::gantt::layout::weeks_in_range(g.range);
        let regions = g.cell_hit_regions(&theme, Vec2::new(1920.0, 800.0));
        assert_eq!(regions.len(), g.rows.len() * weeks.len());
        // First cell is row 0, week 0.
        assert_eq!(
            regions[0].element,
            ChartElementId::GanttCell {
                row_idx: 0,
                week_idx: 0
            }
        );
        // Last cell is the last row, last week.
        assert_eq!(
            regions.last().unwrap().element,
            ChartElementId::GanttCell {
                row_idx: g.rows.len() - 1,
                week_idx: weeks.len() - 1
            }
        );
    }

    #[test]
    fn cell_hit_regions_start_after_gutter() {
        let g = fixture();
        let theme = Theme::light();
        let regions = g.cell_hit_regions(&theme, Vec2::new(1920.0, 800.0));
        // First cell's left edge must be at or past the gutter.
        assert!(regions[0].rect.min.x >= theme.gantt.gutter_width - 1e-4);
    }

    #[test]
    fn cell_hit_regions_anchor_to_row_top_y() {
        let g = fixture();
        let theme = Theme::light();
        let regions = g.cell_hit_regions(&theme, Vec2::new(1920.0, 800.0));
        // First row's cells anchored at header_height.
        assert!((regions[0].rect.min.y - theme.gantt.header_height).abs() < 1e-4);
    }

    #[test]
    fn cell_hit_regions_empty_range_emits_no_regions() {
        let mut g = fixture();
        // Zero-day range — no weeks.
        g.range = DateRange::from_range(date(2026, 1, 1)..date(2026, 1, 1));
        let regions = g.cell_hit_regions(&Theme::light(), Vec2::new(1920.0, 800.0));
        assert!(regions.is_empty());
    }

    #[test]
    fn cell_week_idx_round_trips_through_weeks_in_range() {
        let g = fixture();
        let theme = Theme::light();
        let weeks = crate::gantt::layout::weeks_in_range(g.range);
        let regions = g.cell_hit_regions(&theme, Vec2::new(1920.0, 800.0));
        // Pick the 5th cell of the first row.
        let target_week = 5_usize;
        let region = &regions[target_week]; // row 0's cells come first
        let ChartElementId::GanttCell { week_idx, .. } = region.element else {
            panic!("expected GanttCell");
        };
        // Recover the DateRange via the same helper the host uses.
        let recovered = &weeks[week_idx];
        assert_eq!(recovered.start, weeks[target_week].start);
    }
}
