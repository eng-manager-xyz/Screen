//! Calendar heatmap — GitHub-style year-in-review. 7 rows × 52
//! columns of daily cells; intensity = value via a
//! [`crate::heatmap::SequentialPalette`].

use glam::Vec2;
use jiff::civil::{Date, date};
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::heatmap::SequentialPalette;
use crate::theme::Theme;

/// One day → value pair.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalendarValue {
    /// The day this value applies to.
    pub date: Date,
    /// Numeric weight (commits, sales, etc).
    pub value: f32,
}

impl CalendarValue {
    /// Construct from a date + value.
    #[must_use]
    pub const fn new(date: Date, value: f32) -> Self {
        Self { date, value }
    }
}

/// Calendar heatmap value type.
#[derive(Clone, Debug)]
pub struct CalendarHeatmap {
    /// Year being rendered. All `values` are clipped to this year.
    pub year: i16,
    /// Per-day values. Missing days render as the palette's
    /// minimum stop.
    pub values: Vec<CalendarValue>,
    /// Colour palette.
    pub palette: SequentialPalette,
}

impl CalendarHeatmap {
    /// Construct with the GitHub palette.
    #[must_use]
    pub fn new(year: i16, values: Vec<CalendarValue>) -> Self {
        Self {
            year,
            values,
            palette: SequentialPalette::github(),
        }
    }

    /// Override the palette.
    #[must_use]
    pub fn palette(mut self, palette: SequentialPalette) -> Self {
        self.palette = palette;
        self
    }

    /// Emit one cell per ISO weekday × week column for the year.
    /// Cells with no value default to the palette's `0.0` stop.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        let pad = 16.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let plot_w = plot_right - plot_left;
        let plot_h = plot_bottom - plot_top;
        // 53 column slots (covers leap-year alignment).
        let cell_w = plot_w / 53.0;
        let cell_h = plot_h / 7.0;
        let cell = cell_w.min(cell_h) - 2.0;

        // Build the value lookup. Index by day-of-year.
        let mut by_doy: Vec<Option<f32>> = vec![None; 367];
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for v in &self.values {
            if v.date.year() != self.year {
                continue;
            }
            #[allow(
                clippy::cast_sign_loss,
                reason = "day_of_year returns 1..=366 per jiff; safe to index"
            )]
            let doy = v.date.day_of_year() as usize;
            by_doy[doy] = Some(v.value);
            lo = lo.min(v.value);
            hi = hi.max(v.value);
        }
        if lo.is_infinite() {
            lo = 0.0;
            hi = 1.0;
        }
        let span = (hi - lo).max(f32::EPSILON);

        // Walk every day of the year.
        let start = date(self.year, 1, 1);
        let end = date(self.year, 12, 31);
        let mut day = start;
        while day <= end {
            #[allow(
                clippy::cast_sign_loss,
                reason = "day_of_year returns 1..=366; safe to index"
            )]
            let doy = day.day_of_year() as usize;
            // Column = ISO week-of-year - 1, row = weekday (0=Mon).
            let week_col = i32::from(day.iso_week_date().week()) - 1;
            // Jan 1 may belong to the previous year's last week
            // (e.g. iso_week == 53). Clamp into [0, 52].
            let col = week_col.clamp(0, 52);
            let weekday_idx = day.weekday().to_monday_zero_offset();
            let t = if let Some(v) = by_doy[doy] {
                ((v - lo) / span).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let color = self.palette.sample(t);
            g.fill(Fill::Solid(chart_to_wisp(color)));
            #[allow(
                clippy::cast_precision_loss,
                reason = "col ≤ 52 + weekday ≤ 6, both fit f32 mantissa"
            )]
            let x = plot_left + col as f32 * cell_w;
            let y = plot_top + f32::from(weekday_idx) * cell_h;
            let rect = px_rect_to_ndc(x, y, cell, cell, viewport_px);
            g.draw_rect(rect);
            day = day
                .tomorrow()
                .expect("year-end + 1 should be representable");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_emits_one_cell_per_day_in_year() {
        let cal = CalendarHeatmap::new(
            2025,
            vec![
                CalendarValue::new(date(2025, 1, 15), 5.0),
                CalendarValue::new(date(2025, 6, 1), 12.0),
            ],
        );
        let theme = Theme::light();
        let g = cal.emit_graphics(&theme, Vec2::new(600.0, 120.0));
        // 365 days in 2025.
        assert_eq!(g.primitive_count(), 365);
    }

    #[test]
    fn calendar_handles_leap_year() {
        let cal = CalendarHeatmap::new(2024, vec![]);
        let theme = Theme::light();
        let g = cal.emit_graphics(&theme, Vec2::new(600.0, 120.0));
        // 2024 is a leap year — 366 cells.
        assert_eq!(g.primitive_count(), 366);
    }

    #[test]
    fn calendar_skips_values_from_other_years() {
        let cal = CalendarHeatmap::new(
            2025,
            vec![
                CalendarValue::new(date(2024, 6, 1), 100.0),
                CalendarValue::new(date(2025, 6, 1), 5.0),
                CalendarValue::new(date(2026, 6, 1), 200.0),
            ],
        );
        let theme = Theme::light();
        let _g = cal.emit_graphics(&theme, Vec2::new(600.0, 120.0));
        // Out-of-year values don't crash the renderer; they're
        // silently ignored. Smoke test only.
    }
}
