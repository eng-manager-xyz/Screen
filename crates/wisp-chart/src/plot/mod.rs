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
pub use mark::Mark;

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color as WispColor, Fill, Graphics};

use crate::scale::{BandScale, LinearScale, OrdinalScale};
use crate::theme::Theme;

/// Top-level grammar-of-graphics facade.
#[derive(Clone, Debug)]
pub struct Plot {
    data: DataFrame,
    mark: Mark,
    encodings: Vec<Encoding>,
}

impl Plot {
    /// Construct from a [`DataFrame`].
    #[must_use]
    pub fn new(data: DataFrame) -> Self {
        Self {
            data,
            mark: Mark::default(),
            encodings: Vec::new(),
        }
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

        // Pick the mark renderer. v1: Bar only.
        match self.mark {
            Mark::Bar { value_labels: _ } => {
                self.render_bars(theme, viewport_px, &mut g);
            }
        }

        g
    }

    fn find_encoding(&self, channel: Channel) -> Option<&Encoding> {
        self.encodings.iter().find(|e| e.channel == channel)
    }

    fn render_bars(&self, theme: &Theme, viewport_px: Vec2, g: &mut Graphics) {
        let Some(x_enc) = self.find_encoding(Channel::X) else {
            return;
        };
        let Some(y_enc) = self.find_encoding(Channel::Y) else {
            return;
        };

        // Plot area — leaves room for axes / legend even though
        // those don't paint yet, so when they ship the bars
        // don't shift.
        let gutter = 60.0;
        let header = 40.0;
        let footer = 40.0;
        let plot_left = gutter;
        let plot_right = viewport_px.x - 20.0;
        let plot_top = header;
        let plot_bottom = viewport_px.y - footer;

        // X scale (must be Band for Bar v1 — Linear bars are a
        // follow-on chart family).
        let Some(categories) = self.data.distinct_categories(&x_enc.field) else {
            return;
        };
        let x_scale = BandScale::new(categories, (plot_left, plot_right)).padding(0.15);

        // Y scale (Linear with auto-derived or explicit domain).
        let (y_lo, y_hi) = if let Some(d) = y_enc.domain_override {
            d
        } else if let Some((lo, hi)) = self.data.numeric_extent(&y_enc.field) {
            (lo.min(0.0), hi)
        } else {
            return;
        };
        // Y range flipped so larger values render at the TOP
        // (smaller pixel y).
        let y_scale = LinearScale::new((y_lo, y_hi), (plot_bottom, plot_top));

        // Color encoding → palette lookup via OrdinalScale.
        let color_lookup = self
            .find_encoding(Channel::Color)
            .and_then(|enc| self.data.distinct_categories(&enc.field).map(|c| (enc, c)))
            .map(|(enc, cats)| (enc.field.clone(), OrdinalScale::new(cats)));

        // Y zero baseline in pixel space.
        let y_zero_px = y_scale.map(0.0_f32.max(y_lo));

        // Iterate rows in DataFrame order.
        let x_col = self.data.column(&x_enc.field);
        let y_col = self.data.column(&y_enc.field);
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
}

/// Pixel-space rect used internally by mark renderers.
#[derive(Clone, Copy, Debug)]
struct PixelRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
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
            .mark(Mark::Bar {
                value_labels: false,
            })
            .encode(x("q", ScaleKind::Band))
            .encode(y("r", ScaleKind::Linear));
        let g = plot.render(&Theme::light(), Vec2::new(960.0, 400.0));
        // 1 bg + 4 bars.
        assert_eq!(g.primitive_count(), 5);
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
