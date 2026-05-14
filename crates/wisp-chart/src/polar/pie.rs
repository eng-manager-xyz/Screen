//! Pie + donut chart — categorical proportions of a whole.
//!
//! `hollow_ratio: 0.0` = full pie. `hollow_ratio: 0.5` = donut
//! with the inner hole spanning 50% of the radius.

use glam::Vec2;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// One slice of a pie/donut chart — a value + label + colour.
#[derive(Clone, Debug, PartialEq)]
pub struct Slice {
    /// Numeric magnitude. Slice's angular span is
    /// `value / sum(all_values) * 2π`.
    pub value: f32,
    /// Display label (rendered separately by the caller as a
    /// `wisp::Text`; not emitted by `emit_graphics`).
    pub label: String,
    /// Fill colour.
    pub color: ChartColor,
}

impl Slice {
    /// Construct a slice from value + label + colour.
    #[must_use]
    pub fn new(value: f32, label: impl Into<String>, color: ChartColor) -> Self {
        Self {
            value,
            label: label.into(),
            color,
        }
    }
}

/// A pie or donut chart.
#[derive(Clone, Debug)]
pub struct Pie {
    /// Slices in render order (first → CCW from `+x` axis).
    pub slices: Vec<Slice>,
    /// `0.0` = pie, `(0.0, 1.0)` = donut. Clamped to `[0, 0.95]`.
    pub hollow_ratio: f32,
}

impl Pie {
    /// Construct from a slice list. Default `hollow_ratio = 0.0`
    /// (full pie).
    #[must_use]
    pub fn new(slices: Vec<Slice>) -> Self {
        Self {
            slices,
            hollow_ratio: 0.0,
        }
    }

    /// Set the donut hole ratio.
    #[must_use]
    pub fn hollow_ratio(mut self, ratio: f32) -> Self {
        self.hollow_ratio = ratio.clamp(0.0, 0.95);
        self
    }

    /// Total of all slice values (denominator for fractions).
    #[must_use]
    pub fn total(&self) -> f32 {
        self.slices.iter().map(|s| s.value).sum()
    }

    /// Emit one annular sector per slice as a `wisp::Graphics`.
    /// Pie centre sits at the centre of `viewport_px`; outer
    /// radius is `min(width, height) * 0.45` (leaves room for
    /// labels).
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        let total = self.total();
        if total.abs() < f32::EPSILON || self.slices.is_empty() {
            return g;
        }
        let centre_px = viewport_px * 0.5;
        let radius_px = (viewport_px.x.min(viewport_px.y)) * 0.45;
        let r_outer_ndc = radius_px / viewport_px.y * 2.0;
        let r_inner_ndc = r_outer_ndc * self.hollow_ratio;
        let centre_ndc = pixel_to_ndc(centre_px, viewport_px);

        let mut angle = 0.0_f32;
        for slice in &self.slices {
            let span = slice.value / total * std::f32::consts::TAU;
            let start = angle;
            let end = angle + span;
            angle = end;
            if span < 1e-5 {
                continue;
            }
            g.fill(Fill::Solid(chart_to_wisp(slice.color)));
            g.draw_annular_sector(centre_ndc, r_inner_ndc, r_outer_ndc, start, end);
        }
        g
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
    fn blue() -> ChartColor {
        ChartColor::from_hex("#3498db").unwrap()
    }

    #[test]
    fn pie_emits_one_sector_per_slice() {
        let pie = Pie::new(vec![
            Slice::new(30.0, "A", red()),
            Slice::new(45.0, "B", green()),
            Slice::new(25.0, "C", blue()),
        ]);
        let theme = Theme::light();
        let g = pie.emit_graphics(&theme, Vec2::new(200.0, 200.0));
        assert_eq!(g.primitive_count(), 3);
    }

    #[test]
    fn pie_total_sums_slice_values() {
        let pie = Pie::new(vec![
            Slice::new(10.0, "A", red()),
            Slice::new(20.0, "B", green()),
            Slice::new(30.0, "C", blue()),
        ]);
        assert!((pie.total() - 60.0).abs() < 1e-5);
    }

    #[test]
    fn donut_hollow_ratio_is_clamped() {
        let pie = Pie::new(vec![Slice::new(1.0, "A", red())]).hollow_ratio(2.0);
        assert!((pie.hollow_ratio - 0.95).abs() < 1e-5);
        let pie = Pie::new(vec![Slice::new(1.0, "A", red())]).hollow_ratio(-1.0);
        assert!(pie.hollow_ratio.abs() < 1e-5);
    }

    #[test]
    fn empty_pie_emits_no_sectors() {
        let pie = Pie::new(vec![]);
        let theme = Theme::light();
        let g = pie.emit_graphics(&theme, Vec2::new(200.0, 200.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
