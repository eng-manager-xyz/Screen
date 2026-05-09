//! RGBA colors in linear f32 space.

use bytemuck::{Pod, Zeroable};

/// RGBA color with linear f32 channels in `[0.0, 1.0]`.
///
/// Layout is `repr(C)` and `Pod`/`Zeroable` so the type can be uploaded
/// directly into GPU buffers. Channels are **not** premultiplied unless
/// [`Color::premultiplied`] has been called explicitly.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Color {
    /// Red channel.
    pub r: f32,
    /// Green channel.
    pub g: f32,
    /// Blue channel.
    pub b: f32,
    /// Alpha channel.
    pub a: f32,
}

impl Color {
    /// Fully transparent.
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);
    /// Opaque black.
    pub const BLACK: Self = Self::rgba(0.0, 0.0, 0.0, 1.0);
    /// Opaque white.
    pub const WHITE: Self = Self::rgba(1.0, 1.0, 1.0, 1.0);
    /// Opaque pure red.
    pub const RED: Self = Self::rgba(1.0, 0.0, 0.0, 1.0);
    /// Opaque pure green.
    pub const GREEN: Self = Self::rgba(0.0, 1.0, 0.0, 1.0);
    /// Opaque pure blue.
    pub const BLUE: Self = Self::rgba(0.0, 0.0, 1.0, 1.0);

    /// Construct an opaque color from f32 RGB channels.
    #[must_use]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Construct a color from f32 RGBA channels.
    #[must_use]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Construct an opaque color from u8 RGB channels (0–255).
    #[must_use]
    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self::rgba_u8(r, g, b, 255)
    }

    /// Construct a color from u8 RGBA channels (0–255).
    #[must_use]
    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::rgba(
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            f32::from(a) / 255.0,
        )
    }

    /// Return a copy with the alpha channel replaced.
    #[must_use]
    pub const fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    /// Return a copy with RGB premultiplied by alpha.
    #[must_use]
    pub fn premultiplied(self) -> Self {
        Self::rgba(self.r * self.a, self.g * self.a, self.b * self.a, self.a)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < f32::EPSILON
    }

    #[test]
    fn rgb_sets_alpha_to_one() {
        let c = Color::rgb(0.5, 0.6, 0.7);
        assert!(approx(c.a, 1.0));
    }

    /// Table-driven mapping of u8 RGBA inputs to expected `Color` constants.
    #[rstest]
    #[case(255, 255, 255, 255, Color::WHITE)]
    #[case(0, 0, 0, 0, Color::TRANSPARENT)]
    #[case(0, 0, 0, 255, Color::BLACK)]
    #[case(255, 0, 0, 255, Color::RED)]
    #[case(0, 255, 0, 255, Color::GREEN)]
    #[case(0, 0, 255, 255, Color::BLUE)]
    fn rgba_u8_matches_constants(
        #[case] r: u8,
        #[case] g: u8,
        #[case] b: u8,
        #[case] a: u8,
        #[case] expected: Color,
    ) {
        assert_eq!(Color::rgba_u8(r, g, b, a), expected);
    }

    #[test]
    fn with_alpha_overrides_only_alpha() {
        let c = Color::WHITE.with_alpha(0.3);
        assert!(approx(c.a, 0.3));
        assert!(approx(c.r, 1.0));
        assert!(approx(c.g, 1.0));
        assert!(approx(c.b, 1.0));
    }

    #[rstest]
    #[case(Color::WHITE.with_alpha(0.0), Color::TRANSPARENT)]
    #[case(Color::rgba(0.5, 0.5, 0.5, 1.0), Color::rgba(0.5, 0.5, 0.5, 1.0))]
    #[case(Color::rgba(1.0, 0.5, 0.0, 0.5), Color::rgba(0.5, 0.25, 0.0, 0.5))]
    fn premultiplied_cases(#[case] input: Color, #[case] expected: Color) {
        assert_eq!(input.premultiplied(), expected);
    }
}
