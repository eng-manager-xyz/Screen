//! Chart `Enter` / `Exit` constructors — opinionated entrance
//! and exit animations for `wisp-chart` value types.
//!
//! Each variant maps to a concrete `Animation` value:
//!
//! - `Enter::Grow` → scale `Tween::new(0.0, 1.0, ...)` with `OutBack`.
//! - `Enter::DrawIn` → `DrawIn` over the path (caller supplies).
//! - `Enter::Sweep` → rotation `Tween::new(0.0, TAU, ...)`.
//! - `Enter::Fade` → alpha `Tween::new(0.0, 1.0, ...)`.
//!
//! These are convenience constructors; the underlying primitives
//! are public, so callers wanting custom shapes go around the
//! enum and build their own.

use std::time::Duration;

use crate::{AnimTheme, Ease, Tween};

/// Entrance animation kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Enter {
    /// Scale 0 → 1 with overshoot.
    #[default]
    Grow,
    /// Path stroke draw-on. Caller pairs with `DrawIn` over the
    /// chart's outline.
    DrawIn,
    /// Rotation 0 → 2π sweep.
    Sweep,
    /// Alpha 0 → 1 fade.
    Fade,
}

/// Exit animation kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Exit {
    /// Scale 1 → 0 with anticipation.
    #[default]
    Shrink,
    /// Alpha 1 → 0 fade.
    FadeOut,
}

impl Enter {
    /// Produce a scale tween for `Enter::Grow` / `Enter::Sweep`
    /// shaped by the theme's duration.
    #[must_use]
    pub fn scale_tween(self, theme: &AnimTheme) -> Tween<f32> {
        match self {
            Self::Grow => Tween::new(0.0, 1.0, theme.default_duration).ease(Ease::OutBack),
            Self::Sweep | Self::DrawIn | Self::Fade => Tween::new(1.0, 1.0, theme.default_duration),
        }
    }

    /// Produce an alpha tween for `Enter::Fade`. Other variants
    /// return a constant-1.0 tween so callers can compose without
    /// branching.
    #[must_use]
    pub fn alpha_tween(self, theme: &AnimTheme) -> Tween<f32> {
        match self {
            Self::Fade => Tween::new(0.0, 1.0, theme.default_duration).ease(Ease::OutCubic),
            _ => Tween::new(1.0, 1.0, Duration::ZERO),
        }
    }

    /// Produce a rotation tween for `Enter::Sweep`.
    #[must_use]
    pub fn rotation_tween(self, theme: &AnimTheme) -> Tween<f32> {
        match self {
            Self::Sweep => {
                Tween::new(0.0, std::f32::consts::TAU, theme.default_duration).ease(Ease::OutCubic)
            }
            _ => Tween::new(0.0, 0.0, Duration::ZERO),
        }
    }
}

impl Exit {
    /// Produce a scale tween for `Exit::Shrink`.
    #[must_use]
    pub fn scale_tween(self, theme: &AnimTheme) -> Tween<f32> {
        match self {
            Self::Shrink => Tween::new(1.0, 0.0, theme.default_duration).ease(Ease::InBack),
            Self::FadeOut => Tween::new(1.0, 1.0, Duration::ZERO),
        }
    }

    /// Produce an alpha tween for `Exit::FadeOut`.
    #[must_use]
    pub fn alpha_tween(self, theme: &AnimTheme) -> Tween<f32> {
        match self {
            Self::FadeOut => Tween::new(1.0, 0.0, theme.default_duration).ease(Ease::InCubic),
            Self::Shrink => Tween::new(1.0, 1.0, Duration::ZERO),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Animation;

    #[test]
    fn grow_starts_at_zero_ends_at_one() {
        let theme = AnimTheme::snappy();
        let t = Enter::Grow.scale_tween(&theme);
        assert!((t.sample(Duration::ZERO) - 0.0).abs() < 1e-3);
        assert!((t.sample(theme.default_duration) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn sweep_rotates_through_tau() {
        let theme = AnimTheme::smooth();
        let t = Enter::Sweep.rotation_tween(&theme);
        assert!(t.sample(theme.default_duration) > std::f32::consts::PI);
    }

    #[test]
    fn fade_alpha_goes_to_one() {
        let theme = AnimTheme::snappy();
        let t = Enter::Fade.alpha_tween(&theme);
        assert!((t.sample(theme.default_duration) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn exit_shrink_inverts_grow() {
        let theme = AnimTheme::snappy();
        let t = Exit::Shrink.scale_tween(&theme);
        assert!((t.sample(Duration::ZERO) - 1.0).abs() < 1e-3);
        // InBack undershoots before settling — check it lands near 0.
        let end = t.sample(theme.default_duration);
        assert!(end.abs() < 1e-3);
    }

    #[test]
    fn exit_fadeout_inverts_fade() {
        let theme = AnimTheme::smooth();
        let t = Exit::FadeOut.alpha_tween(&theme);
        assert!((t.sample(Duration::ZERO) - 1.0).abs() < 1e-3);
        assert!(t.sample(theme.default_duration).abs() < 1e-3);
    }
}
