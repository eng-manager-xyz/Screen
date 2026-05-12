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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_options_non_empty() {
        assert!(!sample_overlay_options().is_empty());
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
