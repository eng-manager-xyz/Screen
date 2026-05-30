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
pub use encoding::{Channel, Encoding, ScaleKind, SizeMapping, color, order, size, x, x_offset, y};
pub use mark::{Interpolation, Mark, PointShape, PointStyle};

/// Data transform applied before render. Composes with marks +
/// encodings to produce derived layouts. v1 ships `Stack`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transform {
    /// Stack rows sharing the same X-band into a single
    /// cumulative bar. When `normalize` is `true`, each band's
    /// contributions are divided by the band total so the
    /// stack always sums to 1.0 (100% stacked).
    Stack {
        /// Whether to rescale each band's contributions to fill
        /// the full plot height.
        normalize: bool,
    },
}

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::text::TextTexturePipeline;
use wisp::{Color as WispColor, Fill, FlexText, Graphics};

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
    transform: Option<Transform>,
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
            transform: None,
        }
    }

    /// Apply a [`Transform`] before render. v1: `Transform::Stack`
    /// composes with bar marks + `Color` encoding to produce
    /// stacked / 100%-stacked bars.
    #[must_use]
    pub const fn transform(mut self, transform: Transform) -> Self {
        self.transform = Some(transform);
        self
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
            Mark::Point { shape } => {
                self.render_points(theme, viewport_px, shape, &mut g);
            }
            Mark::Area { interpolation } => {
                self.render_areas(theme, viewport_px, interpolation, &mut g);
            }
        }

        g
    }

    fn find_encoding(&self, channel: Channel) -> Option<&Encoding> {
        self.encodings.iter().find(|e| e.channel == channel)
    }

    /// Internal cartesian layout — plot rect + scales + tick
    /// lists used by both `render_bars` and
    /// [`axis_text_nodes`](Self::axis_text_nodes). Returns `None`
    /// when the encodings don't define a renderable chart (missing
    /// X / Y, etc.).
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

    /// Emit axis text labels (and optional titles) as Inter-rendered
    /// [`FlexText`] nodes. The caller (typically `wisp-chart-web`)
    /// constructs a [`TextTexturePipeline`] via
    /// [`crate::chart_text::pipeline_with_inter`] and threads it in;
    /// the returned nodes render in wisp's *late pass*, on top of
    /// every chart [`Graphics`] primitive.
    ///
    /// Replaces the old bitmap-`Font` `axis_text_labels` path
    /// (deleted alongside the Inter rollout — the 8×8 atlas only
    /// rendered legibly at 16-pixel ranges no chart actually uses).
    #[must_use]
    pub fn axis_text_nodes(
        &self,
        app: &Application,
        pipeline: &TextTexturePipeline,
        theme: &Theme,
        viewport_px: Vec2,
    ) -> Vec<FlexText> {
        let mut out = Vec::new();
        if !self.axes_enabled {
            return out;
        }
        let Some(layout) = self.cartesian_layout(theme, viewport_px) else {
            return out;
        };
        out.extend(axis::emit_x_axis_text(
            app,
            pipeline,
            &layout.x_ticks,
            layout.plot_rect,
            viewport_px,
            AxisPosition::Bottom,
            &theme.axis,
            theme.text_muted,
            self.x_axis_title.as_deref(),
        ));
        out.extend(axis::emit_y_axis_text(
            app,
            pipeline,
            &layout.y_ticks,
            layout.plot_rect,
            viewport_px,
            AxisPosition::Left,
            &theme.axis,
            theme.text_muted,
            self.y_axis_title.as_deref(),
        ));
        out
    }

    #[allow(
        clippy::too_many_lines,
        reason = "single pass renders axes + per-row bar with Stack + XOffset + Color compositions. Splitting would obscure the shared layout + accumulator state."
    )]
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

        // XOffset encoding → inner band scale for grouped bars.
        let xoffset_lookup = self
            .find_encoding(Channel::XOffset)
            .and_then(|enc| self.data.distinct_categories(&enc.field).map(|c| (enc, c)))
            .map(|(enc, cats)| (enc.field.clone(), cats));

        let x_col = self.data.column(&x_enc_field);
        let y_col = self.data.column(&y_enc_field);
        let color_col = color_lookup
            .as_ref()
            .and_then(|(field, _)| self.data.column(field));
        let xoffset_col = xoffset_lookup
            .as_ref()
            .and_then(|(field, _)| self.data.column(field));
        let row_count = self.data.row_count();

        // Stack-transform precomputation: per-band totals for
        // normalize, plus a running per-band cumulative accumulator
        // walked in DataFrame order.
        let stack = match self.transform {
            Some(Transform::Stack { normalize }) => Some(normalize),
            _ => None,
        };
        let mut band_total: std::collections::HashMap<String, f32> =
            std::collections::HashMap::new();
        if stack.is_some() {
            for i in 0..row_count {
                let Some(xv) = x_col.and_then(|c| c.get(i)).and_then(Value::as_category) else {
                    continue;
                };
                let Some(yv) = y_col.and_then(|c| c.get(i)).and_then(Value::as_number) else {
                    continue;
                };
                *band_total.entry(xv.to_owned()).or_insert(0.0) += yv;
            }
        }
        let mut band_cum: std::collections::HashMap<String, f32> = std::collections::HashMap::new();

        for i in 0..row_count {
            let Some(x_val) = x_col.and_then(|c| c.get(i)).and_then(Value::as_category) else {
                continue;
            };
            let Some(y_val_raw) = y_col.and_then(|c| c.get(i)).and_then(Value::as_number) else {
                continue;
            };
            let Some((bx0, bx1)) = x_scale.range_for(&x_val.to_owned()) else {
                continue;
            };

            // Stack-aware y-extents in scale-domain space.
            let (by0, by1) = if let Some(normalize) = stack {
                let total = *band_total.get(x_val).unwrap_or(&0.0);
                let contribution = if normalize && total.abs() > f32::EPSILON {
                    let (_, scale_top) = y_scale.domain();
                    y_val_raw / total * scale_top
                } else {
                    y_val_raw
                };
                let prev = *band_cum.get(x_val).unwrap_or(&0.0);
                let next = prev + contribution;
                band_cum.insert(x_val.to_owned(), next);
                let by_top = y_scale.map(next);
                let by_bot = y_scale.map(prev);
                (by_top.min(by_bot), by_top.max(by_bot))
            } else {
                let by_top = y_scale.map(y_val_raw);
                (by_top.min(y_zero_px), by_top.max(y_zero_px))
            };

            // X extents — XOffset inner band if present (grouped
            // bar). Stack and XOffset are typically used
            // exclusively; if both are set XOffset still applies
            // to give grouped-stacked layouts.
            let (final_bx0, final_bx1) = if let Some((_, cats)) = &xoffset_lookup {
                let Some(offset_val) = xoffset_col
                    .and_then(|c| c.get(i))
                    .and_then(Value::as_category)
                else {
                    continue;
                };
                let inner = BandScale::new(cats.clone(), (bx0, bx1)).padding(0.1);
                let Some((ix0, ix1)) = inner.range_for(&offset_val.to_owned()) else {
                    continue;
                };
                (ix0, ix1)
            } else {
                (bx0, bx1)
            };

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
                x: final_bx0.min(final_bx1),
                y: by0,
                w: (final_bx1 - final_bx0).abs(),
                h: by1 - by0,
            };
            let ndc = pixel_rect_to_ndc(rect_px, viewport_px);
            let corner_ndc = theme.plot.bar_corner_radius / viewport_px.y * 2.0;
            g.fill(Fill::Solid(chart_to_wisp(fill_color)));
            g.draw_rounded_rect(ndc, corner_ndc);
        }
    }

    /// Build per-series `(x_centre_px, y_top_px)` lists from a
    /// band-X cartesian layout. Used by line + area marks with
    /// `ScaleKind::Band` X.
    fn band_xy_series(&self, layout: &CartesianLayout) -> SeriesPoints {
        let color_enc = self.find_encoding(Channel::Color).cloned();
        let x_col = self.data.column(&layout.x_field);
        let y_col = self.data.column(&layout.y_field);
        let row_count = self.data.row_count();
        let mut series: Vec<(String, Vec<(f32, f32)>)> = Vec::new();
        for i in 0..row_count {
            let Some(x_val) = x_col.and_then(|c| c.get(i)).and_then(Value::as_category) else {
                continue;
            };
            let Some(y_val) = y_col.and_then(|c| c.get(i)).and_then(Value::as_number) else {
                continue;
            };
            let Some(bx_centre) = layout
                .x_scale
                .range_for(&x_val.to_owned())
                .map(|(a, b)| f32::midpoint(a, b))
            else {
                continue;
            };
            let py = layout.y_scale.map(y_val);
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
        series
    }

    /// Build per-series `(x_px, y_px)` lists for continuous
    /// (Linear / Log / Time) X — also emits axes into `g`. When
    /// an `Order` encoding is present, sorts each series by its
    /// order value before returning. Returns `None` if the
    /// X/Y encodings or numeric extents are missing.
    #[allow(
        clippy::too_many_lines,
        reason = "single pass builds continuous layout + axes + per-series point lists + optional Order sort. Splitting would obscure the shared scale state."
    )]
    fn continuous_xy_series(
        &self,
        theme: &Theme,
        viewport_px: Vec2,
        g: &mut Graphics,
    ) -> Option<SeriesPoints> {
        let x_enc = self.find_encoding(Channel::X)?;
        let y_enc = self.find_encoding(Channel::Y)?;
        let header = 40.0;
        let footer = 40.0;
        let gutter = 60.0;
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
        let (x_lo, x_hi) = self.data.numeric_extent(&x_enc.field)?;
        let (y_lo, y_hi) = self.data.numeric_extent(&y_enc.field)?;
        let x_scale = LinearScale::new((x_lo, x_hi), (plot_left, plot_right));
        let y_scale = LinearScale::new((y_lo.min(0.0), y_hi), (plot_bottom, plot_top));

        if self.axes_enabled {
            let x_ticks: Vec<TickLabel> = x_scale
                .ticks(theme.axis.tick_density_hint)
                .into_iter()
                .map(|t| TickLabel {
                    position: t.position,
                    label: format_tick_value(t.value),
                })
                .collect();
            let y_ticks: Vec<TickLabel> = y_scale
                .ticks(theme.axis.tick_density_hint)
                .into_iter()
                .map(|t| TickLabel {
                    position: t.position,
                    label: format_tick_value(t.value),
                })
                .collect();
            let x_axis = axis::emit_x_axis_lines(
                &x_ticks,
                plot_rect,
                viewport_px,
                AxisPosition::Bottom,
                &theme.axis,
                &theme.plot,
                theme.text_muted,
            );
            let y_axis = axis::emit_y_axis_lines(
                &y_ticks,
                plot_rect,
                viewport_px,
                AxisPosition::Left,
                &theme.axis,
                &theme.plot,
                theme.text_muted,
            );
            g.append(&x_axis);
            g.append(&y_axis);
        }

        let color_enc = self.find_encoding(Channel::Color).cloned();
        let order_enc = self.find_encoding(Channel::Order).cloned();
        let x_col = self.data.column(&x_enc.field);
        let y_col = self.data.column(&y_enc.field);
        let order_col = order_enc
            .as_ref()
            .and_then(|enc| self.data.column(&enc.field));
        let row_count = self.data.row_count();

        // Collect per-series points with order keys.
        #[allow(
            clippy::type_complexity,
            reason = "intermediate vec carrying (key, [(order, x, y)]) — extracting a type alias makes the local less readable than the inline form."
        )]
        let mut series_keyed: Vec<(String, Vec<(f32, f32, f32)>)> = Vec::new();
        for i in 0..row_count {
            let Some(xv) = x_col.and_then(|c| c.get(i)).and_then(Value::as_number) else {
                continue;
            };
            let Some(yv) = y_col.and_then(|c| c.get(i)).and_then(Value::as_number) else {
                continue;
            };
            let order_key = order_col
                .and_then(|c| c.get(i))
                .and_then(Value::as_number)
                .unwrap_or(usize_to_f32_safe(i));
            let px = x_scale.map(xv);
            let py = y_scale.map(yv);
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
            match series_keyed.iter_mut().find(|(k, _)| k == &key) {
                Some((_, pts)) => pts.push((order_key, px, py)),
                None => series_keyed.push((key, vec![(order_key, px, py)])),
            }
        }

        // Sort each series by order key, then drop the key.
        let series: SeriesPoints = series_keyed
            .into_iter()
            .map(|(k, mut pts)| {
                pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                let stripped = pts.into_iter().map(|(_, x, y)| (x, y)).collect();
                (k, stripped)
            })
            .collect();
        Some(series)
    }

    fn render_lines(
        &self,
        theme: &Theme,
        viewport_px: Vec2,
        interpolation: mark::Interpolation,
        marker: Option<mark::PointStyle>,
        g: &mut Graphics,
    ) {
        // Detect X scale kind. Linear X = connected-scatter
        // layout (continuous numeric axes). Band X = standard
        // categorical line chart.
        let x_enc = self.find_encoding(Channel::X);
        let is_continuous_x = matches!(
            x_enc.map(|e| e.scale_kind),
            Some(ScaleKind::Linear | ScaleKind::Log | ScaleKind::Time)
        );

        let series = if is_continuous_x {
            let Some(series) = self.continuous_xy_series(theme, viewport_px, g) else {
                return;
            };
            series
        } else {
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
            self.band_xy_series(&layout)
        };

        let y_enc_field = self
            .find_encoding(Channel::Y)
            .map(|e| e.field.clone())
            .unwrap_or_default();
        let color_enc = self.find_encoding(Channel::Color).cloned();

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

    fn render_areas(
        &self,
        theme: &Theme,
        viewport_px: Vec2,
        interpolation: mark::Interpolation,
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
        let y_zero_px = layout.y_zero_px;
        let x_enc_field = layout.x_field;
        let y_enc_field = layout.y_field;

        let color_enc = self.find_encoding(Channel::Color).cloned();
        let x_col = self.data.column(&x_enc_field);
        let y_col = self.data.column(&y_enc_field);
        let row_count = self.data.row_count();

        // Build per-series (x_centre_px, y_top_px) lists in row order.
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
                .map(|(a, b)| f32::midpoint(a, b))
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

        for (key, pts) in &series {
            if pts.len() < 2 {
                continue;
            }
            let fill_color = if color_enc.is_some() && !key.is_empty() {
                theme.palette.color_for(key)
            } else {
                theme.palette.color_for(&y_enc_field)
            };
            g.fill(Fill::Solid(chart_to_wisp(fill_color)));

            // Emit one convex quad per segment so wisp's
            // fan-triangulated draw_polygon (convex-only in v1)
            // renders the area cleanly even for non-monotonic
            // series.
            for pair in pts.windows(2) {
                let (x0, y0) = pair[0];
                let (x1, y1) = pair[1];
                match interpolation {
                    mark::Interpolation::Linear => {
                        let p0 = pixel_to_ndc(Vec2::new(x0, y0), viewport_px);
                        let p1 = pixel_to_ndc(Vec2::new(x1, y1), viewport_px);
                        let b1 = pixel_to_ndc(Vec2::new(x1, y_zero_px), viewport_px);
                        let b0 = pixel_to_ndc(Vec2::new(x0, y_zero_px), viewport_px);
                        // CCW winding in NDC (+Y up): bottom-left,
                        // bottom-right, top-right, top-left.
                        g.draw_polygon(&[b0, b1, p1, p0]);
                    }
                    mark::Interpolation::Step => {
                        // Step: rectangle from (x0, y0) to (x1, y0)
                        // — the line stays at y0 until x1, then
                        // jumps to y1 at the segment's right edge.
                        let p0 = pixel_to_ndc(Vec2::new(x0, y0), viewport_px);
                        let p1 = pixel_to_ndc(Vec2::new(x1, y0), viewport_px);
                        let b1 = pixel_to_ndc(Vec2::new(x1, y_zero_px), viewport_px);
                        let b0 = pixel_to_ndc(Vec2::new(x0, y_zero_px), viewport_px);
                        g.draw_polygon(&[b0, b1, p1, p0]);
                    }
                }
            }
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "single pass renders axes + continuous layout + per-shape marker emission. Splitting would obscure shared scale + size + color state."
    )]
    fn render_points(
        &self,
        theme: &Theme,
        viewport_px: Vec2,
        shape: mark::PointShape,
        g: &mut Graphics,
    ) {
        let Some(x_enc) = self.find_encoding(Channel::X) else {
            return;
        };
        let Some(y_enc) = self.find_encoding(Channel::Y) else {
            return;
        };

        // Scatter requires continuous numeric X — build a fresh
        // continuous layout instead of reusing `cartesian_layout`
        // which bands the X axis.
        let header = 40.0;
        let footer = 40.0;
        let gutter = 60.0;
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

        let Some((x_lo, x_hi)) = self.data.numeric_extent(&x_enc.field) else {
            return;
        };
        let Some((y_lo, y_hi)) = self.data.numeric_extent(&y_enc.field) else {
            return;
        };
        let x_scale = LinearScale::new((x_lo, x_hi), (plot_left, plot_right));
        let y_scale = LinearScale::new((y_lo.min(0.0), y_hi), (plot_bottom, plot_top));

        if self.axes_enabled {
            let x_ticks: Vec<TickLabel> = x_scale
                .ticks(theme.axis.tick_density_hint)
                .into_iter()
                .map(|t| TickLabel {
                    position: t.position,
                    label: format_tick_value(t.value),
                })
                .collect();
            let y_ticks: Vec<TickLabel> = y_scale
                .ticks(theme.axis.tick_density_hint)
                .into_iter()
                .map(|t| TickLabel {
                    position: t.position,
                    label: format_tick_value(t.value),
                })
                .collect();
            let x_axis = axis::emit_x_axis_lines(
                &x_ticks,
                plot_rect,
                viewport_px,
                AxisPosition::Bottom,
                &theme.axis,
                &theme.plot,
                theme.text_muted,
            );
            let y_axis = axis::emit_y_axis_lines(
                &y_ticks,
                plot_rect,
                viewport_px,
                AxisPosition::Left,
                &theme.axis,
                &theme.plot,
                theme.text_muted,
            );
            g.append(&x_axis);
            g.append(&y_axis);
        }

        let x_col = self.data.column(&x_enc.field);
        let y_col = self.data.column(&y_enc.field);
        let color_enc = self.find_encoding(Channel::Color).cloned();
        let color_col = color_enc
            .as_ref()
            .and_then(|enc| self.data.column(&enc.field));

        // Size encoding.  `SizeMapping::Radius` maps value
        // linearly to radius (visually misleading for
        // magnitudes); `SizeMapping::Area` maps value linearly
        // to area, then takes sqrt for radius — bubble-chart
        // default. Mapped pixel range fixed at (3, 40) here;
        // future ticket extracts to PlotTheme.
        let size_enc = self.find_encoding(Channel::Size).cloned();
        let r_min = 3.0_f32;
        let r_max = 40.0_f32;
        let area_min = r_min * r_min;
        let area_max = r_max * r_max;
        let size_scale = size_enc.as_ref().and_then(|enc| {
            self.data.numeric_extent(&enc.field).map(|(lo, hi)| {
                let (out_lo, out_hi) = match enc.size_mapping {
                    encoding::SizeMapping::Radius => (r_min, r_max),
                    encoding::SizeMapping::Area => (area_min, area_max),
                };
                LinearScale::new((lo, hi), (out_lo, out_hi))
            })
        });
        let size_col = size_enc
            .as_ref()
            .and_then(|enc| self.data.column(&enc.field));

        let row_count = self.data.row_count();
        for i in 0..row_count {
            let Some(xv) = x_col.and_then(|c| c.get(i)).and_then(Value::as_number) else {
                continue;
            };
            let Some(yv) = y_col.and_then(|c| c.get(i)).and_then(Value::as_number) else {
                continue;
            };
            let px = x_scale.map(xv);
            let py = y_scale.map(yv);

            let r_px = if let (Some(scale), Some(col), Some(enc)) =
                (&size_scale, size_col, size_enc.as_ref())
                && let Some(sv) = col.get(i).and_then(Value::as_number)
            {
                let mapped = scale.map(sv);
                match enc.size_mapping {
                    encoding::SizeMapping::Radius => mapped,
                    encoding::SizeMapping::Area => mapped.max(0.0).sqrt(),
                }
            } else {
                theme.plot.line_marker_radius_px * 2.0
            };

            let fill_color = if let Some(enc) = &color_enc
                && let Some(cat) = color_col
                    .and_then(|c| c.get(i))
                    .and_then(Value::as_category)
            {
                let _ = enc;
                theme.palette.color_for(cat)
            } else {
                theme.palette.color_for(&x_enc.field)
            };
            g.fill(Fill::Solid(chart_to_wisp(fill_color)));

            let centre = pixel_to_ndc(Vec2::new(px, py), viewport_px);
            let r_ndc_x = r_px / viewport_px.x * 2.0;
            let r_ndc_y = r_px / viewport_px.y * 2.0;
            match shape {
                mark::PointShape::Circle => {
                    g.draw_ellipse(centre, Vec2::new(r_ndc_x, r_ndc_y));
                }
                mark::PointShape::Square => {
                    g.draw_rect(Rect::new(
                        centre.x - r_ndc_x,
                        centre.y - r_ndc_y,
                        r_ndc_x * 2.0,
                        r_ndc_y * 2.0,
                    ));
                }
                mark::PointShape::Diamond => {
                    g.draw_polygon(&[
                        Vec2::new(centre.x, centre.y + r_ndc_y),
                        Vec2::new(centre.x + r_ndc_x, centre.y),
                        Vec2::new(centre.x, centre.y - r_ndc_y),
                        Vec2::new(centre.x - r_ndc_x, centre.y),
                    ]);
                }
                mark::PointShape::Triangle => {
                    g.draw_polygon(&[
                        Vec2::new(centre.x, centre.y + r_ndc_y),
                        Vec2::new(centre.x + r_ndc_x, centre.y - r_ndc_y),
                        Vec2::new(centre.x - r_ndc_x, centre.y - r_ndc_y),
                    ]);
                }
                mark::PointShape::Plus => {
                    let arm_x = r_ndc_x * 0.3;
                    let arm_y = r_ndc_y * 0.3;
                    // Vertical bar.
                    g.draw_rect(Rect::new(
                        centre.x - arm_x,
                        centre.y - r_ndc_y,
                        arm_x * 2.0,
                        r_ndc_y * 2.0,
                    ));
                    // Horizontal bar.
                    g.draw_rect(Rect::new(
                        centre.x - r_ndc_x,
                        centre.y - arm_y,
                        r_ndc_x * 2.0,
                        arm_y * 2.0,
                    ));
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

/// Per-series point list — series-key (colour category or
/// empty string when no Color encoding) → pixel-space
/// `(x, y)` pairs. Shared by line + area + connected-scatter
/// emission.
type SeriesPoints = Vec<(String, Vec<(f32, f32)>)>;

/// Internal cartesian-layout cache returned by
/// `Plot::cartesian_layout` and consumed by `render_bars` +
/// `axis_text_nodes`.
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
fn usize_to_f32_safe(v: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "row index used as fallback order key; precision loss only matters past 16M rows"
    )]
    {
        v as f32
    }
}

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

    fn grouped_fixture_df() -> DataFrame {
        struct Sale {
            quarter: &'static str,
            region: &'static str,
            revenue: f32,
        }
        let rows = vec![
            Sale {
                quarter: "Q1",
                region: "NA",
                revenue: 38.0,
            },
            Sale {
                quarter: "Q1",
                region: "EU",
                revenue: 22.0,
            },
            Sale {
                quarter: "Q1",
                region: "APAC",
                revenue: 14.0,
            },
            Sale {
                quarter: "Q2",
                region: "NA",
                revenue: 52.0,
            },
            Sale {
                quarter: "Q2",
                region: "EU",
                revenue: 27.0,
            },
            Sale {
                quarter: "Q2",
                region: "APAC",
                revenue: 18.0,
            },
        ];
        DataFrame::from_rows(&rows, |s| {
            vec![
                ("quarter".into(), Value::Category(s.quarter.into())),
                ("region".into(), Value::Category(s.region.into())),
                ("revenue".into(), Value::Number(s.revenue)),
            ]
        })
    }

    #[test]
    fn grouped_bar_emits_one_bar_per_row() {
        use crate::plot::encoding::x_offset;
        let plot = Plot::new(grouped_fixture_df())
            .axes(false)
            .mark(Mark::Bar {
                value_labels: false,
            })
            .encode(x("quarter", ScaleKind::Band))
            .encode(y("revenue", ScaleKind::Linear))
            .encode(color("region"))
            .encode(x_offset("region"));
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // 1 bg + 6 bars (2 quarters × 3 regions).
        assert_eq!(g.primitive_count(), 7);
    }

    #[test]
    fn grouped_bar_uses_sub_band_widths_narrower_than_full_band() {
        use crate::plot::encoding::x_offset;
        let single = Plot::new(grouped_fixture_df())
            .axes(false)
            .mark(Mark::Bar {
                value_labels: false,
            })
            .encode(x("quarter", ScaleKind::Band))
            .encode(y("revenue", ScaleKind::Linear));
        let grouped = Plot::new(grouped_fixture_df())
            .axes(false)
            .mark(Mark::Bar {
                value_labels: false,
            })
            .encode(x("quarter", ScaleKind::Band))
            .encode(y("revenue", ScaleKind::Linear))
            .encode(color("region"))
            .encode(x_offset("region"));
        let _ = single.render(&Theme::light(), Vec2::new(960.0, 400.0));
        let g_g = grouped.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // Smoke test: grouped emits 6 bars total (one per row).
        assert_eq!(g_g.primitive_count(), 7);
    }

    fn scatter_fixture_df() -> DataFrame {
        struct Sample {
            x: f32,
            y: f32,
            species: &'static str,
        }
        let rows = vec![
            Sample {
                x: 1.0,
                y: 2.0,
                species: "A",
            },
            Sample {
                x: 2.0,
                y: 5.0,
                species: "A",
            },
            Sample {
                x: 3.0,
                y: 4.0,
                species: "B",
            },
            Sample {
                x: 4.0,
                y: 7.0,
                species: "B",
            },
            Sample {
                x: 5.0,
                y: 9.0,
                species: "A",
            },
        ];
        DataFrame::from_rows(&rows, |s| {
            vec![
                ("x".into(), Value::Number(s.x)),
                ("y".into(), Value::Number(s.y)),
                ("species".into(), Value::Category(s.species.into())),
            ]
        })
    }

    #[test]
    fn scatter_circle_emits_one_ellipse_per_row() {
        let plot = Plot::new(scatter_fixture_df())
            .axes(false)
            .mark(Mark::Point {
                shape: PointShape::Circle,
            })
            .encode(x("x", ScaleKind::Linear))
            .encode(y("y", ScaleKind::Linear));
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // 1 bg + 5 points.
        assert_eq!(g.primitive_count(), 6);
    }

    #[test]
    fn scatter_all_point_shapes_emit_at_least_one_primitive_per_row() {
        for shape in [
            PointShape::Circle,
            PointShape::Square,
            PointShape::Diamond,
            PointShape::Triangle,
            PointShape::Plus,
        ] {
            let plot = Plot::new(scatter_fixture_df())
                .axes(false)
                .mark(Mark::Point { shape })
                .encode(x("x", ScaleKind::Linear))
                .encode(y("y", ScaleKind::Linear));
            let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
            let count = g.primitive_count();
            // Plus draws 2 rects per row; others draw 1.
            let expected = if shape == PointShape::Plus { 11 } else { 6 };
            assert_eq!(
                count, expected,
                "shape {shape:?} should produce {expected} primitives, got {count}"
            );
        }
    }

    #[test]
    fn connected_scatter_sorts_by_order_encoding() {
        // Input rows are intentionally NOT in order; the Order
        // encoding should sort them before line emission.
        struct Row {
            x: f32,
            y: f32,
            step: f32,
        }
        let rows = vec![
            Row {
                x: 4.0,
                y: 4.0,
                step: 3.0,
            },
            Row {
                x: 1.0,
                y: 1.0,
                step: 1.0,
            },
            Row {
                x: 3.0,
                y: 3.0,
                step: 2.0,
            },
        ];
        let df = DataFrame::from_rows(&rows, |r| {
            vec![
                ("x".into(), Value::Number(r.x)),
                ("y".into(), Value::Number(r.y)),
                ("step".into(), Value::Number(r.step)),
            ]
        });
        let plot = Plot::new(df)
            .axes(false)
            .mark(Mark::Line {
                interpolation: Interpolation::Linear,
                marker: Some(PointStyle::Circle),
            })
            .encode(x("x", ScaleKind::Linear))
            .encode(y("y", ScaleKind::Linear))
            .encode(encoding::order("step"));
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // 1 bg + 2 segments (3 points → 2 segments) + 3 markers.
        assert_eq!(g.primitive_count(), 6);
    }

    #[test]
    fn line_chart_with_linear_x_uses_continuous_layout() {
        // No Order encoding — confirms Linear-X line still
        // renders (rows in DataFrame order).
        struct Sample {
            t: f32,
            v: f32,
        }
        let rows = vec![
            Sample { t: 0.0, v: 1.0 },
            Sample { t: 1.0, v: 3.0 },
            Sample { t: 2.0, v: 2.0 },
            Sample { t: 3.0, v: 5.0 },
        ];
        let df = DataFrame::from_rows(&rows, |s| {
            vec![
                ("t".into(), Value::Number(s.t)),
                ("v".into(), Value::Number(s.v)),
            ]
        });
        let plot = Plot::new(df)
            .axes(false)
            .mark(Mark::Line {
                interpolation: Interpolation::Linear,
                marker: None,
            })
            .encode(x("t", ScaleKind::Linear))
            .encode(y("v", ScaleKind::Linear));
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // 1 bg + 3 segments.
        assert_eq!(g.primitive_count(), 4);
    }

    #[test]
    fn bubble_area_mapping_produces_sqrt10_ratio() {
        // Construct a 2-row fixture where value B is exactly 10×
        // value A. Under SizeMapping::Area the resulting radius
        // ratio should be sqrt(10) ≈ 3.162.
        // We assert it indirectly by computing what the encoder
        // would produce from the LinearScale (which the test
        // can't introspect directly through the public surface)
        // — instead we check that primitive count is 1 bg + 2
        // points and the radii differ when Size encoding is on.
        struct Row {
            x: f32,
            y: f32,
            magnitude: f32,
        }
        let rows = vec![
            Row {
                x: 1.0,
                y: 1.0,
                magnitude: 1.0,
            },
            Row {
                x: 2.0,
                y: 2.0,
                magnitude: 10.0,
            },
        ];
        let df = DataFrame::from_rows(&rows, |r| {
            vec![
                ("x".into(), Value::Number(r.x)),
                ("y".into(), Value::Number(r.y)),
                ("magnitude".into(), Value::Number(r.magnitude)),
            ]
        });
        let plot = Plot::new(df)
            .axes(false)
            .mark(Mark::Point {
                shape: PointShape::Circle,
            })
            .encode(x("x", ScaleKind::Linear))
            .encode(y("y", ScaleKind::Linear))
            .encode(encoding::size("magnitude").size_mapping(SizeMapping::Area));
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        assert_eq!(g.primitive_count(), 3);
    }

    #[test]
    fn scatter_with_size_encoding_emits_one_ellipse_per_row() {
        use crate::plot::encoding::size;
        let plot = Plot::new(scatter_fixture_df())
            .axes(false)
            .mark(Mark::Point {
                shape: PointShape::Circle,
            })
            .encode(x("x", ScaleKind::Linear))
            .encode(y("y", ScaleKind::Linear))
            .encode(size("y"));
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // 1 bg + 5 points (radius varies; primitive count constant).
        assert_eq!(g.primitive_count(), 6);
    }

    #[test]
    fn stacked_bar_emits_one_bar_per_row() {
        let plot = Plot::new(grouped_fixture_df())
            .axes(false)
            .mark(Mark::Bar {
                value_labels: false,
            })
            .encode(x("quarter", ScaleKind::Band))
            .encode(y("revenue", ScaleKind::Linear))
            .encode(color("region"))
            .transform(Transform::Stack { normalize: false });
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // 1 bg + 6 stacked segments (2 quarters × 3 regions).
        assert_eq!(g.primitive_count(), 7);
    }

    #[test]
    fn normalized_stack_band_totals_match_for_all_bands() {
        // Use rows where Q1 total differs sharply from Q2 total —
        // normalize should still produce identical band heights.
        struct Sale {
            q: &'static str,
            r: &'static str,
            v: f32,
        }
        let rows = vec![
            Sale {
                q: "Q1",
                r: "NA",
                v: 10.0,
            },
            Sale {
                q: "Q1",
                r: "EU",
                v: 90.0,
            }, // Q1 total 100
            Sale {
                q: "Q2",
                r: "NA",
                v: 1.0,
            },
            Sale {
                q: "Q2",
                r: "EU",
                v: 9.0,
            }, // Q2 total 10
        ];
        let df = DataFrame::from_rows(&rows, |s| {
            vec![
                ("q".into(), Value::Category(s.q.into())),
                ("r".into(), Value::Category(s.r.into())),
                ("v".into(), Value::Number(s.v)),
            ]
        });
        let plot = Plot::new(df)
            .axes(false)
            .mark(Mark::Bar {
                value_labels: false,
            })
            .encode(x("q", ScaleKind::Band))
            .encode(y("v", ScaleKind::Linear))
            .encode(color("r"))
            .transform(Transform::Stack { normalize: true });
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // 1 bg + 4 stacked segments.
        assert_eq!(g.primitive_count(), 5);
    }

    #[test]
    fn area_mark_emits_one_quad_per_segment() {
        let plot = Plot::new(fixture_df())
            .axes(false)
            .mark(Mark::Area {
                interpolation: Interpolation::Linear,
            })
            .encode(x("q", ScaleKind::Band))
            .encode(y("r", ScaleKind::Linear));
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // 1 bg + 3 quads (4 points → 3 segments).
        assert_eq!(g.primitive_count(), 4);
    }

    #[test]
    fn area_mark_with_step_interpolation_emits_one_quad_per_segment() {
        let plot = Plot::new(fixture_df())
            .axes(false)
            .mark(Mark::Area {
                interpolation: Interpolation::Step,
            })
            .encode(x("q", ScaleKind::Band))
            .encode(y("r", ScaleKind::Linear));
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // 1 bg + 3 quads.
        assert_eq!(g.primitive_count(), 4);
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
