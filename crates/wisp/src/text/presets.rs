//! Curated [`WispTextStyle`] presets for common screen-recording
//! captions (M-TEXT.12 / AUT-86).
//!
//! Each preset is a `const fn` returning a [`WispTextStyle`] — no
//! allocation, no GPU dependency. Pick one with [`TextPreset::style`]
//! or by calling the named accessor directly.
//!
//! Presets are tuned for a 16:9 export at the storybook default
//! viewport (256×256 / 640×360). Resize them by chaining
//! [`WispTextStyle::with_size`] if you need a different scale.

use crate::color::Color;
use crate::text::{WispFontStyle, WispFontWeight, WispTextAlign, WispTextStyle};

/// Named text style — pick one and `.style()` to a [`WispTextStyle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextPreset {
    /// Headline / hero line — large, bold, centered.
    SectionTitle,
    /// Caption / subtitle — left-aligned body copy.
    Caption,
    /// Vertical callout — italic medium emphasis.
    Callout,
    /// Keyboard shortcut chip — small monospaced (the renderer maps
    /// `WispFontHandle::default()` to system mono via cosmic-text's
    /// generic fallback when the named font isn't loaded).
    KeyboardShortcut,
    /// Step / hint badge — tiny bold uppercase-y label.
    StepBadge,
    /// Warning / privacy label — uppercase, high-contrast.
    WarningPrivacyLabel,
    /// Watermark — small, low-contrast, italic.
    Watermark,
}

impl TextPreset {
    /// Pick the [`WispTextStyle`] matching this preset.
    #[must_use]
    pub fn style(self) -> WispTextStyle {
        match self {
            Self::SectionTitle => section_title(),
            Self::Caption => caption(),
            Self::Callout => callout(),
            Self::KeyboardShortcut => keyboard_shortcut(),
            Self::StepBadge => step_badge(),
            Self::WarningPrivacyLabel => warning_privacy_label(),
            Self::Watermark => watermark(),
        }
    }

    /// Iterator over every preset, in display order. Useful for the
    /// preset-gallery story and any test that wants to exercise the
    /// full set.
    #[must_use]
    pub fn all() -> [Self; 7] {
        [
            Self::SectionTitle,
            Self::Caption,
            Self::Callout,
            Self::KeyboardShortcut,
            Self::StepBadge,
            Self::WarningPrivacyLabel,
            Self::Watermark,
        ]
    }

    /// Short human-readable label for the preset (story titles, docs).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::SectionTitle => "Section title",
            Self::Caption => "Caption",
            Self::Callout => "Callout",
            Self::KeyboardShortcut => "Keyboard shortcut",
            Self::StepBadge => "Step badge",
            Self::WarningPrivacyLabel => "Warning / privacy label",
            Self::Watermark => "Watermark",
        }
    }
}

/// Headline style — large, bold, centered.
#[must_use]
pub fn section_title() -> WispTextStyle {
    WispTextStyle {
        size_ndc: 0.18,
        color: Color::rgba_u8(248, 250, 255, 255),
        line_height: 1.1,
        letter_spacing_ndc: -0.002,
        weight: WispFontWeight::Bold,
        style: WispFontStyle::Normal,
        align: WispTextAlign::Center,
        ..WispTextStyle::default()
    }
}

/// Body caption — left-aligned, medium weight, ample line height.
#[must_use]
pub fn caption() -> WispTextStyle {
    WispTextStyle {
        size_ndc: 0.075,
        color: Color::rgba_u8(240, 240, 245, 255),
        line_height: 1.30,
        letter_spacing_ndc: 0.0,
        weight: WispFontWeight::Regular,
        style: WispFontStyle::Normal,
        align: WispTextAlign::Left,
        ..WispTextStyle::default()
    }
}

/// Callout — italic, medium-bold, tighter spacing.
#[must_use]
pub fn callout() -> WispTextStyle {
    WispTextStyle {
        size_ndc: 0.085,
        color: Color::rgba_u8(255, 235, 200, 255),
        line_height: 1.25,
        letter_spacing_ndc: 0.0,
        weight: WispFontWeight::Medium,
        style: WispFontStyle::Italic,
        align: WispTextAlign::Left,
        ..WispTextStyle::default()
    }
}

