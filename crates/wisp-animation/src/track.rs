//! `Track` — multi-keyframe animations. Each key has a `(t,
//! value)` pair + an optional per-segment ease. Sampling between
//! two adjacent keys lerps via [`Animatable`] under the
//! segment's ease.
//!
//! For smooth multi-waypoint paths (Catmull-Rom, Hermite, Bezier)
//! see [`Curve`] in this module.

use std::time::Duration;

use glam::Vec2;

use crate::{Animatable, Animation, Ease};

/// One keyframe in a [`Track`].
#[derive(Clone, Debug)]
pub struct Key<V: Animatable> {
    /// When this key fires, measured from the track's t=0.
    pub at: Duration,
    /// Value at this key.
    pub value: V,
    /// Ease used to interpolate FROM the previous key TO this one.
    pub ease: Ease,
}

/// Multi-keyframe animation. Build with [`Track::new`] + `.key`.
#[derive(Clone, Debug)]
pub struct Track<V: Animatable> {
    keys: Vec<Key<V>>,
}

impl<V: Animatable> Default for Track<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Animatable> Track<V> {
    /// Empty track. Sampling returns `V::default()` if the
    /// underlying type implements `Default`; otherwise callers
    /// should always add at least one key.
    #[must_use]
    pub const fn new() -> Self {
        Self { keys: Vec::new() }
    }

    /// Append a key with default `Ease::Linear`.
    #[must_use]
    pub fn key(mut self, at: Duration, value: V) -> Self {
        self.keys.push(Key {
            at,
            value,
            ease: Ease::Linear,
        });
        self
    }

    /// Append a key with a specific ease (applied to the segment
    /// arriving at this key).
    #[must_use]
    pub fn key_eased(mut self, at: Duration, value: V, ease: Ease) -> Self {
        self.keys.push(Key { at, value, ease });
        self
    }

    /// Number of keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether the track has no keys.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

impl<V: Animatable + Default> Animation for Track<V> {
    type Output = V;

    fn duration(&self) -> Duration {
        self.keys.last().map_or(Duration::ZERO, |k| k.at)
    }

    fn sample(&self, t: Duration) -> V {
        if self.keys.is_empty() {
            return V::default();
        }
        if t <= self.keys[0].at {
            return self.keys[0].value.clone();
        }
        let last = self.keys.last().expect("non-empty above");
        if t >= last.at {
            return last.value.clone();
        }
        // Find adjacent pair.
        for pair in self.keys.windows(2) {
            let a = &pair[0];
            let b = &pair[1];
            if t >= a.at && t <= b.at {
                let span = b.at.saturating_sub(a.at);
                if span.is_zero() {
                    return b.value.clone();
                }
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "progress is bounded in [0, 1]"
                )]
                let raw = ((t - a.at).as_secs_f64() / span.as_secs_f64()) as f32;
                let eased = b.ease.eval(raw.clamp(0.0, 1.0));
                return V::lerp(&a.value, &b.value, eased);
            }
        }
        last.value.clone()
    }
}

// ---------------------------------------------------------------------
// Curve — Catmull-Rom + Hermite + Bezier interpolation for Vec2.
// Returns a position as a function of normalised parameter `0..=1`
// over the *whole* curve.
// ---------------------------------------------------------------------

/// Smooth 2-D curve through control points. Two flavours:
///
/// - `catmull_rom` — passes through every control point. Tangents
///   are derived from neighbours.
/// - `bezier_chain` — control polygon defines tangents. Doesn't
///   pass through interior control points.
#[derive(Clone, Debug)]
pub struct Curve {
    points: Vec<Vec2>,
    kind: CurveKind,
    duration: Duration,
}

#[derive(Clone, Copy, Debug)]
enum CurveKind {
    CatmullRom,
    Bezier,
}

impl Curve {
    /// Catmull-Rom spline through the supplied control points.
    /// Curve passes through every control point.
    #[must_use]
    pub fn catmull_rom(points: Vec<Vec2>, duration: Duration) -> Self {
        Self {
            points,
            kind: CurveKind::CatmullRom,
            duration,
        }
    }

    /// Cubic-Bezier chain — every 4 points define one segment;
    /// neighbouring segments share endpoints (`[P0 P1 P2 P3] [P3 P4 P5 P6] …`).
    #[must_use]
    pub fn bezier_chain(points: Vec<Vec2>, duration: Duration) -> Self {
        Self {
            points,
            kind: CurveKind::Bezier,
            duration,
        }
    }

    /// Number of control points.
    #[must_use]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Whether the curve has no control points.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Sample at parameter `s ∈ [0, 1]`.
    #[must_use]
    pub fn sample_normalised(&self, s: f32) -> Vec2 {
        if self.points.is_empty() {
            return Vec2::ZERO;
        }
        let s = s.clamp(0.0, 1.0);
        if s >= 1.0 - f32::EPSILON {
            return *self.points.last().unwrap();
        }
        if s <= f32::EPSILON {
            return *self.points.first().unwrap();
        }
        match self.kind {
            CurveKind::CatmullRom => catmull_rom_sample(&self.points, s),
            CurveKind::Bezier => bezier_chain_sample(&self.points, s),
        }
    }
}

