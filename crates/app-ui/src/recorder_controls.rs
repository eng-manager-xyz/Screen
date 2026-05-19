//! `<RecorderControls />` — master Record button + elapsed-time
//! display + per-stream health LEDs (M-RECORD.2 of M-RECORD-EXPORT).
//!
//! Mounts at the top of the Recorder surface above the four per-
//! channel pickers. One click starts the coordinated session via
//! M-RECORD.1's `start_recording` command; one click stops it.
//!
//! ```admonish important title="Channel selection vs. device selection"
//! This component owns four checkboxes — *which channels to record*.
//! The per-channel pickers (CameraPicker / MicPicker / etc.) below
//! own *which device per channel*. Default: all four channels on;
//! the user can untick to skip a channel for this session.
//! ```
//!
//! Subscribes to the `recording-status` event for live updates of
//! elapsed time + per-stream health LEDs.

use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::recording_ipc::{
    RecordingConfigView, RecordingStatusViewIpc, SessionStreamsView, StreamHealthView,
    default_output_path, recording_status, reveal_in_file_manager, start_recording, stop_recording,
};

/// `<RecorderControls />` — master Record button + per-stream LEDs.
#[component]
#[allow(
    clippy::too_many_lines,
    reason = "Top-level Leptos component composing record button + elapsed display + channel checks + LEDs + error region; splitting helpers would lose the view! macro's single-tree expansion that keeps reactivity intact."
)]
pub fn RecorderControls() -> impl IntoView {
    // Status snapshot driven by the `recording-status` event push
    // (every 500 ms) + the synchronous mount-time fetch below.
    let status = RwSignal::new(RecordingStatusViewIpc::idle());
    // Channel selection (default all-on). User unticks to skip a
    // channel for this session.
    let cam_on = RwSignal::new(true);
    let mic_on = RwSignal::new(true);
    let screen_on = RwSignal::new(true);
    let sys_audio_on = RwSignal::new(true);
    let error_msg = RwSignal::new(Option::<String>::None);
    // M-EXPORT.4 — format dropdown selection + last-recorded path
    // for the Reveal-in-Finder button.
    let format_slug = RwSignal::new("mp4-h264".to_string());
    let last_output_path = RwSignal::new(Option::<String>::None);

    // Mount-time seed: ask the Rust side if a session is already up
    // (e.g. tray-popover → main window re-open).
    spawn_local(async move {
        let view = recording_status().await;
        status.set(view);
    });

    install_recording_status_listener(status);

    let is_recording = move || status.get().is_recording();
    let elapsed_label = move || format_elapsed(status.get().elapsed_ms);

    let on_record_click = move |_: MouseEvent| {
        if is_recording() {
            // Stop path.
            spawn_local(async move {
                match stop_recording().await {
                    Ok(summary) => {
                        tracing_log(&format!(
                            "recording stopped — session {} ran for {} ms ({} streams) → {}",
                            summary.session_id,
                            summary.elapsed_ms,
                            summary.streams.len(),
                            summary.output_path.as_deref().unwrap_or("(no file)"),
                        ));
                        error_msg.set(None);
                        // Remember the output path so the Reveal
                        // button can target it.
                        last_output_path.set(summary.output_path);
                        status.set(RecordingStatusViewIpc::idle());
                    }
                    Err(err) => {
                        error_msg.set(Some(err));
                    }
                }
            });
        } else {
            // Start path. Pull picker selections from LocalStorage,
            // resolve the default output path via the M-EXPORT.4
            // helper (uses current wall-clock time + format slug).
            let selected_format = format_slug.get();
            let format_for_path = selected_format.clone();
            spawn_local(async move {
                let path = default_output_path(Some(&format_for_path)).await;
                let resolved_path = if path.is_empty() { None } else { Some(path) };
                let config = RecordingConfigView {
                    streams: SessionStreamsView {
                        camera: cam_on.get(),
                        screen: screen_on.get(),
                        microphone: mic_on.get(),
                        system_audio: sys_audio_on.get(),
                    },
                    camera_id: read_localstorage_string("screen.camera.last_used_id"),
                    microphone_id: read_localstorage_string("screen.mic.last_used_id"),
                    screen_source_id: read_localstorage_optional_string(
                        "screen.screen_capture.last_source_id",
                    ),
                    output_path: resolved_path,
                    format: Some(selected_format),
                };
                match start_recording(config).await {
                    Ok(id) => {
                        tracing_log(&format!("recording started — session {id}"));
                        error_msg.set(None);
                        last_output_path.set(None);
                    }
                    Err(err) => {
                        error_msg.set(Some(err));
                    }
                }
            });
        }
    };

    let on_reveal_click = move |_: MouseEvent| {
        if let Some(path) = last_output_path.get() {
            spawn_local(async move {
                if let Err(err) = reveal_in_file_manager(&path).await {
                    error_msg.set(Some(err));
                }
            });
        }
    };

    let on_format_change = move |evt: leptos::ev::Event| {
        // SAFETY: change events on <select> targets always carry an
        // HtmlSelectElement.
        use wasm_bindgen::JsCast as _;
        if let Some(target) = evt
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
        {
            format_slug.set(target.value());
        }
    };

    view! {
        <div class="recorder-controls" data-recording=move || if is_recording() { "true" } else { "false" }>
            <div class="recorder-controls-row">
                <button
                    type="button"
                    class="recorder-controls-record"
                    aria-pressed=is_recording
                    on:click=on_record_click
                >
                    <span class="recorder-controls-record-icon" aria-hidden="true">
                        {move || if is_recording() { "■" } else { "●" }}
                    </span>
                    <span class="recorder-controls-record-label">
                        {move || if is_recording() { "Stop" } else { "Record" }}
                    </span>
                </button>
                <div class="recorder-controls-elapsed">
                    <span class="recorder-controls-elapsed-time">{elapsed_label}</span>
                    <span class="recorder-controls-elapsed-tag">
                        {move || status.get().state.clone()}
                    </span>
                </div>
                <ChannelChecks
                    cam=cam_on
                    mic=mic_on
                    screen=screen_on
                    sys_audio=sys_audio_on
                    disabled=Signal::derive(is_recording)
                />
                <select
                    class="recorder-controls-format"
                    prop:disabled=is_recording
                    on:change=on_format_change
                >
                    <option value="mp4-h264" selected=move || format_slug.get() == "mp4-h264">"MP4 · H.264"</option>
                    <option value="mp4-h265" selected=move || format_slug.get() == "mp4-h265">"MP4 · H.265"</option>
                    <option value="webm-vp9" selected=move || format_slug.get() == "webm-vp9">"WebM · VP9"</option>
                    <option value="webm-av1" selected=move || format_slug.get() == "webm-av1">"WebM · AV1"</option>
                </select>
            </div>
            <Show when=move || last_output_path.get().is_some() && !is_recording() fallback=|| view! { <></> }>
                <div class="recorder-controls-saved">
                    <span class="recorder-controls-saved-label">
                        {"Saved to: "}
                        <code>{move || last_output_path.get().unwrap_or_default()}</code>
                    </span>
                    <button
                        type="button"
                        class="recorder-controls-reveal"
                        on:click=on_reveal_click
                    >
                        {"Reveal in file manager"}
                    </button>
                </div>
            </Show>
            <Show when=move || !status.get().streams.is_empty() fallback=|| view! { <></> }>
                <div class="recorder-controls-leds" role="status">
                    {move || status.get().streams.into_iter().map(led_for_stream).collect_view()}
                </div>
            </Show>
            <Show when=move || error_msg.get().is_some() fallback=|| view! { <></> }>
                <div class="recorder-controls-error" role="alert">
                    {move || error_msg.get().unwrap_or_default()}
                </div>
            </Show>
        </div>
    }
}

