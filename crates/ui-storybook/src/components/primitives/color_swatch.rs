//! `ColorSwatch` — circular color tile for Cursor Studio (M-UI.4 /
//! AUT-124).

use leptos::prelude::*;

#[component]
pub fn ColorSwatch(
    /// CSS color string (`#hex` or named).
    #[prop(into)]
    color: String,
    /// `true` to render the selected outline.
    #[prop(optional)]
    selected: bool,
    /// Accessible label (color name).
    #[prop(into)]
    label: String,
) -> impl IntoView {
    let mut class = String::from("color-swatch");
    if selected {
        class.push_str(" color-swatch-selected");
    }
    let style = format!("background:{color}");
    let label_title = label.clone();
    view! {
        <button
            class=class
            style=style
            aria-label=label
            aria-pressed=selected
            title=label_title
        >
            <span class="color-swatch-ring" aria-hidden="true"></span>
        </button>
    }
}
