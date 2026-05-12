//! UI components — Leptos `#[component]` definitions matching rust-ui's
//! visual aesthetic (dark zinc palette, subtle borders, soft shadows).
//!
//! Organised into seven subgroups that mirror the recorder's product
//! surfaces:
//! - [`primitives`] — `Button`, `Card`, and reusable control primitives
//!   (UI-01 / UI-04).
//! - [`shell`] — application chrome: `DropZone`, `StatusBar`,
//!   `NavigationRail`, `AppShell` (UI-02).
//! - [`menus`] — popovers, dropdowns, command menus (UI-03 / UI-05 /
//!   UI-10).
//! - [`recorder`] — tray recorder + capture mode + device pickers
//!   (UI-06 through UI-13).
//! - [`library`] — recordings library sidebar + grid (UI-14 / UI-15).
//! - [`editor`] — editor shell, Wisp canvas host, inspector, timeline,
//!   dope sheet, player controls (UI-16 through UI-19).
//! - [`cursor`] — Cursor Studio shell + preview canvas (UI-20 / UI-21).
//!
//! Each subgroup re-exports its public types so
//! `ui_storybook::components::Button` resolves identically to
//! `ui_storybook::components::primitives::Button` — callers (notably
//! `app_ui`) pin to the short path.
//!
//! Every component re-exported from here must have a story in
//! [`crate::stories`]. The story drives the SSR snapshot test.

// Leptos `#[component]` rewrites the function into a builder-pattern struct
// plus a wrapper fn; clippy's `must_use_candidate` and `needless_pass_by_value`
// fire on the rewritten code regardless of attribute placement on the source
// fn. Allowing them at the module level keeps every component clean without
// per-fn pragma noise.
#![allow(
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    reason = "Leptos `#[component]` macro rewrites these patterns; lints fire on generated code"
)]
// UI components are documented via mdBook stories under `_docs/book/src/ui/`,
// not via rustdoc — every visible variant has a screenshot + use case there.
// `missing_docs` would otherwise fire on macro-generated builder structs we
// don't author directly.
#![allow(
    missing_docs,
    reason = "components documented in mdBook stories; rustdoc gate would fire on macro-generated builder structs"
)]

pub mod cursor;
pub mod editor;
pub mod library;
pub mod menus;
pub mod primitives;
pub mod recorder;
pub mod shell;

// Re-export from each populated subgroup so callers can use the short
// `ui_storybook::components::<Name>` path. Empty subgroups (menus, library,
// cursor for now) have nothing to re-export until later tickets fill them in.
pub use editor::{
    DopeSheet, DopeSheetKeyframe, DopeSheetTrack, KeyframeKind, PlayState, PlayerControls,
    TrackKind,
};
pub use menus::{
    MenuBadgeView, MenuFooter, MenuList, MenuRow, MenuRowKind, MenuSection, PopoverPlacement,
    PopoverSurface,
};
pub use primitives::{
    Badge, BadgeKind, Button, ButtonSize, ButtonVariant, Card, CardBody, CardHeader, ColorSwatch,
    Divider, DividerOrientation, IconButton, IconButtonVariant, IconTile, IconTileKind, Kbd, Meter,
    Segment, SegmentedControl, SelectPill, Slider, Surface, SurfaceKind, ToggleSwitch,
};
pub use recorder::{
    AppIconView, AudioAppView, AudioFilter, CaptureModeTabs, CaptureSourceKind, CaptureSourceRow,
    CaptureSourceView, DeviceOptionView, DevicePickerMenu, DevicePickerState, DeviceThumb,
    DisplayPreviewFrame, DisplayPreviewView, DisplaySourceCard, DisplaySourceView,
    OnScreenOptionKind, OnScreenOptionView, OnScreenOptionsPopover, PreviewWindowChip,
    RecordingState, RecordingToolbar, SystemAudioAppList, SystemAudioRow, SystemAudioView,
    aspect_ratio_css, format_selection_count,
};
pub use shell::{
    AppSection, AppShell, DropZone, DropZoneState, NavItemView, NavigationRail, StatusBar,
    StatusKind, UserAvatar, UserAvatarView, WorkspaceBadge, WorkspaceBadgeView,
    WorkspaceSwitcherMenu, WorkspaceView, format_member_count,
};
