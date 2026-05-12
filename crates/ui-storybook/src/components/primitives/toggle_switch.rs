//! `ToggleSwitch` — on/off switch (M-UI.4 / AUT-124).
//!
//! Renders visual state from `checked`; does not flip itself. The
//! parent owns the boolean and re-renders with the new value when the
//! callback (omitted in SSR stories) fires.

use leptos::prelude::*;

#[component]
pub fn ToggleSwitch(
    /// Current on/off state.
    checked: bool,
    /// `true` to render disabled (dimmed + non-interactive).
    #[prop(optional)]
    disabled: bool,
    /// Accessible label.
    #[prop(into)]
    label: String,
) -> impl IntoView {
    let mut class = String::from("toggle");
    if checked {
        class.push_str(" toggle-on");
    }
    if disabled {
        class.push_str(" toggle-disabled");
    }
    view! {
        <button
            class=class
            role="switch"
            aria-checked=checked
            aria-label=label
            disabled=disabled
        >
            <span class="toggle-track">
                <span class="toggle-knob"></span>
            </span>
        </button>
    }
}
