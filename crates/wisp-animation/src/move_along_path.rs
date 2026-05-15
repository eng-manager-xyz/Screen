//! `MoveAlongPath` — produce a `(position, tangent)` pair from
//! a polyline as a function of `t`. Optional `auto_rotate`
//! returns the tangent angle so callers can feed it directly
//! into `Transform::from_rotation`.

use std::time::Duration;

use glam::Vec2;

use crate::Animation;

/// Position + tangent angle (radians) along a polyline.
#[derive(Clone, Copy, Debug, Default)]
pub struct PathPose {
    /// Position at the sampled fraction.
    pub position: Vec2,
    /// Tangent angle in radians (`atan2(dy, dx)`).
    pub angle: f32,
}

/// Animate a node along an arbitrary polyline.
#[derive(Clone, Debug)]
pub struct MoveAlongPath {
    /// Polyline.
    pub path: Vec<Vec2>,
    /// Total traversal duration.
    pub duration: Duration,
    /// When `true`, sampled `PathPose.angle` is the tangent
    /// angle. When `false`, angle is always `0.0`.
    pub auto_rotate: bool,
}

impl MoveAlongPath {
    /// Construct. Defaults to `auto_rotate = false`.
    #[must_use]
    pub const fn new(path: Vec<Vec2>, duration: Duration) -> Self {
        Self {
            path,
            duration,
            auto_rotate: false,
        }
    }

    /// Enable tangent-aligned rotation.
    #[must_use]
    pub const fn auto_rotate(mut self, on: bool) -> Self {
        self.auto_rotate = on;
        self
    }
}

impl Animation for MoveAlongPath {
    type Output = PathPose;

    fn duration(&self) -> Duration {
        self.duration
    }

    fn sample(&self, t: Duration) -> PathPose {
        if self.path.is_empty() {
            return PathPose::default();
        }
        if self.path.len() == 1 {
            return PathPose {
                position: self.path[0],
                angle: 0.0,
            };
        }
        let frac = if self.duration.is_zero() {
            1.0
        } else {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "progress is in [0, 1]"
            )]
            let raw = (t.as_secs_f64() / self.duration.as_secs_f64()) as f32;
            raw.clamp(0.0, 1.0)
        };
        if frac >= 1.0 - f32::EPSILON {
            let last = self.path[self.path.len() - 1];
            let prev = self.path[self.path.len() - 2];
            let angle = if self.auto_rotate {
                (last.y - prev.y).atan2(last.x - prev.x)
            } else {
                0.0
            };
            return PathPose {
                position: last,
                angle,
            };
        }
        let segments = (self.path.len() - 1) as f32;
        let target = frac * segments;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "target in [0, segments]"
        )]
        let whole = (target.floor() as usize).min(self.path.len() - 2);
        let local = (target - target.floor()).clamp(0.0, 1.0);
        let a = self.path[whole];
        let b = self.path[whole + 1];
        let pos = a + (b - a) * local;
        let angle = if self.auto_rotate {
            let d = b - a;
            d.y.atan2(d.x)
        } else {
            0.0
        };
        PathPose {
            position: pos,
            angle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_is_path_start() {
        let p = MoveAlongPath::new(
            vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)],
            Duration::from_secs(1),
        );
        let pose = p.sample(Duration::ZERO);
        assert!((pose.position - Vec2::ZERO).length() < 1e-3);
    }

    #[test]
    fn end_is_path_end() {
        let p = MoveAlongPath::new(
            vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)],
            Duration::from_secs(1),
        );
        let pose = p.sample(Duration::from_secs(1));
        assert!((pose.position - Vec2::new(10.0, 0.0)).length() < 1e-3);
    }

    #[test]
    fn auto_rotate_picks_segment_tangent() {
        let p = MoveAlongPath::new(
            vec![Vec2::new(0.0, 0.0), Vec2::new(0.0, 10.0)],
            Duration::from_secs(1),
        )
        .auto_rotate(true);
        let pose = p.sample(Duration::from_millis(500));
        // Tangent of (0, 10) - (0, 0) = (0, 10) → atan2(10, 0) = π/2.
        assert!((pose.angle - std::f32::consts::FRAC_PI_2).abs() < 1e-3);
    }

    #[test]
    fn no_rotate_returns_zero_angle() {
        let p = MoveAlongPath::new(
            vec![Vec2::new(0.0, 0.0), Vec2::new(0.0, 10.0)],
            Duration::from_secs(1),
        );
        let pose = p.sample(Duration::from_millis(500));
        assert!(pose.angle.abs() < f32::EPSILON);
    }

    #[test]
    fn empty_path_returns_default() {
        let p = MoveAlongPath::new(Vec::new(), Duration::from_secs(1));
        let pose = p.sample(Duration::from_millis(500));
        assert_eq!(pose.position, Vec2::ZERO);
    }
}
