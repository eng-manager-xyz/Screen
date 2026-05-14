//! `Plot` — grammar-of-graphics facade. Build a chart by
//! composing data + a mark + encodings:
//!
//! ```ignore
//! let plot = Plot::new(df)
//!     .mark(Mark::Bar { value_labels: false })
//!     .encode(plot::x("quarter", ScaleKind::Band))
//!     .encode(plot::y("revenue", ScaleKind::Linear))
//!     .encode(plot::color("region"));
//! let graphics = plot.render(&theme, viewport_px);
//! ```
//!
//! v1 ships Bar mark with X (Band) + Y (Linear) + optional Color
//! (Ordinal) encodings — enough to render a single-series or
//! multi-series bar chart at chart-scale resolution. Axes,
//! legend, and additional marks (Line / Area / Point / Cell)
//! ship in follow-on tickets without breaking this surface.

pub mod dataframe;
pub mod encoding;
pub mod mark;

pub use dataframe::{DataFrame, Value};
pub use encoding::{Channel, Encoding, ScaleKind, color, x, y};
pub use mark::{Interpolation, Mark, PointStyle};

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color as WispColor, Fill, Font, Graphics, Text};

use crate::axis::{self, AxisPosition, TickLabel};
use crate::legend::{Legend, LegendOrientation, SwatchStyle};
use crate::scale::{BandScale, LinearScale, OrdinalScale};
use crate::theme::Theme;

/// Top-level grammar-of-graphics facade.
#[derive(Clone, Debug)]
pub struct Plot {
    data: DataFrame,
    mark: Mark,
    encodings: Vec<Encoding>,
    axes_enabled: bool,
    x_axis_title: Option<String>,
    y_axis_title: Option<String>,
}

impl Plot {
    /// Construct from a [`DataFrame`].
    #[must_use]
    pub fn new(data: DataFrame) -> Self {
        Self {
            data,
            mark: Mark::default(),
            encodings: Vec::new(),
            axes_enabled: true,
            x_axis_title: None,
            y_axis_title: None,
        }
    }

    /// Toggle automatic axes (lines + ticks + labels) rendering.
    /// Default `true`. Tests / minimalist renders can opt out.
    #[must_use]
    pub fn axes(mut self, enabled: bool) -> Self {
        self.axes_enabled = enabled;
        self
    }

    /// Build a [`Legend`] for the plot's `Color` encoding (if
    /// any). Returns an empty legend when the chart has no
    /// `Color` channel or when no rows have a colour value.
    ///
    /// Callers integrate the legend into the stage themselves —
    /// it's not baked into `render` because positioning + layout
    /// is application-specific (a card UI may put the legend in
    /// a side rail; a small chart may overlay it).
    #[must_use]
    pub fn legend(&self, theme: &Theme) -> Legend {
        let Some(color_enc) = self.find_encoding(Channel::Color) else {
            return Legend::new();
        };
        let Some(cats) = self.data.distinct_categories(&color_enc.field) else {
            return Legend::new();
        };
        let mut legend = Legend::new();
        for cat in cats {
            let chart_color = theme.palette.color_for(&cat);
            legend = legend.item(cat, SwatchStyle::ColorBox(chart_color));
        }
        legend.orientation(LegendOrientation::Vertical)
    }

    /// Set the X-axis title (rendered below tick labels).
    #[must_use]
    pub fn x_title(mut self, title: impl Into<String>) -> Self {
        self.x_axis_title = Some(title.into());
        self
    }

    /// Set the Y-axis title (rendered to the left of tick
    /// labels, rotated `-π/2`).
    #[must_use]
    pub fn y_title(mut self, title: impl Into<String>) -> Self {
        self.y_axis_title = Some(title.into());
        self
    }

    /// Set the mark type.
    #[must_use]
    pub fn mark(mut self, mark: Mark) -> Self {
        self.mark = mark;
        self
    }

