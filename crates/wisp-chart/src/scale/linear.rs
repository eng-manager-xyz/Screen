//! Linear scale + nice-tick generator.

#![allow(
    clippy::cast_precision_loss,
    reason = "tick count_hint is small (typically ≤ 20) — well below f32 precision."
)]

use super::Tick;

/// Linear scale — maps a continuous `f32` domain to a continuous
/// `f32` range.
///
/// # Example
///
/// ```
/// use wisp_chart::scale::LinearScale;
/// let x = LinearScale::new((0.0, 100.0), (0.0, 960.0));
/// assert!((x.map(50.0) - 480.0).abs() < 1e-4);
/// assert!((x.map(0.0) - 0.0).abs() < 1e-6);
/// assert!((x.map(100.0) - 960.0).abs() < 1e-6);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearScale {
    domain: (f32, f32),
    range: (f32, f32),
}

impl LinearScale {
    /// Construct from a domain `(min, max)` and range `(min, max)`.
    ///
    /// Both pairs may be reversed (`max < min`) — useful for Y
    /// axes where the pixel range is top-down but the data domain
    /// is bottom-up. The mapping stays linear in either direction.
    #[must_use]
    pub fn new(domain: (f32, f32), range: (f32, f32)) -> Self {
        Self { domain, range }
    }

    /// Domain accessor (returned in the order it was constructed).
    #[must_use]
    pub fn domain(&self) -> (f32, f32) {
        self.domain
    }

    /// Range accessor (returned in the order it was constructed).
    #[must_use]
    pub fn range(&self) -> (f32, f32) {
        self.range
    }

    /// Project a domain value into the range.
    ///
    /// Returns the range start when the domain has zero width
    /// (avoids divide-by-zero NaN).
    #[must_use]
    pub fn map(&self, value: f32) -> f32 {
        let (d0, d1) = self.domain;
        let (r0, r1) = self.range;
        let span = d1 - d0;
        if span.abs() < f32::EPSILON {
            return r0;
        }
        let t = (value - d0) / span;
        r0 + t * (r1 - r0)
    }

    /// Generate "nice" tick stops near the requested count.
    ///
    /// Uses the d3-style 1/2/5 cadence at decade granularity.
    /// Returned ticks fall inside `[min(domain), max(domain)]`
    /// (clamped if the nice-stop algorithm would step outside) and
    /// are sorted ascending in domain order.
    ///
    /// `count_hint` is a target, not a guarantee — the generator
    /// trades exactness for round-numbered stops.
    #[must_use]
    pub fn ticks(&self, count_hint: usize) -> Vec<Tick<f32>> {
        let (d0, d1) = self.domain;
        let lo = d0.min(d1);
        let hi = d0.max(d1);
        if (hi - lo).abs() < f32::EPSILON || count_hint == 0 {
            return Vec::new();
        }
        let step = nice_step((hi - lo) / count_hint as f32);
        let first = (lo / step).ceil() * step;
        let mut out = Vec::new();
        let mut v = first;
        let epsilon = step * 1e-4;
        while v <= hi + epsilon {
            if v >= lo - epsilon {
                out.push(Tick {
                    value: v,
                    position: self.map(v),
                });
            }
            v += step;
        }
        out
    }
}

/// d3-style nice step rounding — pick a multiple of `1 * 10^k`,
/// `2 * 10^k`, or `5 * 10^k` closest to the requested step.
fn nice_step(raw_step: f32) -> f32 {
    if raw_step <= 0.0 {
        return 1.0;
    }
    let exp = raw_step.log10().floor();
    let pow10 = 10f32.powf(exp);
    let frac = raw_step / pow10;
    let nice_frac = if frac <= 1.5 {
        1.0
    } else if frac <= 3.0 {
        2.0
    } else if frac <= 7.0 {
        5.0
    } else {
        10.0
    };
    nice_frac * pow10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_zero_maps_to_range_start() {
        let s = LinearScale::new((0.0, 100.0), (0.0, 960.0));
        assert!((s.map(0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn map_domain_max_maps_to_range_max() {
        let s = LinearScale::new((0.0, 100.0), (0.0, 960.0));
        assert!((s.map(100.0) - 960.0).abs() < 1e-4);
    }

    #[test]
    fn map_midpoint_is_linear() {
        let s = LinearScale::new((10.0, 30.0), (200.0, 600.0));
        assert!((s.map(20.0) - 400.0).abs() < 1e-4);
    }

    #[test]
    fn map_handles_reversed_range() {
        let s = LinearScale::new((0.0, 100.0), (600.0, 60.0));
        // Y-axis flipped: domain min → pixel bottom (600), max → top (60).
        assert!((s.map(0.0) - 600.0).abs() < 1e-4);
        assert!((s.map(100.0) - 60.0).abs() < 1e-4);
        assert!((s.map(50.0) - 330.0).abs() < 1e-4);
    }

    #[test]
    fn zero_width_domain_returns_range_start() {
        let s = LinearScale::new((5.0, 5.0), (0.0, 100.0));
        assert!((s.map(5.0) - 0.0).abs() < 1e-6);
        assert!((s.map(10.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn ticks_for_0_to_100_produces_round_stops() {
        let s = LinearScale::new((0.0, 100.0), (0.0, 960.0));
        let ticks: Vec<f32> = s.ticks(10).iter().map(|t| t.value).collect();
        // 1/2/5 cadence at decade 10 → step 10 → 0, 10, ..., 100.
        assert!(ticks.contains(&0.0));
        assert!(ticks.contains(&50.0));
        assert!(ticks.contains(&100.0));
    }

    #[test]
    fn ticks_for_0_to_73_chooses_step_10() {
        // 73/8 ≈ 9.1 → nice step is 10.
        let s = LinearScale::new((0.0, 73.0), (0.0, 960.0));
        let ticks: Vec<f32> = s.ticks(8).iter().map(|t| t.value).collect();
        for v in [0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0] {
            assert!(ticks.contains(&v), "expected {v} in {ticks:?}");
        }
    }

    #[test]
    fn ticks_monotonic_and_bounded() {
        let s = LinearScale::new((-50.0, 150.0), (0.0, 100.0));
        let ticks = s.ticks(8);
        for w in ticks.windows(2) {
            assert!(w[0].value < w[1].value);
        }
        for t in &ticks {
            assert!(t.value >= -50.0 && t.value <= 150.0);
        }
    }

    #[test]
    fn ticks_zero_count_hint_returns_empty() {
        let s = LinearScale::new((0.0, 100.0), (0.0, 1000.0));
        assert!(s.ticks(0).is_empty());
    }

    #[test]
    fn tick_positions_match_map() {
        let s = LinearScale::new((0.0, 100.0), (0.0, 960.0));
        for t in s.ticks(10) {
            let projected = s.map(t.value);
            assert!((t.position - projected).abs() < 1e-4);
        }
    }
}
