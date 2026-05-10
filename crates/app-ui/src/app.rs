//! Top-level `App` component for the recorder shell.

use leptos::html;
use leptos::prelude::*;
use ui_storybook::components::{
    DropZone, DropZoneState, PlayState, PlayerControls, RecordingState, RecordingToolbar,
    StatusBar, StatusKind,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{CustomEvent, HtmlVideoElement};

use crate::player_ipc::{
    self, PlayerStatus, SessionState, convert_file_src, install_player_status_listener,
    screen_open, screen_pause, screen_play,
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

    // Drag-over visual feedback (M-POLISH.1). Flips on `file-drag-enter`,
    // resets on `file-drag-leave` (which Tauri fires after a successful
    // drop too — see main.rs's `on_window_event`).
    let (is_dragging, set_dragging) = signal::<bool>(false);

    install_file_drop_listener(set_loaded);
    install_player_status_listener(set_player_status);
    install_drag_state_listeners(set_dragging);

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
                    fallback=move || {
                        let drop_state = move || if is_dragging.get() {
                            DropZoneState::Active
                        } else {
                            DropZoneState::Idle
                        };
                        view! {
                            <div class="shell-drop-wrap" on:click=on_demo_load>
                                {move || view! {
                                    <DropZone
                                        state=drop_state()
                                        hint="Drop an MP4 here, or click for a demo"
                                    />
                                }}
                            </div>
                        }
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
    let video_ref: NodeRef<html::Video> = NodeRef::new();

    // Resolve the dropped file path → asset:// URL for the <video> tag.
    // `convert_file_src` returns `None` outside Tauri (browser-only
    // `trunk serve` path) — render a plain message in that case.
    let video_src = move || -> Option<String> {
        let path = loaded.get()?;
        convert_file_src(&path)
    };
    let path_label = move || loaded.get().unwrap_or_default();

    // Catch-up effect: when Tauri's state changes for non-click reasons
    // (Ended on EOF, future seek, ...), keep the <video> element in
    // sync. Idempotent — won't fight the click-handler driven path.
    Effect::new(move |_| {
        let status = player_status.get();
        let Some(video) = video_ref.get() else {
            return;
        };
        let video: HtmlVideoElement = video;
        let should_play = status.state == SessionState::Playing;
        let is_paused = video.paused();
        if should_play && is_paused {
            let _ = video.play();
        } else if !should_play && !is_paused {
            let _ = video.pause();
        }
    });

    let controls = move || {
        let status = player_status.get();
        let play_state = match status.state {
            SessionState::Playing => PlayState::Playing,
            _ => PlayState::Paused,
        };
        let position = player_ipc::position(&status);
        let duration = player_ipc::duration_seconds(&status);
        let toggle = Callback::new(move |()| {
            let cur = player_status.get_untracked();
            let want_playing = cur.state != SessionState::Playing;

            // Drive the <video> element synchronously inside the click
            // handler — WebKit blocks programmatic .play() outside a
            // user gesture, so the catch-up Effect alone isn't enough.
            if let Some(video) = video_ref.get_untracked() {
                let video: HtmlVideoElement = video;
                let _ = if want_playing {
                    let _ = video.play();
                    Ok::<(), JsValue>(())
                } else {
                    video.pause()
                };
            }

            // Drive Tauri Player state in parallel — source of truth
            // for elapsed/duration/EOF transitions.
            let _ = if want_playing {
                screen_play()
            } else {
                screen_pause()
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
                {move || video_src().map(|src| view! {
                    <video
                        node_ref=video_ref
                        class="player-video"
                        src=src
                        preload="auto"
                    />
                })}
                <div class="player-surface-label">
                    "Preview surface · " {path_label}
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

/// Wire the `file-drag-enter` / `file-drag-leave` browser-event
/// listeners that flip the `is_dragging` signal driving the
/// `<DropZone>`'s active visual state. Both closures are leaked
/// (app-lifetime listeners), matching the file-drop-listener pattern.
fn install_drag_state_listeners(set_dragging: WriteSignal<bool>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let enter = Closure::wrap(Box::new(move |_: web_sys::Event| {
        set_dragging.set(true);
    }) as Box<dyn FnMut(_)>);
    let _ =
        window.add_event_listener_with_callback("file-drag-enter", enter.as_ref().unchecked_ref());
    enter.forget();

    let leave = Closure::wrap(Box::new(move |_: web_sys::Event| {
        set_dragging.set(false);
    }) as Box<dyn FnMut(_)>);
    let _ =
        window.add_event_listener_with_callback("file-drag-leave", leave.as_ref().unchecked_ref());
    leave.forget();
}