    /// Add an encoding (X, Y, Color). Later encodings on the
    /// same channel replace earlier ones.
    #[must_use]
    pub fn encode(mut self, encoding: Encoding) -> Self {
        self.encodings.retain(|e| e.channel != encoding.channel);
        self.encodings.push(encoding);
        self
    }

    /// Render the plot to a `wisp::Graphics` subtree.
    ///
    /// `viewport_px` is the destination's `(width, height)` in
    /// pixels. The returned `Graphics` has the chart background
    /// as its first primitive, followed by one mark per row.
    /// Axes, legend, and value labels are stubbed in v1; they
    /// land as follow-ons that extend this same `render` path.
    #[must_use]
    pub fn render(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let mut g = Graphics::new();

        // Background fill.
        g.fill(Fill::Solid(chart_to_wisp(theme.bg)));
        g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));

        // Pick the mark renderer.
        match self.mark {
            Mark::Bar { value_labels: _ } => {
                self.render_bars(theme, viewport_px, &mut g);
            }
            Mark::Line {
                interpolation,
                marker,
            } => {
                self.render_lines(theme, viewport_px, interpolation, marker, &mut g);
            }
        }

        g
    }

    fn find_encoding(&self, channel: Channel) -> Option<&Encoding> {
        self.encodings.iter().find(|e| e.channel == channel)
    }

    /// Internal cartesian layout — plot rect + scales + tick
    /// lists used by both `render_bars` and
    /// `axis_text_labels`. Returns `None` when the encodings
    /// don't define a renderable chart (missing X / Y, etc.).
    fn cartesian_layout(&self, theme: &Theme, viewport_px: Vec2) -> Option<CartesianLayout> {
        let _ = theme;
        let x_enc = self.find_encoding(Channel::X)?;
        let y_enc = self.find_encoding(Channel::Y)?;
        let gutter = 60.0;
        let header = 40.0;
        let footer = 40.0;
        let plot_left = gutter;
        let plot_right = viewport_px.x - 20.0;
        let plot_top = header;
        let plot_bottom = viewport_px.y - footer;
        let plot_rect = Rect::new(
            plot_left,
            plot_top,
            plot_right - plot_left,
            plot_bottom - plot_top,
        );

        let categories = self.data.distinct_categories(&x_enc.field)?;
        let x_scale = BandScale::new(categories.clone(), (plot_left, plot_right)).padding(0.15);

        let (y_lo, y_hi) = if let Some(d) = y_enc.domain_override {
            d
        } else if let Some((lo, hi)) = self.data.numeric_extent(&y_enc.field) {
            (lo.min(0.0), hi)
        } else {
            return None;
        };
        let y_scale = LinearScale::new((y_lo, y_hi), (plot_bottom, plot_top));

        // X tick labels: centre of each band.
        let mut x_ticks = Vec::with_capacity(categories.len());
        for (i, cat) in categories.iter().enumerate() {
            if let Some(centre) = x_scale.band_centre(i) {
                x_ticks.push(TickLabel {
                    position: centre,
                    label: cat.clone(),
                });
            }
        }
        // Y tick labels: nice stops from the linear scale.
        let mut y_ticks: Vec<TickLabel> = y_scale
            .ticks(theme.axis.tick_density_hint)
            .into_iter()
            .map(|t| TickLabel {
                position: t.position,
                label: format_tick_value(t.value),
            })
            .collect();
        if y_ticks.is_empty() {
            y_ticks.push(TickLabel {
                position: y_scale.map(y_hi),
                label: format_tick_value(y_hi),
            });
        }

        let y_zero_px = y_scale.map(0.0_f32.max(y_lo));
        Some(CartesianLayout {
            plot_rect,
            x_scale,
            y_scale,
            y_zero_px,
            x_field: x_enc.field.clone(),
            y_field: y_enc.field.clone(),
            x_ticks,
            y_ticks,
        })
    }

    /// Emit axis text labels (and optional titles) using `font`.
    /// `Plot::render` itself can't produce these because Text
    /// nodes need a Font, which wisp-chart doesn't carry. The
    /// caller (typically `wisp-chart-web`) builds a Font once
    /// and feeds it in.
    #[must_use]
    pub fn axis_text_labels(&self, theme: &Theme, viewport_px: Vec2, font: &Font) -> Vec<Text> {
        let mut out = Vec::new();
        if !self.axes_enabled {
            return out;
        }
        let Some(layout) = self.cartesian_layout(theme, viewport_px) else {
            return out;
        };
        out.extend(axis::emit_x_axis_text(
            &layout.x_ticks,
            layout.plot_rect,
            viewport_px,
            AxisPosition::Bottom,
            &theme.axis,
            theme.text_muted,
            self.x_axis_title.as_deref(),
            font,
        ));
        out.extend(axis::emit_y_axis_text(
            &layout.y_ticks,
            layout.plot_rect,
            viewport_px,
            AxisPosition::Left,
            &theme.axis,
            theme.text_muted,
            self.y_axis_title.as_deref(),
            font,
        ));
        out
    }

    fn render_bars(&self, theme: &Theme, viewport_px: Vec2, g: &mut Graphics) {
        let Some(layout) = self.cartesian_layout(theme, viewport_px) else {
            return;
        };

        if self.axes_enabled {
            let x_axis = axis::emit_x_axis_lines(
                &layout.x_ticks,
                layout.plot_rect,
                viewport_px,
                AxisPosition::Bottom,
                &theme.axis,
                &theme.plot,
                theme.text_muted,
            );
            let y_axis = axis::emit_y_axis_lines(
                &layout.y_ticks,
                layout.plot_rect,
                viewport_px,
                AxisPosition::Left,
                &theme.axis,
                &theme.plot,
                theme.text_muted,
            );
            // Splice axes primitives into `g` so the whole plot
            // is one Graphics node.
            g.append(&x_axis);
            g.append(&y_axis);
        }

        let x_scale = layout.x_scale;
        let y_scale = layout.y_scale;
        let y_zero_px = layout.y_zero_px;
        let x_enc_field = layout.x_field;
        let y_enc_field = layout.y_field;

        // Color encoding → palette lookup via OrdinalScale.
        let color_lookup = self
            .find_encoding(Channel::Color)
            .and_then(|enc| self.data.distinct_categories(&enc.field).map(|c| (enc, c)))
            .map(|(enc, cats)| (enc.field.clone(), OrdinalScale::new(cats)));

        // Iterate rows in DataFrame order.
        let x_col = self.data.column(&x_enc_field);
        let y_col = self.data.column(&y_enc_field);
        let color_col = color_lookup
            .as_ref()
            .and_then(|(field, _)| self.data.column(field));
        let row_count = self.data.row_count();

        for i in 0..row_count {
            let Some(x_val) = x_col.and_then(|c| c.get(i)).and_then(Value::as_category) else {
                continue;
            };
            let Some(y_val) = y_col.and_then(|c| c.get(i)).and_then(Value::as_number) else {
                continue;
            };
            let Some((bx0, bx1)) = x_scale.range_for(&x_val.to_owned()) else {
                continue;
            };
            let by_top = y_scale.map(y_val);
            let by0 = by_top.min(y_zero_px);
            let by1 = by_top.max(y_zero_px);

            // Pick fill colour. Color encoding → palette lookup;
            // otherwise theme.palette default for the row's
            // x-category.
            let fill_color = if let Some((_, ord)) = &color_lookup {
                let Some(cat) = color_col
                    .and_then(|c| c.get(i))
                    .and_then(Value::as_category)
                else {
                    continue;
                };
                let _index = ord.index_of(&cat.to_owned()).unwrap_or(0);
                theme.palette.color_for(cat)
            } else {
                theme.palette.color_for(x_val)
            };

            let rect_px = PixelRect {
                x: bx0.min(bx1),
                y: by0,
                w: (bx1 - bx0).abs(),
                h: by1 - by0,
            };
            let ndc = pixel_rect_to_ndc(rect_px, viewport_px);
            let corner_ndc = theme.gantt.bar_corner_radius / viewport_px.y * 2.0;
            g.fill(Fill::Solid(chart_to_wisp(fill_color)));
            g.draw_rounded_rect(ndc, corner_ndc);
        }
    }

    fn render_lines(
        &self,
        theme: &Theme,
        viewport_px: Vec2,
        interpolation: mark::Interpolation,
        marker: Option<mark::PointStyle>,
        g: &mut Graphics,
    ) {
        let Some(layout) = self.cartesian_layout(theme, viewport_px) else {
            return;
        };

        if self.axes_enabled {
            let x_axis = axis::emit_x_axis_lines(
                &layout.x_ticks,
                layout.plot_rect,
                viewport_px,
                AxisPosition::Bottom,
                &theme.axis,
                &theme.plot,
                theme.text_muted,
            );
            let y_axis = axis::emit_y_axis_lines(
                &layout.y_ticks,
                layout.plot_rect,
                viewport_px,
                AxisPosition::Left,
                &theme.axis,
                &theme.plot,
                theme.text_muted,
            );
            g.append(&x_axis);
            g.append(&y_axis);
        }

        let x_scale = layout.x_scale;
        let y_scale = layout.y_scale;
        let x_enc_field = layout.x_field;
        let y_enc_field = layout.y_field;

        // Color encoding → splits the data into series. When
        // absent, the whole DataFrame is one series.
        let color_enc = self.find_encoding(Channel::Color).cloned();
        let x_col = self.data.column(&x_enc_field);
        let y_col = self.data.column(&y_enc_field);
        let row_count = self.data.row_count();

        // Build per-series point lists. Series_key is the colour
        // category (when Color encoding present) or empty string
        // for single-series.
        let mut series: Vec<(String, Vec<(f32, f32)>)> = Vec::new();
        for i in 0..row_count {
            let Some(x_val) = x_col.and_then(|c| c.get(i)).and_then(Value::as_category) else {
                continue;
            };
            let Some(y_val) = y_col.and_then(|c| c.get(i)).and_then(Value::as_number) else {
                continue;
            };
            let Some(bx_centre) = x_scale
                .range_for(&x_val.to_owned())
                .map(|(a, b)| (a + b) * 0.5)
            else {
                continue;
            };
            let py = y_scale.map(y_val);
            let key = match &color_enc {
                Some(enc) => self
                    .data
                    .column(&enc.field)
                    .and_then(|c| c.get(i))
                    .and_then(Value::as_category)
                    .unwrap_or("")
                    .to_owned(),
                None => String::new(),
            };
            match series.iter_mut().find(|(k, _)| k == &key) {
                Some((_, pts)) => pts.push((bx_centre, py)),
                None => series.push((key, vec![(bx_centre, py)])),
            }
        }

        let line_w_ndc = theme.plot.line_width_px / viewport_px.y * 2.0;

        for (key, pts) in &series {
            if pts.is_empty() {
                continue;
            }
            let stroke_color = if color_enc.is_some() && !key.is_empty() {
                theme.palette.color_for(key)
            } else {
                theme.palette.color_for(&y_enc_field)
            };
            g.fill(Fill::Solid(chart_to_wisp(stroke_color)));

            // Segments.
            for pair in pts.windows(2) {
                let (x0, y0) = pair[0];
                let (x1, y1) = pair[1];
                let a = pixel_to_ndc(Vec2::new(x0, y0), viewport_px);
                let b = pixel_to_ndc(Vec2::new(x1, y1), viewport_px);
                match interpolation {
                    mark::Interpolation::Linear => {
                        g.draw_line(a, b, line_w_ndc);
                    }
                    mark::Interpolation::Step => {
                        // Step: horizontal then vertical.
                        let mid = pixel_to_ndc(Vec2::new(x1, y0), viewport_px);
                        g.draw_line(a, mid, line_w_ndc);
                        g.draw_line(mid, b, line_w_ndc);
                    }
                }
            }

            // Markers.
            if matches!(marker, Some(mark::PointStyle::Circle)) {
                let r = theme.plot.line_marker_radius_px;
                let radii_ndc = Vec2::new(r / viewport_px.x * 2.0, r / viewport_px.y * 2.0);
                for (x, y) in pts {
                    let centre = pixel_to_ndc(Vec2::new(*x, *y), viewport_px);
                    g.draw_ellipse(centre, radii_ndc);
                }
            }
        }
    }
}

