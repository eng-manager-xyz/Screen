//! `GanttHitResolver` — semantic hit resolution for Gantt pointer
//! events.
//!
//! ## Why this lives here, not in `wisp-interaction`
//!
//! `wisp-interaction::Wisp2dHitTest` resolves hits against a
//! `wisp::Stage` + `PickableMap` keyed by `NodeId`. That model
//! works when every pickable element is a real scene-graph node —
//! the recorder editor, the 404 pyramid, individual button widgets.
//!
//! A Gantt chart is a SINGLE `Graphics` object emitted by
//! `emit_with_interaction_laned` (one `wisp::NodeId` per chart);
//! its 600+ pickable sub-regions (bars + cells + rows + header)
//! are sub-primitive — they don't have their own `NodeId`s. Forcing
//! every region into its own empty `Container` would explode the
//! scene graph for no rendering payoff.
//!
//! `GanttHitResolver` skips the `NodeId` layer: it caches the
//! pre-computed `GanttHitRegion` list (rows + cells + header +
//! bars) and resolves a viewport pointer directly into a
//! [`ChartElementId`] by transforming the pointer through the
//! pane's pan offset and then rectangle-testing each region in
//! topmost-first order.
//!
//! Hosts that need the full `wisp-interaction` pointer state
//! machine (press/drag/release, multi-touch, modifier snapshots)
//! still use [`PointerDispatcher`](
//! https://docs.rs/screen-wisp-interaction/latest/wisp_interaction/struct.PointerDispatcher.html)
//! upstream of this resolver — feed the dispatcher's `Hit` results
//! by resolving each event's viewport position through `resolve`.
//!
//! ## Hit priority
//!
//! From topmost to bottommost:
//!
//! 1. Bars (`GanttBar`) inside the body pane.
//! 2. Header markers — quarter ticks first (smaller, more specific),
//!    then holiday pips.
//! 3. Header week columns (`GanttHeaderWeek`).
//! 4. Cells (`GanttCell`) inside the body pane.
//! 5. Rows (`GanttRow`) spanning the full width.
//!
//! Index 0 always resolves; higher-precision regions win. Hosts
//! that want a different priority (e.g. "row hover wins over
//! bar hover") rebuild a custom resolver from `Gantt`'s
//! `*_hit_regions` helpers in their own order.

use glam::Vec2;
use wisp::math::Rect;

use crate::gantt::data::Gantt;
use crate::gantt::hit_regions::GanttHitRegion;
use crate::gantt::layout::bar_pixel_rect_laned;
use crate::gantt::pan::GanttViewport;
use crate::interaction::ChartElementId;
use crate::theme::Theme;

/// Cached hit-region set for one Gantt chart at one viewport size.
///
/// Rebuilt by the host whenever the Gantt data, theme, or
/// viewport size changes. Pan changes do NOT require a rebuild —
/// the resolver applies the current `GanttViewport.body_offset`
/// per `resolve` call.
pub struct GanttHitResolver {
    /// Bars in body-pane local space.
    bars: Vec<(Rect, ChartElementId)>,
    /// Body-pane row bands.
    rows: Vec<(Rect, ChartElementId)>,
    /// Body-pane cells.
    cells: Vec<(Rect, ChartElementId)>,
    /// Header-pane regions (week columns, holiday pips, quarter ticks).
    header: Vec<(Rect, ChartElementId)>,
    /// Theme + viewport snapshot — needed to compute pane offsets
    /// at resolve time.
    header_height: f32,
    gutter_width: f32,
}

impl GanttHitResolver {
    /// Build the resolver from a Gantt + theme + viewport size.
    /// Cost: `O(rows × weeks + bars + markers)`.
    #[must_use]
    pub fn from_gantt(gantt: &Gantt, theme: &Theme, viewport_px: Vec2) -> Self {
        let mut bars: Vec<(Rect, ChartElementId)> = Vec::new();
        for (bar_idx, bar) in gantt.bars.iter().enumerate() {
            if let Some(r) = bar_pixel_rect_laned(bar, gantt, theme, viewport_px.x) {
                bars.push((
                    Rect::new(r.x, r.y, r.w, r.h),
                    ChartElementId::GanttBar(bar_idx),
                ));
            }
        }
        Self {
            bars,
            rows: regions_to_rects(gantt.row_hit_regions(theme, viewport_px)),
            cells: regions_to_rects(gantt.cell_hit_regions(theme, viewport_px)),
            header: regions_to_rects(gantt.header_hit_regions(theme, viewport_px)),
            header_height: theme.gantt.header_height,
            gutter_width: theme.gantt.gutter_width,
        }
    }

