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
        id: "display-built-in",
        name: "Built-in Retina display",
        size_label: "14\"",
        dimensions_label: "3024 × 1964",
        is_favorite: true,
        is_selected,
        preview: DisplayPreviewView {
            aspect_ratio: (16, 10),
            overlay_label: Some("Built-in 14\""),
            mock_windows: vec![
                PreviewWindowChip {
                    label: "Safari",
                    color: "rgba(99, 102, 241, 0.85)",
                    left_pct: 6,
                    top_pct: 12,
                    width_pct: 56,
                    height_pct: 70,
                },
                PreviewWindowChip {
                    label: "Terminal",
                    color: "rgba(15, 15, 15, 0.85)",
                    left_pct: 56,
                    top_pct: 28,
                    width_pct: 36,
                    height_pct: 50,
                },
                PreviewWindowChip {
                    label: "Notes",
                    color: "rgba(250, 204, 21, 0.40)",
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
        id: "display-ultrawide",
        name: "LG UltraWide",
        size_label: "34\"",
        dimensions_label: "3440 × 1440",
        is_favorite: false,
        is_selected: true,
        preview: DisplayPreviewView {
            aspect_ratio: (21, 9),
            overlay_label: Some("LG 34\""),
            mock_windows: vec![
                PreviewWindowChip {
                    label: "Zoom",
                    color: "rgba(56, 189, 248, 0.80)",
                    left_pct: 4,
                    top_pct: 12,
                    width_pct: 42,
                    height_pct: 75,
                },
                PreviewWindowChip {
                    label: "Slack",
                    color: "rgba(244, 114, 182, 0.65)",
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
        id: "display-cinema",
        name: "Apple Cinema",
        size_label: "20\"",
        dimensions_label: "1680 × 1050",
        is_favorite: false,
        is_selected: false,
        preview: DisplayPreviewView {
            aspect_ratio: (16, 10),
            overlay_label: None,
            mock_windows: vec![PreviewWindowChip {
                label: "Finder",
                color: "rgba(15, 15, 15, 0.7)",
                left_pct: 22,
                top_pct: 25,
                width_pct: 56,
                height_pct: 55,
            }],
        },
    }
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
