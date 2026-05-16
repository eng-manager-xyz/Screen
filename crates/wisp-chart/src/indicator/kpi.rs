//! KPI card — big number + label + colour-coded delta + optional
//! sparkline. The default top-of-dashboard summary tile.

use glam::Vec2;
use wisp::application::Application;
use wisp::text::TextTexturePipeline;
use wisp::{Color, Fill, FlexText, Graphics, WispFontWeight};

use crate::chart_text::{ChartTextSpec, TextAnchor, build_text_node};
use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// Sign of a delta — drives the arrow glyph + colour lookup.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DeltaKind {
    /// Up-arrow + `theme.indicator.delta_up` colour.
    Up,
    /// Down-arrow + `theme.indicator.delta_down` colour.
    Down,
    /// Neutral indicator + `theme.indicator.delta_neutral` colour.
    #[default]
    Neutral,
}

/// Period-over-period change displayed beside a KPI value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delta {
    /// Up / Down / Neutral — drives colour + glyph.
    pub kind: DeltaKind,
    /// Pre-formatted text (e.g. `"+12.4% vs last mo"`). Free-form
    /// so the caller controls locale + precision.
    pub formatted: String,
}

/// A KPI card — big value, label, optional delta, optional
/// sparkline.
#[derive(Clone, Debug)]
pub struct Kpi {
    /// The big-number value.
    pub value: f64,
    /// One-line label under the big number.
    pub label: String,
    /// Period-over-period change.
    pub delta: Option<Delta>,
    /// Optional sparkline points (Y-only — X is implicit row
    /// index). Drawn inside the KPI card's bottom band.
    pub sparkline: Option<Vec<f32>>,
}

impl Kpi {
    /// Emit the sparkline (if present) as `wisp::Graphics`.
    /// Caller positions the resulting node inside the KPI's
    /// rectangle on the stage; the function uses
    /// `viewport_px` to compute NDC for the sparkline path.
    ///
    /// Sparkline is laid out in the bottom 25% of the
    /// viewport with `8 px` of horizontal padding.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let mut g = Graphics::new();
        let Some(spark) = self.sparkline.as_ref() else {
            return g;
        };
        if spark.len() < 2 {
            return g;
        }

        let pad_x = 8.0_f32;
        let plot_left = pad_x;
        let plot_right = viewport_px.x - pad_x;
        let plot_top = viewport_px.y * 0.75;
        let plot_bottom = viewport_px.y - 8.0;

        let mut min = spark[0];
        let mut max = spark[0];
        for &v in spark.iter().skip(1) {
            if v < min {
                min = v;
            }
            if v > max {
                max = v;
            }
        }
        if (max - min).abs() < f32::EPSILON {
            // Flat line — centre.
            max = min + 1.0;
        }
        let dx = (plot_right - plot_left) / usize_to_f32(spark.len() - 1);
        let map_y = |v: f32| -> f32 {
            let t = (v - min) / (max - min);
            plot_bottom - t * (plot_bottom - plot_top)
        };

        let stroke = chart_to_wisp(theme.indicator.sparkline_color);
        g.fill(Fill::Solid(stroke));
        let line_w_ndc = theme.indicator.sparkline_width_px / viewport_px.y * 2.0;