    /// Resolve a viewport-space pointer to the topmost
    /// [`ChartElementId`] under it, respecting the pan offsets.
    ///
    /// Returns `None` when the pointer is over empty chrome (e.g.
    /// the gutter background outside any row) or off-canvas.
    ///
    /// Priority order: bar > quarter > holiday > week > cell > row.
    /// See module docs for the rationale.
    #[must_use]
    pub fn resolve(&self, viewport_pointer: Vec2, pan: &GanttViewport) -> Option<ChartElementId> {
        // Pointer in BODY-PANE local space: subtract the body pan
        // offset (which moves the underlying content in screen
        // space).
        let body_local = viewport_pointer - pan.body_offset;
        // Pointer in HEADER-PANE local space: only the X pan
        // applies; Y stays at viewport y (so the header band is
        // always at y < header_height).
        let header_local = Vec2::new(viewport_pointer.x - pan.body_offset.x, viewport_pointer.y);

        // Header regions take priority when the pointer is in the
        // header band.
        if viewport_pointer.y < self.header_height {
            // Quarter ticks first (heaviest, narrowest).
            if let Some(id) = topmost_in_kind(&self.header, header_local, |id| {
                matches!(id, ChartElementId::GanttHeaderQuarter(_))
            }) {
                return Some(id);
            }
            if let Some(id) = topmost_in_kind(&self.header, header_local, |id| {
                matches!(id, ChartElementId::GanttHeaderHoliday(_))
            }) {
                return Some(id);
            }
            if let Some(id) = topmost_in_kind(&self.header, header_local, |id| {
                matches!(id, ChartElementId::GanttHeaderWeek(_))
            }) {
                return Some(id);
            }
            // Corner area: no element to resolve.
            return None;
        }

        // Body: bar > cell > row.
        if viewport_pointer.x >= self.gutter_width {
            if let Some(id) = topmost(&self.bars, body_local) {
                return Some(id);
            }
            if let Some(id) = topmost(&self.cells, body_local) {
                return Some(id);
            }
        }

        // Row band spans the full viewport width (gutter + body),
        // so the gutter half resolves too.
        topmost(&self.rows, body_local)
    }

    /// Total region count, for diagnostics.
    #[must_use]
    pub fn region_count(&self) -> usize {
        self.bars.len() + self.rows.len() + self.cells.len() + self.header.len()
    }
}

fn regions_to_rects(regions: Vec<GanttHitRegion>) -> Vec<(Rect, ChartElementId)> {
    regions.into_iter().map(|r| (r.rect, r.element)).collect()
}

fn topmost(set: &[(Rect, ChartElementId)], pointer: Vec2) -> Option<ChartElementId> {
    for (rect, id) in set.iter().rev() {
        if rect.contains(pointer) {
            return Some(*id);
        }
    }
    None
}

