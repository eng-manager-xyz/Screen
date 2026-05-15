//! Easing curves — pure functions `f(t: f32) -> f32` with the
//! convention that `f(0) = 0` and `f(1) = 1`. Named eases mirror
//! anime.js v4's spelling so docs are pasteable between codebases.
//!
//! The set covers Penner's classics (`Quad`/`Cubic`/`Expo`/
//! `Back`/`Elastic`/`Bounce` in `In`/`Out`/`InOut` flavours) plus
//! `Linear`, parametric `CubicBezier`, discrete `Steps`, and a
//! one-shot `ThereAndBack` rate function for reveal-then-hide.

#![allow(
    clippy::too_many_lines,
    clippy::derivable_impls,
    reason = "Ease::eval covers ~20 named curves; splitting it would just push the match into a private helper without reducing complexity. Default impl is explicit so the variant choice is documented."
)]

use std::f32::consts::TAU;

/// Easing curve used by [`crate::Tween`] and other timeline-anchored
/// primitives.
///
/// Variants are deliberately a flat enum (no boxed trait object) so
/// the compiler can monomorphise `Tween::sample` per-easing and the
/// match dispatch stays branch-predictable. Custom eases live on
/// the [`Ease::Fn`] variant.
#[derive(Clone, Copy, Debug)]
pub enum Ease {
    /// `f(t) = t`.
    Linear,
    /// `f(t) = t²`.
    InQuad,
    /// `f(t) = 1 - (1 - t)²`.
    OutQuad,
    /// In then out at `t = 0.5`.
    InOutQuad,
    /// `f(t) = t³`.
    InCubic,
    /// `f(t) = 1 - (1 - t)³`.
    OutCubic,
    /// In then out cubic.
    InOutCubic,
    /// `f(t) = 2^(10(t - 1))` clamped at endpoints.
    InExpo,
    /// `f(t) = 1 - 2^(-10t)`.
    OutExpo,
    /// In then out exponential.
    InOutExpo,
    /// Overshoots before settling.
    InBack,
    /// Undershoots then settles.
    OutBack,
    /// Both ends overshoot.
    InOutBack,
    /// Spring-y rebound on the start.
    InElastic,
    /// Spring-y rebound on the end.
    OutElastic,
    /// Both ends are elastic.
    InOutElastic,
    /// Accelerating bouncing in.
    InBounce,
    /// Decelerating bouncing out.
    OutBounce,
    /// Both ends bounce.
    InOutBounce,
    /// CSS-style cubic-bezier with control points `(x1, y1, x2, y2)`.
    /// Endpoints are anchored at `(0,0)` and `(1,1)`.
    CubicBezier(f32, f32, f32, f32),
    /// Discrete staircase with `n` plateaus.
    Steps(u32),
    /// Rate function that goes `0 → 1 → 0` — useful for one-shot
    /// reveal-then-hide tweens.
    ThereAndBack,
    /// Caller-provided function pointer.
    Fn(fn(f32) -> f32),
}

impl Default for Ease {
    fn default() -> Self {
        Self::Linear
    }
}

impl Ease {
    /// Evaluate the curve at `t` in `[0.0, 1.0]`. Output may
    /// exceed `[0, 1]` for overshoot eases (Back, Elastic).
    #[must_use]
    pub fn eval(self, t: f32) -> f32 {
        match self {
            Self::Linear => t,
            Self::InQuad => t * t,
            Self::OutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Self::InOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - 2.0 * (1.0 - t) * (1.0 - t)
                }
            }
            Self::InCubic => t * t * t,
            Self::OutCubic => {
                let u = 1.0 - t;
                1.0 - u * u * u
            }
            Self::InOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    let u = 1.0 - t;
                    1.0 - 4.0 * u * u * u
                }
            }
            Self::InExpo => {
                if t <= 0.0 {
                    0.0
                } else {
                    (2.0_f32).powf(10.0 * (t - 1.0))
                }
            }
            Self::OutExpo => {
                if t >= 1.0 {
                    1.0
                } else {
                    1.0 - (2.0_f32).powf(-10.0 * t)
                }
            }
            Self::InOutExpo => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else if t < 0.5 {
                    0.5 * (2.0_f32).powf(20.0 * t - 10.0)
                } else {
                    1.0 - 0.5 * (2.0_f32).powf(-20.0 * t + 10.0)
                }
            }
            Self::InBack => {
                const C1: f32 = 1.701_58;
                t * t * ((C1 + 1.0) * t - C1)
            }
            Self::OutBack => {
                const C1: f32 = 1.701_58;
                let u = t - 1.0;
                1.0 + (C1 + 1.0) * u * u * u + C1 * u * u
            }
            Self::InOutBack => {
                const C2: f32 = 2.594_91;
                if t < 0.5 {
                    let r = 2.0 * t;
                    0.5 * r * r * ((C2 + 1.0) * r - C2)
                } else {
                    let r = 2.0 * t - 2.0;
                    0.5 * (r * r * ((C2 + 1.0) * r + C2) + 2.0)
                }
            }
            Self::InElastic => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    let c4 = TAU / 3.0;
                    -((2.0_f32).powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * c4).sin()
                }
            }
            Self::OutElastic => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    let c4 = TAU / 3.0;
                    (2.0_f32).powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
                }
            }
            Self::InOutElastic => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    let c5 = TAU / 4.5;
                    if t < 0.5 {
                        -((2.0_f32).powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c5).sin()) * 0.5
                    } else {
                        (2.0_f32).powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c5).sin() * 0.5
                            + 1.0
                    }
                }
            }
            Self::InBounce => 1.0 - out_bounce(1.0 - t),
            Self::OutBounce => out_bounce(t),
            Self::InOutBounce => {
                if t < 0.5 {
                    (1.0 - out_bounce(1.0 - 2.0 * t)) * 0.5
                } else {
                    (1.0 + out_bounce(2.0 * t - 1.0)) * 0.5
                }
            }
            Self::CubicBezier(x1, y1, x2, y2) => cubic_bezier_eval(x1, y1, x2, y2, t),
            Self::Steps(n) => {
                if n == 0 {
                    return t;
                }
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "step count typically < 100; well within f32 mantissa"
                )]
                let n_f = n as f32;
                (t * n_f).floor() / n_f
            }
            Self::ThereAndBack => {
                // Triangle wave: 0 → 1 → 0 with peak at t = 0.5.
                if t < 0.5 { 2.0 * t } else { 2.0 * (1.0 - t) }
            }
            Self::Fn(f) => f(t),
        }
    }
}

