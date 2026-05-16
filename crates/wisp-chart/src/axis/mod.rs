//! Axis renderer — tick lines (in `wisp::Graphics`) + tick
//! labels + axis title (as `wisp::Text` nodes).
//!
//! Decoupled from any specific `Scale` trait because
//! the v1 scale family is concrete-typed (`LinearScale` /
//! `BandScale` / etc. without a unifying `Scale` trait yet).
//! The axis renderer takes a flat list of `(pixel_position,
//! label)` pairs — the Plot facade or any chart that consumes
//! the axis renderer is responsible for asking its scale for
//! ticks via the scale's own `ticks()` / `band_centre()` /
//! `ticks_at()` methods.
//!
//! This module emits TWO things per axis:
//!
//! * A `wisp::Graphics` containing the axis line + tick marks +
//!   optional gridline extensions. Returned for the caller to
//!   add to its scene.
//! * A `Vec<wisp::Text>` containing the tick labels + optional
//!   axis title. Text is a separate Node type from Graphics, so
//!   the caller adds both lists.
//!
//! Render order convention: axes (lines + ticks) emit BEFORE
//! marks so bars / lines composite on top of the gridline
//! extensions. Tick labels emit AFTER marks so they're never
//! occluded.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::text::TextTexturePipeline;
use wisp::{Color, Fill, Graphics, WispFontWeight};

use crate::chart_text::{ChartTextSpec, TextAnchor, build_text_node};
use crate::color::Color as ChartColor;
use crate::theme::{AxisTheme, PlotTheme};

/// Which side of the plot area the axis lives on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisPosition {
    /// X axis below the plot area; tick labels below tick marks.
    Bottom,
    /// X axis above the plot area; tick labels above tick marks.
    Top,
    /// Y axis left of the plot area; tick labels left of marks.
    Left,
    /// Y axis right of the plot area; tick labels right of marks.
    Right,
}

/// One labelled tick. `position` is the pixel coordinate along
/// the axis (x for horizontal axes, y for vertical).
#[derive(Clone, Debug, PartialEq)]
pub struct TickLabel {
    /// Pixel coordinate along the axis (`x` for horizontal axes,
    /// `y` for vertical) where the tick mark + label sit.
    pub position: f32,
    /// Display string for the tick.
    pub label: String,
}

/// Emit a horizontal-axis (X) line + tick marks + optional
/// gridline extensions as a `Graphics`.
///
/// `plot_rect` is the chart's plot area in pixel space (top-left
/// origin). For [`AxisPosition::Bottom`] the axis line sits at
/// `plot_rect.max().y`; for [`AxisPosition::Top`] at
/// `plot_rect.min.y`.
#[must_use]
pub fn emit_x_axis_lines(
    ticks: &[TickLabel],
    plot_rect: Rect,
    viewport_px: Vec2,
    position: AxisPosition,
    axis_theme: &AxisTheme,
    plot_theme: &PlotTheme,
    line_color: ChartColor,
) -> Graphics {
    let mut g = Graphics::new();
    let axis_y = match position {
        AxisPosition::Bottom => plot_rect.max().y,
        AxisPosition::Top => plot_rect.min.y,
        _ => return g,
    };
    let tick_dir: f32 = if matches!(position, AxisPosition::Bottom) {
        1.0
    } else {
        -1.0
    };

    let wisp_line = chart_to_wisp(line_color);
    g.fill(Fill::Solid(wisp_line));

    // Axis line — runs across the plot width at `axis_y`.
    let axis_a = pixel_to_ndc(Vec2::new(plot_rect.min.x, axis_y), viewport_px);
    let axis_b = pixel_to_ndc(Vec2::new(plot_rect.max().x, axis_y), viewport_px);
    let axis_w = ndc_length_y(axis_theme.tick_length_px.clamp(1.0, 2.0), viewport_px);
    g.draw_line(axis_a, axis_b, axis_w);

    // Per-tick: tick mark + optional gridline extension.
    let grid_extension = plot_theme.gridline_minor;
    let grid_color = chart_to_wisp(grid_extension.color);
    let grid_w = ndc_length_y(grid_extension.width, viewport_px);

    for tick in ticks {
        // Gridline extension up through the plot area.
        if grid_extension.width > 0.0 {
            g.fill(Fill::Solid(grid_color));
            let top = pixel_to_ndc(Vec2::new(tick.position, plot_rect.min.y), viewport_px);
            let bottom = pixel_to_ndc(Vec2::new(tick.position, plot_rect.max().y), viewport_px);
            g.draw_line(top, bottom, grid_w);
        }

        // Tick mark beyond the axis line (away from plot).
        g.fill(Fill::Solid(wisp_line));
        let tick_a = pixel_to_ndc(Vec2::new(tick.position, axis_y), viewport_px);
        let tick_b = pixel_to_ndc(
            Vec2::new(tick.position, axis_y + axis_theme.tick_length_px * tick_dir),
            viewport_px,
        );
        let tw = ndc_length_y(1.0, viewport_px);
        g.draw_line(tick_a, tick_b, tw);
    }

    g
}

