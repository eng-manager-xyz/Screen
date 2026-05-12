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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_options_non_empty() {
        assert!(!sample_overlay_options().is_empty());
    }
}
