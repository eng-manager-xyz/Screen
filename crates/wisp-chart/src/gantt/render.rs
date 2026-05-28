//! Scene-graph emission for [`crate::Gantt`].
//!
//! v1 surface: [`Gantt::emit_graphics`] returns a single
//! [`wisp::Graphics`] primitive containing the chart background and
//! every bar as a rounded-rect filled with its owner's colour.
//! Coordinates are NDC (`-1.0..=1.0`); the caller supplies the
//! viewport pixel size so the layout math can convert.
//!
//! Out of scope for v1 (deferred to follow-on M-CHART chunks):
//! header band fill, alt-row tints, week / month grid lines, row
//! labels + day / month text. The shape of those is "more
//! Graphics primitives in the same return value" so adding them
//! later won't break the public API.

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::gantt::Gantt;
use crate::gantt::data::GanttRole;
use crate::gantt::layout::{bar_pixel_rect, bar_pixel_rect_laned};
use crate::theme::Theme;

impl Gantt {
    /// Emit a [`wisp::Graphics`] subtree drawing this Gantt at
    /// `viewport_px` pixels.
    ///
    /// The returned `Graphics` has the chart background as its
    /// first primitive (so the renderer's clear colour is
    /// irrelevant) and one rounded-rect per bar laid out by
    /// [`crate::gantt::layout::bar_pixel_rect`]. Bars referencing
    /// unknown rows are silently skipped — they're a data error,
    /// not a render error.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        self.emit_with_interaction(theme, viewport_px).graphics
    }

    /// Like [`emit_graphics`](Self::emit_graphics) but additionally
    /// returns a reverse-lookup table mapping each bar's primitive
    /// index → [`ChartElementId::GanttBar`](crate::interaction::ChartElementId::GanttBar)
    /// keyed by the bar's index in [`Gantt::bars`].
    ///
    /// The first primitive is the cosmetic chart background — it has
    /// NO entry in `elements` (clicks on empty canvas resolve to no
    /// gantt bar). Bars referencing unknown rows are still skipped
    /// (matches `emit_graphics`); the elements vector therefore can
    /// be SHORTER than `self.bars.len()`, but every entry carries the
    /// bar's ORIGINAL index in `self.bars` so the caller can still
    /// map back to the source data.
    #[must_use]
    pub fn emit_with_interaction(
        &self,
        theme: &Theme,
        viewport_px: Vec2,
    ) -> crate::interaction::EmittedChart {
        let mut g = Graphics::new();
        let mut elements: Vec<(usize, crate::interaction::ChartElementId)> = Vec::new();

        // Full-frame background — cosmetic, NOT pickable.
        g.fill(Fill::Solid(chart_to_wisp(theme.bg)));
        g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));

        // Bars.
        for (bar_idx, bar) in self.bars.iter().enumerate() {
            let Some(rect_px) = bar_pixel_rect(bar, self, theme, viewport_px.x) else {
                continue;
            };
            let fill = resolve_bar_fill(self, theme, &bar.owner);
            g.fill(Fill::Solid(fill));
            let ndc = pixel_rect_to_ndc(rect_px, viewport_px);
            let corner_ndc = theme.gantt.bar_corner_radius / viewport_px.y * 2.0;
            g.draw_rounded_rect(ndc, corner_ndc);
            elements.push((
                g.primitive_count() - 1,
                crate::interaction::ChartElementId::GanttBar(bar_idx),
            ));
        }

        crate::interaction::EmittedChart {
            graphics: g,
            elements,
        }
    }

    /// Lane-aware render (WG.3 / AUT-323).
    ///
    /// Renders concurrent bars in the same row as STACKED LANES
    /// rather than overdrawing on top of each other. Row height
    /// grows with lane count via
    /// [`crate::gantt::layout::row_height_for_row`]. Per-bar
    /// emissions:
    ///
    /// 1. Bar rounded-rect (mapped to `ChartElementId::GanttBar`).
    /// 2. Avatar circle on the bar's leading edge (one circle
    ///    primitive — text initials are layered via `wisp::Text`
    ///    in WG.2's chrome pass).
    /// 3. Allocation cap on the bar's trailing edge when
    ///    `bar.allocation_pct` is set — a filled dark circle the
    ///    host overlays with white % text in WG.2.
    /// 4. Tech-lead diamond marker when the bar carries
    ///    [`GanttRole::TechLead`].
    ///
    /// Bars with no avatar / allocation / tech-lead role emit
    /// JUST the rounded-rect — matches `emit_with_interaction`
    /// output exactly for single-bar rows with no extras, so
    /// callers can adopt this incrementally.
    ///
    /// Total primitive count per bar: 1 (rect) + 1 (avatar) +
    /// optional 1 (allocation cap) + optional 1 (tech-lead).
    ///
    /// Only the rounded-rect primitive carries a
    /// `ChartElementId::GanttBar` element entry — avatar +
    /// allocation cap + tech-lead are cosmetic and resolve to
    /// the same bar via hit-test depth ordering.
    #[must_use]
    pub fn emit_with_interaction_laned(
        &self,
        theme: &Theme,
        viewport_px: Vec2,
    ) -> crate::interaction::EmittedChart {
        let mut g = Graphics::new();
        let mut elements: Vec<(usize, crate::interaction::ChartElementId)> = Vec::new();

        // Full-frame background — cosmetic, NOT pickable.
        g.fill(Fill::Solid(chart_to_wisp(theme.bg)));
        g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));

        let avatar_radius_px = theme.gantt.bar_height * 0.42;
        let alloc_radius_px = theme.gantt.bar_height * 0.4;
        let diamond_size_px = theme.gantt.bar_height * 0.35;

        for (bar_idx, bar) in self.bars.iter().enumerate() {
            let Some(rect_px) = bar_pixel_rect_laned(bar, self, theme, viewport_px.x) else {
                continue;
            };
            // 1. Base bar.
            let fill = resolve_bar_fill(self, theme, &bar.owner);
            g.fill(Fill::Solid(fill));
            let ndc = pixel_rect_to_ndc(rect_px, viewport_px);
            let corner_ndc = theme.gantt.bar_corner_radius / viewport_px.y * 2.0;
            g.draw_rounded_rect(ndc, corner_ndc);
            elements.push((
                g.primitive_count() - 1,
                crate::interaction::ChartElementId::GanttBar(bar_idx),
            ));

            // Skip per-bar decorations when the bar is too thin to
            // host them legibly.
            if rect_px.w < theme.gantt.bar_height * 1.8 {
                continue;
            }

            // 2. Avatar circle on the bar's leading edge.
            let avatar_centre_px = Vec2::new(
                rect_px.x + theme.gantt.bar_height * 0.5,
                rect_px.y + theme.gantt.bar_height * 0.5,
            );
            g.fill(Fill::Solid(chart_to_wisp_white()));
            g.draw_ellipse(
                px_to_ndc(avatar_centre_px, viewport_px),
                Vec2::new(
                    avatar_radius_px / viewport_px.x * 2.0,
                    avatar_radius_px / viewport_px.y * 2.0,
                ),
            );

            // 3. Allocation cap (only when bar.allocation_pct set).
            if bar.allocation_pct.is_some() {
                let alloc_centre_px = Vec2::new(
                    rect_px.x + rect_px.w - theme.gantt.bar_height * 0.5,
                    rect_px.y + theme.gantt.bar_height * 0.5,
                );
                g.fill(Fill::Solid(chart_to_wisp_dark()));
                g.draw_ellipse(
                    px_to_ndc(alloc_centre_px, viewport_px),
                    Vec2::new(
                        alloc_radius_px / viewport_px.x * 2.0,
                        alloc_radius_px / viewport_px.y * 2.0,
                    ),
                );
            }

            // 4. Tech-lead diamond.
            if bar.roles.contains(&GanttRole::TechLead) {
                let diamond_centre_px = Vec2::new(
                    rect_px.x + rect_px.w * 0.5,
                    rect_px.y + theme.gantt.bar_height * 0.5,
                );
                g.fill(Fill::Solid(chart_to_wisp_dark()));
                let half = diamond_size_px * 0.5;
                let centre_ndc = px_to_ndc(diamond_centre_px, viewport_px);
                let dx = half / viewport_px.x * 2.0;
                let dy = half / viewport_px.y * 2.0;
                g.draw_polygon(&[
                    Vec2::new(centre_ndc.x + dx, centre_ndc.y),
                    Vec2::new(centre_ndc.x, centre_ndc.y + dy),
                    Vec2::new(centre_ndc.x - dx, centre_ndc.y),
                    Vec2::new(centre_ndc.x, centre_ndc.y - dy),
                ]);
            }
        }

        crate::interaction::EmittedChart {
            graphics: g,
            elements,
        }
    }
}

