//! DOM popover bridge for the WebGPU Gantt.
//!
//! Tooltips and rich popovers remain normal DOM UI, but their
//! position and content come from semantic [`ChartElementId`]s
//! resolved by [`GanttHitResolver`](super::resolver::GanttHitResolver)
//! — not from app-specific XY math over a DOM grid. This module
//! ships the typed payload + the anchor-rect computation.
//!
//! ## Flow
//!
//! 1. Browser pointer event fires.
//! 2. Host translates it to a viewport `Vec2`.
//! 3. Host calls
//!    `GanttHitResolver::resolve(pointer, &pan) -> Option<ChartElementId>`.
//! 4. Host calls
//!    `gantt.popover_anchor_for(element, theme, viewport_px, &pan)
//!    -> Option<PopoverAnchor>`.
//! 5. Host mounts / repositions the DOM tooltip at
//!    `anchor.viewport_rect`, rendering content from
//!    `anchor.metadata`.
//!
//! ## Pan-anchored positioning
//!
//! The anchor `viewport_rect` is in CURRENT viewport coords, so
//! the host doesn't apply any pan-aware transform itself.
//! `popover_anchor_for` does the pan math:
//! body anchors apply the full `pan.body_offset`, header anchors
//! apply only `pan.body_offset.x`, gutter anchors apply only
//! `pan.body_offset.y` (matching `GanttPanController`'s four-pane
//! transforms).

use glam::Vec2;
use wisp::math::Rect;

use crate::gantt::data::{Gantt, GanttMarker};
use crate::gantt::layout::{bar_pixel_rect_laned, date_to_x};
use crate::gantt::pan::GanttViewport;
use crate::interaction::ChartElementId;
use crate::theme::Theme;

/// Kind of element a popover is anchored to. Mirrors the semantic
/// shape of [`ChartElementId`] but in a popover-domain enum so
/// hosts can switch on the kind without re-matching every variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PopoverKind {
    /// Assignment bar.
    Bar,
    /// Project / row band.
    Row,
    /// Timeline (row, week) cell.
    Cell,
    /// Header week column.
    HeaderWeek,
    /// Header holiday pip.
    HeaderHoliday,
    /// Header quarter tick.
    HeaderQuarter,
}

/// Structured metadata for a popover. Every field is optional —
/// hosts use what's relevant per kind. Strings are owned so the
/// host can mount the popover at its own React/Vue/DOM lifecycle
/// without the Gantt's lifetime.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PopoverMetadata {
    /// Row id (when applicable).
    pub row_id: Option<String>,
    /// Row label (when applicable).
    pub row_label: Option<String>,
    /// Row subtitle (when applicable).
    pub row_subtitle: Option<String>,
    /// Date range (cell, bar, holiday, planning overlay).
    pub date_start: Option<jiff::civil::Date>,
    /// Inclusive end (caller may want to subtract 1 day for
    /// human display).
    pub date_end_exclusive: Option<jiff::civil::Date>,
    /// Bar id (when applicable).
    pub bar_id: Option<String>,
    /// Bar label.
    pub bar_label: Option<String>,
    /// Bar owner.
    pub bar_owner: Option<String>,
    /// Bar allocation percent.
    pub allocation_pct: Option<f32>,
    /// Marker label (holiday name, quarter tag).
    pub marker_label: Option<String>,
}

/// Anchor payload — host positions the popover at
/// `viewport_rect` and renders content from `metadata`.
#[derive(Debug, Clone, PartialEq)]
pub struct PopoverAnchor {
    /// Which kind the popover anchors to.
    pub kind: PopoverKind,
    /// The originating `ChartElementId`.
    pub element: ChartElementId,
    /// Viewport-space rect (pixel coords, top-left origin, +Y down).
    /// Already pan-corrected — host positions DOM at this rect.
    pub viewport_rect: Rect,
    /// Structured metadata.
    pub metadata: PopoverMetadata,
}

impl Gantt {
    /// Compute the popover anchor for a resolved
    /// [`ChartElementId`].
    ///
    /// Returns `None` when the element isn't a Gantt variant (the
    /// host called this with a non-Gantt id), or when the underlying
    /// data has gone stale (e.g. the row index no longer exists).
    #[must_use]
    pub fn popover_anchor_for(
        &self,
        element: ChartElementId,
        theme: &Theme,
        viewport_px: Vec2,
        pan: GanttViewport,
    ) -> Option<PopoverAnchor> {
        match element {
            ChartElementId::GanttBar(idx) => self.anchor_bar(idx, theme, viewport_px, pan),
            ChartElementId::GanttRow(idx) => self.anchor_row(idx, theme, viewport_px, pan),
            ChartElementId::GanttCell { row_idx, week_idx } => {
                self.anchor_cell(row_idx, week_idx, theme, viewport_px, pan)
            }
            ChartElementId::GanttHeaderWeek(week_idx) => {
                self.anchor_header_week(week_idx, theme, viewport_px, pan)
            }
            ChartElementId::GanttHeaderHoliday(idx) => {
                self.anchor_header_holiday(idx, theme, viewport_px, pan)
            }
            ChartElementId::GanttHeaderQuarter(idx) => {
                self.anchor_header_quarter(idx, theme, viewport_px, pan)
            }
            _ => None,
        }
    }

