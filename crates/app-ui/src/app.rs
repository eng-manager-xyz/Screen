//! Top-level `App` component for the recorder shell.

use leptos::prelude::*;
use ui_storybook::components::{
    DropZone, DropZoneState, PlayState, PlayerControls, RecordingState, RecordingToolbar,
    StatusBar, StatusKind,
};

/// The recorder shell. Composes the toolbar, the main surface (drop-zone
/// or player view), and the status bar.
#[component]
pub fn App() -> impl IntoView {
    // Loaded-recording signal. `None` = drop-zone view; `Some(name)` =
    // player view. The Tauri shell will write to this from a file-drop
    // event in M-INT.2.
    let (loaded, set_loaded) = signal::<Option<String>>(None);

    // Demo affordance: clicking the drop-zone area in CSR mode sets a
    // synthetic loaded state so reviewers can flip between the two views
    // without the Tauri IPC wiring landing first.
    let on_demo_load = move |_| {
        set_loaded.set(Some("Recording 01.mp4".into()));
    };

    view! {
        <div class="shell">
            <RecordingToolbar
                state=RecordingState::Idle
                elapsed_seconds=0.0
                source="Built-in Display"
            />

            <main class="shell-main">
                <Show
                    when=move || loaded.get().is_some()
                    fallback=move || view! {
                        <div class="shell-drop-wrap" on:click=on_demo_load>
                            <DropZone
                                state=DropZoneState::Idle
                                hint="Click anywhere to load a demo recording"
                            />
                        </div>
                    }
                >
                    <PlayerView />
                </Show>
            </main>

            <StatusBar
                fps=60.0
                encoder="H.264 · idle"
                file_bytes=0
                kind=StatusKind::Ready
            />
        </div>
    }
}

#[component]
fn PlayerView() -> impl IntoView {
    view! {
        <div class="player-view">
            <div class="player-surface">
                <div class="player-surface-label">"Preview surface"</div>
            </div>
            <PlayerControls
                state=PlayState::Paused
                position=0.0
                duration_seconds=84.0
            />
        </div>
    }
}