/// Shared bounce-out kernel — Penner's standard four-segment bounce.
fn out_bounce(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;
    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let s = t - 1.5 / D1;
        N1 * s * s + 0.75
    } else if t < 2.5 / D1 {
        let s = t - 2.25 / D1;
        N1 * s * s + 0.9375
    } else {
        let s = t - 2.625 / D1;
        N1 * s * s + 0.984_375
    }
}

/// Newton-Raphson root-finder for cubic-bezier easing. Standard
/// CSS approach: parametric `(x(s), y(s))` with `s` in `[0, 1]`;
/// solve `x(s) = t` for `s`, then return `y(s)`.
fn cubic_bezier_eval(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let cx = 3.0 * x1;
    let bx = 3.0 * (x2 - x1) - cx;
    let ax = 1.0 - cx - bx;
    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - cy - by;
    let sample_curve_x = |s: f32| ((ax * s + bx) * s + cx) * s;
    let sample_curve_y = |s: f32| ((ay * s + by) * s + cy) * s;
    let sample_curve_derivative_x = |s: f32| (3.0 * ax * s + 2.0 * bx) * s + cx;
    let mut s = t;
    for _ in 0..8 {
        let x = sample_curve_x(s) - t;
        if x.abs() < 1e-5 {
            return sample_curve_y(s);
        }
        let dx = sample_curve_derivative_x(s);
        if dx.abs() < 1e-6 {
            break;
        }
        s -= x / dx;
    }
    // Fall back to bisection if Newton diverged.
    let (mut lo, mut hi, mut mid) = (0.0_f32, 1.0_f32, s);
    for _ in 0..32 {
        let x = sample_curve_x(mid) - t;
        if x.abs() < 1e-5 {
            return sample_curve_y(mid);
        }
        if x > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
        mid = (lo + hi) * 0.5;
    }
    sample_curve_y(mid)
}

impl<F: Fn(f32) -> f32 + 'static + Copy> From<F> for Ease
where
    F: Into<fn(f32) -> f32>,
{
    fn from(f: F) -> Self {
        Self::Fn(f.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn every_named_ease_hits_endpoints() {
        let eases = [
            Ease::Linear,
            Ease::InQuad,
            Ease::OutQuad,
            Ease::InOutQuad,
            Ease::InCubic,
            Ease::OutCubic,
            Ease::InOutCubic,
            Ease::InExpo,
            Ease::OutExpo,
            Ease::InOutExpo,
            Ease::InBack,
            Ease::OutBack,
            Ease::InOutBack,
            Ease::InElastic,
            Ease::OutElastic,
            Ease::InOutElastic,
            Ease::InBounce,
            Ease::OutBounce,
            Ease::InOutBounce,
        ];
        for e in eases {
            assert!(approx(e.eval(0.0), 0.0), "{e:?}: f(0) != 0");
            assert!(approx(e.eval(1.0), 1.0), "{e:?}: f(1) != 1");
        }
    }

    #[test]
    fn cubic_bezier_matches_css_reference() {
        // `cubic-bezier(0.25, 0.1, 0.25, 1.0)` is the CSS `ease`
        // default. Reference values from CSS spec.
        let e = Ease::CubicBezier(0.25, 0.1, 0.25, 1.0);
        assert!(approx(e.eval(0.0), 0.0));
        assert!(approx(e.eval(1.0), 1.0));
        // Midpoint should be > 0.5 — CSS ease front-loads.
        assert!(e.eval(0.5) > 0.5);
    }

    #[test]
    fn steps_produces_n_plateaus() {
        let e = Ease::Steps(5);
        assert!(approx(e.eval(0.0), 0.0));
        assert!(approx(e.eval(0.19), 0.0));
        assert!(approx(e.eval(0.21), 0.2));
        assert!(approx(e.eval(0.99), 0.8));
    }

    #[test]
    fn there_and_back_peaks_at_half() {
        let e = Ease::ThereAndBack;
        assert!(approx(e.eval(0.0), 0.0));
        assert!(approx(e.eval(0.5), 1.0));
        assert!(approx(e.eval(1.0), 0.0));
    }

    #[test]
    fn custom_fn_dispatches() {
        let e = Ease::Fn(|t| t * t * t * t);
        assert!(approx(e.eval(0.5), 0.0625));
    }

    #[test]
    fn linear_default_is_identity() {
        assert!(approx(Ease::default().eval(0.42), 0.42));
    }
}
