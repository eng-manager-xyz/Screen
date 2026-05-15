//! `AnimTheme` — default duration / ease / stagger gap used when
//! callers omit those parameters. Two built-in presets:
//! `snappy()` (250ms / `OutCubic` / 30ms) and `smooth()` (450ms /
//! `OutExpo` / 60ms).
//!
//! The theme is a plain value — callers pass it into `Tween::from_theme`,
//! `Stagger::from_theme`, etc., so it composes without a global.

use std::time::Duration;

use crate::Animatable;
use crate::{Ease, Stagger, StaggerFrom, Tween};

/// Default motion parameters for a host app.
#[derive(Clone, Copy, Debug)]
pub struct AnimTheme {
    /// Default tween duration.
    pub default_duration: Duration,
    /// Default ease for tweens that don't specify.
    pub default_ease: Ease,
    /// Default stagger gap.
    pub default_stagger_each: Duration,
}

impl AnimTheme {
    /// Snappy preset — 250ms / `OutCubic` / 30ms.
    #[must_use]
    pub const fn snappy() -> Self {
        Self {
            default_duration: Duration::from_millis(250),
            default_ease: Ease::OutCubic,
            default_stagger_each: Duration::from_millis(30),
        }
    }

    /// Smooth preset — 450ms / `OutExpo` / 60ms.
    #[must_use]
    pub const fn smooth() -> Self {
        Self {
            default_duration: Duration::from_millis(450),
            default_ease: Ease::OutExpo,
            default_stagger_each: Duration::from_millis(60),
        }
    }

    /// Build a `Tween` for the given endpoints using this theme's
    /// duration + ease.
    #[must_use]
    pub fn tween<V: Animatable>(&self, from: V, to: V) -> Tween<V> {
        Tween::new(from, to, self.default_duration).ease(self.default_ease)
    }

    /// Build a `Stagger` using this theme's per-step gap.
    #[must_use]
    pub fn stagger(&self) -> Stagger {
        Stagger::each(self.default_stagger_each).from(StaggerFrom::Start)
    }
}

impl Default for AnimTheme {
    fn default() -> Self {
        Self::smooth()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snappy_has_short_duration() {
        let t = AnimTheme::snappy();
        assert_eq!(t.default_duration, Duration::from_millis(250));
        assert!(matches!(t.default_ease, Ease::OutCubic));
    }

    #[test]
    fn smooth_has_long_duration() {
        let t = AnimTheme::smooth();
        assert_eq!(t.default_duration, Duration::from_millis(450));
    }

    #[test]
    fn tween_inherits_theme() {
        let theme = AnimTheme::snappy();
        let tw = theme.tween(0.0_f32, 1.0);
        assert_eq!(tw.duration, Duration::from_millis(250));
        assert!(matches!(tw.ease, Ease::OutCubic));
    }

    #[test]
    fn theme_default_is_smooth() {
        let d = AnimTheme::default();
        assert_eq!(d.default_duration, AnimTheme::smooth().default_duration);
    }

    #[test]
    fn stagger_uses_theme_gap() {
        let s = AnimTheme::snappy().stagger();
        assert_eq!(s.offset_for(2, 5), Duration::from_millis(60));
    }
}
