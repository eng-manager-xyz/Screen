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

    /// Pickable header regions. Emits three categories in one call,
    /// matching the three `GanttHeader*` variants in
    /// [`ChartElementId`]:
    ///
    /// - One [`ChartElementId::GanttHeaderWeek`] region per week
    ///   column above the timeline.
    /// - One [`ChartElementId::GanttHeaderHoliday`] region per
    ///   `GanttMarker::Holiday` in `gantt.markers` (in marker
    ///   iteration order). Holiday pips render inside the header
    ///   band.
    /// - One [`ChartElementId::GanttHeaderQuarter`] region per
    ///   `GanttMarker::QuarterStart`.
    ///
    /// All regions live within `y ∈ [0, theme.gantt.header_height)`
    /// — they don't intrude into the body rows, so they coexist
    /// cleanly with row / cell / bar hit regions even when the
    /// host registers all four sets at once.
    ///
    /// The width of holiday + quarter regions is intentionally
    /// generous (8 viewport pixels minimum) so users can click
    /// without sub-pixel precision.
    #[must_use]
    pub fn header_hit_regions(&self, theme: &Theme, viewport_px: Vec2) -> Vec<GanttHitRegion> {
        let weeks = crate::gantt::layout::weeks_in_range(self.range);
        let mut out = Vec::new();
        let header_h = theme.gantt.header_height;

        // Week columns inside the header band.
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
                Rect::new(x_start, 0.0, (x_end - x_start).max(0.0), header_h),
                ChartElementId::GanttHeaderWeek(week_idx),
            ));
        }

        // Holiday + quarter markers — track separate indices so
        // the host can iterate `Gantt::markers` and pair indices.
        let mut holiday_idx = 0_usize;
        let mut quarter_idx = 0_usize;
        for marker in &self.markers {
            match marker {
                crate::gantt::GanttMarker::Holiday { range, .. } => {
                    let x_start = crate::gantt::layout::date_to_x(
                        range.start,
                        self.range,
                        theme.gantt.gutter_width,
                        viewport_px.x,
                    );
                    let x_end = crate::gantt::layout::date_to_x(
                        range.end,
                        self.range,
                        theme.gantt.gutter_width,
                        viewport_px.x,
                    );
                    let w = (x_end - x_start).max(8.0); // clickable minimum
                    out.push(GanttHitRegion::new(
                        Rect::new(x_start, 0.0, w, header_h),
                        ChartElementId::GanttHeaderHoliday(holiday_idx),
                    ));
                    holiday_idx += 1;
                }
                crate::gantt::GanttMarker::QuarterStart { date, .. } => {
                    let x = crate::gantt::layout::date_to_x(
                        *date,
                        self.range,
                        theme.gantt.gutter_width,
                        viewport_px.x,
                    );
                    // 8 px wide centred on the quarter tick.
                    out.push(GanttHitRegion::new(
                        Rect::new(x - 4.0, 0.0, 8.0, header_h),
                        ChartElementId::GanttHeaderQuarter(quarter_idx),
                    ));
                    quarter_idx += 1;
                }
                // CurrentDate + PlanningOverlay are body-pane
                // overlays — not header hit targets.
                _ => {}
            }
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
    fn header_hit_regions_emit_one_per_week() {
        let g = fixture();
        let theme = Theme::light();
        let weeks = crate::gantt::layout::weeks_in_range(g.range);
        let regions = g.header_hit_regions(&theme, Vec2::new(1920.0, 800.0));
        // Only week regions (no markers in fixture).
        assert_eq!(regions.len(), weeks.len());
        assert_eq!(regions[0].element, ChartElementId::GanttHeaderWeek(0));
        assert_eq!(
            regions.last().unwrap().element,
            ChartElementId::GanttHeaderWeek(weeks.len() - 1)
        );
    }

    #[test]
    fn header_hit_regions_stay_inside_header_band() {
        let g = fixture();
        let theme = Theme::light();
        let regions = g.header_hit_regions(&theme, Vec2::new(1920.0, 800.0));
        for r in &regions {
            // y starts at 0, never spills below header_height.
            assert!(r.rect.min.y >= -1e-4);
            assert!((r.rect.min.y + r.rect.size.y) <= theme.gantt.header_height + 1e-4);
        }
    }

    #[test]
    fn header_hit_regions_include_holiday_and_quarter_markers() {
        use crate::gantt::GanttMarker;
        let mut g = fixture();
        g.markers.push(GanttMarker::Holiday {
            range: crate::gantt::DateRange::day(date(2026, 7, 4)),
            label: "Independence Day".into(),
        });
        g.markers.push(GanttMarker::QuarterStart {
            date: date(2026, 4, 1),
            label: Some("Q2 2026".into()),
        });
        // CurrentDate + PlanningOverlay should be IGNORED by the
        // header pass (they live in the body pane).
        g.markers.push(GanttMarker::CurrentDate {
            date: date(2026, 6, 15),
        });
        let theme = Theme::light();
        let weeks = crate::gantt::layout::weeks_in_range(g.range);
        let regions = g.header_hit_regions(&theme, Vec2::new(1920.0, 800.0));
        // weeks + 1 holiday + 1 quarter. CurrentDate skipped.
        assert_eq!(regions.len(), weeks.len() + 2);
        // Holiday + quarter elements present.
        assert!(
            regions
                .iter()
                .any(|r| matches!(r.element, ChartElementId::GanttHeaderHoliday(0)))
        );
        assert!(
            regions
                .iter()
                .any(|r| matches!(r.element, ChartElementId::GanttHeaderQuarter(0)))
        );
    }

    #[test]
    fn header_hit_region_indices_are_distinct_per_kind() {
        use crate::gantt::GanttMarker;
        let mut g = fixture();
        // Two holidays — distinct indices.
        g.markers.push(GanttMarker::Holiday {
            range: crate::gantt::DateRange::day(date(2026, 7, 4)),
            label: "Indep".into(),
        });
        g.markers.push(GanttMarker::Holiday {
            range: crate::gantt::DateRange::day(date(2026, 12, 25)),
            label: "Xmas".into(),
        });
        g.markers.push(GanttMarker::QuarterStart {
            date: date(2026, 4, 1),
            label: None,
        });
        g.markers.push(GanttMarker::QuarterStart {
            date: date(2026, 7, 1),
            label: None,
        });
        let theme = Theme::light();
        let regions = g.header_hit_regions(&theme, Vec2::new(1920.0, 800.0));
        let holidays: Vec<_> = regions
            .iter()
            .filter_map(|r| match r.element {
                ChartElementId::GanttHeaderHoliday(i) => Some(i),
                _ => None,
            })
            .collect();
        let quarters: Vec<_> = regions
            .iter()
            .filter_map(|r| match r.element {
                ChartElementId::GanttHeaderQuarter(i) => Some(i),
                _ => None,
            })
            .collect();
        assert_eq!(holidays, vec![0, 1]);
        assert_eq!(quarters, vec![0, 1]);
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