fn topmost_in_kind(
    set: &[(Rect, ChartElementId)],
    pointer: Vec2,
    kind: impl Fn(&ChartElementId) -> bool,
) -> Option<ChartElementId> {
    for (rect, id) in set.iter().rev() {
        if kind(id) && rect.contains(pointer) {
            return Some(*id);
        }
    }
    None
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
            bars: vec![Bar::new("vec", date(2026, 2, 1)..date(2026, 5, 1), "Matt")],
            people,
            markers: Vec::new(),
        }
    }

    #[test]
    fn resolve_bar_hit_inside_body_pane_returns_gantt_bar() {
        let g = fixture();
        let theme = Theme::light();
        let vp = Vec2::new(1920.0, 800.0);
        let resolver = GanttHitResolver::from_gantt(&g, &theme, vp);
        // Bar 0 sits in row 0 (M-VEC) from Feb 1 to May 1. Centre
        // x is roughly at year-fraction 0.25 = (gutter + 0.25 * plot).
        // We click ON the bar's known pixel rect via the same helper.
        let bar_rect = bar_pixel_rect_laned(&g.bars[0], &g, &theme, vp.x).unwrap();
        let centre = Vec2::new(bar_rect.x + bar_rect.w * 0.5, bar_rect.y + bar_rect.h * 0.5);
        let pan = GanttViewport::new();
        assert_eq!(
            resolver.resolve(centre, &pan),
            Some(ChartElementId::GanttBar(0))
        );
    }

    #[test]
    fn resolve_empty_cell_returns_gantt_cell() {
        let g = fixture();
        let theme = Theme::light();
        let vp = Vec2::new(1920.0, 800.0);
        let resolver = GanttHitResolver::from_gantt(&g, &theme, vp);
        // Click at a cell that the bar doesn't cover (row 1, week 0).
        let weeks = crate::gantt::layout::weeks_in_range(g.range);
        // Pick the first week of row 1.
        let row1_top = theme.gantt.header_height + theme.gantt.row_height;
        let x = crate::gantt::layout::date_to_x(
            weeks[0].start,
            g.range,
            theme.gantt.gutter_width,
            vp.x,
        ) + 4.0;
        let y = row1_top + theme.gantt.row_height * 0.5;
        let id = resolver.resolve(Vec2::new(x, y), &GanttViewport::new());
        assert_eq!(
            id,
            Some(ChartElementId::GanttCell {
                row_idx: 1,
                week_idx: 0
            })
        );
    }

    #[test]
    fn resolve_header_returns_header_week_when_no_markers() {
        let g = fixture();
        let theme = Theme::light();
        let vp = Vec2::new(1920.0, 800.0);
        let resolver = GanttHitResolver::from_gantt(&g, &theme, vp);
        // Click in the header band over week 5.
        let weeks = crate::gantt::layout::weeks_in_range(g.range);
        let x = crate::gantt::layout::date_to_x(
            weeks[5].start,
            g.range,
            theme.gantt.gutter_width,
            vp.x,
        ) + 4.0;
        let y = 10.0; // inside header band
        assert_eq!(
            resolver.resolve(Vec2::new(x, y), &GanttViewport::new()),
            Some(ChartElementId::GanttHeaderWeek(5))
        );
    }

    #[test]
    fn resolve_quarter_marker_wins_over_week_column() {
        use crate::gantt::data::GanttMarker;
        let mut g = fixture();
        g.markers.push(GanttMarker::QuarterStart {
            date: date(2026, 4, 1),
            label: Some("Q2 2026".into()),
        });
        let theme = Theme::light();
        let vp = Vec2::new(1920.0, 800.0);
        let resolver = GanttHitResolver::from_gantt(&g, &theme, vp);
        // Click directly on the quarter tick x.
        let x = crate::gantt::layout::date_to_x(
            date(2026, 4, 1),
            g.range,
            theme.gantt.gutter_width,
            vp.x,
        );
        let y = 10.0;
        let id = resolver.resolve(Vec2::new(x, y), &GanttViewport::new());
        assert_eq!(id, Some(ChartElementId::GanttHeaderQuarter(0)));
    }

    #[test]
    fn resolve_gutter_returns_row_for_label_area_in_body_band() {
        let g = fixture();
        let theme = Theme::light();
        let vp = Vec2::new(1920.0, 800.0);
        let resolver = GanttHitResolver::from_gantt(&g, &theme, vp);
        // Click in the gutter (x < gutter_width) at row 1.
        let row1_y =
            theme.gantt.header_height + theme.gantt.row_height + theme.gantt.row_height * 0.5;
        let id = resolver.resolve(Vec2::new(40.0, row1_y), &GanttViewport::new());
        assert_eq!(id, Some(ChartElementId::GanttRow(1)));
    }

    #[test]
    fn resolve_respects_body_pan_offset_for_bar_hit() {
        let g = fixture();
        let theme = Theme::light();
        let vp = Vec2::new(1920.0, 800.0);
        let resolver = GanttHitResolver::from_gantt(&g, &theme, vp);
        // Without pan, the bar is at its native rect.
        let bar_rect = bar_pixel_rect_laned(&g.bars[0], &g, &theme, vp.x).unwrap();
        let native_centre = Vec2::new(bar_rect.x + bar_rect.w * 0.5, bar_rect.y + bar_rect.h * 0.5);
        let pan_zero = GanttViewport::new();
        let pan_shifted = GanttViewport {
            body_offset: Vec2::new(100.0, 50.0),
        };

        // Clicking the native bar centre with no pan: hit.
        assert_eq!(
            resolver.resolve(native_centre, &pan_zero),
            Some(ChartElementId::GanttBar(0))
        );
        // Clicking the SAME viewport coord with pan applied: miss
        // (the bar moved on-screen by (100, 50)).
        assert_ne!(
            resolver.resolve(native_centre, &pan_shifted),
            Some(ChartElementId::GanttBar(0))
        );
        // Clicking the SHIFTED screen position (native + pan): hit.
        let shifted_centre = native_centre + Vec2::new(100.0, 50.0);
        assert_eq!(
            resolver.resolve(shifted_centre, &pan_shifted),
            Some(ChartElementId::GanttBar(0))
        );
    }

    #[test]
    fn resolve_header_ignores_y_pan_keeps_x_pan() {
        // The header pane only pans X (matches GanttPanController).
        let g = fixture();
        let theme = Theme::light();
        let vp = Vec2::new(1920.0, 800.0);
        let resolver = GanttHitResolver::from_gantt(&g, &theme, vp);
        let weeks = crate::gantt::layout::weeks_in_range(g.range);
        let week5_x = crate::gantt::layout::date_to_x(
            weeks[5].start,
            g.range,
            theme.gantt.gutter_width,
            vp.x,
        ) + 4.0;
        let pan = GanttViewport {
            body_offset: Vec2::new(50.0, 999.0),
        };
        // Pointer screen x = week5_x + 50 (after X pan); y stays in
        // header band; y pan does NOT affect header resolution.
        let id = resolver.resolve(Vec2::new(week5_x + 50.0, 12.0), &pan);
        assert_eq!(id, Some(ChartElementId::GanttHeaderWeek(5)));
    }

    #[test]
    fn region_count_returns_all_buckets() {
        let g = fixture();
        let theme = Theme::light();
        let vp = Vec2::new(1920.0, 800.0);
        let r = GanttHitResolver::from_gantt(&g, &theme, vp);
        assert!(
            r.region_count() > 100,
            "expected many regions: weekly × 2 rows + headers + bars"
        );
    }
}
