//! Top-level `App` component for the recorder shell.

use leptos::prelude::*;
use ui_storybook::components::{
    DropZone, DropZoneState, PlayState, PlayerControls, RecordingState, RecordingToolbar,
    StatusBar, StatusKind,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::CustomEvent;

use crate::player_ipc::{
    self, PlayerStatus, SessionState, install_player_status_listener, screen_open, screen_pause,
    screen_play,
};

/// The recorder shell. Composes the toolbar, the main surface (drop-zone
/// or player view), and the status bar.
#[component]
pub fn App() -> impl IntoView {
    // Loaded-recording signal. `None` = drop-zone view; `Some(path)` =
    // player view. Two paths to set it:
    //   1. Tauri's `file-dropped` event → JS bridge dispatches a browser
    //      `CustomEvent("file-dropped")` → the listener below.
    //   2. The CSR demo-affordance click handler (still wired so the
    //      browser-only `trunk serve` path stays exercisable).
    let (loaded, set_loaded) = signal::<Option<String>>(None);

    // Pushed `player-status` events flow into this signal — the player
    // view re-renders whenever the Rust-side session state changes.
    let (player_status, set_player_status) = signal::<PlayerStatus>(PlayerStatus::default());

    install_file_drop_listener(set_loaded);
    install_player_status_listener(set_player_status);

    let on_demo_load = move |_| {
        set_loaded.set(Some("Recording 01.mp4 (demo)".into()));
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
                                hint="Drop an MP4 here, or click for a demo"
                            />
                        </div>
                    }
                >
                    <PlayerView loaded=loaded player_status=player_status />
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
fn PlayerView(
    loaded: ReadSignal<Option<String>>,
    player_status: ReadSignal<PlayerStatus>,
) -> impl IntoView {
    let path = move || loaded.get().unwrap_or_default();

    // The PlayerControls component is purely presentational; we wrap it
    // in a reactive view so updates to `player_status` re-render the
    // controls with the current state/position/duration.
    let controls = move || {
        let status = player_status.get();
        let play_state = match status.state {
            SessionState::Playing => PlayState::Playing,
            _ => PlayState::Paused,
        };
        let position = player_ipc::position(&status);
        let duration = player_ipc::duration_seconds(&status);
        let toggle = Callback::new(move |()| {
            // Read untracked — the callback should fire commands without
            // creating a reactive dependency on its own output.
            let cur = player_status.get_untracked();
            let _ = if cur.state == SessionState::Playing {
                screen_pause()
            } else {
                screen_play()
            };
        });
        view! {
            <PlayerControls
                state=play_state
                position=position
                duration_seconds=duration
                on_toggle=toggle
            />
        }
    };

    view! {
        <div class="player-view">
            <div class="player-surface">
                <div class="player-surface-label">
                    "Preview surface · " {path}
                </div>
            </div>
            {controls}
        </div>
    }
}

/// Install a `file-dropped` browser-event listener on the global `window`.
///
/// The Tauri shell's JS bridge in `index.html` re-emits Tauri's native
/// drag-drop event as a `CustomEvent`. We listen here and forward the
/// payload into the loaded signal *and* invoke the Rust-side
/// `player_open` command. The Tauri command will then push a
/// `player-status` event back with the freshly-decoded file's metadata.
///
/// The closure is leaked via `Closure::forget` because it has app
/// lifetime — the listener should never be removed.
fn install_file_drop_listener(set_loaded: WriteSignal<Option<String>>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        if let Ok(ce) = event.dyn_into::<CustomEvent>()
            && let Some(path) = ce.detail().as_string()
        {
            // Best-effort IPC call. Errors (e.g. running outside Tauri
            // via `trunk serve`) are intentionally ignored — the drop-
            // zone-to-player transition still works for the demo path.
            let _ = screen_open(&path);
            set_loaded.set(Some(path));
        }
    }) as Box<dyn FnMut(_)>);
    let _ =
        window.add_event_listener_with_callback("file-dropped", closure.as_ref().unchecked_ref());
    closure.forget();
}
