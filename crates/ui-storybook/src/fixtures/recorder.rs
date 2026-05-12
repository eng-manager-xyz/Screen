//! Recorder fixtures — capture mode, source selection, on-screen options,
//! recording state. Filled in across UI-06..13. Minimal scaffolding for
//! UI-00 so the smoke tests pass.

/// Capture mode tab — Screen / Window / Area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    /// Full-display recording.
    Screen,
    /// Single application window.
    Window,
    /// User-drawn rectangle on the desktop.
    Area,
}

/// Recording state shared by the toolbar, footer, and tray popover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecorderState {
    /// Configuring sources, not yet recording.
    Idle,
    /// Countdown before recording starts.
    Countdown {
        /// Seconds remaining.
        seconds: u8,
    },
    /// Actively capturing.
    Recording,
    /// Recording but paused.
    Paused,
}

/// One on-screen overlay option (keypress badges, cursor highlights, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayOptionFixture {
    /// Stable id.
    pub id: &'static str,
    /// Display label.
    pub label: &'static str,
    /// Short description shown under the label.
    pub hint: &'static str,
    /// `true` when the toggle is on.
    pub enabled: bool,
}

/// Sample overlay options for the on-screen-options popover.
#[must_use]
pub fn sample_overlay_options() -> Vec<OverlayOptionFixture> {
    vec![
        OverlayOptionFixture {
            id: "overlay-keypress",
            label: "Show keypresses",
            hint: "Render ⌘C / ⌃-space badges on top of the recording",
            enabled: true,
        },
        OverlayOptionFixture {
            id: "overlay-clicks",
            label: "Highlight clicks",
            hint: "Soft ripple around each mouse click",
            enabled: true,
        },
        OverlayOptionFixture {
            id: "overlay-desktop",
            label: "Hide desktop icons",
            hint: "Blur the wallpaper area while recording",
            enabled: false,
        },
        OverlayOptionFixture {
            id: "overlay-sensitive",
            label: "Redact sensitive areas",
            hint: "Auto-blur regions marked as sensitive",
            enabled: false,
        },
    ]
}

/// Sample on-screen-option rows for the `OnScreenOptionsPopover`
/// (UI-10).
#[must_use]
pub fn sample_on_screen_options(
    sensitive_disabled: bool,
) -> Vec<crate::components::recorder::OnScreenOptionView> {
    use crate::components::recorder::{OnScreenOptionKind, OnScreenOptionView};
    vec![
        OnScreenOptionView {
            id: OnScreenOptionKind::CleanDesktop,
            title: "Clean up the desktop",
            description: "Hide icons + the dock for a clean recording. Wallpaper stays.",
            enabled: true,
            disabled: false,
        },
        OnScreenOptionView {
            id: OnScreenOptionKind::ShowKeys,
            title: "Show keys you press",
            description: "Render ⌘C / ⌃-space badges over the recording so viewers can follow along.",
            enabled: true,
            disabled: false,
        },
        OnScreenOptionView {
            id: OnScreenOptionKind::BlurSensitiveInfo,
            title: "Blur sensitive info",
            description: "Auto-detect password fields and other sensitive regions, and blur them. Coming soon.",
            enabled: false,
            disabled: sensitive_disabled,
        },
    ]
}

/// All-on variant for the `all-on` story.
#[must_use]
pub fn sample_on_screen_options_all_on() -> Vec<crate::components::recorder::OnScreenOptionView> {
    sample_on_screen_options(false)
        .into_iter()
        .map(|mut o| {
            o.enabled = true;
            o.disabled = false;
            o
        })
        .collect()
}

/// Long-copy variant for the truncation story.
#[must_use]
pub fn sample_on_screen_options_long_copy() -> Vec<crate::components::recorder::OnScreenOptionView>
{
    use crate::components::recorder::{OnScreenOptionKind, OnScreenOptionView};
    vec![
        OnScreenOptionView {
            id: OnScreenOptionKind::CleanDesktop,
            title: "Clean up the desktop background and remove distracting icons",
            description: "Hide every desktop icon plus the entire dock for a noise-free recording. Wallpaper remains visible. This setting reverts when the recording ends; nothing changes permanently on disk.",
            enabled: true,
            disabled: false,
        },
        OnScreenOptionView {
            id: OnScreenOptionKind::ShowKeys,
            title: "Show keys you press during the recording",
            description: "Render small badges in the bottom-right showing each modifier + key press. Useful for tutorial-style content where viewers want to follow along with what shortcut was used.",
            enabled: false,
            disabled: false,
        },
    ]
}

