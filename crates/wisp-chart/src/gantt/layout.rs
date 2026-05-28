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

/// Vertical gap (in pixels) between stacked lanes inside a
/// multi-lane row. Matched empirically to the H2 planning DOM:
/// 4 px reads as "the same row, just two people" rather than
/// "two separate rows."
pub const LANE_GAP_PX: f32 = 4.0;

/// Vertical padding above + below the lane stack inside a row.
pub const LANE_PADDING_PX: f32 = 4.0;

/// How many distinct lanes a row uses, derived from the
/// `bar.lane.unwrap_or(0)` values of bars referencing that row's id.
///
/// Returns `1` for rows with no bars (so the layout still
/// reserves a default-height empty row).
#[must_use]
pub fn row_lane_count(gantt: &Gantt, row_idx: usize) -> u16 {
    let Some(row) = gantt.rows.get(row_idx) else {
        return 1;
    };
    let max_lane = gantt
        .bars
        .iter()
        .filter(|b| b.row_id == row.id)
        .map(|b| b.lane.unwrap_or(0))
        .max();
    match max_lane {
        Some(m) => m.saturating_add(1).max(1),
        None => 1,
    }
}

/// Pixel height of a row, accounting for lane stacking. A row
/// with one lane uses `theme.gantt.row_height` (unchanged).
/// A row with `N > 1` lanes uses
/// `N * (bar_height + LANE_GAP_PX) - LANE_GAP_PX + 2 * LANE_PADDING_PX`.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "lane count is u16; well within f32 mantissa for any plausible row"
)]
pub fn row_height_for_row(gantt: &Gantt, row_idx: usize, theme: &Theme) -> f32 {
    let lanes = row_lane_count(gantt, row_idx);
    if lanes <= 1 {
        theme.gantt.row_height
    } else {
        let lanes_f = f32::from(lanes);
        lanes_f * (theme.gantt.bar_height + LANE_GAP_PX) - LANE_GAP_PX + 2.0 * LANE_PADDING_PX
    }
}

/// Top y of row `row_idx`, walking rows `0..row_idx` and summing
/// each row's dynamic height (so multi-lane rows above push
/// subsequent rows down). Includes the frozen header.
#[must_use]
pub fn dynamic_row_top_y(gantt: &Gantt, row_idx: usize, theme: &Theme) -> f32 {
    let mut y = theme.gantt.header_height;
    for i in 0..row_idx {
        y += row_height_for_row(gantt, i, theme);
    }
    y
}

/// Lane-aware pixel rect for `bar`. The bar's y is placed inside
/// its assigned lane stack — lane 0 sits at the top of the row
/// (after `LANE_PADDING_PX`), lane 1 below it, etc.
///
/// Returns `None` if the bar's `row_id` doesn't match any row.
#[must_use]
pub fn bar_pixel_rect_laned(
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
    let lane = bar.lane.unwrap_or(0);
    let lane_count = row_lane_count(gantt, row_index);
    let row_top = dynamic_row_top_y(gantt, row_index, theme);
    let y_top = if lane_count <= 1 {
        // Single lane: centre in the row exactly like
        // `bar_pixel_rect` does — preserves visual parity.
        row_top + (theme.gantt.row_height - theme.gantt.bar_height) * 0.5
    } else {
        row_top + LANE_PADDING_PX + f32::from(lane) * (theme.gantt.bar_height + LANE_GAP_PX)
    };
    Some(PixelRect {
        x: x_start,
        y: y_top,
        w: (x_end - x_start).max(0.0),
        h: theme.gantt.bar_height,
    })
}

