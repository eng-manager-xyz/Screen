//! Recorder components — the tray recorder popover, capture-mode tabs, device
//! pickers, audio source list, on-screen overlay options, recording-controls
//! footer. Filled in across UI-06 through UI-13.

pub mod capture_mode_tabs;
pub mod capture_source_row;
pub mod device_picker;
pub mod display_source;
pub mod on_screen_options;
pub mod recording_controls_footer;
pub mod recording_selects;
pub mod recording_status_button;
pub mod recording_toolbar;
pub mod save_panel;
pub mod system_audio;
pub mod tray_record_popover;

pub use capture_mode_tabs::CaptureModeTabs;
pub use capture_source_row::{CaptureSourceKind, CaptureSourceRow, CaptureSourceView};
pub use device_picker::{DeviceOptionView, DevicePickerMenu, DevicePickerState, DeviceThumb};
pub use display_source::{
    DisplayPreviewFrame, DisplayPreviewView, DisplaySourceCard, DisplaySourceView,
    PreviewWindowChip, aspect_ratio_css,
};
pub use on_screen_options::{OnScreenOptionKind, OnScreenOptionView, OnScreenOptionsPopover};
pub use recording_controls_footer::{
    RecordingControlsFooter, RecordingControlsView, StartRecordingButton, StartRecordingState,
};
pub use recording_selects::{
    AutoZoomSelect, CountdownSelect, ShortcutBadgeGroup, format_auto_zoom_label,
    format_countdown_label,
};
pub use recording_status_button::{
    CompactRecordingState, CountdownBadge, RecordingStatusButton, format_countdown_seconds,
};
pub use recording_toolbar::{RecordingState, RecordingToolbar};
pub use save_panel::{SaveFormat, SavePanel, SavePanelView};
pub use system_audio::{
    AppIconView, AudioAppView, AudioFilter, SystemAudioAppList, SystemAudioRow, SystemAudioView,
    format_selection_count,
};
pub use tray_record_popover::{
    OnScreenSummaryView, OpenRecorderPopoverKind, TrayOverflowHint, TrayRecordPopover,
    TrayRecordPopoverView, WorkspaceSwitcherView, format_on_screen_summary,
    format_system_audio_summary,
};
