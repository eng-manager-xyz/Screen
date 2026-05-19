//! Device fixtures — cameras, microphones, system-audio sources.
//! Filled in across UI-08 (`CaptureSourceRow`) / UI-09
//! (`SystemAudioPickerList`).
//! For UI-00 we ship minimal placeholder shapes so the smoke test sees
//! non-empty collections; UI-08 / UI-09 will widen the fields.

/// A camera or microphone the user could pick as a capture source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceFixture {
    /// Stable id (e.g. `"built-in-mic"`); matches the underlying OS handle
    /// in production but is just a string here.
    pub id: &'static str,
    /// Human label shown in pickers.
    pub label: &'static str,
    /// Connection / availability hint shown in muted text under the label.
    pub hint: &'static str,
    /// Whether the device is currently selected (`true` = filled radio).
    pub selected: bool,
}

/// A system-audio source — an application emitting audio, or "all desktop
/// audio". UI-09 expands this with per-app icons and grouping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemAudioFixture {
    /// Stable id.
    pub id: &'static str,
    /// Display label ("Spotify", "Desktop audio").
    pub label: &'static str,
    /// `true` when this row is in the active selection set.
    pub selected: bool,
}

/// Sample microphones for stories that need a populated picker.
#[must_use]
pub fn sample_microphones() -> Vec<DeviceFixture> {
    vec![
        DeviceFixture {
            id: "mic-built-in",
            label: "Built-in microphone",
            hint: "MacBook · Default",
            selected: true,
        },
        DeviceFixture {
            id: "mic-airpods",
            label: "AirPods Pro",
            hint: "Bluetooth · 86% battery",
            selected: false,
        },
        DeviceFixture {
            id: "mic-shure",
            label: "Shure MV7",
            hint: "USB · Connected",
            selected: false,
        },
    ]
}

/// Sample cameras for stories that need a populated picker.
#[must_use]
pub fn sample_cameras() -> Vec<DeviceFixture> {
    vec![
        DeviceFixture {
            id: "cam-facetime",
            label: "FaceTime HD camera",
            hint: "MacBook · 1080p",
            selected: true,
        },
        DeviceFixture {
            id: "cam-iphone",
            label: "iPhone 15 Pro",
            hint: "Continuity Camera · 4K",
            selected: false,
        },
    ]
}

/// Sample display source view used by the `DisplaySourceCard`
/// stories.
#[must_use]
pub fn sample_display_source(is_selected: bool) -> crate::components::recorder::DisplaySourceView {
    use crate::components::recorder::{DisplayPreviewView, DisplaySourceView, PreviewWindowChip};
    DisplaySourceView {
        id: "display-built-in".to_owned(),
        name: "Built-in Retina display".to_owned(),
        size_label: "14\"".to_owned(),
        dimensions_label: "3024 × 1964".to_owned(),
        is_favorite: true,
        is_selected,
        preview: DisplayPreviewView {
            aspect_ratio: (16, 10),
            overlay_label: Some("Built-in 14\"".to_owned()),
            mock_windows: vec![
                PreviewWindowChip {
                    label: "Safari".to_owned(),
                    color: "rgba(99, 102, 241, 0.85)".to_owned(),
                    left_pct: 6,
                    top_pct: 12,
                    width_pct: 56,
                    height_pct: 70,
                },
                PreviewWindowChip {
                    label: "Terminal".to_owned(),
                    color: "rgba(15, 15, 15, 0.85)".to_owned(),
                    left_pct: 56,
                    top_pct: 28,
                    width_pct: 36,
                    height_pct: 50,
                },
                PreviewWindowChip {
                    label: "Notes".to_owned(),
                    color: "rgba(250, 204, 21, 0.40)".to_owned(),
                    left_pct: 16,
                    top_pct: 40,
                    width_pct: 32,
                    height_pct: 30,
                },
            ],
        },
    }
}

/// Wide variant — ultrawide aspect with two windows.
#[must_use]
pub fn sample_display_source_wide() -> crate::components::recorder::DisplaySourceView {
    use crate::components::recorder::{DisplayPreviewView, DisplaySourceView, PreviewWindowChip};
    DisplaySourceView {
        id: "display-ultrawide".to_owned(),
        name: "LG UltraWide".to_owned(),
        size_label: "34\"".to_owned(),
        dimensions_label: "3440 × 1440".to_owned(),
        is_favorite: false,
        is_selected: true,
        preview: DisplayPreviewView {
            aspect_ratio: (21, 9),
            overlay_label: Some("LG 34\"".to_owned()),
            mock_windows: vec![
                PreviewWindowChip {
                    label: "Zoom".to_owned(),
                    color: "rgba(56, 189, 248, 0.80)".to_owned(),
                    left_pct: 4,
                    top_pct: 12,
                    width_pct: 42,
                    height_pct: 75,
                },
                PreviewWindowChip {
                    label: "Slack".to_owned(),
                    color: "rgba(244, 114, 182, 0.65)".to_owned(),
                    left_pct: 52,
                    top_pct: 12,
                    width_pct: 44,
                    height_pct: 75,
                },
            ],
        },
    }
}