/// Pure white fill for avatar circles. Hosts overlay initials via
/// `wisp::Text` in WG.2 (chrome render pass).
fn chart_to_wisp_white() -> Color {
    Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    }
}

/// Dark fill for allocation caps + tech-lead diamonds. Picked to
/// stay legible against any owner palette colour.
fn chart_to_wisp_dark() -> Color {
    Color {
        r: 0.12,
        g: 0.12,
        b: 0.12,
        a: 1.0,
    }
}

/// Convert a pixel-space point (top-left origin, +Y down) to NDC
/// (`[-1, 1]`, +Y up).
fn px_to_ndc(p: Vec2, viewport_px: Vec2) -> Vec2 {
    Vec2::new(
        p.x / viewport_px.x * 2.0 - 1.0,
        1.0 - p.y / viewport_px.y * 2.0,
    )
}

/// Resolve an owner name → wisp colour.
///
/// `gantt.people` overrides win; otherwise the theme's
/// [`crate::OwnerPalette`] hashes the name to a stable colour.
fn resolve_bar_fill(gantt: &Gantt, theme: &Theme, owner: &str) -> Color {
    if let Some(person) = gantt.people.get(owner) {
        chart_to_wisp(person.color)
    } else {
        chart_to_wisp(theme.palette.color_for(owner))
    }
}

