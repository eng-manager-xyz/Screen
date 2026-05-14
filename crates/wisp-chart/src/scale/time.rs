//! Time scale — `jiff::civil::Date` → continuous `f32` range
//! with multi-unit tick generation (year / month / week / day).

use jiff::civil::Date;

use crate::gantt::{DateRange, layout};

use super::Tick;

/// Tick granularity for time scales. The renderer picks one based
/// on the range's span and the requested tick density, or the
/// caller can pin it explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeUnit {
    /// One tick per January 1st in range.
    Year,
    /// One tick per month-start in range.
    Month,
    /// One tick per Monday in range.
    Week,
    /// One tick per calendar day in range.
    Day,
}

/// Maps a [`DateRange`] to a continuous pixel range. Reuses
/// [`crate::gantt::layout::date_fraction`] for the projection so
/// every consumer (Gantt, line-chart-with-time-x, candlestick)
/// agrees on the same arithmetic.
///
/// # Example
///
/// ```
/// use jiff::civil::date;
/// use wisp_chart::gantt::DateRange;
/// use wisp_chart::scale::TimeScale;
/// let scale = TimeScale::new(DateRange::year(2026), (180.0, 960.0));
/// // Jan 1, 2026 → start of range.
/// assert!((scale.map(date(2026, 1, 1)) - 180.0).abs() < 1e-4);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeScale {
    range: DateRange,
    pixel_range: (f32, f32),
}

impl TimeScale {
    /// Construct from a [`DateRange`] and pixel `(range_min,
    /// range_max)` bounds.
    #[must_use]
    pub fn new(range: DateRange, pixel_range: (f32, f32)) -> Self {
        Self { range, pixel_range }
    }

    /// The date domain.
    #[must_use]
    pub fn domain(&self) -> DateRange {
        self.range
    }

    /// Project a date to a pixel coordinate. Dates outside the
    /// range clamp to the nearest edge (per `date_fraction`).
    #[must_use]
    pub fn map(&self, date: Date) -> f32 {
        let f = layout::date_fraction(date, self.range);
        let (a, b) = self.pixel_range;
        a + f * (b - a)
    }

    /// Generate ticks at the given [`TimeUnit`] cadence. Returns
    /// in chronological order; the projected `position` is the
    /// pixel coordinate.
    #[must_use]
    pub fn ticks_at(&self, unit: TimeUnit) -> Vec<Tick<Date>> {
        match unit {
            TimeUnit::Year => self.ticks_year(),
            TimeUnit::Month => self.ticks_month(),
            TimeUnit::Week => self.ticks_week(),
            TimeUnit::Day => self.ticks_day(),
        }
    }

    /// Pick a reasonable [`TimeUnit`] for the given density hint.
    /// Heuristic:
    /// * Total span ≥ 5 years → `Year`.
    /// * Span ≥ 6 months → `Month`.
    /// * Span ≥ 21 days → `Week`.
    /// * Otherwise `Day`.
    #[must_use]
    pub fn pick_unit(&self, _count_hint: usize) -> TimeUnit {
        let days = layout::days_between(self.range.start, self.range.end).max(0);
        if days >= 365 * 5 {
            TimeUnit::Year
        } else if days >= 180 {
            TimeUnit::Month
        } else if days >= 21 {
            TimeUnit::Week
        } else {
            TimeUnit::Day
        }
    }

    fn ticks_year(&self) -> Vec<Tick<Date>> {
        let mut out = Vec::new();
        let mut y = self.range.start.year();
        loop {
            let Ok(d) = Date::new(y, 1, 1) else { break };
            if d >= self.range.end {
                break;
            }
            if d >= self.range.start {
                out.push(Tick {
                    value: d,
                    position: self.map(d),
                });
            }
            y += 1;
        }
        out
    }

    fn ticks_month(&self) -> Vec<Tick<Date>> {
        let mut out = Vec::new();
        let mut y = self.range.start.year();
        let mut m = self.range.start.month();
        // Advance to the first month-start ≥ range.start.
        let first = Date::new(y, m, 1).unwrap_or(self.range.start);
        if first < self.range.start {
            m += 1;
            if m > 12 {
                m = 1;
                y += 1;
            }
        }
        loop {
            let Ok(d) = Date::new(y, m, 1) else { break };
            if d >= self.range.end {
                break;
            }
            out.push(Tick {
                value: d,
                position: self.map(d),
            });
            m += 1;
            if m > 12 {
                m = 1;
                y += 1;
            }
        }
        out
    }

