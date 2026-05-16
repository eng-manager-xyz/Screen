//! Gauge chart — semicircular arc with threshold zones + needle
//! pointing at the current value.
//!
//! Layout: arc spans the top half of a circle, from `0` rad
//! (right, value = max) CCW to `π` rad (left, value = min).
//! Zones are sector slices coloured per `Zone.color`; the
//! needle is a thin triangle from the gauge centre to the
//! value's angle on the arc.

use glam::Vec2;
use wisp::application::Application;
use wisp::text::TextTexturePipeline;
use wisp::{Color, Fill, FlexText, Graphics, WispFontWeight};

use crate::chart_text::{ChartTextSpec, TextAnchor, build_text_node};
use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// One colour-coded threshold band on the gauge arc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zone {
    /// Inclusive value range: `(lo, hi)`. Must satisfy
    /// `lo <= hi` and fit within the gauge's `domain`.
    pub range: (f32, f32),
    /// Zone fill colour.
    pub color: ChartColor,
}

impl Zone {
    /// Construct a zone from `(lo, hi)` + colour.
    #[must_use]
    pub const fn new(range: (f32, f32), color: ChartColor) -> Self {
        Self { range, color }
    }
}

/// A semicircular gauge.
#[derive(Clone, Debug)]
pub struct Gauge {
    /// Current value displayed by the needle.
    pub value: f32,
    /// Value domain — `(min, max)`. Maps to angles `(π, 0)`.
    pub domain: (f32, f32),
    /// Threshold zones drawn as coloured sectors. Zones may
    /// overlap; later zones paint over earlier ones.
    pub zones: Vec<Zone>,
}

impl Gauge {
    /// Convert a domain value to its arc angle in radians.
    /// `value = domain.0` → `π`; `value = domain.1` → `0`.
    /// Values outside `domain` are clamped.
    #[must_use]
    pub fn angle_for(&self, value: f32) -> f32 {
        let (lo, hi) = self.domain;
        let span = hi - lo;
        if span.abs() < f32::EPSILON {
            return std::f32::consts::PI;
        }
        let t = ((value - lo) / span).clamp(0.0, 1.0);
        std::f32::consts::PI * (1.0 - t)
    }

    /// Emit zone arcs + needle as `wisp::Graphics`.
    ///
    /// The gauge fills `viewport_px` such that the semicircle's
    /// diameter equals the smaller of `viewport_px.x` and
    /// `2 * viewport_px.y` (so the arc fits vertically). The
    /// centre sits at the bottom-centre of the viewport.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let mut g = Graphics::new();
        let centre_px = Vec2::new(viewport_px.x * 0.5, viewport_px.y * 0.9);
        let radius_px = (viewport_px.x * 0.5).min(viewport_px.y * 0.9) - 16.0;
        let track_w = theme.indicator.gauge_track_width_px;
        let r_outer = radius_px;
        let r_inner = (radius_px - track_w).max(0.0);

        let centre_ndc = pixel_to_ndc(centre_px, viewport_px);
        let r_inner_ndc = r_inner / viewport_px.y * 2.0;
        let r_outer_ndc = r_outer / viewport_px.y * 2.0;

        // Track background — single neutral arc spanning the
        // full semicircle. Painted first so zone arcs land on
        // top.
        g.fill(Fill::Solid(chart_to_wisp(theme.plot.gridline_minor.color)));
        g.draw_annular_sector(
            centre_ndc,
            r_inner_ndc,
            r_outer_ndc,
            0.0,
            std::f32::consts::PI,
        );

        // Zone arcs.
        for zone in &self.zones {
            let a_hi = self.angle_for(zone.range.0); // value = lo → π
            let a_lo = self.angle_for(zone.range.1); // value = hi → 0
            if a_hi <= a_lo {
                continue;
            }
            g.fill(Fill::Solid(chart_to_wisp(zone.color)));
            g.draw_annular_sector(centre_ndc, r_inner_ndc, r_outer_ndc, a_lo, a_hi);
        }

