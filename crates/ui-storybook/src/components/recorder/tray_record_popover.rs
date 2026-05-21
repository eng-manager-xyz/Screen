//! `TrayRecordPopover` (M-UI.12 / AUT-132) — full tray recorder setup
//! popover. Composes UI-02..UI-11 into the floating black rounded
//! window the user sees first. The popover does NOT own which sub-menu
//! is open; the parent passes [`OpenRecorderPopoverKind`] and the
//! component renders the matching overlay.

use leptos::prelude::*;

use crate::components::menus::{MenuList, MenuRow, PopoverPlacement, PopoverSurface};
use crate::components::primitives::{IconTile, IconTileKind};
use crate::components::recorder::{
    CaptureModeTabs, CaptureSourceKind, CaptureSourceRow, CaptureSourceView, DevicePickerMenu,
    DevicePickerState, DisplaySourceCard, DisplaySourceView, OnScreenOptionView,
    OnScreenOptionsPopover, RecordingControlsFooter, RecordingControlsView, SystemAudioAppList,
    SystemAudioRow, SystemAudioView, format_selection_count,
};
use crate::components::shell::{AppSection, WorkspaceSwitcherMenu, WorkspaceView};
use crate::fixtures::recorder::CaptureMode;
use crate::fixtures::recorder::sample_recording_controls;
use crate::fixtures::recorder::sample_recording_controls_compact;

// Silence the two unused fixture imports above when nothing in this
// module references them; they're retained as a hint that the
// recorder fixtures co-evolve with this composition.
const _: fn() = || {
    let _ = sample_recording_controls;
    let _ = sample_recording_controls_compact;
};

/// Which secondary surface (menu, popover, picker) is visually open
/// over the tray record popover.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OpenRecorderPopoverKind {
    /// No overlay — the popover is in its default state.
    #[default]
    None,
    /// Workspace switcher anchored over the rail badge.
    WorkspaceMenu,
    /// Display-source picker (rare; usually the card itself selects).
    DisplayMenu,
    /// Camera device picker.
    CameraMenu,
    /// Microphone device picker.
    MicrophoneMenu,
    /// System-audio app list expanded under the row.
    SystemAudioMenu,
    /// On-screen overlay options popover.
    OnScreenOptions,
    /// Auto-zoom strength menu.
    AutoZoomMenu,
    /// Countdown-length menu.
    CountdownMenu,
}

impl OpenRecorderPopoverKind {
    /// Stable kebab-case slug.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            OpenRecorderPopoverKind::None => "none",
            OpenRecorderPopoverKind::WorkspaceMenu => "workspace",
            OpenRecorderPopoverKind::DisplayMenu => "display",
            OpenRecorderPopoverKind::CameraMenu => "camera",
            OpenRecorderPopoverKind::MicrophoneMenu => "microphone",
            OpenRecorderPopoverKind::SystemAudioMenu => "system-audio",
            OpenRecorderPopoverKind::OnScreenOptions => "on-screen-options",
            OpenRecorderPopoverKind::AutoZoomMenu => "auto-zoom",
            OpenRecorderPopoverKind::CountdownMenu => "countdown",
        }
    }
}

/// View-model bundling the workspace rail header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSwitcherView {
    /// Workspaces in display order.
    pub workspaces: Vec<WorkspaceView>,
    /// Currently-selected workspace id.
    pub selected_id: &'static str,
}

/// Summary chip rendered above the on-screen-options popover trigger.
/// Lists the currently-enabled options as a short comma-separated
/// string for the parent to display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnScreenSummaryView {
    /// Comma-separated label (`"Hide desktop, Show keys"`).
    pub summary: String,
    /// Full option set — used when the popover is opened.
    pub options: Vec<OnScreenOptionView>,
}

/// Composite view-model for the whole tray popover.
#[derive(Debug, Clone, PartialEq)]
pub struct TrayRecordPopoverView {
    /// Which app section is active in the rail (always `Record` for
    /// the tray, but the prop exists so the rail is still a controlled
    /// component).
    pub active_section: AppSection,
    /// Selected capture mode tab.
    pub capture_mode: CaptureMode,
    /// Workspace header.
    pub workspaces: WorkspaceSwitcherView,
    /// Display source card.
    pub display: DisplaySourceView,
    /// Camera row.
    pub camera: CaptureSourceView,
    /// Microphone row.
    pub microphone: CaptureSourceView,
    /// System audio row.
    pub system_audio: SystemAudioView,
    /// On-screen overlay summary.
    pub on_screen_summary: OnScreenSummaryView,
    /// Recording-controls footer.
    pub controls: RecordingControlsView,
    /// Which secondary surface is open over the popover.
    pub open: OpenRecorderPopoverKind,
}