    fn anchor_bar(
        &self,
        idx: usize,
        theme: &Theme,
        viewport_px: Vec2,
        pan: GanttViewport,
    ) -> Option<PopoverAnchor> {
        let bar = self.bars.get(idx)?;
        let rect_px = bar_pixel_rect_laned(bar, self, theme, viewport_px.x)?;
        let viewport_rect =
            body_to_viewport_rect(Rect::new(rect_px.x, rect_px.y, rect_px.w, rect_px.h), pan);
        Some(PopoverAnchor {
            kind: PopoverKind::Bar,
            element: ChartElementId::GanttBar(idx),
            viewport_rect,
            metadata: PopoverMetadata {
                row_id: Some(bar.row_id.clone()),
                row_label: self
                    .rows
                    .iter()
                    .find(|r| r.id == bar.row_id)
                    .map(|r| r.label.clone()),
                date_start: Some(bar.range.start),
                date_end_exclusive: Some(bar.range.end),
                bar_id: Some(bar.id.clone()),
                bar_label: bar.label.clone(),
                bar_owner: Some(bar.owner.clone()),
                allocation_pct: bar.allocation_pct,
                ..Default::default()
            },
        })
    }

    fn anchor_row(
        &self,
        idx: usize,
        theme: &Theme,
        viewport_px: Vec2,
        pan: GanttViewport,
    ) -> Option<PopoverAnchor> {
        let row = self.rows.get(idx)?;
        let y = crate::gantt::layout::dynamic_row_top_y(self, idx, theme);
        let h = crate::gantt::layout::row_height_for_row(self, idx, theme);
        let rect_native = Rect::new(0.0, y, viewport_px.x, h);
        let viewport_rect = body_to_viewport_rect(rect_native, pan);
        Some(PopoverAnchor {
            kind: PopoverKind::Row,
            element: ChartElementId::GanttRow(idx),
            viewport_rect,
            metadata: PopoverMetadata {
                row_id: Some(row.id.clone()),
                row_label: Some(row.label.clone()),
                row_subtitle: row.subtitle.clone(),
                ..Default::default()
            },
        })
    }

    fn anchor_cell(
        &self,
        row_idx: usize,
        week_idx: usize,
        theme: &Theme,
        viewport_px: Vec2,
        pan: GanttViewport,
    ) -> Option<PopoverAnchor> {
        let row = self.rows.get(row_idx)?;
        let weeks = crate::gantt::layout::weeks_in_range(self.range);
        let week = weeks.get(week_idx)?;
        let x_start = date_to_x(
            week.start,
            self.range,
            theme.gantt.gutter_width,
            viewport_px.x,
        );
        let x_end = date_to_x(
            week.end,
            self.range,
            theme.gantt.gutter_width,
            viewport_px.x,
        );
        let y = crate::gantt::layout::dynamic_row_top_y(self, row_idx, theme);
        let h = crate::gantt::layout::row_height_for_row(self, row_idx, theme);
        let rect_native = Rect::new(x_start, y, (x_end - x_start).max(0.0), h);
        let viewport_rect = body_to_viewport_rect(rect_native, pan);
        Some(PopoverAnchor {
            kind: PopoverKind::Cell,
            element: ChartElementId::GanttCell { row_idx, week_idx },
            viewport_rect,
            metadata: PopoverMetadata {
                row_id: Some(row.id.clone()),
                row_label: Some(row.label.clone()),
                date_start: Some(week.start),
                date_end_exclusive: Some(week.end),
                ..Default::default()
            },
        })
    }

