//! Logarithmic scale + 1/2/5-per-decade tick generator.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "decade exponents fit in i32 trivially; small powi exponents stay within f32."
)]

use super::Tick;

/// Log scale — maps `f32` domain values (must be strictly
/// positive) to a continuous `f32` range via `log_base`.
///
/// Default base is 10 (the most common chart use). Other bases
/// are accepted but the tick generator's 1/2/5 cadence is tuned
/// for base 10; with non-decimal bases the ticks land at decade
/// equivalents in the chosen base.
///
/// # Example
///
/// ```
/// use wisp_chart::scale::LogScale;
/// let s = LogScale::new((1.0, 1_000.0), (0.0, 600.0));
/// // log10(1) = 0 → range_min; log10(1000) = 3 → range_max.
/// assert!((s.map(1.0) - 0.0).abs() < 1e-4);
/// assert!((s.map(1_000.0) - 600.0).abs() < 1e-4);
/// // log10(10) = 1 / 3 of the way through.
/// assert!((s.map(10.0) - 200.0).abs() < 1.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogScale {
    domain: (f32, f32),
    range: (f32, f32),
    base: f32,
}

impl LogScale {
    /// Construct with base 10. Both domain endpoints must be
    /// strictly positive; zero / negative values produce
    /// undefined behaviour (the renderer will not validate).
    #[must_use]
    pub fn new(domain: (f32, f32), range: (f32, f32)) -> Self {
        Self {
            domain,
            range,
            base: 10.0,
        }
    }

    /// Builder: override the logarithm base (default 10).
    #[must_use]
    pub fn base(mut self, base: f32) -> Self {
        self.base = base;
        self
    }

    /// Project a domain value into the range.
    #[must_use]
    pub fn map(&self, value: f32) -> f32 {
        let v = value.max(f32::MIN_POSITIVE);
        let (d0, d1) = self.domain;
        let log_d0 = d0.max(f32::MIN_POSITIVE).log(self.base);
        let log_d1 = d1.max(f32::MIN_POSITIVE).log(self.base);
        let log_v = v.log(self.base);
        let span = log_d1 - log_d0;
        if span.abs() < f32::EPSILON {
            return self.range.0;
        }
        let t = (log_v - log_d0) / span;
        let (r0, r1) = self.range;
        r0 + t * (r1 - r0)
    }

    /// Generate ticks at each decade boundary plus the 1/2/5
    /// minor stops inside each decade. Returns in ascending
    /// domain order, bounded by the configured domain.
    #[must_use]
    pub fn ticks(&self) -> Vec<Tick<f32>> {
        let (lo, hi) = if self.domain.0 <= self.domain.1 {
            self.domain
        } else {
            (self.domain.1, self.domain.0)
        };
        if lo <= 0.0 || hi <= 0.0 {
            return Vec::new();
        }
        let log_lo = lo.log(self.base).floor() as i32;
        let log_hi = hi.log(self.base).ceil() as i32;
        let mut out = Vec::new();
        for decade in log_lo..=log_hi {
            for mantissa in [1.0_f32, 2.0, 5.0] {
                let v = mantissa * self.base.powi(decade);
                if v >= lo && v <= hi {
                    out.push(Tick {
                        value: v,
                        position: self.map(v),
                    });
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_endpoints_match_range() {
        let s = LogScale::new((1.0, 1000.0), (0.0, 600.0));
        assert!((s.map(1.0) - 0.0).abs() < 1e-4);
        assert!((s.map(1000.0) - 600.0).abs() < 1e-4);
    }

    #[test]
    fn map_10x_step_advances_by_one_decade_fraction() {
        // 3 decades total; each decade is 200 px wide.
        let s = LogScale::new((1.0, 1000.0), (0.0, 600.0));
        assert!((s.map(10.0) - 200.0).abs() < 1.0);
        assert!((s.map(100.0) - 400.0).abs() < 1.0);
    }

    #[test]
    fn ticks_for_1_to_1000_includes_decade_stops() {
        let s = LogScale::new((1.0, 1000.0), (0.0, 600.0));
        let values: Vec<f32> = s.ticks().iter().map(|t| t.value).collect();
        for v in [1.0, 10.0, 100.0, 1000.0] {
            assert!(values.contains(&v), "{v} not in {values:?}");
        }
    }

    #[test]
    fn ticks_include_intermediate_2_and_5_per_decade() {
        let s = LogScale::new((1.0, 100.0), (0.0, 400.0));
        let values: Vec<f32> = s.ticks().iter().map(|t| t.value).collect();
        for v in [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0] {
            assert!(values.contains(&v), "{v} not in {values:?}");
        }
    }

    #[test]
    fn ticks_for_non_decimal_base_2() {
        let s = LogScale::new((1.0, 16.0), (0.0, 400.0)).base(2.0);
        // 1, 2, 4, 8, 16 are decade boundaries in base 2.
        let values: Vec<f32> = s.ticks().iter().map(|t| t.value).collect();
        for v in [1.0, 2.0, 4.0, 8.0, 16.0] {
            assert!(values.contains(&v), "{v} not in {values:?}");
        }
    }

    #[test]
    fn tick_positions_match_map() {
        let s = LogScale::new((1.0, 1000.0), (0.0, 600.0));
        for t in s.ticks() {
            let projected = s.map(t.value);
            assert!((t.position - projected).abs() < 1e-3);
        }
    }
}
