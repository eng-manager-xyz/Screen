//! Sequential palette — maps `[0, 1]` to a colour via a list of
//! evenly-spaced stops. Linear interpolation between adjacent
//! stops.

use crate::color::Color as ChartColor;

/// Sequential colour palette — stops at evenly-spaced positions
/// in `[0, 1]`.
#[derive(Clone, Debug)]
pub struct SequentialPalette {
    stops: Vec<ChartColor>,
}

impl SequentialPalette {
    /// Construct from a list of stops. Empty input is replaced
    /// with a 2-stop white→black palette so `sample` never
    /// panics.
    #[must_use]
    pub fn new(stops: Vec<ChartColor>) -> Self {
        if stops.is_empty() {
            return Self {
                stops: vec![
                    ChartColor::from_hex("#ffffff").unwrap(),
                    ChartColor::from_hex("#000000").unwrap(),
                ],
            };
        }
        Self { stops }
    }

    /// 3-stop blue palette — light blue → mid blue → dark blue.
    /// The default for table heatmaps.
    #[must_use]
    pub fn blues() -> Self {
        Self::new(vec![
            ChartColor::from_hex("#deebf7").unwrap(),
            ChartColor::from_hex("#6baed6").unwrap(),
            ChartColor::from_hex("#08519c").unwrap(),
        ])
    }

    /// 4-stop GitHub-style contribution-graph palette.
    #[must_use]
    pub fn github() -> Self {
        Self::new(vec![
            ChartColor::from_hex("#ebedf0").unwrap(),
            ChartColor::from_hex("#9be9a8").unwrap(),
            ChartColor::from_hex("#40c463").unwrap(),
            ChartColor::from_hex("#30a14e").unwrap(),
            ChartColor::from_hex("#216e39").unwrap(),
        ])
    }

    /// Magma-style 4-stop heat palette.
    #[must_use]
    pub fn magma() -> Self {
        Self::new(vec![
            ChartColor::from_hex("#fcfdbf").unwrap(),
            ChartColor::from_hex("#fc8961").unwrap(),
            ChartColor::from_hex("#b73779").unwrap(),
            ChartColor::from_hex("#51127c").unwrap(),
            ChartColor::from_hex("#000004").unwrap(),
        ])
    }

    /// Sample at `t in [0, 1]`. Clamped before lookup.
    #[must_use]
    pub fn sample(&self, t: f32) -> ChartColor {
        let t = t.clamp(0.0, 1.0);
        if self.stops.len() == 1 {
            return self.stops[0];
        }
        let n = self.stops.len() - 1;
        #[allow(
            clippy::cast_precision_loss,
            reason = "palette stops ≤ ~10 in practice"
        )]
        let scaled = t * n as f32;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "scaled is in [0, n] which fits usize"
        )]
        let idx = (scaled.floor() as usize).min(n.saturating_sub(1));
        let local = scaled - {
            #[allow(clippy::cast_precision_loss, reason = "idx ≤ ~10 fits f32 mantissa")]
            {
                idx as f32
            }
        };
        lerp(self.stops[idx], self.stops[idx + 1], local)
    }
}

impl Default for SequentialPalette {
    fn default() -> Self {
        Self::blues()
    }
}

fn lerp(a: ChartColor, b: ChartColor, t: f32) -> ChartColor {
    ChartColor {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_at_zero_returns_first_stop() {
        let pal = SequentialPalette::new(vec![
            ChartColor::from_hex("#ffffff").unwrap(),
            ChartColor::from_hex("#000000").unwrap(),
        ]);
        let c = pal.sample(0.0);
        assert!((c.r - 1.0).abs() < 1e-5);
    }

    #[test]
    fn sample_at_one_returns_last_stop() {
        let pal = SequentialPalette::new(vec![
            ChartColor::from_hex("#ffffff").unwrap(),
            ChartColor::from_hex("#000000").unwrap(),
        ]);
        let c = pal.sample(1.0);
        assert!(c.r.abs() < 1e-5);
    }

    #[test]
    fn sample_at_midpoint_interpolates() {
        let pal = SequentialPalette::new(vec![
            ChartColor::from_hex("#000000").unwrap(),
            ChartColor::from_hex("#ffffff").unwrap(),
        ]);
        let c = pal.sample(0.5);
        assert!((c.r - 0.5).abs() < 1e-4);
    }

    #[test]
    fn sample_clamps_out_of_range() {
        let pal = SequentialPalette::blues();
        let _ = pal.sample(-1.0);
        let _ = pal.sample(2.0);
    }

    #[test]
    fn empty_input_does_not_panic() {
        let pal = SequentialPalette::new(vec![]);
        let _ = pal.sample(0.5);
    }
}
