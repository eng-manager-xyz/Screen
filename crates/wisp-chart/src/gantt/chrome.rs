//! Gantt timeline chrome — header background, alt-row tints, week
//! gridlines, holiday pips, quarter ticks, current-date marker,
//! and planning overlays.
//!
//! ## WG.2 scope split
//!
//! This module ships the GEOMETRY of the chrome — every line, rect,
//! and pip the chart needs to draw. Text labels (week dates,
//! project gutter names, quarter tags) are NOT emitted here:
//! they require an [`Application`](wisp::application::Application)
//! to allocate the cosmic-text pipeline (`chart_text.rs` /
//! [`build_text_node`](crate::chart_text::build_text_node)). Hosts
//! invoke `emit_chrome` for the wgpu Graphics path and then add
//! per-label text nodes in a second pass.
//!
//! This keeps `emit_chrome` callable from unit tests without
//! bringing up a wgpu device. The H2 planning demo (WG.8) wires
//! the chrome + text together.
//!
//! ## Pane routing
//!
//! Every primitive carries enough information for WG.4 to route it
//! to the correct frozen-pane scene-graph branch:
//!
//! - **Header band** (y < `theme.gantt.header_height`): header bg,
//!   holiday pips, quarter ticks.
//! - **Body** (y >= `theme.gantt.header_height`): alt-row tints, gridlines,
//!   current-date dashed line, planning overlays.
//!
//! WG.4 picks routes by `y` against the header threshold — no
//! tagging needed.

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::gantt::Gantt;
use crate::gantt::data::GanttMarker;
use crate::theme::Theme;

/// Currently-recommended dash length for the current-date marker
/// (pixels). Matched to the H2 planning DOM's `border-style: dashed`
/// rendering for visual parity.
pub const CURRENT_DATE_DASH_PX: f32 = 6.0;

/// Gap between dashes for the current-date marker.
pub const CURRENT_DATE_GAP_PX: f32 = 4.0;

