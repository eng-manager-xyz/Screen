//! `SelectPill` — compact pill button that opens a popover (M-UI.4 /
//! AUT-124).
//!
//! Visual only. Clicking opens the popover; the parent owns the open
//! state and the popover content.

use leptos::prelude::*;

use crate::components::primitives::ChevronDown;

#[component]
pub fn SelectPill(
    /// Optional leading glyph.
    #[prop(optional, into)]
    icon: Option<String>,
    /// Selected label shown inside the pill.
    #[prop(into)]
    label: String,
    /// `true` when the parent's popover is open. Drives the rotation of
    /// the trailing chevron.
    #[prop(optional)]
    open: bool,
    /// `true` to render disabled.
    #[prop(optional)]
    disabled: bool,
) -> impl IntoView {
    let mut class = String::from("select-pill");
    if open {
        class.push_str(" select-pill-open");
    }
    view! {
        <button
            class=class
            aria-haspopup="listbox"
            aria-expanded=open
            disabled=disabled
        >
            {icon.map(|i| view! { <span class="select-pill-icon" aria-hidden="true">{i}</span> })}
            <span class="select-pill-label">{label}</span>
            <span class="select-pill-chevron" aria-hidden="true">
                <ChevronDown />
            </span>
        </button>
    }
}
