//! Layout math for [`crate::Gantt`].
//!
//! Pure functions. No `wisp` types, no wgpu. Output is in pixel
//! space with the renderer convention used by the rest of the
//! `wisp` codebase: top-left origin, `+Y` down. The render layer
//! ([`crate::gantt::render`]) converts to whatever the target
//! requires (NDC for `wisp::Graphics`).
//!
//! Viewport ↔ pixel ↔ NDC conversion is the caller's job; this
//! module just answers "where does this bar/date go in pixels?".

use jiff::civil::Date;

use crate::Theme;
use crate::gantt::{Bar, DateRange, Gantt};

/// Number of whole days between two dates: `end - start`.
///
/// `jiff::Span` from a `Date::until` call yields a span whose
/// `days` component is the total day count when the unit is
/// pinned to `Day`. Negative when `end < start`.
#[must_use]
pub fn days_between(start: Date, end: Date) -> i64 {
    match start.until((jiff::Unit::Day, end)) {
        Ok(span) => i64::from(span.get_days()),
        Err(_) => 0,
    }
}

/// Fraction `0.0..=1.0` of how far `date` falls inside `range`.
/// Returns `0.0` for `date <= range.start`, `1.0` for
/// `date >= range.end`. Stable against zero-length ranges (returns
/// `0.0`).
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "elapsed/total are bounded by realistic year-scale day counts (≤ 10^5); \
              precision loss only matters above 2^24 which we can't reach."
)]
pub fn date_fraction(date: Date, range: DateRange) -> f32 {
    let total = days_between(range.start, range.end).max(0);
    if total == 0 {
        return 0.0;
    }
    let elapsed = days_between(range.start, date).clamp(0, total);
    elapsed as f32 / total as f32
}

/// Pixel x-coordinate for `date` given a viewport's gutter and
/// plot widths.
///
/// Plot area is `[gutter_width, viewport_width)`.
#[must_use]
pub fn date_to_x(date: Date, range: DateRange, gutter_width: f32, viewport_width: f32) -> f32 {
    let plot_width = (viewport_width - gutter_width).max(0.0);
    gutter_width + date_fraction(date, range) * plot_width
}

/// Pixel y-coordinate of the **top edge** of row `row_index`,
/// given a theme's `header_height` + `row_height`.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "row_index is the index into gantt.rows; realistic chart sizes are well \
              below the 2^24 precision boundary of f32."
)]
pub fn row_top_y(row_index: usize, theme: &Theme) -> f32 {
    theme.gantt.header_height + (row_index as f32) * theme.gantt.row_height
}

/// Pixel rect for the bar centred inside its row.
///
/// Returns `None` if `bar.row_id` doesn't match any row in
/// `gantt.rows`. The bar's y-band is centred vertically within
/// the row using `(row_height - bar_height) / 2` as the inset.
#[must_use]
pub fn bar_pixel_rect(
    bar: &Bar,
    gantt: &Gantt,
    theme: &Theme,
    viewport_width: f32,
) -> Option<PixelRect> {
    let row_index = gantt.rows.iter().position(|r| r.id == bar.row_id)?;
    let x_start = date_to_x(
        bar.range.start,
        gantt.range,
        theme.gantt.gutter_width,
        viewport_width,
    );
    let x_end = date_to_x(
        bar.range.end,
        gantt.range,
        theme.gantt.gutter_width,
        viewport_width,
    );
    let y_top =
        row_top_y(row_index, theme) + (theme.gantt.row_height - theme.gantt.bar_height) * 0.5;
    Some(PixelRect {
        x: x_start,
        y: y_top,
        w: (x_end - x_start).max(0.0),
        h: theme.gantt.bar_height,
    })
}

/// Pixel-space rectangle, top-left origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelRect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gantt::{Bar, Row};
    use jiff::civil::date;

    fn year_2026() -> DateRange {
        DateRange::year(2026)
    }

    fn light() -> Theme {
        Theme::light()
    }

    #[test]
    fn days_between_full_year_is_365() {
        assert_eq!(days_between(date(2026, 1, 1), date(2027, 1, 1)), 365);
    }

    #[test]
    fn days_between_negative_when_reversed() {
        assert_eq!(days_between(date(2027, 1, 1), date(2026, 1, 1)), -365);
    }

    #[test]
    fn date_fraction_jan1_is_zero() {
        let f = date_fraction(date(2026, 1, 1), year_2026());
        assert!(f.abs() < 1e-6);
    }

    #[test]
    fn date_fraction_dec31_close_to_one() {
        let f = date_fraction(date(2026, 12, 31), year_2026());
        // 364/365
        assert!((f - 364.0 / 365.0).abs() < 1e-5);
    }

    #[test]
    fn date_fraction_clamps_outside_range() {
        assert!(date_fraction(date(2024, 1, 1), year_2026()).abs() < 1e-6);
        assert!((date_fraction(date(2028, 1, 1), year_2026()) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn date_to_x_starts_at_gutter() {
        let x = date_to_x(date(2026, 1, 1), year_2026(), 180.0, 1920.0);
        assert!((x - 180.0).abs() < 1e-4);
    }

    #[test]
    fn date_to_x_end_is_full_viewport_minus_one_day() {
        let x = date_to_x(date(2026, 12, 31), year_2026(), 180.0, 1920.0);
        let expected = 180.0 + (364.0 / 365.0) * (1920.0 - 180.0);
        assert!((x - expected).abs() < 1e-3);
    }

    #[test]
    fn row_top_y_offsets_by_header() {
        let theme = light();
        assert!((row_top_y(0, &theme) - theme.gantt.header_height).abs() < 1e-6);
        assert!(
            (row_top_y(2, &theme) - (theme.gantt.header_height + 2.0 * theme.gantt.row_height))
                .abs()
                < 1e-6
        );
    }

    #[test]
    fn bar_pixel_rect_centred_in_row() {
        let theme = light();
        let mut gantt = Gantt {
            range: year_2026(),
            rows: vec![Row::new("a", "A"), Row::new("b", "B")],
            bars: vec![],
            people: crate::PersonMap::default(),
        };
        gantt
            .bars
            .push(Bar::new("b", date(2026, 2, 1)..date(2026, 3, 1), "Matt"));
        let bar = &gantt.bars[0];
        let rect = bar_pixel_rect(bar, &gantt, &theme, 1920.0).expect("row resolves");
        // Row 1, y centred: 60 + 44 + (44 - 28) / 2 = 112.
        assert!((rect.y - 112.0).abs() < 1e-4);
        assert!((rect.h - theme.gantt.bar_height).abs() < 1e-6);
        // Width > 0 — the bar spans 28 days, so a sliver of the
        // plot width.
        assert!(rect.w > 0.0);
    }

    #[test]
    fn bar_pixel_rect_unknown_row_returns_none() {
        let theme = light();
        let gantt = Gantt {
            range: year_2026(),
            rows: vec![Row::new("a", "A")],
            bars: vec![Bar::new(
                "ghost",
                date(2026, 2, 1)..date(2026, 3, 1),
                "Matt",
            )],
            people: crate::PersonMap::default(),
        };
        assert!(bar_pixel_rect(&gantt.bars[0], &gantt, &theme, 1920.0).is_none());
    }
}