fn pixel_to_ndc(p: Vec2, viewport_px: Vec2) -> Vec2 {
    Vec2::new(
        p.x / viewport_px.x * 2.0 - 1.0,
        1.0 - p.y / viewport_px.y * 2.0,
    )
}

/// Pixel-space rect used internally by mark renderers.
#[derive(Clone, Copy, Debug)]
struct PixelRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// Internal cartesian-layout cache returned by
/// `Plot::cartesian_layout` and consumed by `render_bars` +
/// `axis_text_labels`.
struct CartesianLayout {
    plot_rect: Rect,
    x_scale: BandScale<String>,
    y_scale: LinearScale,
    y_zero_px: f32,
    x_field: String,
    y_field: String,
    x_ticks: Vec<TickLabel>,
    y_ticks: Vec<TickLabel>,
}

/// Format a numeric tick value for display. Drops trailing zeros
/// so `10.0` renders as `"10"`, `10.5` as `"10.5"`.
fn format_tick_value(v: f32) -> String {
    if (v.fract()).abs() < 1e-6 {
        // Integer-like — format without decimal part. Clamp into
        // f32-representable integer range first; nice-tick stops
        // never exceed magnitudes that would overflow i64, but
        // clippy wants the cast guarded.
        #[allow(
            clippy::cast_possible_truncation,
            reason = "tick values come from LinearScale::ticks which produces nice round numbers; never exceeds f32 integer-precise range"
        )]
        let i = v as i64;
        format!("{i}")
    } else {
        format!("{v:.1}")
    }
}