#[component]
fn ChannelChecks(
    cam: RwSignal<bool>,
    mic: RwSignal<bool>,
    screen: RwSignal<bool>,
    sys_audio: RwSignal<bool>,
    disabled: Signal<bool>,
) -> impl IntoView {
    view! {
        <div class="recorder-controls-channels">
            <ChannelCheck label="Camera" signal=cam disabled=disabled />
            <ChannelCheck label="Microphone" signal=mic disabled=disabled />
            <ChannelCheck label="Screen" signal=screen disabled=disabled />
            <ChannelCheck label="System audio" signal=sys_audio disabled=disabled />
        </div>
    }
}

#[component]
fn ChannelCheck(
    label: &'static str,
    signal: RwSignal<bool>,
    disabled: Signal<bool>,
) -> impl IntoView {
    let on_change = move |_| {
        signal.set(!signal.get());
    };
    view! {
        <label class="recorder-controls-channel">
            <input
                type="checkbox"
                prop:checked=move || signal.get()
                prop:disabled=move || disabled.get()
                on:change=on_change
            />
            <span>{label}</span>
        </label>
    }
}

/// Render one LED dot per enabled stream. Colour ramp:
/// - green: last frame within ~1s
/// - yellow: last frame between 1s and 5s
/// - red: no recent frame / Idle / Stopping
fn led_for_stream(h: StreamHealthView) -> impl IntoView {
    let colour = match h.last_frame_ms_ago {
        Some(ms) if ms < 1_000 => "green",
        Some(ms) if ms < 5_000 => "yellow",
        _ if matches!(h.lifecycle.as_str(), "Running") => "green",
        _ if matches!(h.lifecycle.as_str(), "Starting") => "yellow",
        _ => "red",
    };
    let label = format!("{}: {} ({} frames)", h.kind, h.lifecycle, h.frame_count);
    view! {
        <span class="recorder-controls-led" data-colour=colour title=label>
            <span class="recorder-controls-led-dot" aria-hidden="true"></span>
            <span class="recorder-controls-led-kind">{h.kind.clone()}</span>
        </span>
    }
}

