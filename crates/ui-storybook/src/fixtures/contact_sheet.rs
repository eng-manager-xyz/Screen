//! Contact-sheet aggregator (M-UI.22 / AUT-142).
//!
//! Each per-surface fixture module declares its own canonical
//! samples. This module re-collects them into a single deterministic
//! `UiFixtureSet` so the contact-sheet story + design-review docs
//! have one source of truth.

use crate::components::cursor::CursorAppearanceView;
use crate::components::library::RecordingCardView;
use crate::components::recorder::{AudioAppView, CaptureSourceView, DisplaySourceView};
use crate::components::shell::WorkspaceView;

/// Aggregated fixture bundle. Field order is stable; tests assert
/// non-empty.
#[derive(Clone, Debug, PartialEq)]
pub struct UiFixtureSet {
    /// Workspace switcher rows.
    pub workspaces: Vec<WorkspaceView>,
    /// Display sources for the screen tab.
    pub displays: Vec<DisplaySourceView>,
    /// Camera + microphone capture rows.
    pub devices: DeviceFixtureSet,
    /// System-audio app rows.
    pub audio_apps: Vec<AudioAppView>,
    /// Library recording cards.
    pub recordings: Vec<RecordingCardView>,
    /// Cursor appearance presets.
    pub cursor_presets: Vec<CursorAppearanceView>,
}

/// Pair of capture-source rows so callers don't have to fish them
/// out of the device fixture module individually.
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceFixtureSet {
    /// Camera capture row.
    pub camera: CaptureSourceView,
    /// Microphone capture row.
    pub microphone: CaptureSourceView,
}

/// Deterministic fixture bundle used by the contact sheet + the
/// design-review chapter.
#[must_use]
pub fn default_ui_fixtures() -> UiFixtureSet {
    use crate::fixtures::audio_apps::sample_audio_apps;
    use crate::fixtures::cursor::{
        sample_cursor_appearance, sample_cursor_appearance_pulse,
        sample_cursor_appearance_spotlight, sample_cursor_appearance_trail_on,
    };
    use crate::fixtures::devices::{
        sample_capture_source_camera, sample_capture_source_microphone, sample_display_source,
        sample_display_source_small, sample_display_source_wide,
    };
    use crate::fixtures::library::sample_recording_cards;
    use crate::fixtures::workspaces::sample_workspace_views;

    UiFixtureSet {
        workspaces: sample_workspace_views(),
        displays: vec![
            sample_display_source(true),
            sample_display_source_wide(),
            sample_display_source_small(),
        ],
        devices: DeviceFixtureSet {
            camera: sample_capture_source_camera(true, false),
            microphone: sample_capture_source_microphone(true, false, Some(0.35)),
        },
        audio_apps: sample_audio_apps(),
        recordings: sample_recording_cards(),
        cursor_presets: vec![
            sample_cursor_appearance(),
            sample_cursor_appearance_pulse(),
            sample_cursor_appearance_spotlight(),
            sample_cursor_appearance_trail_on(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures_are_non_empty_and_deterministic() {
        let a = default_ui_fixtures();
        let b = default_ui_fixtures();
        assert_eq!(a, b);
        assert!(!a.workspaces.is_empty());
        assert!(!a.displays.is_empty());
        assert!(!a.audio_apps.is_empty());
        assert!(!a.recordings.is_empty());
        assert!(!a.cursor_presets.is_empty());
    }

    #[test]
    fn fixture_ids_are_stable() {
        let a = default_ui_fixtures();
        let ids: Vec<_> = a.recordings.iter().map(|r| r.id.clone()).collect();
        // Documented stability — the first card is always `rec-01`.
        assert_eq!(ids.first().map(String::as_str), Some("rec-01"));
    }
}
