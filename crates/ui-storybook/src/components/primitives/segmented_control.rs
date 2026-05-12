//! `SegmentedControl` — pill-style radio-tab control (M-UI.4 / AUT-124).
//!
//! The active segment is a string id passed via `active`. The component
//! never owns that id; callers re-render with the new id when a segment
//! is selected.

use leptos::prelude::*;

/// One segment in a `SegmentedControl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// Stable id matched against `active`. Use a kebab-case string.
    pub id: &'static str,
    /// Visible label.
    pub label: &'static str,
    /// Optional leading glyph.
    pub icon: Option<&'static str>,
    /// `true` to render disabled.
    pub disabled: bool,
}

#[component]
pub fn SegmentedControl(
    /// Segments in display order.
    segments: Vec<Segment>,
    /// Id of the currently-active segment.
    #[prop(into)]
    active: String,
    /// Accessible label for the whole control.
    #[prop(into)]
    label: String,
) -> impl IntoView {
    let active = active.clone();
    view! {
        <div class="segmented" role="radiogroup" aria-label=label>
            {segments.into_iter().map(|seg| {
                let is_active = seg.id == active;
                let mut class = String::from("segment");
                if is_active { class.push_str(" segment-active"); }
                if seg.disabled { class.push_str(" segment-disabled"); }
                view! {
                    <button
                        class=class
                        role="radio"
                        aria-checked=is_active
                        aria-disabled=seg.disabled
                        disabled=seg.disabled
                        data-id=seg.id
                    >
                        {seg.icon.map(|i| view! { <span class="segment-icon" aria-hidden="true">{i}</span> })}
                        <span class="segment-label">{seg.label}</span>
                    </button>
                }
            }).collect_view()}
        </div>
    }
}