/// `wisp_chart::Color` → `wisp::Color`. Channels pass through;
/// see `gantt::render::chart_to_wisp` for the sRGB-vs-linear
/// rationale (same trick).
fn chart_to_wisp(c: crate::color::Color) -> WispColor {
    WispColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// Pixel rect (top-left origin, `+Y` down) → NDC `[-1, 1]` rect
/// for `wisp::Graphics`. wisp's renderer puts `+Y` up in NDC, so
/// we flip.
fn pixel_rect_to_ndc(rect: PixelRect, viewport_px: Vec2) -> Rect {
    let x = rect.x / viewport_px.x * 2.0 - 1.0;
    let y = 1.0 - (rect.y + rect.h) / viewport_px.y * 2.0;
    let w = rect.w / viewport_px.x * 2.0;
    let h = rect.h / viewport_px.y * 2.0;
    Rect::new(x, y, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_df() -> DataFrame {
        struct Sale {
            q: &'static str,
            r: f32,
        }
        let rows = vec![
            Sale { q: "Q1", r: 38.0 },
            Sale { q: "Q2", r: 52.0 },
            Sale { q: "Q3", r: 47.0 },
            Sale { q: "Q4", r: 64.0 },
        ];
        DataFrame::from_rows(&rows, |s| {
            vec![
                ("q".into(), Value::Category(s.q.into())),
                ("r".into(), Value::Number(s.r)),
            ]
        })
    }

    #[test]
    fn plot_builds_with_encoding_chain() {
        let plot = Plot::new(fixture_df())
            .mark(Mark::Bar {
                value_labels: false,
            })
            .encode(x("q", ScaleKind::Band))
            .encode(y("r", ScaleKind::Linear));
        assert_eq!(plot.encodings.len(), 2);
    }

    #[test]
    fn duplicate_channel_encoding_replaces_earlier() {
        let plot = Plot::new(fixture_df())
            .encode(y("a", ScaleKind::Linear))
            .encode(y("r", ScaleKind::Linear));
        assert_eq!(plot.encodings.len(), 1);
        assert_eq!(plot.encodings[0].field, "r");
    }

    #[test]
    fn render_emits_background_plus_one_bar_per_row() {
        let plot = Plot::new(fixture_df())
            .axes(false)
            .mark(Mark::Bar {
                value_labels: false,
            })
            .encode(x("q", ScaleKind::Band))
            .encode(y("r", ScaleKind::Linear));
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // 1 bg + 4 bars (axes disabled).
        assert_eq!(g.primitive_count(), 5);
    }

    #[test]
    fn render_with_axes_emits_more_than_just_bars() {
        let plot_no_axes = Plot::new(fixture_df())
            .axes(false)
            .mark(Mark::Bar {
                value_labels: false,
            })
            .encode(x("q", ScaleKind::Band))
            .encode(y("r", ScaleKind::Linear));
        let plot_axes = Plot::new(fixture_df())
            .mark(Mark::Bar {
                value_labels: false,
            })
            .encode(x("q", ScaleKind::Band))
            .encode(y("r", ScaleKind::Linear));
        let g_no_axes = plot_no_axes.render(&Theme::light(), Vec2::new(960.0, 400.0));
        let g_axes = plot_axes.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // Axes must contribute strictly more primitives than the bare bars.
        assert!(
            g_axes.primitive_count() > g_no_axes.primitive_count(),
            "axes-enabled count {} should exceed axes-disabled count {}",
            g_axes.primitive_count(),
            g_no_axes.primitive_count(),
        );
    }

    #[test]
    fn line_mark_emits_one_segment_per_pair_of_points() {
        let plot = Plot::new(fixture_df())
            .axes(false)
            .mark(Mark::Line {
                interpolation: Interpolation::Linear,
                marker: None,
            })
            .encode(x("q", ScaleKind::Band))
            .encode(y("r", ScaleKind::Linear));
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // 1 bg + 3 segments (4 points → 3 segments).
        assert_eq!(g.primitive_count(), 4);
    }

    #[test]
    fn line_step_interpolation_doubles_segment_count() {
        let plot = Plot::new(fixture_df())
            .axes(false)
            .mark(Mark::Line {
                interpolation: Interpolation::Step,
                marker: None,
            })
            .encode(x("q", ScaleKind::Band))
            .encode(y("r", ScaleKind::Linear));
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // 1 bg + 6 segments (4 points → 3 pairs × 2 segments each
        // for step = horizontal + vertical).
        assert_eq!(g.primitive_count(), 7);
    }

    #[test]
    fn line_with_circle_markers_adds_one_ellipse_per_point() {
        let plot = Plot::new(fixture_df())
            .axes(false)
            .mark(Mark::Line {
                interpolation: Interpolation::Linear,
                marker: Some(PointStyle::Circle),
            })
            .encode(x("q", ScaleKind::Band))
            .encode(y("r", ScaleKind::Linear));
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // 1 bg + 3 segments + 4 markers.
        assert_eq!(g.primitive_count(), 8);
    }

    #[test]
    fn render_without_required_encodings_emits_only_background() {
        let plot = Plot::new(fixture_df()).mark(Mark::Bar {
            value_labels: false,
        });
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // Just the background — no X / Y encoding so no bars.
        assert_eq!(g.primitive_count(), 1);
    }

    #[test]
    fn ndc_conversion_handles_full_viewport_rect() {
        let r = pixel_rect_to_ndc(
            PixelRect {
                x: 0.0,
                y: 0.0,
                w: 1920.0,
                h: 800.0,
            },
            Vec2::new(1920.0, 800.0),
        );
        assert!((r.min.x - -1.0).abs() < 1e-6);
        assert!((r.min.y - -1.0).abs() < 1e-6);
        assert!((r.max().x - 1.0).abs() < 1e-6);
        assert!((r.max().y - 1.0).abs() < 1e-6);
    }
}
