//! `DevicePickerMenu` + `DevicePickerRow` — expanded camera /
//! microphone picker (M-UI.8 / AUT-128). Composes UI-03 menu
//! primitives.

use leptos::prelude::*;

use crate::components::menus::{
    MenuFooter, MenuList, MenuSection, PopoverPlacement, PopoverSurface,
};
use crate::components::primitives::{Badge, BadgeKind, Meter};

use super::capture_source_row::CaptureSourceKind;

/// Mock thumbnail content for a camera row — a CSS-positioned chip
/// matching the device-card preview pattern from UI-07.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceThumb {
    /// Background CSS color.
    pub background: String,
    /// Overlay glyph (1-2 chars).
    pub glyph: String,
}

/// View-model for one option inside the picker.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceOptionView {
    /// Stable id.
    pub id: String,
    /// Primary text.
    pub name: String,
    /// Secondary text (resolution, connection hint).
    pub detail: String,
    /// Optional small badge ("Wireless", "New").
    pub badge: Option<String>,
    /// `true` when this option is currently selected.
    pub selected: bool,
    /// Optional live audio level — microphone rows only.
    pub level: Option<f32>,
    /// Optional camera thumbnail.
    pub thumbnail: Option<DeviceThumb>,
}

/// State of the picker beyond a normal device list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevicePickerState {
    /// One or more devices available.
    Populated,
    /// No devices detected.
    Empty,
    /// Permission has not been granted by the user yet.
    PermissionNeeded,
}

#[component]
pub fn DevicePickerMenu(
    /// Whether this picker is for the camera or microphone slot.
    kind: CaptureSourceKind,
    /// Device list — typically `fixtures::devices::sample_camera_options()`.
    /// Ignored when `state != Populated`.
    devices: Vec<DeviceOptionView>,
    /// Empty / permission-needed override.
    #[prop(optional, default = DevicePickerState::Populated)]
    state: DevicePickerState,
) -> impl IntoView {
    let title = match kind {
        CaptureSourceKind::Camera => "Cameras",
        CaptureSourceKind::Microphone => "Microphones",
    };
    let footer_action = match kind {
        CaptureSourceKind::Camera => "Connect new camera",
        CaptureSourceKind::Microphone => "Pair Bluetooth microphone",
    };
    let rows: Vec<_> = if matches!(state, DevicePickerState::Populated) {
        devices.into_iter().map(render_row).collect()
    } else {
        Vec::new()
    };
    let body = match state {
        DevicePickerState::Populated => view! {
            <MenuList label=title.to_string()>
                <MenuSection heading="Available".to_string()>{rows}</MenuSection>
            </MenuList>
        }
        .into_any(),
        DevicePickerState::Empty => view! {
            <div class="device-picker-state">
                <div class="device-picker-state-glyph" aria-hidden="true">"⚠"</div>
                <div class="device-picker-state-title">"No devices detected"</div>
                <div class="device-picker-state-subtitle">
                    "Plug in or pair a device, then re-open this menu."
                </div>
            </div>
        }
        .into_any(),
        DevicePickerState::PermissionNeeded => view! {
            <div class="device-picker-state">
                <div class="device-picker-state-glyph device-picker-state-glyph-warn" aria-hidden="true">"⚠"</div>
                <div class="device-picker-state-title">
                    {match kind {
                        CaptureSourceKind::Camera => "Camera access required",
                        CaptureSourceKind::Microphone => "Microphone access required",
                    }}
                </div>
                <div class="device-picker-state-subtitle">
                    "Grant access in System Settings → Privacy & Security, then re-open this menu."
                </div>
            </div>
        }
        .into_any(),
    };
    view! {
        <PopoverSurface
            placement=PopoverPlacement::BottomLeft
            width_px=320_u16
            title=title.to_string()
            footer=ToChildren::to_children(move || view! {
                <MenuFooter>
                    <span style="font-size:11px;color:var(--text-tertiary)">"Devices update live."</span>
                    <button class="btn btn-default btn-sm">{footer_action}</button>
                </MenuFooter>
            })
        >
            {body}
        </PopoverSurface>
    }
}

fn render_row(opt: DeviceOptionView) -> impl IntoView {
    let mut row_class = String::from("device-picker-row");
    if opt.selected {
        row_class.push_str(" device-picker-row-selected");
    }
    let badge = opt
        .badge
        .map(|b| view! { <Badge kind=BadgeKind::Plan>{b}</Badge> });
    let level_view = opt.level.map(|l| view! { <Meter level=l bar_count=10 /> });
    let thumb_view = opt.thumbnail.as_ref().map(|t| {
        let style = format!("background:{}", t.background);
        let glyph = t.glyph.clone();
        view! {
            <span class="device-picker-thumb" style=style aria-hidden="true">
                <span class="device-picker-thumb-glyph">{glyph}</span>
            </span>
        }
    });
    view! {
        <li class="device-picker-row-item" role="none">
            <button class=row_class role="menuitem" aria-pressed=opt.selected>
                {thumb_view}
                <span class="device-picker-row-text">
                    <span class="device-picker-row-name">{opt.name}</span>
                    <span class="device-picker-row-detail">{opt.detail}</span>
                </span>
                {badge}
                {level_view.map(|l| view! { <span class="device-picker-row-meter">{l}</span> })}
                {opt.selected.then(|| view! {
                    <span class="device-picker-row-check" aria-hidden="true">"✓"</span>
                })}
            </button>
        </li>
    }
}