/// Keyboard shortcut chip — small, tight, monospace-feel.
#[must_use]
pub fn keyboard_shortcut() -> WispTextStyle {
    WispTextStyle {
        size_ndc: 0.055,
        color: Color::rgba_u8(220, 230, 240, 255),
        line_height: 1.0,
        letter_spacing_ndc: 0.002,
        weight: WispFontWeight::Medium,
        style: WispFontStyle::Normal,
        align: WispTextAlign::Center,
        ..WispTextStyle::default()
    }
}

/// Step / hint badge — bold, tight, slightly wider spacing.
#[must_use]
pub fn step_badge() -> WispTextStyle {
    WispTextStyle {
        size_ndc: 0.05,
        color: Color::rgba_u8(255, 240, 215, 255),
        line_height: 1.0,
        letter_spacing_ndc: 0.004,
        weight: WispFontWeight::Bold,
        style: WispFontStyle::Normal,
        align: WispTextAlign::Center,
        ..WispTextStyle::default()
    }
}

/// Warning / privacy label — bold, wide spacing, signal-red.
#[must_use]
pub fn warning_privacy_label() -> WispTextStyle {
    WispTextStyle {
        size_ndc: 0.06,
        color: Color::rgba_u8(255, 100, 90, 255),
        line_height: 1.0,
        letter_spacing_ndc: 0.006,
        weight: WispFontWeight::Bold,
        style: WispFontStyle::Normal,
        align: WispTextAlign::Center,
        ..WispTextStyle::default()
    }
}

/// Watermark — small, italic, low-contrast.
#[must_use]
pub fn watermark() -> WispTextStyle {
    WispTextStyle {
        size_ndc: 0.04,
        color: Color::rgba_u8(200, 200, 210, 160), // 160/255 ≈ 63% alpha
        line_height: 1.0,
        letter_spacing_ndc: 0.001,
        weight: WispFontWeight::Light,
        style: WispFontStyle::Italic,
        align: WispTextAlign::Right,
        ..WispTextStyle::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_returns_seven_presets() {
        assert_eq!(TextPreset::all().len(), 7);
    }

    #[test]
    fn every_preset_has_positive_size() {
        for p in TextPreset::all() {
            let style = p.style();
            assert!(style.size_ndc > 0.0, "{} has non-positive size", p.label());
            assert!(style.line_height > 0.0);
        }
    }

    #[test]
    fn section_title_is_centered_bold() {
        let s = section_title();
        assert_eq!(s.align, WispTextAlign::Center);
        assert_eq!(s.weight, WispFontWeight::Bold);
    }

    #[test]
    fn warning_is_signal_red() {
        let c = warning_privacy_label().color;
        // Red channel dominant, green + blue much lower.
        assert!(c.r > 0.8 && c.g < 0.5 && c.b < 0.5);
    }

    #[test]
    fn watermark_is_low_contrast_and_italic() {
        let s = watermark();
        assert_eq!(s.style, WispFontStyle::Italic);
        assert!(s.color.a < 1.0, "watermark should have alpha < 1");
    }

    #[test]
    fn callout_is_italic() {
        assert_eq!(callout().style, WispFontStyle::Italic);
    }

    #[test]
    fn distinct_sizes_across_presets() {
        // No two presets should be byte-identical.
        let styles: Vec<_> = TextPreset::all().iter().map(|p| p.style()).collect();
        for i in 0..styles.len() {
            for j in (i + 1)..styles.len() {
                assert_ne!(styles[i], styles[j], "presets {i} and {j} are identical");
            }
        }
    }

    #[test]
    fn labels_are_unique_and_non_empty() {
        let mut labels: Vec<_> = TextPreset::all().iter().map(|p| p.label()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), TextPreset::all().len());
        for label in &labels {
            assert!(!label.is_empty());
        }
    }
}