/// Sample recording-controls footer view (UI-11 / AUT-131). The
/// `start_state` lets stories sweep all `StartRecordingState`
/// variants.
#[must_use]
pub fn sample_recording_controls(
    state: crate::components::recorder::StartRecordingState,
) -> crate::components::recorder::RecordingControlsView {
    use crate::components::recorder::{
        RecordingControlsView, format_auto_zoom_label, format_countdown_label,
    };
    RecordingControlsView {
        auto_zoom_label: format_auto_zoom_label(Some(2.0)),
        countdown_label: format_countdown_label(3),
        shortcuts: vec!["⌘".to_owned(), "⇧".to_owned(), "2".to_owned()],
        start_state: state,
    }
}

/// Compact variant — no auto-zoom, no countdown. Used by the
/// `recording-footer-compact` story.
#[must_use]
pub fn sample_recording_controls_compact() -> crate::components::recorder::RecordingControlsView {
    use crate::components::recorder::{
        RecordingControlsView, StartRecordingState, format_auto_zoom_label, format_countdown_label,
    };
    RecordingControlsView {
        auto_zoom_label: format_auto_zoom_label(None),
        countdown_label: format_countdown_label(0),
        shortcuts: vec!["⌘".to_owned(), "R".to_owned()],
        start_state: StartRecordingState::Ready,
    }
}

/// Sample tray-record-popover view (UI-12 / AUT-132).
#[must_use]
pub fn sample_tray_record_popover(
    open: crate::components::recorder::OpenRecorderPopoverKind,
) -> crate::components::recorder::TrayRecordPopoverView {
    use crate::components::recorder::{
        OnScreenSummaryView, StartRecordingState, TrayRecordPopoverView, WorkspaceSwitcherView,
        format_on_screen_summary,
    };
    use crate::components::shell::AppSection;
    use crate::fixtures::audio_apps::{sample_audio_apps, sample_system_audio_view};
    use crate::fixtures::devices::{
        sample_capture_source_camera, sample_capture_source_microphone, sample_display_source,
    };
    use crate::fixtures::workspaces::sample_workspace_views;

    let workspaces = sample_workspace_views();
    let on_screen = sample_on_screen_options(false);
    let summary = format_on_screen_summary(&on_screen);
    let apps = sample_audio_apps();
    let total = apps.len();
    TrayRecordPopoverView {
        active_section: AppSection::Record,
        capture_mode: CaptureMode::Screen,
        workspaces: WorkspaceSwitcherView {
            workspaces,
            selected_id: "ws-northwind",
        },
        display: sample_display_source(true),
        camera: sample_capture_source_camera(true, false),
        microphone: sample_capture_source_microphone(true, false, Some(0.35)),
        system_audio: sample_system_audio_view(true, false, &apps, total),
        on_screen_summary: OnScreenSummaryView {
            summary,
            options: on_screen,
        },
        controls: sample_recording_controls(StartRecordingState::Ready),
        open,
    }
}

/// Variant with Start recording disabled (no source selected yet).
#[must_use]
pub fn sample_tray_record_popover_start_disabled()
-> crate::components::recorder::TrayRecordPopoverView {
    use crate::components::recorder::{OpenRecorderPopoverKind, StartRecordingState};
    let mut v = sample_tray_record_popover(OpenRecorderPopoverKind::None);
    v.controls = sample_recording_controls(StartRecordingState::Disabled);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_options_non_empty() {
        assert!(!sample_overlay_options().is_empty());
    }

    #[test]
    fn tray_record_popover_is_record_section_by_default() {
        use crate::components::recorder::OpenRecorderPopoverKind;
        use crate::components::shell::AppSection;
        let v = sample_tray_record_popover(OpenRecorderPopoverKind::None);
        assert!(matches!(v.active_section, AppSection::Record));
        assert!(!v.on_screen_summary.summary.is_empty());
    }

    #[test]
    fn recording_controls_preserve_shortcut_order() {
        use crate::components::recorder::StartRecordingState;
        let v = sample_recording_controls(StartRecordingState::Ready);
        assert_eq!(v.shortcuts, vec!["⌘", "⇧", "2"]);
        // The button renders shortcuts in input order; if a future
        // refactor were to sort them this test catches it.
    }

    #[test]
    fn recording_controls_compact_drops_zoom_and_countdown() {
        let v = sample_recording_controls_compact();
        assert!(v.auto_zoom_label.contains("off"));
        assert!(v.countdown_label.starts_with("No"));
    }

    #[test]
    fn on_screen_options_have_one_per_kind() {
        use crate::components::recorder::OnScreenOptionKind;
        let opts = sample_on_screen_options(false);
        assert_eq!(opts.len(), 3);
        for kind in [
            OnScreenOptionKind::CleanDesktop,
            OnScreenOptionKind::ShowKeys,
            OnScreenOptionKind::BlurSensitiveInfo,
        ] {
            assert!(opts.iter().any(|o| o.id == kind));
        }
    }
}