/// Emit a vertical-axis (Y) line + tick marks + optional
/// gridline extensions as a `Graphics`.
#[must_use]
pub fn emit_y_axis_lines(
    ticks: &[TickLabel],
    plot_rect: Rect,
    viewport_px: Vec2,
    position: AxisPosition,
    axis_theme: &AxisTheme,
    plot_theme: &PlotTheme,
    line_color: ChartColor,
) -> Graphics {
    let mut g = Graphics::new();
    let axis_x = match position {
        AxisPosition::Left => plot_rect.min.x,
        AxisPosition::Right => plot_rect.max().x,
        _ => return g,
    };
    let tick_dir: f32 = if matches!(position, AxisPosition::Left) {
        -1.0
    } else {
        1.0
    };

    let wisp_line = chart_to_wisp(line_color);
    g.fill(Fill::Solid(wisp_line));

    let axis_a = pixel_to_ndc(Vec2::new(axis_x, plot_rect.min.y), viewport_px);
    let axis_b = pixel_to_ndc(Vec2::new(axis_x, plot_rect.max().y), viewport_px);
    let axis_w = ndc_length_x(axis_theme.tick_length_px.clamp(1.0, 2.0), viewport_px);
    g.draw_line(axis_a, axis_b, axis_w);

    let grid = plot_theme.gridline_minor;
    let grid_color = chart_to_wisp(grid.color);
    let grid_w = ndc_length_x(grid.width, viewport_px);

    for tick in ticks {
        if grid.width > 0.0 {
            g.fill(Fill::Solid(grid_color));
            let left = pixel_to_ndc(Vec2::new(plot_rect.min.x, tick.position), viewport_px);
            let right = pixel_to_ndc(Vec2::new(plot_rect.max().x, tick.position), viewport_px);
            g.draw_line(left, right, grid_w);
        }
        g.fill(Fill::Solid(wisp_line));
        let a = pixel_to_ndc(Vec2::new(axis_x, tick.position), viewport_px);
        let b = pixel_to_ndc(
            Vec2::new(axis_x + axis_theme.tick_length_px * tick_dir, tick.position),
            viewport_px,
        );
        let tw = ndc_length_x(1.0, viewport_px);
        g.draw_line(a, b, tw);
    }

    g
}


/// Emit X-axis tick labels + optional title as Inter-text [`wisp::FlexText`]
/// nodes. The new path that replaces the bitmap-font
/// [`emit_x_axis_text`] — every chart that emits text should call
/// this and `emit_y_axis_text` instead.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "axis text emission unavoidably needs: app, pipeline, tick data, plot bounds, viewport, side, theme, color, optional title. Grouping into a struct would just rename the same 9 fields."
)]
pub fn emit_x_axis_text(
    app: &Application,
    pipeline: &TextTexturePipeline,
    ticks: &[TickLabel],
    plot_rect: Rect,
    viewport_px: Vec2,
    position: AxisPosition,
    axis_theme: &AxisTheme,
    text_color: ChartColor,
    title: Option<&str>,
) -> Vec<wisp::FlexText> {
    let mut out = Vec::new();
    if !matches!(position, AxisPosition::Bottom | AxisPosition::Top) {
        return out;
    }
    let axis_y = if matches!(position, AxisPosition::Bottom) {
        plot_rect.max().y
    } else {
        plot_rect.min.y
    };
    let label_offset_y = axis_theme.tick_length_px + axis_theme.tick_label_font_size * 0.6;
    let label_y_dir: f32 = if matches!(position, AxisPosition::Bottom) {
        1.0
    } else {
        -1.0
    };

    for tick in ticks {
        let anchor_px = Vec2::new(tick.position, axis_y + label_offset_y * label_y_dir);
        let spec = ChartTextSpec {
            content: tick.label.clone(),
            anchor_px,
            size_px: axis_theme.tick_label_font_size,
            color: text_color,
            anchor: if matches!(position, AxisPosition::Bottom) {
                TextAnchor::TopCentre
            } else {
                // Top axis: anchor by the *bottom-centre* of the
                // label; we don't have that anchor today, so flip
                // the offset sign and keep TopCentre.
                TextAnchor::TopCentre
            },
            weight: WispFontWeight::Regular,
        };
        out.push(build_text_node(app, pipeline, viewport_px, &spec));
    }

    if let Some(t) = title {
        let title_offset_y = label_offset_y + axis_theme.tick_label_font_size * 2.2;
        let centre_x = (plot_rect.min.x + plot_rect.max().x) * 0.5;
        let anchor_px = Vec2::new(centre_x, axis_y + title_offset_y * label_y_dir);
        let spec = ChartTextSpec {
            content: t.to_owned(),
            anchor_px,
            size_px: axis_theme.tick_label_font_size * 1.05,
            color: text_color,
            anchor: TextAnchor::TopCentre,
            weight: WispFontWeight::Regular,
        };
        out.push(build_text_node(app, pipeline, viewport_px, &spec));
    }
    out
}

