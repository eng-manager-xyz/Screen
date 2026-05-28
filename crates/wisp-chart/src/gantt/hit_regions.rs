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
}