    fn anchor_header_week(
        &self,
        week_idx: usize,
        theme: &Theme,
        viewport_px: Vec2,
        pan: GanttViewport,
    ) -> Option<PopoverAnchor> {
        let weeks = crate::gantt::layout::weeks_in_range(self.range);
        let week = weeks.get(week_idx)?;
        let x_start = date_to_x(
            week.start,
            self.range,
            theme.gantt.gutter_width,
            viewport_px.x,
        );
        let x_end = date_to_x(
            week.end,
            self.range,
            theme.gantt.gutter_width,
            viewport_px.x,
        );
        let rect_native = Rect::new(
            x_start,
            0.0,
            (x_end - x_start).max(0.0),
            theme.gantt.header_height,
        );
        // Header pans X only.
        let viewport_rect = header_to_viewport_rect(rect_native, pan);
        Some(PopoverAnchor {
            kind: PopoverKind::HeaderWeek,
            element: ChartElementId::GanttHeaderWeek(week_idx),
            viewport_rect,
            metadata: PopoverMetadata {
                date_start: Some(week.start),
                date_end_exclusive: Some(week.end),
                ..Default::default()
            },
        })
    }

    fn anchor_header_holiday(
        &self,
        idx: usize,
        theme: &Theme,
        viewport_px: Vec2,
        pan: GanttViewport,
    ) -> Option<PopoverAnchor> {
        let holiday = self
            .markers
            .iter()
            .filter_map(|m| match m {
                GanttMarker::Holiday { range, label } => Some((*range, label.clone())),
                _ => None,
            })
            .nth(idx)?;
        let x_start = date_to_x(
            holiday.0.start,
            self.range,
            theme.gantt.gutter_width,
            viewport_px.x,
        );
        let x_end = date_to_x(
            holiday.0.end,
            self.range,
            theme.gantt.gutter_width,
            viewport_px.x,
        );
        let rect_native = Rect::new(
            x_start,
            0.0,
            (x_end - x_start).max(8.0),
            theme.gantt.header_height,
        );
        let viewport_rect = header_to_viewport_rect(rect_native, pan);
        Some(PopoverAnchor {
            kind: PopoverKind::HeaderHoliday,
            element: ChartElementId::GanttHeaderHoliday(idx),
            viewport_rect,
            metadata: PopoverMetadata {
                date_start: Some(holiday.0.start),
                date_end_exclusive: Some(holiday.0.end),
                marker_label: Some(holiday.1),
                ..Default::default()
            },
        })
    }

    fn anchor_header_quarter(
        &self,
        idx: usize,
        theme: &Theme,
        viewport_px: Vec2,
        pan: GanttViewport,
    ) -> Option<PopoverAnchor> {
        let q = self
            .markers
            .iter()
            .filter_map(|m| match m {
                GanttMarker::QuarterStart { date, label } => Some((*date, label.clone())),
                _ => None,
            })
            .nth(idx)?;
        let x = date_to_x(q.0, self.range, theme.gantt.gutter_width, viewport_px.x);
        let rect_native = Rect::new(x - 6.0, 0.0, 12.0, theme.gantt.header_height);
        let viewport_rect = header_to_viewport_rect(rect_native, pan);
        Some(PopoverAnchor {
            kind: PopoverKind::HeaderQuarter,
            element: ChartElementId::GanttHeaderQuarter(idx),
            viewport_rect,
            metadata: PopoverMetadata {
                date_start: Some(q.0),
                marker_label: q.1,
                ..Default::default()
            },
        })
    }
}

/// Apply both axes of `pan.body_offset` to a body-pane rect.
fn body_to_viewport_rect(rect: Rect, pan: GanttViewport) -> Rect {
    Rect::new(
        rect.min.x + pan.body_offset.x,
        rect.min.y + pan.body_offset.y,
        rect.size.x,
        rect.size.y,
    )
}

