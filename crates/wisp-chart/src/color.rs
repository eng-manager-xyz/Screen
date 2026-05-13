//! Chart-scoped colour type + contrast utilities.
//!
//! `wisp-chart`'s `Color` is intentionally distinct from any future
//! `wisp::Color`. The boundary rule (AUT-180): chart's colour type
//! does not bleed upward into `wisp`. If `wisp` later needs a
//! colour type, it gets its own.

/// 32-bit straight-alpha RGBA colour, sRGB-encoded (display
/// space, not linear). Components are 0.0–1.0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    /// Red component, sRGB-encoded, 0.0–1.0.
    pub r: f32,
    /// Green component, sRGB-encoded, 0.0–1.0.
    pub g: f32,
    /// Blue component, sRGB-encoded, 0.0–1.0.
    pub b: f32,
    /// Straight alpha, 0.0–1.0.
    pub a: f32,
}

impl Color {
    /// Opaque white `#ffffff`.
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    /// Opaque black `#000000`.
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    /// Construct from 8-bit RGB. Alpha defaults to 1.0.
    #[must_use]
    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: f32::from(r) / 255.0,
            g: f32::from(g) / 255.0,
            b: f32::from(b) / 255.0,
            a: 1.0,
        }
    }

    /// Construct from a CSS-style `#rrggbb` hex string.
    ///
    /// Returns `None` if the input is not exactly 7 chars
    /// starting with `#` and the remaining 6 are valid hex.
    #[must_use]
    pub fn from_hex(s: &str) -> Option<Self> {
        let bytes = s.as_bytes();
        if bytes.len() != 7 || bytes[0] != b'#' {
            return None;
        }
        let r = u8::from_str_radix(&s[1..3], 16).ok()?;
        let g = u8::from_str_radix(&s[3..5], 16).ok()?;
        let b = u8::from_str_radix(&s[5..7], 16).ok()?;
        Some(Self::rgb_u8(r, g, b))
    }

    /// Relative luminance per WCAG 2.x (sRGB → linearised
    /// weighted sum). Used by [`contrast_text_color`] to pick
    /// black vs white text against a bar fill.
    #[must_use]
    pub fn luminance(self) -> f32 {
        fn srgb_to_linear(c: f32) -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        let r = srgb_to_linear(self.r);
        let g = srgb_to_linear(self.g);
        let b = srgb_to_linear(self.b);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
}

/// Pick black or white text for maximum contrast against `bg`.
///
/// Returns [`Color::BLACK`] if `bg`'s luminance is above the
/// 0.179 threshold (matches the common WCAG 4.5:1 rule of
/// thumb), otherwise [`Color::WHITE`].
#[must_use]
pub fn contrast_text_color(bg: Color) -> Color {
    if bg.luminance() > 0.179 {
        Color::BLACK
    } else {
        Color::WHITE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_hex_parses_white_and_black() {
        assert_eq!(Color::from_hex("#ffffff"), Some(Color::WHITE));
        assert_eq!(Color::from_hex("#000000"), Some(Color::BLACK));
    }

    #[test]
    fn from_hex_rejects_bad_input() {
        assert_eq!(Color::from_hex("ffffff"), None);
        assert_eq!(Color::from_hex("#fffff"), None);
        assert_eq!(Color::from_hex("#fffggg"), None);
    }

    #[test]
    fn luminance_white_is_one_black_is_zero() {
        assert!((Color::WHITE.luminance() - 1.0).abs() < 1e-4);
        assert!(Color::BLACK.luminance().abs() < 1e-6);
    }

    #[test]
    fn contrast_text_dark_bg_returns_white() {
        let navy = Color::from_hex("#0072b2").unwrap();
        assert_eq!(contrast_text_color(navy), Color::WHITE);
    }

    #[test]
    fn contrast_text_light_bg_returns_black() {
        let yellow = Color::from_hex("#f0e442").unwrap();
        assert_eq!(contrast_text_color(yellow), Color::BLACK);
    }
}
