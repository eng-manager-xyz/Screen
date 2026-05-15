//! Colour-space-aware interpolation for `wisp::Color`. Inline
//! sRGB ↔ Oklab math; no external crate.
//!
//! `Animatable for Color` (in [`crate::animatable`]) defaults to
//! linear sRGB. For perceptual blends — palette transitions, hue
//! sweeps — opt into Oklab or Oklch via [`ColorTween`].

#![allow(
    clippy::many_single_char_names,
    clippy::doc_markdown,
    clippy::excessive_precision,
    clippy::redundant_closure,
    reason = "Single-char names match the Oklab spec exactly (L, a, b, l_, m_, s_); doc text references colour-space names that aren't Rust identifiers; precision in Oklab matrix coefficients is the spec's value."
)]

use std::time::Duration;

use wisp::Color;

use crate::{Animation, Ease};

/// Interpolation space for a colour tween.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorSpace {
    /// Component-wise lerp in linear sRGB. Matches `wisp`'s
    /// compositing space — fastest, least surprising default.
    #[default]
    LinearRgb,
    /// Perceptual lerp in Oklab. Smooths through dark and
    /// chromatic regions without the muddy midtones LinearRgb
    /// produces for opposite hues.
    Oklab,
    /// Oklch — Oklab with polar `(C, H)`. Best for hue sweeps
    /// (rainbow gradients, palette rotation).
    Oklch,
}

/// `Tween<Color>` with explicit colour-space control.
#[derive(Clone, Copy, Debug)]
pub struct ColorTween {
    /// Starting colour.
    pub from: Color,
    /// Ending colour.
    pub to: Color,
    /// Total duration.
    pub duration: Duration,
    /// Ease.
    pub ease: Ease,
    /// Interpolation space.
    pub space: ColorSpace,
}

impl ColorTween {
    /// Construct with `LinearRgb` default + `Linear` ease.
    #[must_use]
    pub fn new(from: Color, to: Color, duration: Duration) -> Self {
        Self {
            from,
            to,
            duration,
            ease: Ease::Linear,
            space: ColorSpace::LinearRgb,
        }
    }

    /// Switch to Oklab interpolation.
    #[must_use]
    pub const fn in_oklab(mut self) -> Self {
        self.space = ColorSpace::Oklab;
        self
    }

    /// Switch to Oklch interpolation.
    #[must_use]
    pub const fn in_oklch(mut self) -> Self {
        self.space = ColorSpace::Oklch;
        self
    }

    /// Switch to linear sRGB interpolation (default).
    #[must_use]
    pub const fn in_linear_rgb(mut self) -> Self {
        self.space = ColorSpace::LinearRgb;
        self
    }

    /// Override ease.
    #[must_use]
    pub const fn ease(mut self, ease: Ease) -> Self {
        self.ease = ease;
        self
    }
}

impl Animation for ColorTween {
    type Output = Color;

    fn duration(&self) -> Duration {
        self.duration
    }

    fn sample(&self, t: Duration) -> Color {
        if self.duration.is_zero() {
            return self.to;
        }
        #[allow(
            clippy::cast_possible_truncation,
            reason = "progress is bounded [0, 1]"
        )]
        let raw = (t.as_secs_f64() / self.duration.as_secs_f64()) as f32;
        let p = self.ease.eval(raw.clamp(0.0, 1.0));
        match self.space {
            ColorSpace::LinearRgb => lerp_linear(self.from, self.to, p),
            ColorSpace::Oklab => lerp_oklab(self.from, self.to, p),
            ColorSpace::Oklch => lerp_oklch(self.from, self.to, p),
        }
    }
}