impl Gantt {
    /// Emit non-text chrome geometry: header bg, alt-row tints,
    /// gridlines, holiday pips, quarter ticks, current-date dashed
    /// line, planning overlays. **Does not emit text labels** —
    /// those require an `Application` and live in the host's text
    /// pipeline.
    ///
    /// The resulting `Graphics` is meant to render BEHIND the bars
    /// (which `emit_with_interaction*` produces). Host order:
    ///
    /// 1. `gantt.emit_chrome(theme, vp)` — bottom layer.
    /// 2. `gantt.emit_with_interaction_laned(theme, vp).graphics`
    ///    — bars + avatars + markers on top.
    /// 3. Optional: per-text `chart_text` nodes for labels.
    #[must_use]
    #[allow(
        clippy::too_many_lines,
        reason = "Linear chrome emission — every step is a distinct primitive class (header bg, alt-row tints, week lines, row lines, gutter sep, marker types). Splitting into one-call-per-primitive helpers would force every helper to re-derive viewport / theme / NDC conversions; keeping the body flat reads more linearly for the chrome list."
    )]
    pub fn emit_chrome(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let mut g = Graphics::new();

        // 0. Header band background fill.
        let header_h = theme.gantt.header_height;
        g.fill(Fill::Solid(chart_to_wisp(theme.gantt.header_bg)));
        g.draw_rect(pixel_rect_to_ndc(
            crate::gantt::layout::PixelRect {
                x: 0.0,
                y: 0.0,
                w: viewport_px.x,
                h: header_h,
            },
            viewport_px,
        ));

        // 1. Alternating row tints (when theme provides a colour).
        if let Some(alt) = theme.gantt.row_alt_bg {
            g.fill(Fill::Solid(chart_to_wisp(alt)));
            for (idx, _row) in self.rows.iter().enumerate() {
                if !idx.is_multiple_of(2) {
                    continue; // Tint EVERY OTHER row, starting from row 0.
                }
                let y = crate::gantt::layout::dynamic_row_top_y(self, idx, theme);
                let h = crate::gantt::layout::row_height_for_row(self, idx, theme);
                g.draw_rect(pixel_rect_to_ndc(
                    crate::gantt::layout::PixelRect {
                        x: 0.0,
                        y,
                        w: viewport_px.x,
                        h,
                    },
                    viewport_px,
                ));
            }
        }

        // 2. Week vertical gridlines inside the body pane.
        let weeks = crate::gantt::layout::weeks_in_range(self.range);
        let body_y_top = header_h;
        let body_y_bot = viewport_px.y;
        // Computed body height across all rows; used by gridlines.
        let vline_width_ndc = theme.gantt.grid_week.width / viewport_px.x * 2.0;
        g.fill(Fill::Solid(line_color(theme.gantt.grid_week.color)));
        for week in &weeks {
            let x = crate::gantt::layout::date_to_x(
                week.start,
                self.range,
                theme.gantt.gutter_width,
                viewport_px.x,
            );
            let x_ndc = x / viewport_px.x * 2.0 - 1.0;
            let y_top_ndc = 1.0 - body_y_top / viewport_px.y * 2.0;
            let y_bot_ndc = 1.0 - body_y_bot / viewport_px.y * 2.0;
            g.draw_rect(Rect::new(
                x_ndc,
                y_bot_ndc,
                vline_width_ndc,
                y_top_ndc - y_bot_ndc,
            ));
        }

        // 3. Horizontal row gridlines below each row.
        let hline_thickness_ndc = theme.gantt.grid_week.width / viewport_px.y * 2.0;
        for (idx, _) in self.rows.iter().enumerate() {
            let y = crate::gantt::layout::dynamic_row_top_y(self, idx, theme)
                + crate::gantt::layout::row_height_for_row(self, idx, theme);
            let y_ndc = 1.0 - y / viewport_px.y * 2.0;
            g.draw_rect(Rect::new(-1.0, y_ndc, 2.0, hline_thickness_ndc));
        }

        // 4. Vertical separator between gutter and body — looks like
        // a heavier grid line. Uses month-grid style for emphasis.
        g.fill(Fill::Solid(line_color(theme.gantt.grid_month.color)));
        let gutter_x_ndc = theme.gantt.gutter_width / viewport_px.x * 2.0 - 1.0;
        let sep_w_ndc = theme.gantt.grid_month.width / viewport_px.x * 2.0;
        g.draw_rect(Rect::new(gutter_x_ndc, -1.0, sep_w_ndc, 2.0));

        // 5. Markers — header pips + body overlays.
        for marker in &self.markers {
            match marker {
                GanttMarker::Holiday { range, .. } => {
                    // Holiday pip inside the header band (small
                    // filled circle/dot above the cell). Render as
                    // a small rect for now — host can swap for an
                    // ellipse via `draw_ellipse` if preferred.
                    let x_start = crate::gantt::layout::date_to_x(
                        range.start,
                        self.range,
                        theme.gantt.gutter_width,
                        viewport_px.x,
                    );
                    let x_end = crate::gantt::layout::date_to_x(
                        range.end,
                        self.range,
                        theme.gantt.gutter_width,
                        viewport_px.x,
                    );
                    let pip_h = 6.0;
                    let pip_y = header_h - pip_h - 4.0;
                    g.fill(Fill::Solid(holiday_pip_color()));
                    g.draw_rect(pixel_rect_to_ndc(
                        crate::gantt::layout::PixelRect {
                            x: x_start,
                            y: pip_y,
                            w: (x_end - x_start).max(4.0),
                            h: pip_h,
                        },
                        viewport_px,
                    ));
                }
                GanttMarker::QuarterStart { date, .. } => {
                    // Heavy vertical tick across the header band.
                    let x = crate::gantt::layout::date_to_x(
                        *date,
                        self.range,
                        theme.gantt.gutter_width,
                        viewport_px.x,
                    );
                    g.fill(Fill::Solid(quarter_tick_color()));
                    g.draw_rect(pixel_rect_to_ndc(
                        crate::gantt::layout::PixelRect {
                            x: x - 1.0,
                            y: 0.0,
                            w: 2.0,
                            h: header_h,
                        },
                        viewport_px,
                    ));
                }
                GanttMarker::CurrentDate { date } => {
                    // Vertical dashed line through the BODY pane
                    // (skip the header). Emit one rect per dash
                    // segment so the dash pattern survives the
                    // wgpu render without bringing in a stroke
                    // pipeline.
                    let x = crate::gantt::layout::date_to_x(
                        *date,
                        self.range,
                        theme.gantt.gutter_width,
                        viewport_px.x,
                    );
                    g.fill(Fill::Solid(current_date_color()));
                    let mut y = header_h;
                    while y < viewport_px.y {
                        let dash_h = CURRENT_DATE_DASH_PX.min(viewport_px.y - y);
                        g.draw_rect(pixel_rect_to_ndc(
                            crate::gantt::layout::PixelRect {
                                x: x - 1.0,
                                y,
                                w: 2.0,
                                h: dash_h,
                            },
                            viewport_px,
                        ));
                        y += CURRENT_DATE_DASH_PX + CURRENT_DATE_GAP_PX;
                    }
                }
                GanttMarker::PlanningOverlay { range, color, .. } => {
                    // Full-body-height rect behind bars.
                    let x_start = crate::gantt::layout::date_to_x(
                        range.start,
                        self.range,
                        theme.gantt.gutter_width,
                        viewport_px.x,
                    );
                    let x_end = crate::gantt::layout::date_to_x(
                        range.end,
                        self.range,
                        theme.gantt.gutter_width,
                        viewport_px.x,
                    );
                    g.fill(Fill::Solid(chart_to_wisp(*color)));
                    g.draw_rect(pixel_rect_to_ndc(
                        crate::gantt::layout::PixelRect {
                            x: x_start,
                            y: header_h,
                            w: (x_end - x_start).max(0.0),
                            h: viewport_px.y - header_h,
                        },
                        viewport_px,
                    ));
                }
            }
        }

        g
    }
}

