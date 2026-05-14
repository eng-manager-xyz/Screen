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
use crate::gantt::layout::bar_pixel_rect;
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
        let mut g = Graphics::new();

        // Full-frame background. Rect in NDC.
        g.fill(Fill::Solid(chart_to_wisp(theme.bg)));
        g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));

        // Bars.
        for bar in &self.bars {
            let Some(rect_px) = bar_pixel_rect(bar, self, theme, viewport_px.x) else {
                continue;
            };
            let fill = resolve_bar_fill(self, theme, &bar.owner);
            g.fill(Fill::Solid(fill));
            let ndc = pixel_rect_to_ndc(rect_px, viewport_px);
            let corner_ndc = theme.gantt.bar_corner_radius / viewport_px.y * 2.0;
            g.draw_rounded_rect(ndc, corner_ndc);
        }

        g
    }
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