        for pair in spark.windows(2) {
            let i = spark
                .iter()
                .position(|p| (p - pair[0]).abs() < f32::EPSILON)
                .unwrap_or(0);
            let x0 = plot_left + usize_to_f32(i) * dx;
            let x1 = x0 + dx;
            let y0 = map_y(pair[0]);
            let y1 = map_y(pair[1]);
            let a = pixel_to_ndc(Vec2::new(x0, y0), viewport_px);
            let b = pixel_to_ndc(Vec2::new(x1, y1), viewport_px);
            g.draw_line(a, b, line_w_ndc);
        }
        g
    }

    /// Emit the big-value + label + delta as [`FlexText`] nodes
    /// (Inter via the late-pass render pipeline so the text reads
    /// on top of any sparkline or background graphic).
    ///
    /// Layout: big value at `y = 12 px`, label below it, delta
    /// below the label. All left-aligned with `8 px` left padding.
    #[must_use]
    pub fn emit_text_nodes(
        &self,
        app: &Application,
        pipeline: &TextTexturePipeline,
        theme: &Theme,
        viewport_px: Vec2,
    ) -> Vec<FlexText> {
        let mut out = Vec::new();
        let pad_x = 8.0_f32;

        // Big value.
        let value_str = format_value(self.value);
        let value_spec = ChartTextSpec {
            content: value_str,
            anchor_px: Vec2::new(pad_x, 12.0),
            size_px: theme.indicator.numeric_font_size,
            color: theme.text_primary,
            anchor: TextAnchor::TopLeft,
            weight: WispFontWeight::Bold,
        };
        out.push(build_text_node(app, pipeline, viewport_px, &value_spec));

        // Label.
        let label_y = 12.0 + theme.indicator.numeric_font_size + 8.0;
        let label_spec = ChartTextSpec {
            content: self.label.clone(),
            anchor_px: Vec2::new(pad_x, label_y),
            size_px: theme.indicator.label_font_size,
            color: theme.text_muted,
            anchor: TextAnchor::TopLeft,
            weight: WispFontWeight::Regular,
        };
        out.push(build_text_node(app, pipeline, viewport_px, &label_spec));

        // Delta.
        if let Some(delta) = self.delta.as_ref() {
            let glyph = match delta.kind {
                DeltaKind::Up => "▲ ",
                DeltaKind::Down => "▼ ",
                DeltaKind::Neutral => "– ",
            };
            let colour = match delta.kind {
                DeltaKind::Up => theme.indicator.delta_up,
                DeltaKind::Down => theme.indicator.delta_down,
                DeltaKind::Neutral => theme.indicator.delta_neutral,
            };
            let delta_y = label_y + theme.indicator.label_font_size + 8.0;
            let delta_spec = ChartTextSpec {
                content: format!("{glyph}{text}", text = delta.formatted),
                anchor_px: Vec2::new(pad_x, delta_y),
                size_px: theme.indicator.delta_font_size,
                color: colour,
                anchor: TextAnchor::TopLeft,
                weight: WispFontWeight::Regular,
            };
            out.push(build_text_node(app, pipeline, viewport_px, &delta_spec));
        }

        out
    }
}

/// Render a numeric value as a short label — "1.23M", "456K",
/// "789" — with one decimal place when truncated.
///
/// Locale-stable (no thousand separators). Caller can override
/// by setting `Delta.formatted` or by formatting the value
/// itself before assigning to `Kpi.value`.
#[must_use]
pub fn format_value(v: f64) -> String {
    let abs = v.abs();
    if abs >= 1.0e9 {
        format!("{:.2}B", v / 1.0e9)
    } else if abs >= 1.0e6 {
        format!("{:.2}M", v / 1.0e6)
    } else if abs >= 1.0e3 {
        format!("{:.1}K", v / 1.0e3)
    } else if (v.fract()).abs() < 1e-6 {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "branch only taken when fract is zero and abs < 1e3; integer part fits f64 mantissa easily"
        )]
        let i = v as i64;
        format!("{i}")
    } else {
        format!("{v:.2}")
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

fn usize_to_f32(v: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "sparkline lengths fit easily in f32 mantissa (~hundreds practical max)"
    )]
    {
        v as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_value_compacts_large_magnitudes() {
        assert_eq!(format_value(1_234_567.0), "1.23M");
        assert_eq!(format_value(2_500_000_000.0), "2.50B");
        assert_eq!(format_value(789.0), "789");
        assert_eq!(format_value(1.5), "1.50");
    }

    #[test]
    fn format_value_handles_negatives() {
        assert_eq!(format_value(-1_234_567.0), "-1.23M");
        assert_eq!(format_value(-12.0), "-12");
    }

    #[test]
    fn emit_graphics_with_no_sparkline_returns_empty() {
        let kpi = Kpi {
            value: 100.0,
            label: "Users".into(),
            delta: None,
            sparkline: None,
        };
        let theme = crate::theme::Theme::light();
        let g = kpi.emit_graphics(&theme, Vec2::new(240.0, 120.0));
        assert_eq!(g.primitive_count(), 0);
    }

    #[test]
    fn emit_graphics_with_sparkline_emits_n_minus_1_segments() {
        let kpi = Kpi {
            value: 100.0,
            label: "Users".into(),
            delta: None,
            sparkline: Some(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        };
        let theme = crate::theme::Theme::light();
        let g = kpi.emit_graphics(&theme, Vec2::new(240.0, 120.0));
        // 5 points → 4 segments.
        assert_eq!(g.primitive_count(), 4);
    }
}
