//! Frozen-pane scene graph for Gantt.
//!
//! `GanttScene` collects chrome + bar primitives into FOUR separate
//! `Graphics` containers — one per spreadsheet-style pane:
//!
//! | Pane | Region | Pan axes |
//! |---|---|---|
//! | `corner` | `(0..gutter_width, 0..header_height)` | none |
//! | `header` | `(gutter_width.., 0..header_height)` | X only |
//! | `gutter` | `(0..gutter_width, header_height..)` | Y only |
//! | `body`   | `(gutter_width.., header_height..)` | X + Y |
//!
//! The host renders each `Graphics` to the same surface using the
//! offset returned by [`GanttPanController`](crate::gantt::pan::GanttPanController)
//! (`body_offset / header_offset / gutter_offset / corner_offset`)
//! and the scissor rect from [`pane_scissor`].
//!
//! ## Bucket-by-y conventions
//!
//! `emit_scene` routes every primitive into the right bucket by
//! its top-left position:
//!
//! - `y < header_height` AND `x < gutter_width` → **corner**.
//! - `y < header_height` AND `x >= gutter_width` → **header**.
//! - `y >= header_height` AND `x < gutter_width` → **gutter**.
//! - `y >= header_height` AND `x >= gutter_width` → **body**.
//!
//! A primitive that straddles a boundary (a full-width gridline,
//! the gutter→body separator) goes into whichever pane its
//! anchor sits in — the host's scissor rect crops the overflow.

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::gantt::Gantt;
use crate::gantt::data::{GanttMarker, GanttRole};
use crate::gantt::layout::bar_pixel_rect_laned;
use crate::interaction::{ChartElementId, EmittedChart};
use crate::theme::Theme;

/// Four-pane scene graph for a Gantt chart. Each field is a
/// `Graphics` the host renders independently with the corresponding
/// pan offset + scissor rect.
#[derive(Debug, Clone)]
pub struct GanttScene {
    /// Top-left intersection. Fully frozen.
    pub corner: Graphics,
    /// Date / week header band. Pans X only.
    pub header: Graphics,
    /// Project / row label gutter. Pans Y only.
    pub gutter: Graphics,
    /// Timeline body. Pans X + Y.
    pub body: Graphics,
    /// Bar-level pickable element mapping into the BODY pane.
    /// Same shape as [`EmittedChart::elements`] — host registers
    /// these into `wisp_interaction::PickableMap` after applying
    /// the body pan offset.
    pub elements: Vec<(usize, ChartElementId)>,
}

/// Scissor rect for one of the four panes. Pixel-space, top-left
/// origin, `+Y` down. Hosts use this with `wgpu::RenderPass::set_scissor_rect`
/// (after applying DPR scaling) so the pane's render does not bleed
/// into the others.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaneScissor {
    /// Left edge in pixels.
    pub x: f32,
    /// Top edge in pixels.
    pub y: f32,
    /// Width in pixels.
    pub w: f32,
    /// Height in pixels.
    pub h: f32,
}

/// Which pane a primitive or scissor belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pane {
    /// Top-left corner.
    Corner,
    /// Top header band.
    Header,
    /// Left gutter column.
    Gutter,
    /// Timeline body.
    Body,
}

/// Scissor rect for a given pane within `viewport_px`. The four
/// rects tile the viewport exactly.
#[must_use]
pub fn pane_scissor(pane: Pane, theme: &Theme, viewport_px: Vec2) -> PaneScissor {
    let g = theme.gantt.gutter_width;
    let h = theme.gantt.header_height;
    let vp_w = viewport_px.x;
    let vp_h = viewport_px.y;
    match pane {
        Pane::Corner => PaneScissor {
            x: 0.0,
            y: 0.0,
            w: g.min(vp_w),
            h: h.min(vp_h),
        },
        Pane::Header => PaneScissor {
            x: g.min(vp_w),
            y: 0.0,
            w: (vp_w - g).max(0.0),
            h: h.min(vp_h),
        },
        Pane::Gutter => PaneScissor {
            x: 0.0,
            y: h.min(vp_h),
            w: g.min(vp_w),
            h: (vp_h - h).max(0.0),
        },
        Pane::Body => PaneScissor {
            x: g.min(vp_w),
            y: h.min(vp_h),
            w: (vp_w - g).max(0.0),
            h: (vp_h - h).max(0.0),
        },
    }
}