impl Animation for Curve {
    type Output = Vec2;

    fn duration(&self) -> Duration {
        self.duration
    }

    fn sample(&self, t: Duration) -> Vec2 {
        if self.duration.is_zero() {
            return self.sample_normalised(1.0);
        }
        #[allow(
            clippy::cast_possible_truncation,
            reason = "progress is bounded in [0, 1] before reaching the cast"
        )]
        let raw = (t.as_secs_f64() / self.duration.as_secs_f64()) as f32;
        self.sample_normalised(raw.clamp(0.0, 1.0))
    }
}

fn catmull_rom_sample(points: &[Vec2], s: f32) -> Vec2 {
    let n = points.len();
    if n == 1 {
        return points[0];
    }
    if n == 2 {
        return points[0].lerp(points[1], s);
    }
    let segments = (n - 1) as f32;
    let seg_f = s * segments;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "seg_f in [0, segments]; bounded by clamp upstream"
    )]
    let seg_idx = (seg_f.floor() as usize).min(n - 2);
    let local = (seg_f - seg_f.floor()).clamp(0.0, 1.0);
    let p0 = points[seg_idx.saturating_sub(1)];
    let p1 = points[seg_idx];
    let p2 = points[seg_idx + 1];
    let p3 = points[(seg_idx + 2).min(n - 1)];
    // Standard Catmull-Rom with tension 0.5.
    let t = local;
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

fn bezier_chain_sample(points: &[Vec2], s: f32) -> Vec2 {
    if points.len() < 4 {
        // Not enough points for one full segment — fall back to lerp.
        if points.len() == 2 {
            return points[0].lerp(points[1], s);
        }
        return points[0];
    }
    let n_segments = (points.len() - 1) / 3;
    let seg_f = s * n_segments as f32;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "seg_f in [0, n_segments]"
    )]
    let seg_idx = (seg_f.floor() as usize).min(n_segments.saturating_sub(1));
    let local = (seg_f - seg_f.floor()).clamp(0.0, 1.0);
    let base = seg_idx * 3;
    let p0 = points[base];
    let p1 = points[base + 1];
    let p2 = points[base + 2];
    let p3 = points[base + 3];
    let one_minus = 1.0 - local;
    one_minus.powi(3) * p0
        + 3.0 * one_minus.powi(2) * local * p1
        + 3.0 * one_minus * local.powi(2) * p2
        + local.powi(3) * p3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_sample_at_key_returns_key_value() {
        let track: Track<f32> = Track::new()
            .key(Duration::ZERO, 0.0)
            .key(Duration::from_millis(500), 50.0)
            .key(Duration::from_millis(1000), 100.0);
        assert!((track.sample(Duration::ZERO) - 0.0).abs() < 1e-3);
        assert!((track.sample(Duration::from_millis(500)) - 50.0).abs() < 1e-3);
        assert!((track.sample(Duration::from_millis(1000)) - 100.0).abs() < 1e-3);
    }

    #[test]
    fn track_interpolates_between_keys() {
        let track: Track<f32> = Track::new()
            .key(Duration::ZERO, 0.0)
            .key(Duration::from_secs(1), 100.0);
        assert!((track.sample(Duration::from_millis(500)) - 50.0).abs() < 1e-3);
    }

    #[test]
    fn track_per_segment_ease_applies() {
        let linear: Track<f32> = Track::new()
            .key(Duration::ZERO, 0.0)
            .key(Duration::from_secs(1), 100.0);
        let eased: Track<f32> = Track::new()
            .key(Duration::ZERO, 0.0)
            .key_eased(Duration::from_secs(1), 100.0, Ease::InQuad);
        let t = Duration::from_millis(500);
        assert!((linear.sample(t) - 50.0).abs() < 1e-3);
        assert!((eased.sample(t) - 25.0).abs() < 1e-3); // InQuad: 0.5² × 100
    }

    #[test]
    fn track_clamps_outside_range() {
        let track: Track<f32> = Track::new()
            .key(Duration::ZERO, 10.0)
            .key(Duration::from_secs(1), 20.0);
        assert!((track.sample(Duration::from_secs(99)) - 20.0).abs() < 1e-3);
    }

    #[test]
    fn catmull_rom_passes_through_control_points() {
        let pts = vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), Vec2::new(2.0, 0.0)];
        let c = Curve::catmull_rom(pts, Duration::from_secs(1));
        // First control point at s=0; last at s=1.
        let first = c.sample_normalised(0.0);
        let last = c.sample_normalised(1.0);
        assert!((first - Vec2::new(0.0, 0.0)).length() < 1e-3);
        assert!((last - Vec2::new(2.0, 0.0)).length() < 1e-3);
    }

    #[test]
    fn bezier_chain_endpoints() {
        let pts = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 5.0),
            Vec2::new(10.0, 5.0),
            Vec2::new(10.0, 0.0),
        ];
        let c = Curve::bezier_chain(pts, Duration::from_secs(1));
        let first = c.sample_normalised(0.0);
        let last = c.sample_normalised(1.0);
        assert!((first - Vec2::new(0.0, 0.0)).length() < 1e-3);
        assert!((last - Vec2::new(10.0, 0.0)).length() < 1e-3);
    }
}
