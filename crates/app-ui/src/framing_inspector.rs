//! Framing inspector — output aspect ratio + crop (ED.15 / M-EDIT).
//!
//! Project-level "look" controls in the inspector: the **aspect ratio**
//! (the hard matte that decides the export canvas — 16:9, 9:16, 1:1, 4:3)
//! and the **crop** (a normalized sub-rect of the source). Both are stored
//! on the [`EditProject`] and re-derived at render time; the visible
//! reframe of the preview + the export `videocrop` ride with the
//! render-integration / export pass (ED.20 / ED.21). This chunk is the
//! authoring side: presets + numeric crop entry, undoable through the
//! shared [`edit::History`].

use edit::EditProject;
use edit::style::{AspectRatio, CropRect};
use leptos::prelude::*;

type ProjectSignal = Option<RwSignal<Option<EditProject>>>;
type HistoryStore = Option<StoredValue<Option<edit::History>>>;

/// Aspect-ratio presets, in display order.
pub const ASPECTS: [(AspectRatio, &str); 4] = [
    (AspectRatio::Wide, "16:9"),
    (AspectRatio::Vertical, "9:16"),
    (AspectRatio::Square, "1:1"),
    (AspectRatio::Classic, "4:3"),
];

/// Crop field labels, in `[x, y, w, h]` order.
pub const CROP_FIELDS: [&str; 4] = ["X", "Y", "W", "H"];

/// A crop rect as a `[x, y, width, height]` array.
#[must_use]
pub fn crop_to_array(c: CropRect) -> [f32; 4] {
    [c.x, c.y, c.width, c.height]
}

/// Rebuild a [`CropRect`] from a `[x, y, width, height]` array.
#[must_use]
pub fn crop_from_array(a: [f32; 4]) -> CropRect {
    CropRect {
        x: a[0],
        y: a[1],
        width: a[2],
        height: a[3],
    }
}

/// Parse a percent string (`"0".."100"`) to a unit fraction, clamped to
/// `[0, 1]`. A non-numeric entry reads as `0`.
#[must_use]
pub fn parse_crop_pct(s: &str) -> f32 {
    s.trim().parse::<f32>().unwrap_or(0.0).clamp(0.0, 100.0) / 100.0
}

/// Display a unit fraction as a whole-percent string (`0.8 → "80"`).
#[must_use]
pub fn crop_pct_label(v: f32) -> String {
    format!("{}", (v * 100.0).round())
}

fn aspect_buttons(project: ProjectSignal, history: HistoryStore) -> AnyView {
    let current = project
        .and_then(|s| s.get().map(|p| p.aspect))
        .unwrap_or_default();
    ASPECTS
        .into_iter()
        .map(|(ratio, label)| {
            let mut class = String::from("aspect-preset");
            if ratio == current {
                class.push_str(" aspect-preset--active");
            }
            view! {
                <button
                    class=class
                    on:click=move |_| {
                        if let (Some(p), Some(h)) = (project, history) {
                            crate::editor_edits::set_aspect(p, h, ratio);
                        }
                    }
                >
                    {label}
                </button>
            }
        })
        .collect_view()
        .into_any()
}

fn crop_inputs(project: ProjectSignal, history: HistoryStore) -> AnyView {
    let cur = project
        .and_then(|s| s.get().and_then(|p| p.crop))
        .unwrap_or_else(CropRect::full);
    let arr = crop_to_array(cur);
    CROP_FIELDS
        .into_iter()
        .enumerate()
        .map(|(i, label)| {
            let value = crop_pct_label(arr[i]);
            view! {
                <label class="crop-field">
                    <span class="crop-field-label">{label}</span>
                    <input
                        class="crop-field-input"
                        type="number"
                        min="0"
                        max="100"
                        prop:value=value
                        on:change=move |ev| {
                            if let (Some(p), Some(h)) = (project, history) {
                                let mut a = p
                                    .get_untracked()
                                    .and_then(|pr| pr.crop)
                                    .map_or_else(|| crop_to_array(CropRect::full()), crop_to_array);
                                a[i] = parse_crop_pct(&event_target_value(&ev));
                                crate::editor_edits::set_crop(p, h, crop_from_array(a));
                            }
                        }
                    />
                </label>
            }
        })
        .collect_view()
        .into_any()
}

/// The framing inspector: aspect-ratio presets + numeric crop entry.
#[component]
pub fn FramingInspector() -> impl IntoView {
    let project = use_context::<RwSignal<Option<EditProject>>>();
    let history = use_context::<StoredValue<Option<edit::History>>>();
    view! {
        <div class="framing-inspector">
            <div class="clip-inspector-section">
                <h3 class="clip-inspector-title">"Aspect ratio"</h3>
                <div class="aspect-presets">{move || aspect_buttons(project, history)}</div>
            </div>
            <div class="clip-inspector-section">
                <h3 class="clip-inspector-title">"Crop (%)"</h3>
                <div class="crop-fields">{move || crop_inputs(project, history)}</div>
                <button
                    class="crop-reset"
                    on:click=move |_| {
                        if let (Some(p), Some(h)) = (project, history) {
                            crate::editor_edits::set_crop(p, h, CropRect::full());
                        }
                    }
                >
                    "Reset crop"
                </button>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_round_trips_within_tolerance() {
        assert!((parse_crop_pct("80") - 0.8).abs() < 1e-6);
        assert_eq!(crop_pct_label(0.8), "80");
        assert_eq!(crop_pct_label(0.0), "0");
        assert_eq!(crop_pct_label(1.0), "100");
    }

    #[test]
    fn percent_parse_clamps_and_defaults() {
        assert!((parse_crop_pct("150") - 1.0).abs() < 1e-6); // clamp high
        assert!((parse_crop_pct("-5")).abs() < 1e-6); // clamp low
        assert!((parse_crop_pct("abc")).abs() < 1e-6); // non-numeric → 0
    }

    #[test]
    fn crop_array_round_trips() {
        let c = CropRect {
            x: 0.1,
            y: 0.2,
            width: 0.7,
            height: 0.6,
        };
        assert_eq!(crop_from_array(crop_to_array(c)), c);
    }

    #[test]
    fn aspect_presets_cover_all_ratios() {
        assert_eq!(ASPECTS.len(), 4);
        assert!(ASPECTS.iter().any(|(r, _)| *r == AspectRatio::Wide));
        assert!(ASPECTS.iter().any(|(r, _)| *r == AspectRatio::Vertical));
    }
}
