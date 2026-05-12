//! Recorder components — the tray recorder popover, capture-mode tabs, device
//! pickers, audio source list, on-screen overlay options, recording-controls
//! footer. Filled in across UI-06 through UI-13.

pub mod capture_mode_tabs;
pub mod recording_toolbar;

pub use capture_mode_tabs::CaptureModeTabs;
pub use recording_toolbar::{RecordingState, RecordingToolbar};