fn render_overlay_stack(
    open: OpenRecorderPopoverKind,
    workspaces: Vec<WorkspaceView>,
    selected_workspace: String,
    on_screen_options: Vec<OnScreenOptionView>,
) -> impl IntoView {
    let workspace = (open == OpenRecorderPopoverKind::WorkspaceMenu).then(|| {
        view! {
            <div class="tray-popover-overlay tray-popover-overlay-workspace">
                <WorkspaceSwitcherMenu workspaces=workspaces selected_id=selected_workspace />
            </div>
        }
    });
    let camera = (open == OpenRecorderPopoverKind::CameraMenu).then(|| {
        view! {
            <div class="tray-popover-overlay tray-popover-overlay-camera">
                <DevicePickerMenu
                    kind=CaptureSourceKind::Camera
                    state=DevicePickerState::Populated
                    devices=crate::fixtures::devices::sample_camera_options()
                />
            </div>
        }
    });
    let microphone = (open == OpenRecorderPopoverKind::MicrophoneMenu).then(|| {
        view! {
            <div class="tray-popover-overlay tray-popover-overlay-microphone">
                <DevicePickerMenu
                    kind=CaptureSourceKind::Microphone
                    state=DevicePickerState::Populated
                    devices=crate::fixtures::devices::sample_microphone_options()
                />
            </div>
        }
    });
    let on_screen = (open == OpenRecorderPopoverKind::OnScreenOptions).then(|| {
        view! {
            <div class="tray-popover-overlay tray-popover-overlay-on-screen">
                <OnScreenOptionsPopover options=on_screen_options />
            </div>
        }
    });
    view! { <>{workspace}{camera}{microphone}{on_screen}</> }
}

#[component]
pub fn TrayRecordPopover(view: TrayRecordPopoverView) -> impl IntoView {
    let TrayRecordPopoverView {
        active_section,
        capture_mode,
        workspaces,
        display,
        camera,
        microphone,
        system_audio,
        on_screen_summary,
        controls,
        open,
    } = view;
    let active_slug = active_section.slug();
    let mode_slug = match capture_mode {
        CaptureMode::Screen => "screen",
        CaptureMode::Window => "window",
        CaptureMode::Area => "area",
    };
    let workspace_chip = workspaces
        .workspaces
        .iter()
        .find(|w| w.id == workspaces.selected_id)
        .cloned();
    let overlays = render_overlay_stack(
        open,
        workspaces.workspaces.clone(),
        workspaces.selected_id.to_string(),
        on_screen_summary.options.clone(),
    );
    let workspace_label = workspace_chip.as_ref().map_or("Personal", |w| w.name);
    let workspace_initials = workspace_chip.as_ref().map_or("PS", |w| w.initials);

    let system_audio_expanded =
        system_audio.expanded || open == OpenRecorderPopoverKind::SystemAudioMenu;
    let system_audio_for_row = SystemAudioView {
        expanded: system_audio_expanded,
        ..system_audio.clone()
    };

    let summary_label = on_screen_summary.summary.clone();

    view! {
        <section class="tray-record-popover" data-section=active_slug data-mode=mode_slug data-open=open.slug() role="dialog" aria-label="Tray record setup">
            <header class="tray-record-header">
                <button class="tray-record-workspace-chip" aria-haspopup="dialog" aria-expanded=open == OpenRecorderPopoverKind::WorkspaceMenu>
                    <span class="icon-tile icon-tile-workspace" aria-hidden="true">{workspace_initials}</span>
                    <span class="tray-record-workspace-label">{workspace_label}</span>
                    <span class="tray-record-chevron" aria-hidden="true">"▾"</span>
                </button>
                <CaptureModeTabs selected=capture_mode />
            </header>

            <div class="tray-record-body">
                <section class="tray-record-display">
                    <DisplaySourceCard view=display />
                </section>

                <section class="tray-record-sources">
                    <CaptureSourceRow view=camera />
                    <CaptureSourceRow view=microphone />
                </section>

                <section class="tray-record-audio">
                    <SystemAudioRow view=system_audio_for_row />
                    {system_audio_expanded.then(|| view! {
                        <SystemAudioAppList apps=crate::fixtures::audio_apps::sample_audio_apps() />
                    })}
                </section>

                <section class="tray-record-on-screen">
                    <button
                        class="tray-record-on-screen-row"
                        aria-haspopup="dialog"
                        aria-expanded=open == OpenRecorderPopoverKind::OnScreenOptions
                    >
                        <IconTile kind=IconTileKind::Action>"✦"</IconTile>
                        <span class="tray-record-on-screen-text">
                            <span class="tray-record-on-screen-title">"On-screen"</span>
                            <span class="tray-record-on-screen-summary">{summary_label}</span>
                        </span>
                        <span class="tray-record-chevron" aria-hidden="true">"▸"</span>
                    </button>
                </section>
            </div>

            <RecordingControlsFooter view=controls />

            {overlays}
        </section>
    }
}

