//! Lucide-style SVG icons for the navigation rail. Paint with
//! `currentColor`. Path data copied verbatim from upstream lucide.

use leptos::prelude::*;

/// Filled disc — Record.
#[component]
pub fn CircleDot(#[prop(optional)] size: Option<u16>) -> impl IntoView {
    let size = size.unwrap_or(20);
    view! {
        <svg
            class="lucide lucide-circle-dot"
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="currentColor"
            stroke="none"
            aria-hidden="true"
        >
            <circle cx="12" cy="12" r="9" />
        </svg>
    }
}

/// Lucide `folder` — Library.
#[component]
pub fn Folder(#[prop(optional)] size: Option<u16>) -> impl IntoView {
    let size = size.unwrap_or(20);
    view! {
        <svg
            class="lucide lucide-folder"
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
            <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
        </svg>
    }
}

/// Lucide `layout-panel-top` — Editor.
#[component]
pub fn LayoutPanelTop(#[prop(optional)] size: Option<u16>) -> impl IntoView {
    let size = size.unwrap_or(20);
    view! {
        <svg
            class="lucide lucide-layout-panel-top"
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
            <rect width="18" height="7" x="3" y="3" rx="1" />
            <rect width="7" height="7" x="3" y="14" rx="1" />
            <rect width="7" height="7" x="14" y="14" rx="1" />
        </svg>
    }
}

/// Lucide `mouse-pointer-2` — Cursor.
#[component]
pub fn MousePointer2(#[prop(optional)] size: Option<u16>) -> impl IntoView {
    let size = size.unwrap_or(20);
    view! {
        <svg
            class="lucide lucide-mouse-pointer-2"
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
            <path d="M4.037 4.688a.495.495 0 0 1 .651-.651l16 6.5a.5.5 0 0 1-.063.947l-6.124 1.58a2 2 0 0 0-1.438 1.435l-1.579 6.126a.5.5 0 0 1-.947.063z" />
        </svg>
    }
}

/// Lucide `settings` — Prefs.
#[component]
pub fn Settings(#[prop(optional)] size: Option<u16>) -> impl IntoView {
    let size = size.unwrap_or(20);
    view! {
        <svg
            class="lucide lucide-settings"
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
            <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
            <circle cx="12" cy="12" r="3" />
        </svg>
    }
}
