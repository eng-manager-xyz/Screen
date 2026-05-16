//! `Theme` — the visual configuration applied when rendering a
//! chart.
//!
//! Composed of a parent [`Theme`] holding shared fields (`bg`,
//! `text_primary`, `text_muted`, `palette`) and five sub-themes
//! that each future chart family populates as it lands:
//!
//! * [`PlotTheme`] — plot-area padding, gridline styles, axis line
//!   styles, default mark fill/stroke widths. Read by cartesian
//!   charts (bar, line, scatter, etc.) when they ship.
//! * [`AxisTheme`] — tick length, tick label font + colour, axis
//!   title font + colour, tick density hints. Read by
//!   [`crate::gantt::layout`] today indirectly via Gantt's row /
//!   gutter sizing, and by cartesian axis renderers when they
//!   ship.
//! * [`LegendTheme`] — swatch size, item spacing, item font,
//!   legend background. Read by any chart with a categorical
//!   colour encoding.
//! * [`IndicatorTheme`] — KPI / gauge / bullet numeric font,
//!   delta colour rules. Read by the indicator chart family.
//! * [`GanttTheme`] — the existing Gantt-specific knobs gathered
//!   together so other chart families don't accidentally reach
//!   into Gantt-shaped fields.
//!
//! Boundary rule: a downstream chart reads ONLY from its own
//! sub-theme plus the top-level shared fields. Reaching into
//! another chart family's sub-theme is the same kind of
//! structural coupling this refactor exists to prevent.

use crate::color::Color;
use crate::palette::OwnerPalette;

/// Line style for grid lines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineStyle {
    /// Stroke colour.
    pub color: Color,
    /// Stroke width in device pixels.
    pub width: f32,
}

/// Shared plot-area knobs read by cartesian / polar / heatmap
/// chart families. Empty-but-typed today; chart tickets that land
/// new fields extend this struct rather than growing parallel
/// theme structs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlotTheme {
    /// Major gridline (e.g. month boundaries on a time axis).
    pub gridline_major: LineStyle,
    /// Minor gridline (e.g. week boundaries on a time axis).
    pub gridline_minor: LineStyle,
    /// Stroke width for line marks (Line / Step / Monotone) in
    /// device pixels.
    pub line_width_px: f32,
    /// Radius of a `PointStyle::Circle` marker on a line chart in
    /// device pixels.
    pub line_marker_radius_px: f32,
    /// Corner radius for `Mark::Bar` / `Mark::Column` rectangles in
    /// device pixels. Default `0.0` (square corners — the modern
    /// data-vis convention). Callers that want rounded bars set
    /// this to a non-zero value before rendering; the Gantt family
    /// reads its own [`GanttTheme::bar_corner_radius`] independently.
    pub bar_corner_radius: f32,
}

/// Shared axis-renderer knobs. Read by the cartesian axis
/// renderer when it lands; today only the `tick_label_font_size`
/// is referenced (indirectly via Gantt's gutter sizing).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisTheme {
    /// Length of a tick mark in device pixels.
    pub tick_length_px: f32,
    /// Tick-label text size in device pixels.
    pub tick_label_font_size: f32,
    /// Approximate target number of ticks the auto-tick generator
    /// should aim for. Generators may produce more or fewer to
    /// land on round-numbered stops.
    pub tick_density_hint: usize,
}

/// Shared legend-renderer knobs. Populated once a chart with a
/// categorical colour encoding lands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LegendTheme {
    /// Swatch box / line / marker size in device pixels.
    pub swatch_size_px: f32,
    /// Spacing between legend items in device pixels.
    pub item_spacing_px: f32,
    /// Legend item label font size in device pixels.
    pub item_font_size: f32,
}

