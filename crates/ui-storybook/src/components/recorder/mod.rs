//! Recorder components — the tray recorder popover, capture-mode tabs, device
//! pickers, audio source list, on-screen overlay options, recording-controls
//! footer. Filled in across UI-06 through UI-13.

pub mod capture_mode_tabs;
pub mod capture_source_row;
pub mod device_picker;
pub mod display_source;
pub mod recording_toolbar;

pub use capture_mode_tabs::CaptureModeTabs;
pub use capture_source_row::{CaptureSourceKind, CaptureSourceRow, CaptureSourceView};
pub use device_picker::{DeviceOptionView, DevicePickerMenu, DevicePickerState, DeviceThumb};
pub use display_source::{
    DisplayPreviewFrame, DisplayPreviewView, DisplaySourceCard, DisplaySourceView,
    PreviewWindowChip, aspect_ratio_css,
};
pub use recording_toolbar::{RecordingState, RecordingToolbar};
