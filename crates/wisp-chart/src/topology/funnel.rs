//! Funnel chart — staged conversion / loss visualisation. Each
//! stage is a horizontal band; band width reflects remaining
//! count relative to the first stage.

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// One stage in a funnel.
#[derive(Clone, Debug, PartialEq)]
pub struct FunnelStage {
    /// Display label.
    pub label: String,
    /// Remaining count at this stage.
    pub count: f32,
    /// Fill colour.
    pub color: ChartColor,
}

impl FunnelStage {
    /// Construct from a label, count, and colour.
    #[must_use]
    pub fn new(label: impl Into<String>, count: f32, color: ChartColor) -> Self {
        Self {
            label: label.into(),
            count,
            color,
        }
    }
}

/// Funnel chart value type.
#[derive(Clone, Debug)]
pub struct Funnel {
    /// Stages in top-down order — first stage is the widest.
    pub stages: Vec<FunnelStage>,
}

impl Funnel {
    /// Construct from a stage list.
    #[must_use]
    pub const fn new(stages: Vec<FunnelStage>) -> Self {
        Self { stages }
    }

    /// Emit one trapezoid per stage as a `wisp::Graphics`.
    /// Width is `count / max_count * plot_width`; height is
    /// `plot_height / num_stages`.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        if self.stages.is_empty() {
            return g;
        }
        let pad = 16.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let plot_w = plot_right - plot_left;
        let plot_h = plot_bottom - plot_top;

        let max_count = self
            .stages
            .iter()
            .map(|s| s.count)
            .fold(f32::NEG_INFINITY, f32::max);
        if max_count.abs() < f32::EPSILON {
            return g;
        }
        let n = self.stages.len();
        let stage_h = plot_h / usize_to_f32(n);
        let centre_x = (plot_left + plot_right) * 0.5;

        for (i, stage) in self.stages.iter().enumerate() {
            let band_w = stage.count / max_count * plot_w;
            let y = plot_top + usize_to_f32(i) * stage_h;
            let x = centre_x - band_w * 0.5;
            g.fill(Fill::Solid(chart_to_wisp(stage.color)));
            let rect = px_rect_to_ndc(x, y, band_w, stage_h - 2.0, viewport_px);
            g.draw_rect(rect);
        }
        g
    }
}

fn px_rect_to_ndc(x: f32, y: f32, w: f32, h: f32, viewport_px: Vec2) -> Rect {
    let nx = x / viewport_px.x * 2.0 - 1.0;
    let ny = 1.0 - (y + h) / viewport_px.y * 2.0;
    Rect::new(nx, ny, w / viewport_px.x * 2.0, h / viewport_px.y * 2.0)
}

fn chart_to_wisp(c: ChartColor) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn usize_to_f32(v: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "funnel stage counts ≤ ~10 in practice"
    )]
    {
        v as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(hex: &str) -> ChartColor {
        ChartColor::from_hex(hex).unwrap()
    }

    fn fixture() -> Funnel {
        Funnel::new(vec![
            FunnelStage::new("Visited", 10000.0, c("#0072b2")),
            FunnelStage::new("Signed up", 4000.0, c("#56b4e9")),
            FunnelStage::new("Activated", 1800.0, c("#7faedc")),
            FunnelStage::new("Converted", 600.0, c("#a3c7ea")),
        ])
    }

    #[test]
    fn funnel_emits_one_rect_per_stage() {
        let theme = Theme::light();
        let g = fixture().emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 4);
    }

    #[test]
    fn empty_funnel_emits_nothing() {
        let theme = Theme::light();
        let g = Funnel::new(vec![]).emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
