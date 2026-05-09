//! `DropZone` — the recorder's MP4 import surface.
//!
//! Two visual states: `idle` (subtle dashed outline, neutral copy) and
//! `active` (solid accent border, bigger glyph, "release to drop" copy).
//! The Tauri shell drives the active state from the OS-level drag events
//! (`tauri::Window::on_window_event`), passing it in as a prop.

use leptos::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DropZoneState {
    #[default]
    Idle,
    Active,
}

impl DropZoneState {
    fn css(self) -> &'static str {
        match self {
            DropZoneState::Idle => "drop-zone-idle",
            DropZoneState::Active => "drop-zone-active",
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            DropZoneState::Idle => "⤓",
            DropZoneState::Active => "⇣",
        }
    }

    fn headline(self) -> &'static str {
        match self {
            DropZoneState::Idle => "Drop a recording here",
            DropZoneState::Active => "Release to import",
        }
    }

    fn subtext(self) -> &'static str {
        match self {
            DropZoneState::Idle => "MP4, MOV, or any QuickTime-compatible video",
            DropZoneState::Active => "Will open in the editor",
        }
    }
}

#[component]
pub fn DropZone(
    #[prop(optional)] state: DropZoneState,
    #[prop(optional, into)] hint: String,
) -> impl IntoView {
    let class = format!("drop-zone {}", state.css());
    let glyph = state.glyph();
    let headline = state.headline();
    let subtext = state.subtext();
    let has_hint = !hint.is_empty();
    view! {
        <div class=class role="region" aria-label="Recording drop zone">
            <div class="drop-zone-glyph">{glyph}</div>
            <div class="drop-zone-headline">{headline}</div>
            <div class="drop-zone-subtext">{subtext}</div>
            <Show when=move || has_hint>
                <div class="drop-zone-hint">{hint.clone()}</div>
            </Show>
        </div>
    }
}