fn lerp_linear(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

fn lerp_oklab(a: Color, b: Color, t: f32) -> Color {
    let (la, aa, ba) = linear_srgb_to_oklab(a.r, a.g, a.b);
    let (lb, ab, bb) = linear_srgb_to_oklab(b.r, b.g, b.b);
    let l = la + (lb - la) * t;
    let a_ = aa + (ab - aa) * t;
    let b_ = ba + (bb - ba) * t;
    let (r, g, b2) = oklab_to_linear_srgb(l, a_, b_);
    Color {
        r,
        g,
        b: b2,
        a: a.a + (b.a - a.a) * t,
    }
}

fn lerp_oklch(a: Color, b: Color, t: f32) -> Color {
    let (la, aa, ba) = linear_srgb_to_oklab(a.r, a.g, a.b);
    let (lb, ab, bb) = linear_srgb_to_oklab(b.r, b.g, b.b);
    let ca = (aa * aa + ba * ba).sqrt();
    let cb = (ab * ab + bb * bb).sqrt();
    let ha = ba.atan2(aa);
    let hb = bb.atan2(ab);
    // Shortest-arc hue path
    let mut dh = hb - ha;
    if dh > std::f32::consts::PI {
        dh -= std::f32::consts::TAU;
    } else if dh < -std::f32::consts::PI {
        dh += std::f32::consts::TAU;
    }
    let l = la + (lb - la) * t;
    let c = ca + (cb - ca) * t;
    let h = ha + dh * t;
    let a_ = c * h.cos();
    let b_ = c * h.sin();
    let (r, g, b2) = oklab_to_linear_srgb(l, a_, b_);
    Color {
        r,
        g,
        b: b2,
        a: a.a + (b.a - a.a) * t,
    }
}

/// Inverse of `linear_srgb_to_oklab`. Input/output components are
/// linear (not gamma-corrected) sRGB.
fn linear_srgb_to_oklab(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let l = 0.412_221_47 * r + 0.536_332_55 * g + 0.051_445_995 * b;
    let m = 0.211_903_5 * r + 0.680_699_56 * g + 0.107_396_96 * b;
    let s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;

    let l_cbrt = cbrt_safe(l);
    let m_cbrt = cbrt_safe(m);
    let s_cbrt = cbrt_safe(s);

    let l_ok = 0.210_454_26 * l_cbrt + 0.793_617_8 * m_cbrt - 0.004_072_047 * s_cbrt;
    let a_ok = 1.977_998_5 * l_cbrt - 2.428_592_2 * m_cbrt + 0.450_593_7 * s_cbrt;
    let b_ok = 0.025_904_037 * l_cbrt + 0.782_771_77 * m_cbrt - 0.808_675_77 * s_cbrt;
    (l_ok, a_ok, b_ok)
}

fn oklab_to_linear_srgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;

    let l3 = l_ * l_ * l_;
    let m3 = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;

    let r = 4.076_741_7 * l3 - 3.307_711_6 * m3 + 0.230_969_94 * s3;
    let g = -1.268_438 * l3 + 2.609_757_4 * m3 - 0.341_319_38 * s3;
    let b = -0.004_196_086_3 * l3 - 0.703_418_6 * m3 + 1.707_614_7 * s3;
    (r, g, b)
}

fn cbrt_safe(v: f32) -> f32 {
    // f32 doesn't expose cbrt as `const`; clamp to non-negative to
    // avoid NaN propagation on slightly-negative linear values.
    if v >= 0.0 { v.cbrt() } else { -((-v).cbrt()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_color(a: Color, b: Color, tol: f32) -> bool {
        (a.r - b.r).abs() < tol
            && (a.g - b.g).abs() < tol
            && (a.b - b.b).abs() < tol
            && (a.a - b.a).abs() < tol
    }

    #[test]
    fn endpoints_match_in_every_space() {
        let red = Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        let green = Color {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        };
        for space in [ColorSpace::LinearRgb, ColorSpace::Oklab, ColorSpace::Oklch] {
            let mut t = ColorTween::new(red, green, Duration::from_secs(1));
            t.space = space;
            assert!(approx_color(t.sample(Duration::ZERO), red, 1e-3));
            assert!(approx_color(t.sample(Duration::from_secs(1)), green, 1e-3));
        }
    }

    #[test]
    fn oklab_midpoint_differs_from_linear_rgb() {
        let red = Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        let green = Color {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        };
        let lrgb = ColorTween::new(red, green, Duration::from_secs(1));
        let oklab = ColorTween::new(red, green, Duration::from_secs(1)).in_oklab();
        let mid_l = lrgb.sample(Duration::from_millis(500));
        let mid_o = oklab.sample(Duration::from_millis(500));
        let diff = ((mid_l.r - mid_o.r).powi(2)
            + (mid_l.g - mid_o.g).powi(2)
            + (mid_l.b - mid_o.b).powi(2))
        .sqrt();
        assert!(
            diff > 0.05,
            "oklab midpoint should differ measurably from sRGB; diff = {diff}"
        );
    }

    #[test]
    fn oklch_hue_path_is_short_arc() {
        // Red to magenta should not pass through green.
        let red = Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        let magenta = Color {
            r: 1.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        };
        let t = ColorTween::new(red, magenta, Duration::from_secs(1)).in_oklch();
        let mid = t.sample(Duration::from_millis(500));
        // Midpoint should still have low green; the short-arc path
        // goes via dark red → dark purple, not through green.
        assert!(
            mid.g < 0.3,
            "expected short-arc red→magenta hue path, got mid.g = {}",
            mid.g
        );
    }
}