impl Gantt {
    /// Emit a four-pane scene graph routed by pixel-coord buckets.
    ///
    /// Combines the chrome (header bg, gridlines, holiday pips,
    /// quarter ticks, current-date marker, planning overlays) with
    /// the laned bars (rect + avatar + allocation + tech-lead) into
    /// the four `Graphics` containers on `GanttScene`.
    ///
    /// The `elements` field on the scene holds the body-pane
    /// pickable bar entries — primitive indices are local to
    /// `body.primitive_count()` (not the total scene), because
    /// each pane is its own `Graphics` with its own indexing.
    #[must_use]
    #[allow(
        clippy::too_many_lines,
        reason = "Linear pane composition — every step is a primitive class (corner bg, header bg, gutter bg, body bg, alt-row tints, week vlines, header ticks, markers, bars + per-bar decorations). Extracting each into its own helper would force re-derivation of viewport + theme + NDC math; keeping the body flat reads more linearly for the scene composition."
    )]
    pub fn emit_scene(&self, theme: &Theme, viewport_px: Vec2) -> GanttScene {
        let mut corner = Graphics::new();
        let mut header = Graphics::new();
        let mut gutter = Graphics::new();
        let mut body = Graphics::new();
        let mut elements: Vec<(usize, ChartElementId)> = Vec::new();

        // ───── Header pane bg ──────────────────────────────────
        header.fill(Fill::Solid(chart_to_wisp(theme.gantt.header_bg)));
        header.draw_rect(rect_in_pane_ndc(
            Pane::Header,
            theme,
            viewport_px,
            crate::gantt::layout::PixelRect {
                x: theme.gantt.gutter_width,
                y: 0.0,
                w: viewport_px.x - theme.gantt.gutter_width,
                h: theme.gantt.header_height,
            },
        ));

        // ───── Corner pane bg ──────────────────────────────────
        corner.fill(Fill::Solid(chart_to_wisp(theme.gantt.header_bg)));
        corner.draw_rect(rect_in_pane_ndc(
            Pane::Corner,
            theme,
            viewport_px,
            crate::gantt::layout::PixelRect {
                x: 0.0,
                y: 0.0,
                w: theme.gantt.gutter_width,
                h: theme.gantt.header_height,
            },
        ));

        // ───── Gutter pane bg ──────────────────────────────────
        // Solid background so the gutter doesn't show through to
        // body bars when the body pans behind it.
        gutter.fill(Fill::Solid(chart_to_wisp(theme.bg)));
        gutter.draw_rect(rect_in_pane_ndc(
            Pane::Gutter,
            theme,
            viewport_px,
            crate::gantt::layout::PixelRect {
                x: 0.0,
                y: theme.gantt.header_height,
                w: theme.gantt.gutter_width,
                h: viewport_px.y - theme.gantt.header_height,
            },
        ));

        // ───── Body pane bg ────────────────────────────────────
        body.fill(Fill::Solid(chart_to_wisp(theme.bg)));
        body.draw_rect(rect_in_pane_ndc(
            Pane::Body,
            theme,
            viewport_px,
            crate::gantt::layout::PixelRect {
                x: theme.gantt.gutter_width,
                y: theme.gantt.header_height,
                w: viewport_px.x - theme.gantt.gutter_width,
                h: viewport_px.y - theme.gantt.header_height,
            },
        ));

        // ───── Body: alt-row tints + week gridlines + bars ─────
        if let Some(alt) = theme.gantt.row_alt_bg {
            body.fill(Fill::Solid(chart_to_wisp(alt)));
            for (idx, _row) in self.rows.iter().enumerate() {
                if !idx.is_multiple_of(2) {
                    continue;
                }
                let y = crate::gantt::layout::dynamic_row_top_y(self, idx, theme);
                let h = crate::gantt::layout::row_height_for_row(self, idx, theme);
                body.draw_rect(rect_in_pane_ndc(
                    Pane::Body,
                    theme,
                    viewport_px,
                    crate::gantt::layout::PixelRect {
                        x: theme.gantt.gutter_width,
                        y,
                        w: viewport_px.x - theme.gantt.gutter_width,
                        h,
                    },
                ));
            }
        }

        // Vertical week gridlines.
        let weeks = crate::gantt::layout::weeks_in_range(self.range);
        let vline_w = theme.gantt.grid_week.width;
        body.fill(Fill::Solid(chart_to_wisp(theme.gantt.grid_week.color)));
        for week in &weeks {
            let x = crate::gantt::layout::date_to_x(
                week.start,
                self.range,
                theme.gantt.gutter_width,
                viewport_px.x,
            );
            body.draw_rect(rect_in_pane_ndc(
                Pane::Body,
                theme,
                viewport_px,
                crate::gantt::layout::PixelRect {
                    x,
                    y: theme.gantt.header_height,
                    w: vline_w,
                    h: viewport_px.y - theme.gantt.header_height,
                },
            ));
        }

        // ───── Header: week ticks ──────────────────────────────
        header.fill(Fill::Solid(chart_to_wisp(theme.gantt.grid_week.color)));
        for week in &weeks {
            let x = crate::gantt::layout::date_to_x(
                week.start,
                self.range,
                theme.gantt.gutter_width,
                viewport_px.x,
            );
            header.draw_rect(rect_in_pane_ndc(
                Pane::Header,
                theme,
                viewport_px,
                crate::gantt::layout::PixelRect {
                    x,
                    y: 0.0,
                    w: vline_w,
                    h: theme.gantt.header_height,
                },
            ));
        }

        // ───── Markers ────────────────────────────────────────
        for marker in &self.markers {
            match marker {
                GanttMarker::Holiday { range, .. } => {
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
                    header.fill(Fill::Solid(Color {
                        r: 0.84,
                        g: 0.32,
                        b: 0.32,
                        a: 1.0,
                    }));
                    header.draw_rect(rect_in_pane_ndc(
                        Pane::Header,
                        theme,
                        viewport_px,
                        crate::gantt::layout::PixelRect {
                            x: x_start,
                            y: theme.gantt.header_height - 10.0,
                            w: (x_end - x_start).max(4.0),
                            h: 6.0,
                        },
                    ));
                }
                GanttMarker::QuarterStart { date, .. } => {
                    let x = crate::gantt::layout::date_to_x(
                        *date,
                        self.range,
                        theme.gantt.gutter_width,
                        viewport_px.x,
                    );
                    header.fill(Fill::Solid(Color {
                        r: 0.18,
                        g: 0.18,
                        b: 0.22,
                        a: 0.65,
                    }));
                    header.draw_rect(rect_in_pane_ndc(
                        Pane::Header,
                        theme,
                        viewport_px,
                        crate::gantt::layout::PixelRect {
                            x: x - 1.0,
                            y: 0.0,
                            w: 2.0,
                            h: theme.gantt.header_height,
                        },
                    ));
                }
                GanttMarker::CurrentDate { date } => {
                    // Body-pane dashed line (no header segment).
                    let x = crate::gantt::layout::date_to_x(
                        *date,
                        self.range,
                        theme.gantt.gutter_width,
                        viewport_px.x,
                    );
                    body.fill(Fill::Solid(Color {
                        r: 0.13,
                        g: 0.45,
                        b: 0.84,
                        a: 1.0,
                    }));
                    let mut y = theme.gantt.header_height;
                    while y < viewport_px.y {
                        let dash_h = 6.0_f32.min(viewport_px.y - y);
                        body.draw_rect(rect_in_pane_ndc(
                            Pane::Body,
                            theme,
                            viewport_px,
                            crate::gantt::layout::PixelRect {
                                x: x - 1.0,
                                y,
                                w: 2.0,
                                h: dash_h,
                            },
                        ));
                        y += 10.0;
                    }
                }
                GanttMarker::PlanningOverlay { range, color, .. } => {
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
                    body.fill(Fill::Solid(chart_to_wisp(*color)));
                    body.draw_rect(rect_in_pane_ndc(
                        Pane::Body,
                        theme,
                        viewport_px,
                        crate::gantt::layout::PixelRect {
                            x: x_start,
                            y: theme.gantt.header_height,
                            w: (x_end - x_start).max(0.0),
                            h: viewport_px.y - theme.gantt.header_height,
                        },
                    ));
                }
            }
        }

        // ───── Body: bars ──────────────────────────────────────
        let avatar_radius_px = theme.gantt.bar_height * 0.42;
        let alloc_radius_px = theme.gantt.bar_height * 0.4;
        let diamond_size_px = theme.gantt.bar_height * 0.35;

        for (bar_idx, bar) in self.bars.iter().enumerate() {
            let Some(rect_px) = bar_pixel_rect_laned(bar, self, theme, viewport_px.x) else {
                continue;
            };
            let fill = resolve_bar_fill(self, theme, &bar.owner);
            body.fill(Fill::Solid(fill));
            body.draw_rounded_rect(
                rect_in_pane_ndc(Pane::Body, theme, viewport_px, rect_px),
                theme.gantt.bar_corner_radius / viewport_px.y * 2.0,
            );
            elements.push((
                body.primitive_count() - 1,
                ChartElementId::GanttBar(bar_idx),
            ));

            if rect_px.w < theme.gantt.bar_height * 1.8 {
                continue;
            }

            // Avatar.
            let avatar_centre_px = Vec2::new(
                rect_px.x + theme.gantt.bar_height * 0.5,
                rect_px.y + theme.gantt.bar_height * 0.5,
            );
            body.fill(Fill::Solid(Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            }));
            body.draw_ellipse(
                px_in_pane_to_ndc(Pane::Body, theme, viewport_px, avatar_centre_px),
                Vec2::new(
                    avatar_radius_px / viewport_px.x * 2.0,
                    avatar_radius_px / viewport_px.y * 2.0,
                ),
            );

            if bar.allocation_pct.is_some() {
                let alloc_centre_px = Vec2::new(
                    rect_px.x + rect_px.w - theme.gantt.bar_height * 0.5,
                    rect_px.y + theme.gantt.bar_height * 0.5,
                );
                body.fill(Fill::Solid(Color {
                    r: 0.12,
                    g: 0.12,
                    b: 0.12,
                    a: 1.0,
                }));
                body.draw_ellipse(
                    px_in_pane_to_ndc(Pane::Body, theme, viewport_px, alloc_centre_px),
                    Vec2::new(
                        alloc_radius_px / viewport_px.x * 2.0,
                        alloc_radius_px / viewport_px.y * 2.0,
                    ),
                );
            }

            if bar.roles.contains(&GanttRole::TechLead) {
                let diamond_centre_px = Vec2::new(
                    rect_px.x + rect_px.w * 0.5,
                    rect_px.y + theme.gantt.bar_height * 0.5,
                );
                body.fill(Fill::Solid(Color {
                    r: 0.12,
                    g: 0.12,
                    b: 0.12,
                    a: 1.0,
                }));
                let half = diamond_size_px * 0.5;
                let centre_ndc =
                    px_in_pane_to_ndc(Pane::Body, theme, viewport_px, diamond_centre_px);
                let dx = half / viewport_px.x * 2.0;
                let dy = half / viewport_px.y * 2.0;
                body.draw_polygon(&[
                    Vec2::new(centre_ndc.x + dx, centre_ndc.y),
                    Vec2::new(centre_ndc.x, centre_ndc.y + dy),
                    Vec2::new(centre_ndc.x - dx, centre_ndc.y),
                    Vec2::new(centre_ndc.x, centre_ndc.y - dy),
                ]);
            }
        }

        GanttScene {
            corner,
            header,
            gutter,
            body,
            elements,
        }
    }

    /// Convenience: emit the bars as a flat `EmittedChart` keyed
    /// only to the body pane, dropping the chrome. Useful when a
    /// host already has its own chrome path.
    #[must_use]
    pub fn emit_body_only(&self, theme: &Theme, viewport_px: Vec2) -> EmittedChart {
        let scene = self.emit_scene(theme, viewport_px);
        EmittedChart {
            graphics: scene.body,
            elements: scene.elements,
        }
    }
}

