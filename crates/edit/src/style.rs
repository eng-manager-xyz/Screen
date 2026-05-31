//! The cinematic framing layer — background, cursor, crop, and aspect.
//!
//! These configs are consumed by the renderer (`wisp`) at preview +
//! export time to wrap the raw screen capture in the produced look the
//! editor's Inspector exposes (ED.15 / ED.18 / ED.19). Default values
//! mirror the reference design (padding 64 px, corner radius 14 px,
//! shadow 60, cursor 180 %, auto-zoom hold 1.2 s / max 2.4×).

use serde::{Deserialize, Serialize};

/// The backdrop the framed screen sits on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackgroundSource {
    /// A named wallpaper from the bundled set (e.g. `"aurora"`).
    Wallpaper {
        /// Wallpaper identifier.
        name: String,
    },
    /// A linear gradient between two RGB colors at `angle_deg`.
    Gradient {
        /// Start color, RGB `0..=255`.
        from: [u8; 3],
        /// End color, RGB `0..=255`.
        to: [u8; 3],
        /// Gradient angle in degrees (0 = left→right, 90 = bottom→top).
        angle_deg: f32,
    },
    /// A flat fill.
    Color {
        /// Fill color, RGB `0..=255`.
        rgb: [u8; 3],
    },
}

impl Default for BackgroundSource {
    fn default() -> Self {
        // An "Aurora"-style warm→cool diagonal gradient, matching the
        // reference design's default background swatch.
        Self::Gradient {
            from: [255, 138, 128],
            to: [40, 53, 147],
            angle_deg: 135.0,
        }
    }
}

/// Background framing: the backdrop plus the padding / rounding / shadow
/// that lift the screen capture off it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BackgroundConfig {
    /// The backdrop fill.
    #[serde(default)]
    pub source: BackgroundSource,
    /// Padding between the backdrop edge and the framed screen, in
    /// composed-canvas pixels.
    pub padding: u32,
    /// Corner radius of the framed screen, in composed-canvas pixels.
    pub corner_radius: u32,
    /// Drop-shadow strength, `0..=100`.
    pub shadow: u32,
    /// Inset border (a colored frame just inside the screen edge), in
    /// pixels. `0` disables it.
    pub inset: u32,
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self {
            source: BackgroundSource::default(),
            padding: 64,
            corner_radius: 14,
            shadow: 60,
            inset: 0,
        }
    }
}

/// Auto-zoom detection settings (ED.17 generates `Auto` zoom regions
/// using these thresholds).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoZoomConfig {
    /// Whether to auto-generate zoom regions from cursor/click telemetry.
    pub detect_from_cursor: bool,
    /// How long a generated zoom holds at full amount, in milliseconds.
    pub hold_time_ms: u32,
    /// Maximum zoom factor an auto-generated region may reach.
    pub max_zoom: f64,
}

impl Default for AutoZoomConfig {
    fn default() -> Self {
        Self {
            detect_from_cursor: true,
            hold_time_ms: 1200,
            max_zoom: 2.4,
        }
    }
}

/// Cursor styling and smoothing (ED.19 renders a cursor overlay driven
/// by these settings + the captured cursor track).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CursorConfig {
    /// Cursor size as a percentage of its native size, e.g. `180`.
    pub size_pct: u32,
    /// Motion smoothing strength, `0..=100`.
    pub smoothing: u32,
    /// Render a ripple effect on clicks.
    pub click_ripples: bool,
    /// Hide the cursor while it is stationary.
    pub hide_static: bool,
    /// Auto-zoom detection settings (shown under the cursor inspector in
    /// the reference design).
    #[serde(default)]
    pub auto_zoom: AutoZoomConfig,
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            size_pct: 180,
            smoothing: 80,
            click_ripples: true,
            hide_static: true,
            auto_zoom: AutoZoomConfig::default(),
        }
    }
}

/// A crop rectangle, normalized to `[0, 1]` of the source frame:
/// `(x, y)` is the top-left corner, `(width, height)` the extent. The
/// full frame is `{ x: 0, y: 0, width: 1, height: 1 }`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CropRect {
    /// Left edge, `0.0..=1.0`.
    pub x: f32,
    /// Top edge, `0.0..=1.0`.
    pub y: f32,
    /// Width, `0.0..=1.0`.
    pub width: f32,
    /// Height, `0.0..=1.0`.
    pub height: f32,
}

