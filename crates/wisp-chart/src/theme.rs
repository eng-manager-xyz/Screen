//! `Theme` — the visual configuration applied when rendering a
//! chart. `Theme::light()` ships v1; dark / custom themes follow.

use crate::color::Color;
use crate::palette::OwnerPalette;

/// Line style for grid lines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineStyle {
    /// Stroke colour.
    pub color: Color,
    /// Stroke width in device pixels.
    pub width: f32,
}

/// Visual configuration applied at render time.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// Canvas background fill.
    pub bg: Color,
    /// Alternating row tint (or `None` to disable).
    pub row_alt_bg: Option<Color>,
    /// Week-strip grid lines.
    pub grid_week: LineStyle,
    /// Month-strip grid lines (heavier than weeks).
    pub grid_month: LineStyle,
    /// Header band background.
    pub header_bg: Color,
    /// Primary text colour (labels, headers).
    pub text_primary: Color,
    /// Muted text colour (week labels, secondary info).
    pub text_muted: Color,
    /// Bar corner radius in pixels.
    pub bar_corner_radius: f32,
    /// Bar height in pixels (centred within the row).
    pub bar_height: f32,
    /// Row height in pixels.
    pub row_height: f32,
    /// Left gutter width (project labels) in pixels.
    pub gutter_width: f32,
    /// Header band height in pixels.
    pub header_height: f32,
    /// Owner-colour palette + auto-assignment policy.
    pub palette: OwnerPalette,
}

impl Theme {
    /// The v1 light theme — white background, alt-row tint
    /// `#fafafa`, Wong palette, 44 px rows / 28 px bars / 6 px
    /// corner radius / 180 px gutter / 60 px header.
    #[must_use]
    pub fn light() -> Self {
        Self {
            bg: Color::from_hex("#ffffff").unwrap(),
            row_alt_bg: Color::from_hex("#fafafa"),
            grid_week: LineStyle {
                color: Color::from_hex("#e5e5e5").unwrap(),
                width: 1.0,
            },
            grid_month: LineStyle {
                color: Color::from_hex("#cccccc").unwrap(),
                width: 2.0,
            },
            header_bg: Color::from_hex("#f5f5f5").unwrap(),
            text_primary: Color::from_hex("#222222").unwrap(),
            text_muted: Color::from_hex("#888888").unwrap(),
            bar_corner_radius: 6.0,
            bar_height: 28.0,
            row_height: 44.0,
            gutter_width: 180.0,
            header_height: 60.0,
            palette: OwnerPalette::default(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_theme_uses_white_bg() {
        let t = Theme::light();
        assert_eq!(t.bg, Color::WHITE);
    }

    #[test]
    fn light_theme_dimensions_match_spec() {
        let t = Theme::light();
        assert!((t.bar_height - 28.0).abs() < 1e-6);
        assert!((t.row_height - 44.0).abs() < 1e-6);
        assert!((t.gutter_width - 180.0).abs() < 1e-6);
        assert!((t.header_height - 60.0).abs() < 1e-6);
        assert!((t.bar_corner_radius - 6.0).abs() < 1e-6);
    }
}