/// Emit Y-axis tick labels + optional rotated title as Inter-text
/// [`wisp::FlexText`] nodes. Replaces the bitmap-font
/// [`emit_y_axis_text`].
///
/// The y-axis title is rendered horizontally for now (rotated text
/// flow needs container-level rotation which the sprite path
/// honours via its `transform.rotation` — left as a follow-up).
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "axis text emission unavoidably needs: app, pipeline, tick data, plot bounds, viewport, side, theme, color, optional title."
)]
pub fn emit_y_axis_text(
    app: &Application,
    pipeline: &TextTexturePipeline,
    ticks: &[TickLabel],
    plot_rect: Rect,
    viewport_px: Vec2,
    position: AxisPosition,
    axis_theme: &AxisTheme,
    text_color: ChartColor,
    title: Option<&str>,
) -> Vec<wisp::FlexText> {
    let mut out = Vec::new();
    if !matches!(position, AxisPosition::Left | AxisPosition::Right) {
        return out;
    }
    let axis_x = if matches!(position, AxisPosition::Left) {
        plot_rect.min.x
    } else {
        plot_rect.max().x
    };
    let label_offset_x = axis_theme.tick_length_px + axis_theme.tick_label_font_size * 0.45;
    let label_x_dir: f32 = if matches!(position, AxisPosition::Left) {
        -1.0
    } else {
        1.0
    };

    for tick in ticks {
        let anchor_px = Vec2::new(axis_x + label_offset_x * label_x_dir, tick.position);
        let spec = ChartTextSpec {
            content: tick.label.clone(),
            anchor_px,
            size_px: axis_theme.tick_label_font_size,
            color: text_color,
            anchor: if matches!(position, AxisPosition::Left) {
                TextAnchor::MiddleRight
            } else {
                TextAnchor::MiddleLeft
            },
            weight: WispFontWeight::Regular,
        };
        out.push(build_text_node(app, pipeline, viewport_px, &spec));
    }

    if let Some(t) = title {
        let title_offset_x = label_offset_x + axis_theme.tick_label_font_size * 3.5;
        let centre_y = (plot_rect.min.y + plot_rect.max().y) * 0.5;
        let anchor_px = Vec2::new(axis_x + title_offset_x * label_x_dir, centre_y);
        let spec = ChartTextSpec {
            content: t.to_owned(),
            anchor_px,
            size_px: axis_theme.tick_label_font_size * 1.05,
            color: text_color,
            anchor: TextAnchor::MiddleCentre,
            weight: WispFontWeight::Regular,
        };
        let mut node = build_text_node(app, pipeline, viewport_px, &spec);
        // Rotate -90° so the y-axis title reads bottom-to-top.
        node.container.transform.rotation = -std::f32::consts::FRAC_PI_2;
        out.push(node);
    }
    out
}

fn pixel_to_ndc(p: Vec2, viewport_px: Vec2) -> Vec2 {
    Vec2::new(
        p.x / viewport_px.x * 2.0 - 1.0,
        1.0 - p.y / viewport_px.y * 2.0,
    )
}