/// Split `range` into 7-day buckets, anchored at `range.start`.
/// The final bucket may be shorter if `range`'s length isn't a
/// multiple of 7.
///
/// Used by [`crate::Gantt::cell_hit_regions`] (AUT-318) to emit
/// one pickable region per timeline week.
#[must_use]
pub fn weeks_in_range(range: DateRange) -> Vec<DateRange> {
    let total_days = days_between(range.start, range.end).max(0);
    if total_days == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut cursor = range.start;
    let mut remaining = total_days;
    while remaining > 0 {
        let chunk = remaining.min(7);
        let next = cursor
            .checked_add(jiff::Span::new().days(chunk))
            .unwrap_or(range.end);
        out.push(DateRange {
            start: cursor,
            end: next,
        });
        cursor = next;
        remaining -= chunk;
    }
    out
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
            markers: Vec::new(),
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
    fn row_lane_count_defaults_to_one_for_row_with_no_bars() {
        let g = Gantt {
            range: year_2026(),
            rows: vec![Row::new("a", "A")],
            bars: vec![],
            people: crate::PersonMap::default(),
            markers: Vec::new(),
        };
        assert_eq!(row_lane_count(&g, 0), 1);
    }

    #[test]
    fn row_lane_count_returns_max_lane_plus_one() {
        let g = Gantt {
            range: year_2026(),
            rows: vec![Row::new("vec", "M-VEC")],
            bars: vec![
                Bar::new("vec", date(2026, 2, 1)..date(2026, 3, 1), "Matt").with_lane(0),
                Bar::new("vec", date(2026, 2, 1)..date(2026, 3, 1), "Alice").with_lane(1),
                Bar::new("vec", date(2026, 2, 1)..date(2026, 3, 1), "Bob").with_lane(2),
            ],
            people: crate::PersonMap::default(),
            markers: Vec::new(),
        };
        assert_eq!(row_lane_count(&g, 0), 3);
    }

    #[test]
    fn row_height_grows_linearly_with_lane_count() {
        let theme = light();
        let g1 = Gantt {
            range: year_2026(),
            rows: vec![Row::new("a", "A")],
            bars: vec![Bar::new("a", date(2026, 2, 1)..date(2026, 3, 1), "Matt")],
            people: crate::PersonMap::default(),
            markers: Vec::new(),
        };
        let g3 = Gantt {
            bars: vec![
                Bar::new("a", date(2026, 2, 1)..date(2026, 3, 1), "Matt").with_lane(0),
                Bar::new("a", date(2026, 2, 1)..date(2026, 3, 1), "Alice").with_lane(1),
                Bar::new("a", date(2026, 2, 1)..date(2026, 3, 1), "Bob").with_lane(2),
            ],
            ..g1.clone()
        };
        let h1 = row_height_for_row(&g1, 0, &theme);
        let h3 = row_height_for_row(&g3, 0, &theme);
        assert!((h1 - theme.gantt.row_height).abs() < 1e-4);
        // 3 lanes of bar_height stacked with LANE_GAP_PX gaps + padding.
        let expected_h3 =
            3.0 * (theme.gantt.bar_height + LANE_GAP_PX) - LANE_GAP_PX + 2.0 * LANE_PADDING_PX;
        assert!((h3 - expected_h3).abs() < 1e-4);
        assert!(h3 > h1, "3-lane row taller than 1-lane row");
    }

    #[test]
    fn dynamic_row_top_y_accounts_for_above_row_heights() {
        let theme = light();
        // Two rows: first has 2 lanes (taller), second comes after.
        let g = Gantt {
            range: year_2026(),
            rows: vec![Row::new("a", "A"), Row::new("b", "B")],
            bars: vec![
                Bar::new("a", date(2026, 2, 1)..date(2026, 3, 1), "Matt").with_lane(0),
                Bar::new("a", date(2026, 2, 1)..date(2026, 3, 1), "Alice").with_lane(1),
                Bar::new("b", date(2026, 2, 1)..date(2026, 3, 1), "Bob"),
            ],
            people: crate::PersonMap::default(),
            markers: Vec::new(),
        };
        let row0_top = dynamic_row_top_y(&g, 0, &theme);
        let row1_top = dynamic_row_top_y(&g, 1, &theme);
        assert!((row0_top - theme.gantt.header_height).abs() < 1e-4);
        // Row 1 starts row_height_for_row(row 0) below row 0.
        let expected_row1 = theme.gantt.header_height + row_height_for_row(&g, 0, &theme);
        assert!((row1_top - expected_row1).abs() < 1e-4);
        // Row 1 sits BELOW the single-lane row_height for row 0.
        assert!(row1_top > theme.gantt.header_height + theme.gantt.row_height - 0.5);
    }

    #[test]
    fn bar_pixel_rect_laned_stacks_lanes_vertically() {
        let theme = light();
        let g = Gantt {
            range: year_2026(),
            rows: vec![Row::new("a", "A")],
            bars: vec![
                Bar::new("a", date(2026, 2, 1)..date(2026, 3, 1), "Matt").with_lane(0),
                Bar::new("a", date(2026, 2, 1)..date(2026, 3, 1), "Alice").with_lane(1),
            ],
            people: crate::PersonMap::default(),
            markers: Vec::new(),
        };
        let r0 = bar_pixel_rect_laned(&g.bars[0], &g, &theme, 1920.0).unwrap();
        let r1 = bar_pixel_rect_laned(&g.bars[1], &g, &theme, 1920.0).unwrap();
        // Lane 1 sits LANE_GAP_PX + bar_height below lane 0.
        let dy = r1.y - r0.y;
        assert!(
            (dy - (theme.gantt.bar_height + LANE_GAP_PX)).abs() < 1e-4,
            "expected lane gap, got dy={dy}"
        );
        // Same x range.
        assert!((r0.x - r1.x).abs() < 1e-4);
        assert!((r0.w - r1.w).abs() < 1e-4);
    }

    #[test]
    fn bar_pixel_rect_laned_single_lane_matches_centred_v1() {
        let theme = light();
        let g = Gantt {
            range: year_2026(),
            rows: vec![Row::new("a", "A")],
            bars: vec![Bar::new("a", date(2026, 2, 1)..date(2026, 3, 1), "Matt")],
            people: crate::PersonMap::default(),
            markers: Vec::new(),
        };
        let r_laned = bar_pixel_rect_laned(&g.bars[0], &g, &theme, 1920.0).unwrap();
        let r_v1 = bar_pixel_rect(&g.bars[0], &g, &theme, 1920.0).unwrap();
        // Single lane in WG.3 stays centred — matches v1 exactly.
        assert!((r_laned.y - r_v1.y).abs() < 1e-4);
        assert!((r_laned.h - r_v1.h).abs() < 1e-4);
    }

    #[test]
    fn weeks_in_range_year_emits_53_buckets() {
        // 365 days = 52 full weeks + 1 partial week.
        let weeks = weeks_in_range(year_2026());
        assert_eq!(weeks.len(), 53);
        // First bucket starts on Jan 1.
        assert_eq!(weeks[0].start, date(2026, 1, 1));
        // Last bucket ends on Dec 31 / Jan 1 of next year.
        assert_eq!(weeks.last().unwrap().end, date(2027, 1, 1));
    }

    #[test]
    fn weeks_in_range_empty_for_zero_day_range() {
        let r = DateRange::from_range(date(2026, 1, 1)..date(2026, 1, 1));
        assert!(weeks_in_range(r).is_empty());
    }

    #[test]
    fn weeks_in_range_handles_under_seven_days() {
        let r = DateRange::from_range(date(2026, 1, 1)..date(2026, 1, 4));
        let weeks = weeks_in_range(r);
        assert_eq!(weeks.len(), 1);
        // Single 3-day bucket — shorter than 7.
        assert_eq!(weeks[0].start, date(2026, 1, 1));
        assert_eq!(weeks[0].end, date(2026, 1, 4));
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
            markers: Vec::new(),
        };
        assert!(bar_pixel_rect(&gantt.bars[0], &gantt, &theme, 1920.0).is_none());
    }
}
