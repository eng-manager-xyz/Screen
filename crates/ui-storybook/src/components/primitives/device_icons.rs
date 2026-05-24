//! Lucide-style SVG icons for capture-source rows (camera, mic,
//! system audio). Paint with `currentColor` and stroke at 1.75 so
//! they read as outlined glyphs inside the small device tiles.
//! Path data copied verbatim from upstream lucide.

use leptos::prelude::*;

/// Lucide `camera` — camera capture source.
#[component]
pub fn Camera(#[prop(optional)] size: Option<u16>) -> impl IntoView {
    let size = size.unwrap_or(18);
    view! {
        <svg
            class="lucide lucide-camera"
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z" />
            <circle cx="12" cy="13" r="3" />
        </svg>
    }
}

/// Lucide `mic` — microphone capture source.
#[component]
pub fn Mic(#[prop(optional)] size: Option<u16>) -> impl IntoView {
    let size = size.unwrap_or(18);
    view! {
        <svg
            class="lucide lucide-mic"
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M12 19v3" />
            <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
            <rect width="6" height="13" x="9" y="2" rx="3" />
        </svg>
    }
}

/// Lucide `volume-2` — system-audio source.
#[component]
pub fn Volume2(#[prop(optional)] size: Option<u16>) -> impl IntoView {
    let size = size.unwrap_or(18);
    view! {
        <svg
            class="lucide lucide-volume-2"
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M11 4.702a.705.705 0 0 0-1.203-.498L6.413 7.587A1.4 1.4 0 0 1 5.416 8H3a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h2.416a1.4 1.4 0 0 1 .997.413l3.383 3.384A.705.705 0 0 0 11 19.298z" />
            <path d="M16 9a5 5 0 0 1 0 6" />
            <path d="M19.364 18.364a9 9 0 0 0 0-12.728" />
        </svg>
    }
}
