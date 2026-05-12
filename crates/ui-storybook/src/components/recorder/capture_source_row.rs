//! `CaptureSourceRow` — collapsed camera / microphone row in the tray
//! record popover (M-UI.8 / AUT-128).
//!
//! Composes UI-01 `IconTile`, UI-04 `ToggleSwitch`, and UI-04 `Meter`.
//! Stateless: `enabled` / `expanded` / `level` are props; the parent
//! re-renders with new values when callbacks fire.

use leptos::prelude::*;

use crate::components::primitives::{IconTile, IconTileKind, Meter, ToggleSwitch};

/// Kind of capture source — drives the leading icon glyph + the
/// `aria-label` text on the toggle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureSourceKind {
    /// Camera device.
    Camera,
    /// Microphone device.
    Microphone,
}

impl CaptureSourceKind {
    /// Single-glyph icon used in the leading tile.
    #[must_use]
    pub fn glyph(self) -> &'static str {
        match self {
            CaptureSourceKind::Camera => "📷",
            CaptureSourceKind::Microphone => "🎙",
        }
    }

    /// Accessible label noun.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            CaptureSourceKind::Camera => "camera",
            CaptureSourceKind::Microphone => "microphone",
        }
    }
}

/// View-model for one capture-source row.
#[derive(Debug, Clone, PartialEq)]
pub struct CaptureSourceView {
    /// Stable id matching the underlying device.
    pub id: &'static str,
    /// Camera vs microphone.
    pub kind: CaptureSourceKind,
    /// Primary title (`"FaceTime HD Camera"`).
    pub title: &'static str,
    /// Secondary line (`"MacBook · 1080p"`, `"Bluetooth · 86%"`).
    pub subtitle: &'static str,
    /// `true` when the capture source is on.
    pub enabled: bool,
    /// `true` when the picker popover is open for this row.
    pub expanded: bool,
    /// `true` to render the favourited star glyph.
    pub favorite: bool,
    /// Optional normalized audio level `[0, 1]`. Renders a `Meter`
    /// when `Some` and `kind == Microphone`. Cameras ignore this.
    pub level: Option<f32>,
}

#[component]
pub fn CaptureSourceRow(view: CaptureSourceView) -> impl IntoView {
    let mut class = String::from("capture-source-row");
    if view.expanded {
        class.push_str(" capture-source-row-expanded");
    }
    if !view.enabled {
        class.push_str(" capture-source-row-off");
    }
    let chevron_class = if view.expanded {
        "capture-source-chevron capture-source-chevron-open"
    } else {
        "capture-source-chevron"
    };
    let kind_glyph = view.kind.glyph();
    let kind_label = view.kind.label();
    let toggle_label = format!("Enable {kind_label}");
    let meter_view = if view.kind == CaptureSourceKind::Microphone {
        view.level
            .map(|level| view! { <Meter level=level bar_count=10 /> })
    } else {
        None
    };
    view! {
        <div class=class data-kind=match view.kind {
            CaptureSourceKind::Camera => "camera",
            CaptureSourceKind::Microphone => "microphone",
        }>
            <span class="capture-source-leading">
                <IconTile kind=IconTileKind::Device>{kind_glyph}</IconTile>
            </span>
            <span class="capture-source-text">
                <span class="capture-source-title">
                    {view.title}
                    {view.favorite.then(|| view! {
                        <span class="capture-source-star" aria-label="Favourite" title="Favourite">"★"</span>
                    })}
                </span>
                <span class="capture-source-subtitle">{view.subtitle}</span>
            </span>
            {meter_view.map(|m| view! { <span class="capture-source-meter">{m}</span> })}
            <span class="capture-source-toggle">
                <ToggleSwitch checked=view.enabled label=toggle_label />
            </span>
            <button class=chevron_class aria-label="Expand device picker" aria-expanded=view.expanded>
                "▾"
            </button>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_has_unique_glyph_and_label() {
        assert_ne!(
            CaptureSourceKind::Camera.glyph(),
            CaptureSourceKind::Microphone.glyph(),
        );
        assert_ne!(
            CaptureSourceKind::Camera.label(),
            CaptureSourceKind::Microphone.label(),
        );
    }

    #[test]
    fn labels_are_lowercase_singular() {
        for k in [CaptureSourceKind::Camera, CaptureSourceKind::Microphone] {
            let l = k.label();
            assert_eq!(l.to_lowercase(), l);
            // We use the noun in "Enable {noun}" — must not already
            // start uppercase or include "the".
            assert!(!l.starts_with("the "));
        }
    }
}