fn ndc_length_x(px: f32, viewport_px: Vec2) -> f32 {
    px / viewport_px.x * 2.0
}

fn ndc_length_y(px: f32, viewport_px: Vec2) -> f32 {
    px / viewport_px.y * 2.0
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
    use crate::theme::Theme;

    fn theme() -> Theme {
        Theme::light()
    }

    fn plot_rect() -> Rect {
        Rect::new(60.0, 40.0, 380.0, 240.0)
    }

    #[test]
    fn x_axis_lines_emit_axis_plus_one_tick_each() {
        let ticks = vec![
            TickLabel {
                position: 100.0,
                label: "Q1".into(),
            },
            TickLabel {
                position: 200.0,
                label: "Q2".into(),
            },
            TickLabel {
                position: 300.0,
                label: "Q3".into(),
            },
        ];
        let theme = theme();
        let g = emit_x_axis_lines(
            &ticks,
            plot_rect(),
            Vec2::new(480.0, 320.0),
            AxisPosition::Bottom,
            &theme.axis,
            &theme.plot,
            theme.text_primary,
        );
        // 1 axis line + per-tick (1 gridline + 1 tick mark) = 1 + 3*2 = 7.
        assert_eq!(g.primitive_count(), 7);
    }

    #[test]
    fn y_axis_lines_emit_axis_plus_one_tick_each() {
        let ticks = vec![
            TickLabel {
                position: 80.0,
                label: "10".into(),
            },
            TickLabel {
                position: 160.0,
                label: "20".into(),
            },
        ];
        let theme = theme();
        let g = emit_y_axis_lines(
            &ticks,
            plot_rect(),
            Vec2::new(480.0, 320.0),
            AxisPosition::Left,
            &theme.axis,
            &theme.plot,
            theme.text_primary,
        );
        // 1 axis line + per-tick (1 gridline + 1 tick mark) = 1 + 2*2 = 5.
        assert_eq!(g.primitive_count(), 5);
    }

    fn boot_pipeline() -> (
        wisp::application::Application,
        wisp::text::TextTexturePipeline,
    ) {
        let app = pollster::block_on(wisp::application::Application::new(
            wisp::application::AppConfig::default(),
        ))
        .expect("app");
        let pipeline = crate::chart_text::pipeline_with_inter(
            &app,
            wisp::wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        (app, pipeline)
    }

    #[test]
    fn x_axis_text_count_matches_ticks_plus_title() {
        let ticks = vec![
            TickLabel {
                position: 100.0,
                label: "Q1".into(),
            },
            TickLabel {
                position: 200.0,
                label: "Q2".into(),
            },
        ];
        let theme = theme();
        let (app, pipeline) = boot_pipeline();
        let texts = emit_x_axis_text(
            &app,
            &pipeline,
            &ticks,
            plot_rect(),
            Vec2::new(480.0, 320.0),
            AxisPosition::Bottom,
            &theme.axis,
            theme.text_primary,
            Some("Quarter"),
        );
        // 2 tick labels + 1 title.
        assert_eq!(texts.len(), 3);
    }

    #[test]
    fn y_axis_title_has_negative_pi_over_2_rotation() {
        let theme = theme();
        let (app, pipeline) = boot_pipeline();
        let texts = emit_y_axis_text(
            &app,
            &pipeline,
            &[TickLabel {
                position: 100.0,
                label: "10".into(),
            }],
            plot_rect(),
            Vec2::new(480.0, 320.0),
            AxisPosition::Left,
            &theme.axis,
            theme.text_primary,
            Some("Revenue"),
        );
        // Last text is the title; verify its rotation.
        let title = texts.last().expect("title");
        let expected = -std::f32::consts::FRAC_PI_2;
        assert!(
            (title.container.transform.rotation - expected).abs() < 1e-5,
            "expected Y-axis title rotation -π/2, got {}",
            title.container.transform.rotation
        );
    }

    #[test]
    fn axis_with_no_title_omits_title_text() {
        let theme = theme();
        let (app, pipeline) = boot_pipeline();
        let texts = emit_x_axis_text(
            &app,
            &pipeline,
            &[TickLabel {
                position: 100.0,
                label: "Q1".into(),
            }],
            plot_rect(),
            Vec2::new(480.0, 320.0),
            AxisPosition::Bottom,
            &theme.axis,
            theme.text_primary,
            None,
        );
        assert_eq!(texts.len(), 1);
    }
}