    fn ticks_week(&self) -> Vec<Tick<Date>> {
        // Mondays. jiff's `weekday()` returns ISO weekday (1=Mon).
        let mut out = Vec::new();
        let mut d = self.range.start;
        while d.weekday().to_monday_zero_offset() != 0 {
            d = match d.tomorrow() {
                Ok(d) => d,
                Err(_) => return out,
            };
            if d >= self.range.end {
                return out;
            }
        }
        while d < self.range.end {
            out.push(Tick {
                value: d,
                position: self.map(d),
            });
            // Advance 7 days.
            for _ in 0..7 {
                d = match d.tomorrow() {
                    Ok(d) => d,
                    Err(_) => return out,
                };
            }
        }
        out
    }

    fn ticks_day(&self) -> Vec<Tick<Date>> {
        let mut out = Vec::new();
        let mut d = self.range.start;
        while d < self.range.end {
            out.push(Tick {
                value: d,
                position: self.map(d),
            });
            d = match d.tomorrow() {
                Ok(d) => d,
                Err(_) => return out,
            };
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn map_start_of_range_is_pixel_min() {
        let s = TimeScale::new(DateRange::year(2026), (180.0, 960.0));
        assert!((s.map(date(2026, 1, 1)) - 180.0).abs() < 1e-4);
    }

    #[test]
    fn map_dec31_close_to_pixel_max() {
        let s = TimeScale::new(DateRange::year(2026), (180.0, 960.0));
        let x = s.map(date(2026, 12, 31));
        // 364/365 of the range.
        let expected = 180.0 + (364.0 / 365.0) * (960.0 - 180.0);
        assert!((x - expected).abs() < 1e-3);
    }

    #[test]
    fn month_ticks_for_full_year_produces_12() {
        let s = TimeScale::new(DateRange::year(2026), (0.0, 960.0));
        let ticks = s.ticks_at(TimeUnit::Month);
        assert_eq!(ticks.len(), 12, "expected 12 month-starts in 2026");
        // First tick is Jan 1.
        assert_eq!(ticks[0].value, date(2026, 1, 1));
        // Last is Dec 1.
        assert_eq!(ticks[11].value, date(2026, 12, 1));
    }

    #[test]
    fn year_ticks_for_5_year_span() {
        let range = DateRange::from_range(date(2024, 6, 15)..date(2029, 3, 1));
        let s = TimeScale::new(range, (0.0, 1000.0));
        let ticks = s.ticks_at(TimeUnit::Year);
        // Year-starts in range: 2025, 2026, 2027, 2028, 2029.
        // 2024-01-01 is BEFORE range.start (2024-06-15) so it's
        // skipped.
        assert_eq!(ticks.len(), 5);
        assert_eq!(ticks[0].value, date(2025, 1, 1));
        assert_eq!(ticks[4].value, date(2029, 1, 1));
    }

    #[test]
    fn week_ticks_align_with_mondays() {
        let s = TimeScale::new(
            DateRange::from_range(date(2026, 1, 1)..date(2026, 2, 1)),
            (0.0, 1000.0),
        );
        let ticks = s.ticks_at(TimeUnit::Week);
        for t in &ticks {
            assert_eq!(
                t.value.weekday().to_monday_zero_offset(),
                0,
                "{} is not a Monday",
                t.value
            );
        }
    }

    #[test]
    fn day_ticks_for_one_week_produces_seven() {
        let s = TimeScale::new(
            DateRange::from_range(date(2026, 6, 1)..date(2026, 6, 8)),
            (0.0, 700.0),
        );
        let ticks = s.ticks_at(TimeUnit::Day);
        assert_eq!(ticks.len(), 7);
    }

    #[test]
    fn pick_unit_uses_year_for_5_year_span() {
        let s = TimeScale::new(
            DateRange::from_range(date(2020, 1, 1)..date(2030, 1, 1)),
            (0.0, 1000.0),
        );
        assert_eq!(s.pick_unit(8), TimeUnit::Year);
    }

    #[test]
    fn pick_unit_uses_month_for_year_span() {
        let s = TimeScale::new(DateRange::year(2026), (0.0, 1000.0));
        assert_eq!(s.pick_unit(8), TimeUnit::Month);
    }

    #[test]
    fn pick_unit_uses_week_for_30_day_span() {
        let s = TimeScale::new(
            DateRange::from_range(date(2026, 6, 1)..date(2026, 7, 1)),
            (0.0, 1000.0),
        );
        assert_eq!(s.pick_unit(8), TimeUnit::Week);
    }

    #[test]
    fn pick_unit_uses_day_for_one_week() {
        let s = TimeScale::new(
            DateRange::from_range(date(2026, 6, 1)..date(2026, 6, 8)),
            (0.0, 700.0),
        );
        assert_eq!(s.pick_unit(8), TimeUnit::Day);
    }

    #[test]
    fn tick_positions_match_map() {
        let s = TimeScale::new(DateRange::year(2026), (0.0, 960.0));
        for t in s.ticks_at(TimeUnit::Month) {
            assert!((t.position - s.map(t.value)).abs() < 1e-3);
        }
    }
}
