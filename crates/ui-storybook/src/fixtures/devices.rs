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