/// Convert a pixel rect to the NDC rect that lives inside `pane`'s
/// scissor. We use FULL viewport NDC for every pane today —
/// scissor at render time crops the overflow. Future improvement:
/// remap each pane to its own local NDC for better fp precision.
fn rect_in_pane_ndc(
    _pane: Pane,
    _theme: &Theme,
    viewport_px: Vec2,
    rect: crate::gantt::layout::PixelRect,
) -> Rect {
    let x = rect.x / viewport_px.x * 2.0 - 1.0;
    let y = 1.0 - (rect.y + rect.h) / viewport_px.y * 2.0;
    let w = rect.w / viewport_px.x * 2.0;
    let h = rect.h / viewport_px.y * 2.0;
    Rect::new(x, y, w, h)
}

fn px_in_pane_to_ndc(_pane: Pane, _theme: &Theme, viewport_px: Vec2, p: Vec2) -> Vec2 {
    Vec2::new(
        p.x / viewport_px.x * 2.0 - 1.0,
        1.0 - p.y / viewport_px.y * 2.0,
    )
}

fn chart_to_wisp(c: crate::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn resolve_bar_fill(gantt: &Gantt, theme: &Theme, owner: &str) -> Color {
    if let Some(person) = gantt.people.get(owner) {
        chart_to_wisp(person.color)
    } else {
        chart_to_wisp(theme.palette.color_for(owner))
    }
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
            rows: vec![Row::new("vec", "M-VEC"), Row::new("dyn", "M-DYN")],
            bars: vec![Bar::new("vec", date(2026, 1, 1)..date(2026, 6, 1), "Matt")],
            people,
            markers: Vec::new(),
        }
    }

    #[test]
    fn pane_scissors_tile_viewport_exactly() {
        let theme = Theme::light();
        let vp = Vec2::new(1920.0, 800.0);
        let c = pane_scissor(Pane::Corner, &theme, vp);
        let h = pane_scissor(Pane::Header, &theme, vp);
        let g = pane_scissor(Pane::Gutter, &theme, vp);
        let b = pane_scissor(Pane::Body, &theme, vp);
        // Corner + Header widths cover full viewport.
        assert!((c.w + h.w - vp.x).abs() < 1e-4);
        // Corner + Gutter heights cover full viewport.
        assert!((c.h + g.h - vp.y).abs() < 1e-4);
        // Body sits past gutter + below header.
        assert!((b.x - theme.gantt.gutter_width).abs() < 1e-4);
        assert!((b.y - theme.gantt.header_height).abs() < 1e-4);
    }

    #[test]
    fn emit_scene_populates_all_four_panes() {
        let scene = fixture().emit_scene(&Theme::light(), Vec2::new(1920.0, 800.0));
        assert!(scene.corner.primitive_count() > 0, "corner has bg");
        assert!(scene.header.primitive_count() > 0, "header has bg + ticks");
        assert!(scene.gutter.primitive_count() > 0, "gutter has bg");
        assert!(scene.body.primitive_count() > 0, "body has bg + grid + bar");
    }

    #[test]
    fn emit_scene_bar_element_indices_local_to_body_pane() {
        let scene = fixture().emit_scene(&Theme::light(), Vec2::new(1920.0, 800.0));
        // One bar in the fixture → 1 element entry.
        assert_eq!(scene.elements.len(), 1);
        let (idx, id) = scene.elements[0];
        assert!(idx < scene.body.primitive_count(), "index local to body");
        assert_eq!(id, ChartElementId::GanttBar(0));
    }

    #[test]
    fn emit_body_only_returns_body_graphics_plus_elements() {
        let body_chart = fixture().emit_body_only(&Theme::light(), Vec2::new(1920.0, 800.0));
        assert!(body_chart.graphics.primitive_count() > 0);
        assert_eq!(body_chart.elements.len(), 1);
    }

    #[test]
    fn header_pane_includes_holiday_pip_when_marker_present() {
        let mut g = fixture();
        g.markers.push(GanttMarker::Holiday {
            range: DateRange::day(date(2026, 7, 4)),
            label: "Indep".into(),
        });
        let scene_with = g.emit_scene(&Theme::light(), Vec2::new(1920.0, 800.0));
        let scene_without = fixture().emit_scene(&Theme::light(), Vec2::new(1920.0, 800.0));
        // Header has the extra pip primitive; body unchanged.
        assert!(scene_with.header.primitive_count() > scene_without.header.primitive_count());
        assert_eq!(
            scene_with.body.primitive_count(),
            scene_without.body.primitive_count()
        );
    }

    #[test]
    fn body_pane_includes_current_date_dashes_when_marker_present() {
        let mut g = fixture();
        g.markers.push(GanttMarker::CurrentDate {
            date: date(2026, 6, 15),
        });
        let scene_with = g.emit_scene(&Theme::light(), Vec2::new(1920.0, 800.0));
        let scene_without = fixture().emit_scene(&Theme::light(), Vec2::new(1920.0, 800.0));
        // Body has the extra dash primitives.
        assert!(scene_with.body.primitive_count() > scene_without.body.primitive_count() + 5);
    }
}