        // Needle — a thin radial bar drawn as a rotated rect via
        // draw_line from centre to the value's angle on the
        // outer radius.
        let needle_angle = self.angle_for(self.value);
        let tip_px = Vec2::new(
            centre_px.x + radius_px * needle_angle.cos(),
            centre_px.y - radius_px * needle_angle.sin(),
        );
        let needle_w_ndc = 3.0 / viewport_px.y * 2.0;
        g.fill(Fill::Solid(chart_to_wisp(
            theme.indicator.gauge_needle_color,
        )));
        g.draw_line(centre_ndc, pixel_to_ndc(tip_px, viewport_px), needle_w_ndc);

        // Centre hub — small filled circle at the pivot point.
        let hub_r_ndc = 6.0 / viewport_px.y * 2.0;
        g.draw_ellipse(centre_ndc, Vec2::splat(hub_r_ndc));

        g
    }

    /// Emit the centred numeric value as a [`FlexText`] node.
    /// Formatted by [`crate::indicator::format_value`] for magnitude
    /// compaction. Renders in wisp's late pass, on top of the gauge
    /// arc.
    #[must_use]
    pub fn emit_text_nodes(
        &self,
        app: &Application,
        pipeline: &TextTexturePipeline,
        theme: &Theme,
        viewport_px: Vec2,
    ) -> Vec<FlexText> {
        let value_str = crate::indicator::format_value(f64::from(self.value));
        let spec = ChartTextSpec {
            content: value_str,
            anchor_px: Vec2::new(viewport_px.x * 0.5, viewport_px.y * 0.65),
            size_px: theme.indicator.numeric_font_size,
            color: theme.text_primary,
            anchor: TextAnchor::MiddleCentre,
            weight: WispFontWeight::Bold,
        };
        vec![build_text_node(app, pipeline, viewport_px, &spec)]
    }
}

fn pixel_to_ndc(p: Vec2, viewport_px: Vec2) -> Vec2 {
    Vec2::new(
        p.x / viewport_px.x * 2.0 - 1.0,
        1.0 - p.y / viewport_px.y * 2.0,
    )
}

fn chart_to_wisp(c: ChartColor) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red() -> ChartColor {
        ChartColor::from_hex("#e74c3c").unwrap()
    }
    fn green() -> ChartColor {
        ChartColor::from_hex("#27ae60").unwrap()
    }

    #[test]
    fn angle_for_min_is_pi() {
        let gauge = Gauge {
            value: 0.0,
            domain: (0.0, 100.0),
            zones: vec![],
        };
        assert!((gauge.angle_for(0.0) - std::f32::consts::PI).abs() < 1e-5);
    }

    #[test]
    fn angle_for_max_is_zero() {
        let gauge = Gauge {
            value: 100.0,
            domain: (0.0, 100.0),
            zones: vec![],
        };
        assert!(gauge.angle_for(100.0).abs() < 1e-5);
    }

    #[test]
    fn angle_for_midpoint_is_pi_over_2() {
        let gauge = Gauge {
            value: 50.0,
            domain: (0.0, 100.0),
            zones: vec![],
        };
        assert!((gauge.angle_for(50.0) - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn angle_for_clamps_out_of_domain_values() {
        let gauge = Gauge {
            value: 150.0,
            domain: (0.0, 100.0),
            zones: vec![],
        };
        assert!(gauge.angle_for(150.0).abs() < 1e-5);
        assert!((gauge.angle_for(-50.0) - std::f32::consts::PI).abs() < 1e-5);
    }

    #[test]
    fn emit_graphics_emits_track_plus_zones_plus_needle_plus_hub() {
        let gauge = Gauge {
            value: 73.0,
            domain: (0.0, 100.0),
            zones: vec![
                Zone::new((0.0, 60.0), green()),
                Zone::new((60.0, 100.0), red()),
            ],
        };
        let theme = Theme::light();
        let g = gauge.emit_graphics(&theme, Vec2::new(240.0, 160.0));
        // 1 track + 2 zone arcs + 1 needle line + 1 centre hub.
        assert_eq!(g.primitive_count(), 5);
    }
}
