//! Editor components — editor shell, Wisp canvas host, inspector panel,
//! timeline skeleton, dope sheet, player controls. Filled in across UI-16
//! through UI-19; pre-existing dope-sheet and player-controls live here.

pub mod dope_sheet;
pub mod editor_shell;
pub mod inspector_panel;
pub mod player_controls;
pub mod timeline_skeleton;
pub mod wisp_canvas_host;

pub use dope_sheet::{DopeSheet, DopeSheetKeyframe, DopeSheetTrack, KeyframeKind, TrackKind};
pub use editor_shell::{
    EditorShell, EditorShellView, EditorTitleBar, EditorToolbar, ToolbarActionView,
};
pub use inspector_panel::{
    InspectorPanel, InspectorPanelView, InspectorTab, InspectorTabs, PropertyControlView,
    PropertyRowView, PropertySection, PropertySectionView,
};
pub use player_controls::{PlayState, PlayerControls};
pub use timeline_skeleton::{
    TimelineSkeleton, TimelineTrackRow, TimelineTrackView, TimelineTransport, TimelineView,
};
pub use wisp_canvas_host::{
    CanvasBackendView, DropZoneActionView, EditorDropZoneCanvas, EditorDropZoneView,
    RecentClipView, WispCanvasHost,
};