/// Shared indicator-card knobs (KPI / gauge / bullet).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IndicatorTheme {
    /// Big-number font size for KPI value displays.
    pub numeric_font_size: f32,
    /// Label font size (the description under the big number).
    pub label_font_size: f32,
    /// Delta-text font size.
    pub delta_font_size: f32,
    /// Colour for positive deltas (up arrows, green-positive).
    pub delta_up: Color,
    /// Colour for negative deltas (down arrows, red-negative).
    pub delta_down: Color,
    /// Colour for neutral deltas / no-change indicators.
    pub delta_neutral: Color,
    /// Sparkline stroke colour.
    pub sparkline_color: Color,
    /// Sparkline stroke width in device pixels.
    pub sparkline_width_px: f32,
    /// Gauge arc-track width in device pixels (band thickness).
    pub gauge_track_width_px: f32,
    /// Gauge needle colour.
    pub gauge_needle_color: Color,
    /// Bullet chart — poor (lowest) qualitative range colour.
    pub bullet_poor_color: Color,
    /// Bullet chart — satisfactory (middle) qualitative range colour.
    pub bullet_ok_color: Color,
    /// Bullet chart — good (highest) qualitative range colour.
    pub bullet_good_color: Color,
    /// Bullet chart — current-value bar colour.
    pub bullet_value_color: Color,
    /// Bullet chart — target marker line colour.
    pub bullet_target_color: Color,
}

/// Gantt-specific dimensions and style. Other chart families
/// must NOT reach into these fields — they exist to keep the
/// Gantt renderer self-contained.
#[derive(Clone, Debug, PartialEq)]
pub struct GanttTheme {
    /// Alternating row tint (or `None` to disable).
    pub row_alt_bg: Option<Color>,
    /// Header band background colour.
    pub header_bg: Color,
    /// Week-strip grid lines.
    pub grid_week: LineStyle,
    /// Month-strip grid lines (heavier than weeks).
    pub grid_month: LineStyle,
    /// Bar corner radius in pixels.
    pub bar_corner_radius: f32,
    /// Bar height in pixels (centred within the row).
    pub bar_height: f32,
    /// Row height in pixels.
    pub row_height: f32,
    /// Left gutter width (project labels) in pixels.
    pub gutter_width: f32,
    /// Header band height in pixels.
    pub header_height: f32,
}

/// Visual configuration applied at render time.
///
/// One `Theme` themes every chart family consistently. Customise
/// by overriding fields in the sub-themes you care about and
/// leaving the rest as `Theme::light()` (or `::dark()` once that
/// lands).
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// Canvas background fill — shared across every chart.
    pub bg: Color,
    /// Primary text colour for labels + headers — shared.
    pub text_primary: Color,
    /// Muted text colour for axis labels + secondary info —
    /// shared.
    pub text_muted: Color,
    /// Owner / category colour palette + auto-assignment policy.
    /// Shared across charts that have categorical colour
    /// encodings.
    pub palette: OwnerPalette,
    /// Plot-area shared knobs.
    pub plot: PlotTheme,
    /// Axis-renderer knobs.
    pub axis: AxisTheme,
    /// Legend-renderer knobs.
    pub legend: LegendTheme,
    /// Indicator-card knobs (KPI / gauge / bullet).
    pub indicator: IndicatorTheme,
    /// Gantt-specific dimensions + style.
    pub gantt: GanttTheme,
}