impl CropRect {
    /// The full, uncropped frame.
    #[must_use]
    pub fn full() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }
    }

    /// Whether this rect is (approximately) the full frame.
    #[must_use]
    pub fn is_full(self) -> bool {
        (self.x).abs() < 1e-4
            && (self.y).abs() < 1e-4
            && (self.width - 1.0).abs() < 1e-4
            && (self.height - 1.0).abs() < 1e-4
    }
}

impl Default for CropRect {
    fn default() -> Self {
        Self::full()
    }
}

/// Output aspect ratio. Drives the composed-canvas dimensions; changing
/// it reframes the export (e.g. a 16:9 desktop recording → a 9:16 short).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AspectRatio {
    /// 16:9 widescreen — the default.
    #[default]
    Wide,
    /// 9:16 vertical (shorts / reels).
    Vertical,
    /// 1:1 square.
    Square,
    /// 4:3 classic.
    Classic,
}

impl AspectRatio {
    /// The width:height ratio as integers.
    #[must_use]
    pub fn ratio(self) -> (u32, u32) {
        match self {
            Self::Wide => (16, 9),
            Self::Vertical => (9, 16),
            Self::Square => (1, 1),
            Self::Classic => (4, 3),
        }
    }

    /// Canvas pixel dimensions whose longer edge is `long_edge`,
    /// preserving the ratio. Both edges are rounded down to even numbers
    /// (H.264 chroma subsampling requires even dimensions).
    #[must_use]
    pub fn canvas_dims(self, long_edge: u32) -> (u32, u32) {
        let (rw, rh) = self.ratio();
        let (w, h) = if rw >= rh {
            (long_edge, long_edge * rh / rw)
        } else {
            (long_edge * rw / rh, long_edge)
        };
        (w & !1, h & !1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_defaults_match_reference_design() {
        let bg = BackgroundConfig::default();
        assert_eq!(bg.padding, 64);
        assert_eq!(bg.corner_radius, 14);
        assert_eq!(bg.shadow, 60);
        assert_eq!(bg.inset, 0);
        assert!(matches!(bg.source, BackgroundSource::Gradient { .. }));
    }

    #[test]
    fn cursor_defaults_match_reference_design() {
        let c = CursorConfig::default();
        assert_eq!(c.size_pct, 180);
        assert_eq!(c.smoothing, 80);
        assert!(c.click_ripples);
        assert!(c.hide_static);
        assert!(c.auto_zoom.detect_from_cursor);
        assert_eq!(c.auto_zoom.hold_time_ms, 1200);
        assert!((c.auto_zoom.max_zoom - 2.4).abs() < 1e-9);
    }

    #[test]
    fn crop_full_is_full() {
        assert!(CropRect::full().is_full());
        assert!(CropRect::default().is_full());
        assert!(
            !CropRect {
                x: 0.1,
                y: 0.0,
                width: 0.8,
                height: 1.0,
            }
            .is_full()
        );
    }

    #[test]
    fn aspect_ratios_and_canvas_dims() {
        assert_eq!(AspectRatio::default(), AspectRatio::Wide);
        assert_eq!(AspectRatio::Wide.ratio(), (16, 9));
        assert_eq!(AspectRatio::Vertical.ratio(), (9, 16));
        // 1920 long edge, 16:9 → 1920×1080.
        assert_eq!(AspectRatio::Wide.canvas_dims(1920), (1920, 1080));
        // 9:16 vertical with long edge 1920 → 1080×1920.
        assert_eq!(AspectRatio::Vertical.canvas_dims(1920), (1080, 1920));
        // Square.
        assert_eq!(AspectRatio::Square.canvas_dims(1080), (1080, 1080));
        // Even-dimension guarantee.
        let (w, h) = AspectRatio::Classic.canvas_dims(1001);
        assert_eq!(w % 2, 0);
        assert_eq!(h % 2, 0);
    }

    #[test]
    fn config_partial_json_fills_missing_fields_from_default() {
        // ED.23 forward-compat: container `#[serde(default)]` fills any
        // missing field from the struct's (design-meaningful) Default.
        let bg: BackgroundConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(bg, BackgroundConfig::default());
        let cur: CursorConfig = serde_json::from_str(r#"{"size_pct": 200}"#).unwrap();
        assert_eq!(cur.size_pct, 200);
        assert_eq!(
            cur.smoothing,
            CursorConfig::default().smoothing,
            "missing field defaulted, not zeroed"
        );
        let az: AutoZoomConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(az, AutoZoomConfig::default());
    }
}
