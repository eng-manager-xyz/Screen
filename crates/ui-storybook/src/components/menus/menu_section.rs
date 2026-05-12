//! `MenuSection` — optional group heading + child rows (M-UI.3 /
//! AUT-123).

use leptos::prelude::*;

#[component]
pub fn MenuSection(
    /// Optional uppercase section heading rendered above the rows. Pass
    /// `None` to skip the heading.
    #[prop(optional, into)]
    heading: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <li class="menu-section" role="presentation">
            {heading.map(|h| view! { <div class="menu-section-heading">{h}</div> })}
            <ul class="menu-section-rows" role="group">{children()}</ul>
        </li>
    }
}