impl Theme {
    /// The v1 light theme — white background, alt-row tint
    /// `#fafafa`, Wong palette, 44 px rows / 28 px bars / 6 px
    /// corner radius / 180 px gutter / 60 px header (Gantt
    /// sub-theme); sub-themes for plot / axis / legend /
    /// indicator populated with sensible defaults.
    #[must_use]
    pub fn light() -> Self {
        let grid_minor = LineStyle {
            color: Color::from_hex("#e5e5e5").unwrap(),
            width: 1.0,
        };
        let grid_major = LineStyle {
            color: Color::from_hex("#cccccc").unwrap(),
            width: 2.0,
        };
        Self {
            bg: Color::from_hex("#ffffff").unwrap(),
            text_primary: Color::from_hex("#222222").unwrap(),
            text_muted: Color::from_hex("#888888").unwrap(),
            palette: OwnerPalette::default(),
            plot: PlotTheme {
                gridline_major: grid_major,
                gridline_minor: grid_minor,
                line_width_px: 2.0,
                line_marker_radius_px: 3.0,
                bar_corner_radius: 0.0,
            },
            axis: AxisTheme {
                tick_length_px: 5.0,
                tick_label_font_size: 12.0,
                tick_density_hint: 8,
            },
            legend: LegendTheme {
                swatch_size_px: 14.0,
                item_spacing_px: 8.0,
                item_font_size: 12.0,
            },
            indicator: IndicatorTheme {
                numeric_font_size: 32.0,
                label_font_size: 14.0,
                delta_font_size: 12.0,
                delta_up: Color::from_hex("#27ae60").unwrap(),
                delta_down: Color::from_hex("#e74c3c").unwrap(),
                delta_neutral: Color::from_hex("#888888").unwrap(),
                sparkline_color: Color::from_hex("#3a8ddb").unwrap(),
                sparkline_width_px: 1.5,
                gauge_track_width_px: 18.0,
                gauge_needle_color: Color::from_hex("#222222").unwrap(),
                bullet_poor_color: Color::from_hex("#e0e0e0").unwrap(),
                bullet_ok_color: Color::from_hex("#b8b8b8").unwrap(),
                bullet_good_color: Color::from_hex("#909090").unwrap(),
                bullet_value_color: Color::from_hex("#222222").unwrap(),
                bullet_target_color: Color::from_hex("#e74c3c").unwrap(),
            },
            gantt: GanttTheme {
                row_alt_bg: Color::from_hex("#fafafa"),
                header_bg: Color::from_hex("#f5f5f5").unwrap(),
                grid_week: grid_minor,
                grid_month: grid_major,
                bar_corner_radius: 6.0,
                bar_height: 28.0,
                row_height: 44.0,
                gutter_width: 180.0,
                header_height: 60.0,
            },
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_theme_uses_white_bg() {
        let t = Theme::light();
        assert_eq!(t.bg, Color::WHITE);
    }

    #[test]
    fn light_theme_gantt_dimensions_match_spec() {
        let t = Theme::light();
        assert!((t.gantt.bar_height - 28.0).abs() < 1e-6);
        assert!((t.gantt.row_height - 44.0).abs() < 1e-6);
        assert!((t.gantt.gutter_width - 180.0).abs() < 1e-6);
        assert!((t.gantt.header_height - 60.0).abs() < 1e-6);
        assert!((t.gantt.bar_corner_radius - 6.0).abs() < 1e-6);
    }

    #[test]
    fn light_theme_populates_every_sub_theme() {
        let t = Theme::light();
        // Each sub-theme has at least one non-trivial value
        // populated (no zeros where a real number is required).
        assert!(t.plot.gridline_major.width > 0.0);
        assert!(t.plot.gridline_minor.width > 0.0);
        assert!(t.axis.tick_label_font_size > 0.0);
        assert!(t.axis.tick_length_px > 0.0);
        assert!(t.axis.tick_density_hint > 0);
        assert!(t.legend.swatch_size_px > 0.0);
        assert!(t.legend.item_spacing_px > 0.0);
        assert!(t.indicator.numeric_font_size > 0.0);
        assert_ne!(t.indicator.delta_up, t.indicator.delta_down);
        assert!(t.gantt.row_alt_bg.is_some());
    }

    #[test]
    fn gantt_grid_lines_match_legacy_aliases() {
        // The original flat-Theme stored `grid_week` /
        // `grid_month` aliased to the plot gridlines; the
        // decomposition keeps that invariant in `GanttTheme`'s
        // own fields so Gantt rendering stays byte-identical.
        let t = Theme::light();
        assert_eq!(t.gantt.grid_week, t.plot.gridline_minor);
        assert_eq!(t.gantt.grid_month, t.plot.gridline_major);
    }
}