/// Small / 4:3 variant.
#[must_use]
pub fn sample_display_source_small() -> crate::components::recorder::DisplaySourceView {
    use crate::components::recorder::{DisplayPreviewView, DisplaySourceView, PreviewWindowChip};
    DisplaySourceView {
        id: "display-cinema".to_owned(),
        name: "Apple Cinema".to_owned(),
        size_label: "20\"".to_owned(),
        dimensions_label: "1680 × 1050".to_owned(),
        is_favorite: false,
        is_selected: false,
        preview: DisplayPreviewView {
            aspect_ratio: (16, 10),
            overlay_label: None,
            mock_windows: vec![PreviewWindowChip {
                label: "Finder".to_owned(),
                color: "rgba(15, 15, 15, 0.7)".to_owned(),
                left_pct: 22,
                top_pct: 25,
                width_pct: 56,
                height_pct: 55,
            }],
        },
    }
}

/// Sample `CaptureSourceView` for the camera row in collapsed state.
#[must_use]
pub fn sample_capture_source_camera(
    enabled: bool,
    expanded: bool,
) -> crate::components::recorder::CaptureSourceView {
    use crate::components::recorder::{CaptureSourceKind, CaptureSourceView};
    CaptureSourceView {
        id: "camera-facetime".to_owned(),
        kind: CaptureSourceKind::Camera,
        title: "FaceTime HD Camera".to_owned(),
        subtitle: "MacBook · 1080p".to_owned(),
        enabled,
        expanded,
        favorite: false,
        level: None,
    }
}

/// Sample `CaptureSourceView` for the microphone row in collapsed state.
#[must_use]
pub fn sample_capture_source_microphone(
    enabled: bool,
    expanded: bool,
    level: Option<f32>,
) -> crate::components::recorder::CaptureSourceView {
    use crate::components::recorder::{CaptureSourceKind, CaptureSourceView};
    CaptureSourceView {
        id: "mic-built-in".to_owned(),
        kind: CaptureSourceKind::Microphone,
        title: "MacBook Pro Microphone".to_owned(),
        subtitle: "Built-in · 48 kHz".to_owned(),
        enabled,
        expanded,
        favorite: true,
        level,
    }
}

/// Sample camera options for the expanded picker.
#[must_use]
pub fn sample_camera_options() -> Vec<crate::components::recorder::DeviceOptionView> {
    use crate::components::recorder::{DeviceOptionView, DeviceThumb};
    vec![
        DeviceOptionView {
            id: "cam-facetime".to_owned(),
            name: "FaceTime HD Camera".to_owned(),
            detail: "MacBook · 1080p".to_owned(),
            badge: None,
            selected: true,
            level: None,
            thumbnail: Some(DeviceThumb {
                background: "linear-gradient(135deg, #4338ca, #db2777)".to_owned(),
                glyph: "PE".to_owned(),
            }),
        },
        DeviceOptionView {
            id: "cam-iphone".to_owned(),
            name: "iPhone 15 Pro".to_owned(),
            detail: "Continuity · 4K".to_owned(),
            badge: Some("Wireless".to_owned()),
            selected: false,
            level: None,
            thumbnail: Some(DeviceThumb {
                background: "linear-gradient(135deg, #0ea5e9, #1e293b)".to_owned(),
                glyph: "iP".to_owned(),
            }),
        },
        DeviceOptionView {
            id: "cam-obs".to_owned(),
            name: "OBS Virtual Camera".to_owned(),
            detail: "Not connected".to_owned(),
            badge: None,
            selected: false,
            level: None,
            thumbnail: Some(DeviceThumb {
                background: "linear-gradient(135deg, #525252, #1f1f23)".to_owned(),
                glyph: "OB".to_owned(),
            }),
        },
    ]
}

/// Sample microphone options for the expanded picker.
#[must_use]
pub fn sample_microphone_options() -> Vec<crate::components::recorder::DeviceOptionView> {
    use crate::components::recorder::DeviceOptionView;
    vec![
        DeviceOptionView {
            id: "mic-built-in".to_owned(),
            name: "MacBook Pro Microphone".to_owned(),
            detail: "Built-in · 48 kHz".to_owned(),
            badge: None,
            selected: true,
            level: Some(0.32),
            thumbnail: None,
        },
        DeviceOptionView {
            id: "mic-airpods".to_owned(),
            name: "AirPods Pro".to_owned(),
            detail: "Bluetooth · 86% battery".to_owned(),
            badge: Some("Wireless".to_owned()),
            selected: false,
            level: Some(0.12),
            thumbnail: None,
        },
        DeviceOptionView {
            id: "mic-shure".to_owned(),
            name: "Shure MV7".to_owned(),
            detail: "USB · Connected".to_owned(),
            badge: None,
            selected: false,
            level: Some(0.55),
            thumbnail: None,
        },
    ]
}

/// Sample system-audio rows for the system-audio picker.
#[must_use]
pub fn sample_system_audio() -> Vec<SystemAudioFixture> {
    vec![
        SystemAudioFixture {
            id: "audio-desktop",
            label: "All desktop audio",
            selected: true,
        },
        SystemAudioFixture {
            id: "audio-spotify",
            label: "Spotify",
            selected: false,
        },
        SystemAudioFixture {
            id: "audio-zoom",
            label: "Zoom",
            selected: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn microphones_non_empty_and_has_a_default() {
        let mics = sample_microphones();
        assert!(!mics.is_empty(), "fixture must populate at least one mic");
        assert!(mics.iter().any(|m| m.selected), "exactly one mic selected");
    }

    #[test]
    fn cameras_non_empty() {
        assert!(!sample_cameras().is_empty());
    }

    #[test]
    fn system_audio_non_empty() {
        assert!(!sample_system_audio().is_empty());
    }
}
