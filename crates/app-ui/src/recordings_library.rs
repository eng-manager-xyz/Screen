//! Recordings library — open a finished recording in the editor (ED.24).
//!
//! The trim bin's shelf: every finished recording in the output folder shows
//! up as a tile, and clicking one carries it straight to the cutting bench
//! (the editor). The backend `list_recordings` command scans the folder; the
//! click reuses the [ED.5](crate::editor_ipc::screen_open_in_editor) handoff
//! and flips the nav to the editor. A small "edited" badge marks recordings
//! that already have a saved `.screenproj` beside them (ED.23).

use leptos::prelude::*;

use ui_storybook::components::shell::AppSection;

use crate::editor_ipc::{self, RecordingEntry};

/// One library card. Extracted so the grid stays under clippy's line cap.
fn card(entry: RecordingEntry, active: RwSignal<AppSection>) -> impl IntoView {
    let path = entry.path.clone();
    let badge = entry
        .has_project
        .then(|| view! { <span class="library-card-badge">"edited"</span> });
    view! {
        <button
            class="library-card"
            title=entry.path.clone()
            on:click=move |_| {
                let _ = editor_ipc::screen_open_in_editor(&path);
                active.set(AppSection::Editor);
            }
        >
            <div class="library-card-thumb"></div>
            <span class="library-card-name">{entry.name}</span>
            {badge}
        </button>
    }
}

/// The recordings library: a grid of finished recordings; clicking one opens
/// it in the editor. Refreshes its list each time it mounts.
#[component]
pub fn RecordingsLibrary(
    /// Active nav section — flipped to `AppSection::Editor` when a recording
    /// is opened.
    active: RwSignal<AppSection>,
) -> impl IntoView {
    let entries =
        use_context::<RwSignal<Vec<RecordingEntry>>>().unwrap_or_else(|| RwSignal::new(Vec::new()));
    // Refresh on mount — navigating to Library re-creates this component.
    editor_ipc::list_recordings();
    view! {
        <div class="recordings-library">
            <h1 class="library-title">"Library"</h1>
            {move || {
                let list = entries.get();
                if list.is_empty() {
                    view! {
                        <p class="library-empty">
                            "No recordings yet — finish a recording to see it here."
                        </p>
                    }
                    .into_any()
                } else {
                    view! {
                        <div class="library-grid">
                            {list.into_iter().map(|e| card(e, active)).collect_view()}
                        </div>
                    }
                    .into_any()
                }
            }}
        </div>
    }
}