/// Convert chart sRGB-encoded color to wisp's display color.
fn chart_to_wisp(c: crate::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// Convert a possibly-`Option<Color>` line style colour for fills.
fn line_color(c: crate::color::Color) -> Color {
    chart_to_wisp(c)
}

/// Default holiday-pip colour — desaturated red.
fn holiday_pip_color() -> Color {
    Color {
        r: 0.84,
        g: 0.32,
        b: 0.32,
        a: 1.0,
    }
}

/// Default quarter-tick colour — soft dark.
fn quarter_tick_color() -> Color {
    Color {
        r: 0.18,
        g: 0.18,
        b: 0.22,
        a: 0.65,
    }
}

/// Default current-date dashed-line colour — vivid blue.
fn current_date_color() -> Color {
    Color {
        r: 0.13,
        g: 0.45,
        b: 0.84,
        a: 1.0,
    }
}

/// Local copy of render's pixel→NDC (chrome doesn't import render
/// to keep its dep list small; trivial duplication).
fn pixel_rect_to_ndc(rect: crate::gantt::layout::PixelRect, viewport_px: Vec2) -> Rect {
    let x = rect.x / viewport_px.x * 2.0 - 1.0;
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
            rows: vec![
                Row::new("vec", "M-VEC"),
                Row::new("dyn", "M-DYN"),
                Row::new("text", "M-TEXT"),
            ],
            bars: vec![Bar::new("vec", date(2026, 1, 1)..date(2026, 6, 1), "Matt")],
            people,
            markers: Vec::new(),
        }
    }

    #[test]
    fn emit_chrome_outputs_at_least_header_and_gutter_separator() {
        let g = fixture();
        let theme = Theme::light();
        let out = g.emit_chrome(&theme, Vec2::new(1920.0, 800.0));
        // 1 header bg + alt rows (depends on count) + 53 week lines
        // + 3 row lines + 1 gutter separator = at least 50+.
        assert!(out.primitive_count() > 50);
    }

    #[test]
    fn emit_chrome_holiday_marker_adds_pip_primitive() {
        let mut g = fixture();
        g.markers.push(GanttMarker::Holiday {
            range: DateRange::day(date(2026, 7, 4)),
            label: "Independence Day".into(),
        });
        let theme = Theme::light();
        let count_without = fixture()
            .emit_chrome(&theme, Vec2::new(1920.0, 800.0))
            .primitive_count();
        let count_with = g
            .emit_chrome(&theme, Vec2::new(1920.0, 800.0))
            .primitive_count();
        assert_eq!(count_with - count_without, 1, "one pip per holiday");
    }

    #[test]
    fn emit_chrome_quarter_marker_adds_tick_primitive() {
        let mut g = fixture();
        g.markers.push(GanttMarker::QuarterStart {
            date: date(2026, 4, 1),
            label: Some("Q2 2026".into()),
        });
        let theme = Theme::light();
        let count_without = fixture()
            .emit_chrome(&theme, Vec2::new(1920.0, 800.0))
            .primitive_count();
        let count_with = g
            .emit_chrome(&theme, Vec2::new(1920.0, 800.0))
            .primitive_count();
        assert_eq!(count_with - count_without, 1);
    }

    #[test]
    fn emit_chrome_current_date_emits_multiple_dash_segments() {
        let mut g = fixture();
        g.markers.push(GanttMarker::CurrentDate {
            date: date(2026, 6, 15),
        });
        let theme = Theme::light();
        let count_without = fixture()
            .emit_chrome(&theme, Vec2::new(1920.0, 800.0))
            .primitive_count();
        let count_with = g
            .emit_chrome(&theme, Vec2::new(1920.0, 800.0))
            .primitive_count();
        // Body height ≈ 740; dash + gap = 10 px; ~74 dashes.
        let added = count_with - count_without;
        assert!(added > 1, "expected dashed segments, got {added}");
    }

    #[test]
    fn emit_chrome_planning_overlay_adds_one_rect() {
        let mut g = fixture();
        g.markers.push(GanttMarker::PlanningOverlay {
            range: DateRange::from_range(date(2026, 12, 20)..date(2026, 12, 31)),
            label: "Year-end slowdown".into(),
            color: crate::color::Color::WHITE,
        });
        let theme = Theme::light();
        let count_without = fixture()
            .emit_chrome(&theme, Vec2::new(1920.0, 800.0))
            .primitive_count();
        let count_with = g
            .emit_chrome(&theme, Vec2::new(1920.0, 800.0))
            .primitive_count();
        assert_eq!(count_with - count_without, 1);
    }
}
