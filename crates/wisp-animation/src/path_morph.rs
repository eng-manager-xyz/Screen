//! `PathMorph` interpolates between two equal-length point lists
//! (one Vec2 per index). `DrawIn` reveals a path 0..=t by
//! truncating its point list.
//!
//! Both produce `Vec<Vec2>` and are meant to be fed into a
//! caller-managed `wisp::Graphics::draw_line` chain or used by
//! the future `Path::trimmed` helper once `wisp` exposes it.

use std::time::Duration;

use glam::Vec2;

use crate::{Animatable, Animation};

/// Morph one polyline into another. Both lists must have the
/// same length; debug builds panic on mismatch.
#[derive(Clone, Debug)]
pub struct PathMorph {
    /// Starting polyline.
    pub from: Vec<Vec2>,
    /// Ending polyline.
    pub to: Vec<Vec2>,
    /// Total morph duration.
    pub duration: Duration,
}

impl PathMorph {
    /// Construct with equal-length point lists.
    #[must_use]
    pub fn new(from: Vec<Vec2>, to: Vec<Vec2>, duration: Duration) -> Self {
        debug_assert_eq!(
            from.len(),
            to.len(),
            "PathMorph requires equal vertex counts"
        );
        Self { from, to, duration }
    }

    /// Sample at `t`; allocates a fresh `Vec<Vec2>`.
    #[must_use]
    pub fn sample_into(&self, t: Duration) -> Vec<Vec2> {
        if self.duration.is_zero() || self.from.len() != self.to.len() {
            return self.to.clone();
        }
        #[allow(clippy::cast_possible_truncation, reason = "progress bounded [0, 1]")]
        let raw = (t.as_secs_f64() / self.duration.as_secs_f64()) as f32;
        let p = raw.clamp(0.0, 1.0);
        self.from
            .iter()
            .zip(&self.to)
            .map(|(a, b)| <Vec2 as Animatable>::lerp(a, b, p))
            .collect()
    }
}

impl Animation for PathMorph {
    type Output = Vec<Vec2>;

    fn duration(&self) -> Duration {
        self.duration
    }

    fn sample(&self, t: Duration) -> Vec<Vec2> {
        self.sample_into(t)
    }
}

/// Reveal a polyline from 0 to its full length over `duration`.
/// At `t = 0` returns the empty list; at `t = duration` returns
/// the full path. Intermediate values include the first
/// `ceil(progress · N)` points plus an interpolated trailing
/// vertex.
#[derive(Clone, Debug)]
pub struct DrawIn {
    /// Polyline to reveal.
    pub path: Vec<Vec2>,
    /// Reveal duration.
    pub duration: Duration,
}

impl DrawIn {
    /// Construct.
    #[must_use]
    pub const fn new(path: Vec<Vec2>, duration: Duration) -> Self {
        Self { path, duration }
    }
}

impl Animation for DrawIn {
    type Output = Vec<Vec2>;

    fn duration(&self) -> Duration {
        self.duration
    }

    fn sample(&self, t: Duration) -> Vec<Vec2> {
        if self.path.len() < 2 {
            return self.path.clone();
        }
        if self.duration.is_zero() {
            return self.path.clone();
        }
        #[allow(clippy::cast_possible_truncation, reason = "progress bounded [0, 1]")]
        let raw = (t.as_secs_f64() / self.duration.as_secs_f64()) as f32;
        let p = raw.clamp(0.0, 1.0);
        if p <= 0.0 {
            return Vec::new();
        }
        if p >= 1.0 {
            return self.path.clone();
        }
        // Find segment-fractional position along the polyline.
        #[allow(
            clippy::cast_precision_loss,
            reason = "polyline length well within f32 mantissa precision"
        )]
        let segments = (self.path.len() - 1) as f32;
        let target = p * segments;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "target in [0, segments]"
        )]
        let whole = target.floor() as usize;
        let frac = (target - target.floor()).clamp(0.0, 1.0);
        let mut out: Vec<Vec2> = self.path.iter().take(whole + 1).copied().collect();
        if frac > 0.0 && whole + 1 < self.path.len() {
            let a = self.path[whole];
            let b = self.path[whole + 1];
            out.push(<Vec2 as Animatable>::lerp(&a, &b, frac));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morph_endpoint_is_to() {
        let from = vec![Vec2::ZERO, Vec2::new(1.0, 0.0)];
        let to = vec![Vec2::new(2.0, 2.0), Vec2::new(3.0, 0.0)];
        let m = PathMorph::new(from, to.clone(), Duration::from_secs(1));
        let r = m.sample(Duration::from_secs(1));
        assert_eq!(r, to);
    }

    #[test]
    fn morph_midpoint_is_average() {
        let from = vec![Vec2::new(0.0, 0.0), Vec2::new(0.0, 10.0)];
        let to = vec![Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0)];
        let m = PathMorph::new(from, to, Duration::from_secs(1));
        let r = m.sample(Duration::from_millis(500));
        assert!((r[0] - Vec2::new(5.0, 0.0)).length() < 1e-3);
        assert!((r[1] - Vec2::new(5.0, 10.0)).length() < 1e-3);
    }

    #[test]
    fn draw_in_endpoints() {
        let path = vec![Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0)];
        let d = DrawIn::new(path.clone(), Duration::from_secs(1));
        let zero = d.sample(Duration::ZERO);
        assert!(zero.is_empty());
        let full = d.sample(Duration::from_secs(1));
        assert_eq!(full, path);
    }

    #[test]
    fn draw_in_partial_reveals_through_first_segment() {
        let path = vec![Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(20.0, 0.0)];
        let d = DrawIn::new(path, Duration::from_secs(1));
        // 25% through → 0.5 of segment 0
        let r = d.sample(Duration::from_millis(250));
        assert_eq!(r.len(), 2);
        assert!((r[0] - Vec2::ZERO).length() < 1e-3);
        assert!((r[1] - Vec2::new(5.0, 0.0)).length() < 1e-3);
    }
}