/// Helper: build a one-line "Hide desktop, Show keys" summary from a
/// list of [`OnScreenOptionView`]. Strips trailing punctuation,
/// dedupes ordering by enabled-then-disabled.
#[must_use]
pub fn format_on_screen_summary(options: &[OnScreenOptionView]) -> String {
    let enabled: Vec<&str> = options
        .iter()
        .filter(|o| o.enabled)
        .map(|o| o.title.as_str())
        .collect();
    if enabled.is_empty() {
        "All overlays off".to_owned()
    } else {
        enabled.join(", ")
    }
}

/// Helper: render the "N of M apps selected" string for the
/// system-audio summary slot. Re-uses
/// [`format_selection_count`] so the
/// wording is identical to the standalone row.
#[must_use]
pub fn format_system_audio_summary(view: &SystemAudioView) -> String {
    if view.enabled {
        format_selection_count(view.selected_count, view.total_count)
    } else {
        "Disabled".to_owned()
    }
}

/// Tiny convenience: render a `<MenuRow>`-shaped placeholder used when
/// a sub-menu's natural placement would overflow the popover.
#[component]
pub fn TrayOverflowHint(#[prop(into)] message: String) -> impl IntoView {
    view! {
        <PopoverSurface placement=PopoverPlacement::BottomRight width_px=240_u16>
            <MenuList label="Hint">
                <MenuRow title=message />
            </MenuList>
        </PopoverSurface>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::recorder::{OnScreenOptionKind, OnScreenOptionView};

    fn opt(id: OnScreenOptionKind, enabled: bool) -> OnScreenOptionView {
        let title = match id {
            OnScreenOptionKind::CleanDesktop => "Clean desktop",
            OnScreenOptionKind::ShowKeys => "Show keys",
            OnScreenOptionKind::BlurSensitiveInfo => "Blur sensitive",
        };
        OnScreenOptionView {
            id,
            title: title.to_owned(),
            description: String::new(),
            enabled,
            disabled: false,
        }
    }

    #[test]
    fn open_kind_slugs_unique() {
        let kinds = [
            OpenRecorderPopoverKind::None,
            OpenRecorderPopoverKind::WorkspaceMenu,
            OpenRecorderPopoverKind::DisplayMenu,
            OpenRecorderPopoverKind::CameraMenu,
            OpenRecorderPopoverKind::MicrophoneMenu,
            OpenRecorderPopoverKind::SystemAudioMenu,
            OpenRecorderPopoverKind::OnScreenOptions,
            OpenRecorderPopoverKind::AutoZoomMenu,
            OpenRecorderPopoverKind::CountdownMenu,
        ];
        let slugs: Vec<_> = kinds.iter().map(|k| k.slug()).collect();
        let mut sorted = slugs.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), slugs.len());
        for s in slugs {
            assert!(s.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
        }
    }

    #[test]
    fn on_screen_summary_lists_enabled_only() {
        let opts = vec![
            opt(OnScreenOptionKind::CleanDesktop, true),
            opt(OnScreenOptionKind::ShowKeys, false),
            opt(OnScreenOptionKind::BlurSensitiveInfo, true),
        ];
        assert_eq!(
            format_on_screen_summary(&opts),
            "Clean desktop, Blur sensitive"
        );
    }

    #[test]
    fn on_screen_summary_empty_message_when_all_off() {
        let opts = vec![
            opt(OnScreenOptionKind::CleanDesktop, false),
            opt(OnScreenOptionKind::ShowKeys, false),
        ];
        assert_eq!(format_on_screen_summary(&opts), "All overlays off");
    }

    #[test]
    fn system_audio_summary_falls_back_when_disabled() {
        let v = SystemAudioView {
            enabled: false,
            expanded: false,
            selected_count: 3,
            total_count: 7,
            icon_stack: vec![],
        };
        assert_eq!(format_system_audio_summary(&v), "Disabled");
    }

    #[test]
    fn system_audio_summary_uses_shared_count_when_enabled() {
        let v = SystemAudioView {
            enabled: true,
            expanded: false,
            selected_count: 3,
            total_count: 7,
            icon_stack: vec![],
        };
        assert_eq!(format_system_audio_summary(&v), "3 of 7 apps");
    }
}