/// `wisp_chart::Color` (sRGB-encoded display values) →
/// `wisp::Color` (linear).
///
/// The renderer's non-sRGB framebuffer formats (`Bgra8Unorm`,
/// `Rgba8Unorm` — what `BROWSER_WEBGPU` exposes) don't perform any
/// sRGB encoding on output, so passing the sRGB-encoded byte
/// values through as if they were linear lands the right pixels.
/// This is fine for solid-fill chart bars; it would be wrong for
/// alpha blending or filter pipelines, both out of scope here.
fn chart_to_wisp(c: crate::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// Pixel-space rect (top-left origin, `+Y` down) → NDC
/// `[-1.0, 1.0]` rect for `wisp::Graphics`.
///
/// `wisp`'s renderer puts `+Y` **up** in NDC despite the
/// pixel-space convention used by [`crate::gantt::layout`]
/// (top-left origin, `+Y` down). The conversion below flips Y so
/// row 0 (top in pixel space) lands at large NDC y (top of the
/// rendered frame).
fn pixel_rect_to_ndc(rect: crate::gantt::layout::PixelRect, viewport_px: Vec2) -> Rect {
    let x = rect.x / viewport_px.x * 2.0 - 1.0;
    // `rect.y` is the TOP edge in pixel space; in flipped-Y NDC
    // that's the LARGER y. `Rect::new(x, y, w, h)` takes `(x, y)`
    // as the min corner, so we want the bar's BOTTOM (in pixel
    // space) → NDC min.y.
    let y = 1.0 - (rect.y + rect.h) / viewport_px.y * 2.0;
    let w = rect.w / viewport_px.x * 2.0;
    let h = rect.h / viewport_px.y * 2.0;
    Rect::new(x, y, w, h)
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
            rows: vec![Row::new("a", "A"), Row::new("b", "B")],
            bars: vec![
                Bar::new("a", date(2026, 1, 1)..date(2026, 7, 1), "Matt"),
                Bar::new("b", date(2026, 3, 1)..date(2026, 10, 1), "Alice"),
            ],
            people,
            markers: Vec::new(),
        }
    }

    #[test]
    fn emit_graphics_has_background_plus_one_per_bar() {
        let g = fixture().emit_graphics(&Theme::light(), Vec2::new(1920.0, 800.0));
        // 1 background + 2 bars.
        assert_eq!(g.primitive_count(), 3);
    }

    #[test]
    fn unknown_row_bar_is_skipped() {
        let mut g = fixture();
        g.bars.push(Bar::new(
            "ghost",
            date(2026, 2, 1)..date(2026, 3, 1),
            "Carol",
        ));
        let out = g.emit_graphics(&Theme::light(), Vec2::new(1920.0, 800.0));
        // Still 3 — ghost row bar dropped.
        assert_eq!(out.primitive_count(), 3);
    }

    #[test]
    fn pixel_to_ndc_origin_and_extents() {
        let viewport = Vec2::new(1920.0, 800.0);
        let r = pixel_rect_to_ndc(
            crate::gantt::layout::PixelRect {
                x: 0.0,
                y: 0.0,
                w: 1920.0,
                h: 800.0,
            },
            viewport,
        );
        assert!((r.min.x - -1.0).abs() < 1e-6);
        assert!((r.min.y - -1.0).abs() < 1e-6);
        assert!((r.max().x - 1.0).abs() < 1e-6);
        assert!((r.max().y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn emit_with_interaction_returns_one_element_per_bar_with_background_unmapped() {
        let g = fixture().emit_with_interaction(&Theme::light(), Vec2::new(1920.0, 800.0));
        // 1 background + 2 bars in graphics.
        assert_eq!(g.graphics.primitive_count(), 3);
        // 2 bars in elements — background is NOT mapped.
        assert_eq!(g.elements.len(), 2);
        assert_eq!(
            g.elements[0].1,
            crate::interaction::ChartElementId::GanttBar(0)
        );
        assert_eq!(
            g.elements[1].1,
            crate::interaction::ChartElementId::GanttBar(1)
        );
        // Primitive indices skip the background (index 0).
        assert_eq!(g.elements[0].0, 1);
        assert_eq!(g.elements[1].0, 2);
    }

    #[test]
    fn emit_with_interaction_preserves_original_bar_index_when_unknown_rows_are_skipped() {
        // Bars: [valid(row=a), ghost(row=ghost), valid(row=b)]. Middle one is dropped.
        let mut g = fixture();
        g.bars.insert(
            1,
            Bar::new("ghost", date(2026, 2, 1)..date(2026, 3, 1), "Carol"),
        );
        // Now bars = [valid_0(a), ghost_1, valid_2(b)]
        let emitted = g.emit_with_interaction(&Theme::light(), Vec2::new(1920.0, 800.0));
        // 1 background + 2 valid bars (ghost dropped).
        assert_eq!(emitted.graphics.primitive_count(), 3);
        assert_eq!(emitted.elements.len(), 2);
        // Original indices preserved: bar at position 0 and bar at position 2.
        assert_eq!(
            emitted.elements[0].1,
            crate::interaction::ChartElementId::GanttBar(0)
        );
        assert_eq!(
            emitted.elements[1].1,
            crate::interaction::ChartElementId::GanttBar(2)
        );
    }

    #[test]
    fn emit_with_interaction_laned_single_lane_matches_simple_bar_count() {
        // A row with one bar + no extras → 1 background + 1 bar = 2.
        let g = fixture();
        let theme = Theme::light();
        let out = g.emit_with_interaction_laned(&theme, Vec2::new(1920.0, 800.0));
        // 1 background + 2 bars (each with an avatar circle at this
        // viewport width — bar width > 1.8 * bar_height threshold).
        // 1 background + 2 * (rect + avatar) = 5.
        assert_eq!(out.graphics.primitive_count(), 5);
        // Element entries only for bar rects.
        assert_eq!(out.elements.len(), 2);
        assert_eq!(
            out.elements[0].1,
            crate::interaction::ChartElementId::GanttBar(0)
        );
    }

    #[test]
    fn emit_with_interaction_laned_emits_allocation_cap_only_when_set() {
        let g = Gantt {
            range: DateRange::year(2026),
            rows: vec![Row::new("a", "A")],
            bars: vec![
                Bar::new("a", date(2026, 1, 1)..date(2026, 7, 1), "Matt").with_allocation_pct(50.0),
            ],
            people: PersonMap::default(),
            markers: Vec::new(),
        };
        let out = g.emit_with_interaction_laned(&Theme::light(), Vec2::new(1920.0, 800.0));
        // 1 bg + 1 bar + 1 avatar + 1 allocation = 4.
        assert_eq!(out.graphics.primitive_count(), 4);
    }

    #[test]
    fn emit_with_interaction_laned_emits_tech_lead_diamond_for_role() {
        let g = Gantt {
            range: DateRange::year(2026),
            rows: vec![Row::new("a", "A")],
            bars: vec![
                Bar::new("a", date(2026, 1, 1)..date(2026, 7, 1), "Matt")
                    .with_role(crate::gantt::GanttRole::TechLead),
            ],
            people: PersonMap::default(),
            markers: Vec::new(),
        };
        let out = g.emit_with_interaction_laned(&Theme::light(), Vec2::new(1920.0, 800.0));
        // 1 bg + 1 bar + 1 avatar + 1 diamond = 4.
        assert_eq!(out.graphics.primitive_count(), 4);
    }

    #[test]
    fn emit_with_interaction_laned_three_concurrent_assignments_stack_vertically() {
        // 1 row, 3 bars in lanes 0/1/2, all same date range.
        let g = Gantt {
            range: DateRange::year(2026),
            rows: vec![Row::new("a", "A")],
            bars: vec![
                Bar::new("a", date(2026, 1, 1)..date(2026, 7, 1), "Matt").with_lane(0),
                Bar::new("a", date(2026, 1, 1)..date(2026, 7, 1), "Alice").with_lane(1),
                Bar::new("a", date(2026, 1, 1)..date(2026, 7, 1), "Bob").with_lane(2),
            ],
            people: PersonMap::default(),
            markers: Vec::new(),
        };
        let out = g.emit_with_interaction_laned(&Theme::light(), Vec2::new(1920.0, 800.0));
        // Each bar emits rect + avatar = 2. Plus 1 background. 7 total.
        assert_eq!(out.graphics.primitive_count(), 7);
        // All 3 bars have GanttBar entries.
        assert_eq!(out.elements.len(), 3);
        // Indices preserve original bar order.
        assert_eq!(
            out.elements[0].1,
            crate::interaction::ChartElementId::GanttBar(0)
        );
        assert_eq!(
            out.elements[1].1,
            crate::interaction::ChartElementId::GanttBar(1)
        );
        assert_eq!(
            out.elements[2].1,
            crate::interaction::ChartElementId::GanttBar(2)
        );
    }

    #[test]
    fn emit_with_interaction_laned_thin_bar_skips_decorations() {
        // A 1-day bar at viewport 1920 px is much thinner than
        // 1.8 * bar_height. Decorations should be skipped.
        let g = Gantt {
            range: DateRange::year(2026),
            rows: vec![Row::new("a", "A")],
            bars: vec![
                Bar::new("a", date(2026, 6, 15)..date(2026, 6, 16), "Matt")
                    .with_allocation_pct(75.0)
                    .with_role(crate::gantt::GanttRole::TechLead),
            ],
            people: PersonMap::default(),
            markers: Vec::new(),
        };
        let out = g.emit_with_interaction_laned(&Theme::light(), Vec2::new(1920.0, 800.0));
        // 1 bg + 1 bar = 2. No avatar / allocation / diamond.
        assert_eq!(out.graphics.primitive_count(), 2);
    }

    #[test]
    fn emit_graphics_parity_with_emit_with_interaction() {
        // The thin wrapper must produce primitive-count-identical
        // output. Same fixture, same viewport.
        let f = fixture();
        let theme = Theme::light();
        let vp = Vec2::new(1920.0, 800.0);
        let g_simple = f.emit_graphics(&theme, vp);
        let g_full = f.emit_with_interaction(&theme, vp).graphics;
        assert_eq!(g_simple.primitive_count(), g_full.primitive_count());
    }

    #[test]
    fn explicit_person_override_wins() {
        let gantt = fixture();
        let matt = resolve_bar_fill(&gantt, &Theme::light(), "Matt");
        // `#0072b2` = (0x00, 0x72, 0xb2) — Wong palette navy.
        let expected = chart_to_wisp(crate::color::Color::from_hex("#0072b2").unwrap());
        assert!((matt.r - expected.r).abs() < 1e-6);
        assert!((matt.g - expected.g).abs() < 1e-6);
        assert!((matt.b - expected.b).abs() < 1e-6);
    }
}
