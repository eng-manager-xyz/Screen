//! `MenuFooter` — bottom slot inside `PopoverSurface` for primary
//! action buttons or quiet hint text (M-UI.3 / AUT-123).

use leptos::prelude::*;

#[component]
pub fn MenuFooter(children: Children) -> impl IntoView {
    view! { <div class="menu-footer">{children()}</div> }
}