/// Apply only the X axis of `pan.body_offset` to a header-pane rect.
fn header_to_viewport_rect(rect: Rect, pan: GanttViewport) -> Rect {
    Rect::new(
        rect.min.x + pan.body_offset.x,
        rect.min.y,
        rect.size.x,
        rect.size.y,
    )
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
            rows: vec![
                Row::new("vec", "M-VEC").with_subtitle("Vector math"),
                Row::new("dyn", "M-DYN"),
            ],
            bars: vec![
                Bar::new("vec", date(2026, 2, 1)..date(2026, 5, 1), "Matt")
                    .with_label("Matt — Q2 push")
                    .with_allocation_pct(75.0),
            ],
            people,
            markers: vec![GanttMarker::Holiday {
                range: DateRange::day(date(2026, 7, 4)),
                label: "Independence Day".into(),
            }],
        }
    }

    #[test]
    fn bar_anchor_includes_owner_label_dates_alloc() {
        let g = fixture();
        let a = g
            .popover_anchor_for(
                ChartElementId::GanttBar(0),
                &Theme::light(),
                Vec2::new(1920.0, 800.0),
                GanttViewport::new(),
            )
            .unwrap();
        assert_eq!(a.kind, PopoverKind::Bar);
        assert_eq!(a.metadata.bar_owner.as_deref(), Some("Matt"));
        assert_eq!(a.metadata.bar_label.as_deref(), Some("Matt — Q2 push"));
        assert_eq!(a.metadata.date_start, Some(date(2026, 2, 1)));
        assert!((a.metadata.allocation_pct.unwrap() - 75.0).abs() < 1e-4);
    }

    #[test]
    fn row_anchor_includes_subtitle_and_label() {
        let g = fixture();
        let a = g
            .popover_anchor_for(
                ChartElementId::GanttRow(0),
                &Theme::light(),
                Vec2::new(1920.0, 800.0),
                GanttViewport::new(),
            )
            .unwrap();
        assert_eq!(a.kind, PopoverKind::Row);
        assert_eq!(a.metadata.row_label.as_deref(), Some("M-VEC"));
        assert_eq!(a.metadata.row_subtitle.as_deref(), Some("Vector math"));
    }

    #[test]
    fn cell_anchor_includes_row_and_date_range() {
        let g = fixture();
        let a = g
            .popover_anchor_for(
                ChartElementId::GanttCell {
                    row_idx: 1,
                    week_idx: 5,
                },
                &Theme::light(),
                Vec2::new(1920.0, 800.0),
                GanttViewport::new(),
            )
            .unwrap();
        assert_eq!(a.kind, PopoverKind::Cell);
        assert_eq!(a.metadata.row_id.as_deref(), Some("dyn"));
        assert!(a.metadata.date_start.is_some());
        assert!(a.metadata.date_end_exclusive.is_some());
    }

    #[test]
    fn holiday_anchor_includes_marker_label() {
        let g = fixture();
        let a = g
            .popover_anchor_for(
                ChartElementId::GanttHeaderHoliday(0),
                &Theme::light(),
                Vec2::new(1920.0, 800.0),
                GanttViewport::new(),
            )
            .unwrap();
        assert_eq!(a.kind, PopoverKind::HeaderHoliday);
        assert_eq!(a.metadata.marker_label.as_deref(), Some("Independence Day"));
    }

    #[test]
    fn bar_anchor_applies_full_pan_offset() {
        let g = fixture();
        let pan = GanttViewport {
            body_offset: Vec2::new(100.0, 50.0),
        };
        let theme = Theme::light();
        let vp = Vec2::new(1920.0, 800.0);
        let a_unpanned = g
            .popover_anchor_for(
                ChartElementId::GanttBar(0),
                &theme,
                vp,
                GanttViewport::new(),
            )
            .unwrap();
        let a_panned = g
            .popover_anchor_for(ChartElementId::GanttBar(0), &theme, vp, pan)
            .unwrap();
        assert!(
            (a_panned.viewport_rect.min.x - a_unpanned.viewport_rect.min.x - 100.0).abs() < 1e-4
        );
        assert!(
            (a_panned.viewport_rect.min.y - a_unpanned.viewport_rect.min.y - 50.0).abs() < 1e-4
        );
    }

    #[test]
    fn header_anchor_applies_only_x_pan() {
        let g = fixture();
        let pan = GanttViewport {
            body_offset: Vec2::new(100.0, 50.0),
        };
        let theme = Theme::light();
        let vp = Vec2::new(1920.0, 800.0);
        let a_unpanned = g
            .popover_anchor_for(
                ChartElementId::GanttHeaderHoliday(0),
                &theme,
                vp,
                GanttViewport::new(),
            )
            .unwrap();
        let a_panned = g
            .popover_anchor_for(ChartElementId::GanttHeaderHoliday(0), &theme, vp, pan)
            .unwrap();
        // X panned; Y stays frozen.
        assert!(
            (a_panned.viewport_rect.min.x - a_unpanned.viewport_rect.min.x - 100.0).abs() < 1e-4
        );
        assert!((a_panned.viewport_rect.min.y - a_unpanned.viewport_rect.min.y).abs() < 1e-4);
    }

    #[test]
    fn non_gantt_element_returns_none() {
        let g = fixture();
        let a = g.popover_anchor_for(
            ChartElementId::Slice(0),
            &Theme::light(),
            Vec2::new(1920.0, 800.0),
            GanttViewport::new(),
        );
        assert!(a.is_none());
    }

    #[test]
    fn stale_bar_index_returns_none() {
        let g = fixture();
        let a = g.popover_anchor_for(
            ChartElementId::GanttBar(99),
            &Theme::light(),
            Vec2::new(1920.0, 800.0),
            GanttViewport::new(),
        );
        assert!(a.is_none());
    }
}
