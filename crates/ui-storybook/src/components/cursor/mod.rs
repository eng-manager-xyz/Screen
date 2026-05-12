//! Cursor Studio components — cursor style picker shell, cursor preview canvas
//! with a Wisp-backed render path. Filled in across UI-20 / UI-21.

pub mod cursor_preview_canvas;
pub mod cursor_studio_shell;

pub use cursor_preview_canvas::{
    ClickEffect, CursorAppearancePanel, CursorAppearanceView, CursorBehaviorView, CursorColor,
    CursorPreviewBackend, CursorPreviewCanvas,
};
pub use cursor_studio_shell::{
    CursorStudioShell, CursorStudioShellView, CursorStyle, CursorStylePicker,
    CursorStylePickerView, CursorStyleTile, CursorStyleTileView,
};
