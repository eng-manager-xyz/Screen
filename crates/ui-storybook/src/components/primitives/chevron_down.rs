//! Lucide-style chevron-down SVG. Paints with `currentColor`.

use leptos::prelude::*;

#[component]
pub fn ChevronDown(
    /// Pixel size — sets both `width` and `height`. Defaults to 18.
    #[prop(optional)]
    size: Option<u16>,
) -> impl IntoView {
    let size = size.unwrap_or(18);
    view! {
        <svg
            class="chevron-down"
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="m6 9 6 6 6-6" />
        </svg>
    }
}