/// `mm:ss` formatter for the elapsed-time display.
fn format_elapsed(elapsed_ms: u64) -> String {
    let total_secs = elapsed_ms / 1_000;
    let mm = total_secs / 60;
    let ss = total_secs % 60;
    format!("{mm:02}:{ss:02}")
}

/// Subscribe to Tauri's `recording-status` event (emitted by the
/// M-RECORD.1 status emitter thread every 500 ms while a session is
/// active).
#[cfg(target_arch = "wasm32")]
fn install_recording_status_listener(status: RwSignal<RecordingStatusViewIpc>) {
    use js_sys::Reflect;
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(tauri_obj) = Reflect::get(&window, &JsValue::from_str("__TAURI__")) else {
        return;
    };
    let Ok(event_obj) = Reflect::get(&tauri_obj, &JsValue::from_str("event")) else {
        return;
    };
    let Ok(listen_fn) = Reflect::get(&event_obj, &JsValue::from_str("listen")) else {
        return;
    };
    if !listen_fn.is_function() {
        return;
    }
    let callback = Closure::wrap(Box::new(move |evt: JsValue| {
        let Ok(payload) = Reflect::get(&evt, &JsValue::from_str("payload")) else {
            return;
        };
        if let Ok(parsed) = serde_wasm_bindgen::from_value::<RecordingStatusViewIpc>(payload) {
            status.set(parsed);
        }
    }) as Box<dyn FnMut(JsValue)>);
    let listen_fn: js_sys::Function = listen_fn.unchecked_into();
    let _ = listen_fn.call2(
        &event_obj,
        &JsValue::from_str("recording-status"),
        callback.as_ref().unchecked_ref(),
    );
    callback.forget();
}

#[cfg(not(target_arch = "wasm32"))]
fn install_recording_status_listener(_status: RwSignal<RecordingStatusViewIpc>) {}

#[cfg(target_arch = "wasm32")]
fn read_localstorage_string(key: &str) -> String {
    let Some(window) = web_sys::window() else {
        return String::new();
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return String::new();
    };
    storage.get_item(key).ok().flatten().unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn read_localstorage_string(_key: &str) -> String {
    String::new()
}

#[cfg(target_arch = "wasm32")]
fn read_localstorage_optional_string(key: &str) -> Option<String> {
    let raw = read_localstorage_string(key);
    if raw.is_empty() { None } else { Some(raw) }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_localstorage_optional_string(_key: &str) -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
fn tracing_log(msg: &str) {
    web_sys::console::log_1(&JsValue::from_str(msg));
}

#[cfg(not(target_arch = "wasm32"))]
fn tracing_log(_msg: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elapsed_formatter_pads_zero() {
        assert_eq!(format_elapsed(0), "00:00");
        assert_eq!(format_elapsed(500), "00:00");
        assert_eq!(format_elapsed(1_500), "00:01");
        assert_eq!(format_elapsed(59_000), "00:59");
        assert_eq!(format_elapsed(60_000), "01:00");
        assert_eq!(format_elapsed(125_000), "02:05");
    }

    #[test]
    fn elapsed_formatter_handles_long_durations() {
        // Just over an hour — `mm` keeps counting (no hh:mm:ss
        // wraparound yet; v0 caps at the 2-digit mm display).
        assert_eq!(format_elapsed(3_600_000), "60:00");
        assert_eq!(format_elapsed(3_725_000), "62:05");
    }

    #[test]
    fn idle_status_is_not_recording() {
        let view = RecordingStatusViewIpc::idle();
        assert!(!view.is_recording());
    }

    #[test]
    fn running_status_is_recording() {
        let view = RecordingStatusViewIpc {
            state: "Running".to_string(),
            ..RecordingStatusViewIpc::idle()
        };
        assert!(view.is_recording());
    }

    #[test]
    fn starting_status_is_recording() {
        let view = RecordingStatusViewIpc {
            state: "Starting".to_string(),
            ..RecordingStatusViewIpc::idle()
        };
        assert!(view.is_recording());
    }

    #[test]
    fn stopping_status_is_recording() {
        let view = RecordingStatusViewIpc {
            state: "Stopping".to_string(),
            ..RecordingStatusViewIpc::idle()
        };
        assert!(view.is_recording());
    }
}
