//! `CaptureModeTabs` — Screen / Window / Area tabs at the top of the
//! tray record popover (M-UI.6 / AUT-126). Wraps `SegmentedControl`.

use leptos::prelude::*;

use crate::components::primitives::{Segment, SegmentedControl};
use crate::fixtures::recorder::CaptureMode;

impl CaptureMode {
    /// Stable slug used both for the underlying `SegmentedControl`
    /// segment id and as a kebab-case data attribute.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            CaptureMode::Screen => "screen",
            CaptureMode::Window => "window",
            CaptureMode::Area => "area",
        }
    }
}

#[component]
pub fn CaptureModeTabs(
    /// Currently-selected capture mode. Passed in from above; the
    /// component never owns this state.
    selected: CaptureMode,
    /// Modes to render disabled (e.g. `Area` while permissions are
    /// pending).
    #[prop(optional)]
    disabled_modes: Vec<CaptureMode>,
) -> impl IntoView {
    let segments = vec![
        Segment {
            id: CaptureMode::Screen.slug(),
            label: "Screen",
            icon: Some("▢"),
            disabled: disabled_modes.contains(&CaptureMode::Screen),
        },
        Segment {
            id: CaptureMode::Window.slug(),
            label: "Window",
            icon: Some("◫"),
            disabled: disabled_modes.contains(&CaptureMode::Window),
        },
        Segment {
            id: CaptureMode::Area.slug(),
            label: "Area",
            icon: Some("⊞"),
            disabled: disabled_modes.contains(&CaptureMode::Area),
        },
    ];
    let active = selected.slug();
    view! {
        <SegmentedControl
            segments=segments
            active=active.to_string()
            label="Capture mode".to_string()
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_mode_has_unique_slug() {
        let slugs = [
            CaptureMode::Screen.slug(),
            CaptureMode::Window.slug(),
            CaptureMode::Area.slug(),
        ];
        let mut sorted = slugs.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), slugs.len());
    }

    #[test]
    fn slugs_are_lowercase_ascii() {
        for s in [CaptureMode::Screen, CaptureMode::Window, CaptureMode::Area] {
            assert!(s.slug().chars().all(|c| c.is_ascii_lowercase()));
        }
    }
}
