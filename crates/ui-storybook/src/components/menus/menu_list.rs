//! `MenuList` — `<ul role="menu">` wrapper used by every menu so the
//! list semantics are consistent (M-UI.3 / AUT-123).

use leptos::prelude::*;

#[component]
pub fn MenuList(
    /// Accessible label — required for screen readers.
    #[prop(into)]
    label: String,
    children: Children,
) -> impl IntoView {
    view! { <ul class="menu-list" role="menu" aria-label=label>{children()}</ul> }
}
