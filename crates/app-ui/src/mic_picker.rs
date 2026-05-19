//! `<MicPicker />` — live-data dropdown selector for microphones in
//! the Recorder surface (M-MIC.2 / AUT-279).
//!
//! Mirror of [`crate::camera_picker`] for the audio path. Combines
//! the M-MIC.1 IPC helpers with the storybook `CaptureSourceRow` +
//! `DevicePickerMenu` shapes (M-UI.8 / AUT-128) to provide:
//!
//! - On mount: invoke `microphone_permission_status` + `list_microphones`
//!   to seed the picker.
//! - On open: re-fetch so newly-plugged USB mics + paired Bluetooth
//!   headsets appear.
//! - On select: invoke `start_mic_capture(mic_id)` + persist the
//!   selection to `LocalStorage` so re-opens land on the same mic.
//! - Permission-denied state uses the same three-state contract as
//!   the camera picker (`Granted` / `NotDetermined` / `Denied`).
//! - Auto-start on first mount is **opt-in via a `MicLifecycle::Idle
//!   → Starting`** guard. Unlike the camera path (which auto-starts
//!   the preview to make the canvas alive), the mic worker only
//!   starts on explicit user click — recording from the mic without
//!   the user asking would be surprising even for a default mic.
//!
//! ```admonish note title="Per-device gst selection deferred"
//! M-MIC.1's worker uses `autoaudiosrc`, which always opens the OS
//! default mic regardless of the `mic_id` we send. The picker UX
//! still works — `start_mic_capture(mic_id)` is the IPC contract —
//! but the OS-default-only behaviour means clicking a non-default
//! mic doesn't yet switch the active input. Per-mic wiring
//! (`osxaudiosrc device-uid=…` etc.) is a follow-up that's a
//! drop-in extension to the worker's pipeline args; no UI changes
//! needed.
//! ```

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::camera_ipc::CameraPermission;
use crate::mic_ipc::{self, MicrophoneView};

/// `LocalStorage` key for the "last used" mic id. Cfg-gated to wasm32
/// to match the `read_last_used` / `write_last_used` helpers — the
/// native target's unit tests don't have a `LocalStorage` impl.
#[cfg(target_arch = "wasm32")]
const LAST_USED_KEY: &str = "screen.mic.last_used_id";

/// `<MicPicker />` — live mic picker rendered alongside the camera
/// picker in the Recorder surface.
#[component]
pub fn MicPicker() -> impl IntoView {
    let mics = RwSignal::new(Vec::<MicrophoneView>::new());
    let selected_id = RwSignal::new(Option::<String>::None);
    let permission = RwSignal::new(CameraPermission::Granted);
    let open = RwSignal::new(false);
    // M-AUDIO.METER / AUT-287 — live RMS level from the worker.
    let level = RwSignal::new(0.0_f32);
    // M-RECORD.3 — lock the trigger while a coordinated session is
    // active.
    let recording_lock = RwSignal::new(false);
    crate::recording_ipc::install_recording_lock_listener(recording_lock);

    // Subscribe once at component mount. The Tauri event listener
    // leaks the closure intentionally (see subscribe_mic_level docs);
    // the worker stops emitting when the user clicks stop_mic_capture.
    mic_ipc::subscribe_mic_level(move |l| level.set(l));

    // Initial mount: probe permission, enumerate, restore last-used
    // selection. Note: unlike the camera picker we do NOT auto-start
    // the worker — mic capture is opt-in (see module docs).
    refresh_microphones(mics, selected_id, permission);

    let on_select = move |id: String| {
        write_last_used(&id);
        let id_for_invoke = id.clone();
        spawn_local(async move {
            mic_ipc::start_mic_capture(id_for_invoke).await;
        });
        selected_id.set(Some(id));
        open.set(false);
    };

    view! {
        <div class="mic-picker">
            <button
                class="mic-picker-trigger"
                aria-haspopup="listbox"
                aria-expanded=move || open.get()
                prop:disabled=move || recording_lock.get()
                title=move || if recording_lock.get() { "Recording in progress — stop the recording to change microphones" } else { "" }
                on:click=move |_| {
                    if recording_lock.get() { return; }
                    let will_open = !open.get_untracked();
                    open.set(will_open);
                    if will_open {
                        refresh_microphones(mics, selected_id, permission);
                    }
                }
            >
                <span class="mic-picker-icon" aria-hidden="true">"🎙"</span>
                <span class="mic-picker-label">
                    {move || selected_label(&mics.get(), selected_id.get().as_ref())}
                </span>
                <span class="mic-picker-chevron" aria-hidden="true">"▾"</span>
            </button>
            <div class="audio-meter" aria-label="Microphone input level">
                <div
                    class="audio-meter-bar"
                    style:width=move || format!("{:.1}%", (level.get() * 100.0).clamp(0.0, 100.0))
                ></div>
            </div>
            <Show when=move || open.get() fallback=|| view! { <></> }>
                <div class="mic-picker-menu" role="listbox">
                    <MicPickerBody
                        mics=mics
                        selected_id=selected_id
                        permission=permission
                        on_select=Callback::new(on_select)
                    />
                </div>
            </Show>
        </div>
    }
}

#[component]
fn MicPickerBody(
    mics: RwSignal<Vec<MicrophoneView>>,
    selected_id: RwSignal<Option<String>>,
    permission: RwSignal<CameraPermission>,
    on_select: Callback<String>,
) -> impl IntoView {
    move || {
        match (permission.get(), mics.get()) {
        (CameraPermission::Denied | CameraPermission::NotDetermined, _) => view! {
            <div class="mic-picker-state mic-picker-state--permission">
                <p>{"Microphone access required."}</p>
                <p class="mic-picker-state-help">
                    {"Grant access in System Settings → Privacy & Security, then re-open this picker."}
                </p>
                <button
                    type="button"
                    class="mic-picker-state-button"
                    on:click=move |_| {
                        spawn_local(async move {
                            mic_ipc::open_settings_microphone().await;
                        });
                    }
                >
                    {"Open System Settings → Microphone"}
                </button>
            </div>
        }
        .into_any(),
        (CameraPermission::Granted, list) if list.is_empty() => view! {
            <div class="mic-picker-state mic-picker-state--empty">
                <p>{"No microphones detected."}</p>
                <p class="mic-picker-state-help">
                    {"Plug in a USB mic or pair a Bluetooth headset, then re-open this picker."}
                </p>
            </div>
        }
        .into_any(),
        (CameraPermission::Granted, list) => view! {
            <ul class="mic-picker-list" role="none">
                {list
                    .into_iter()
                    .map(|mic| render_row(mic, selected_id.get(), on_select))
                    .collect_view()}
            </ul>
        }
        .into_any(),
    }
    }
}

fn refresh_microphones(
    mics: RwSignal<Vec<MicrophoneView>>,
    selected_id: RwSignal<Option<String>>,
    permission: RwSignal<CameraPermission>,
) {
    spawn_local(async move {
        let perm = mic_ipc::microphone_permission_status().await;
        permission.set(perm);
        let list = mic_ipc::list_microphones().await;
        let default_id = selected_id
            .get_untracked()
            .filter(|id| list.iter().any(|mic| mic.id == *id))
            .or_else(|| resolve_default(&list));
        selected_id.set(default_id);
        mics.set(list);
    });
}

fn render_row(
    mic: MicrophoneView,
    selected: Option<String>,
    on_select: Callback<String>,
) -> impl IntoView {
    let is_selected = selected.as_deref() == Some(mic.id.as_str());
    let mut class = String::from("mic-picker-row");
    if is_selected {
        class.push_str(" mic-picker-row-selected");
    }
    let id_for_click = mic.id.clone();
    let id_aria = mic.id.clone();
    let label = mic.label.clone();
    let sub = format_subline(&mic);
    view! {
        <li>
            <button
                class=class
                role="option"
                aria-selected=is_selected
                data-mic-id=id_aria
                on:click=move |_| on_select.run(id_for_click.clone())
            >
                <span class="mic-picker-row-label">{label}</span>
                <span class="mic-picker-row-sub">{sub}</span>
                {is_selected.then(|| view! {
                    <span class="mic-picker-row-check" aria-hidden="true">"✓"</span>
                })}
            </button>
        </li>
    }
}

/// Build the per-row "48 kHz · stereo · default" subline. `0` values
/// for `channels` / `sample_rate_hz` (the "unknown" sentinel from
/// M-MIC.0) are omitted entirely rather than rendered as
/// "0 Hz / 0ch" noise.
fn format_subline(mic: &MicrophoneView) -> String {
    let mut parts: Vec<String> = Vec::new();
    if mic.sample_rate_hz > 0 {
        parts.push(format_sample_rate(mic.sample_rate_hz));
    }
    if mic.channels > 0 {
        parts.push(format_channels(mic.channels).to_string());
    }
    if mic.is_default {
        parts.push("default".to_string());
    }
    parts.join(" · ")
}

fn format_sample_rate(rate: u32) -> String {
    if rate.is_multiple_of(1_000) {
        format!("{} kHz", rate / 1_000)
    } else {
        format!("{rate} Hz")
    }
}

fn format_channels(channels: u8) -> &'static str {
    match channels {
        1 => "mono",
        2 => "stereo",
        _ => "multi-ch",
    }
}

fn selected_label(mics: &[MicrophoneView], selected_id: Option<&String>) -> String {
    let Some(id) = selected_id else {
        return "Select microphone".into();
    };
    mics.iter()
        .find(|m| m.id == *id)
        .map_or_else(|| "Select microphone".to_string(), |m| m.label.clone())
}

/// Resolve which mic should be selected on initial mount:
/// 1. Persisted "last used" id (if present + still in the device list).
/// 2. The `is_default = true` device.
/// 3. The first device.
/// 4. `None` if no devices.
fn resolve_default(mics: &[MicrophoneView]) -> Option<String> {
    if mics.is_empty() {
        return None;
    }
    if let Some(last) = read_last_used()
        && mics.iter().any(|m| m.id == last)
    {
        return Some(last);
    }
    mics.iter()
        .find(|m| m.is_default)
        .map(|m| m.id.clone())
        .or_else(|| mics.first().map(|m| m.id.clone()))
}

#[cfg(target_arch = "wasm32")]
fn read_last_used() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok().flatten()?;
    storage.get_item(LAST_USED_KEY).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
fn read_last_used() -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
fn write_last_used(id: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return;
    };
    let _ = storage.set_item(LAST_USED_KEY, id);
}

#[cfg(not(target_arch = "wasm32"))]
fn write_last_used(_id: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn mic(id: &str, label: &str, is_default: bool) -> MicrophoneView {
        MicrophoneView {
            id: id.into(),
            label: label.into(),
            is_default,
            channels: 2,
            sample_rate_hz: 48_000,
            native_id: String::new(),
        }
    }

    #[test]
    fn resolve_default_returns_none_for_empty() {
        assert_eq!(resolve_default(&[]), None);
    }

    #[test]
    fn resolve_default_picks_is_default_when_no_persisted() {
        let list = vec![
            mic("a", "Mic A", false),
            mic("b", "Mic B", true),
            mic("c", "Mic C", false),
        ];
        // LocalStorage doesn't exist in native unit tests, so we
        // fall through to the is_default branch.
        assert_eq!(resolve_default(&list), Some("b".into()));
    }

    #[test]
    fn resolve_default_falls_back_to_first_when_none_default() {
        let list = vec![mic("a", "Mic A", false), mic("b", "Mic B", false)];
        assert_eq!(resolve_default(&list), Some("a".into()));
    }

    #[test]
    fn selected_label_returns_placeholder_when_unselected() {
        assert_eq!(selected_label(&[], None), "Select microphone");
    }

    #[test]
    fn selected_label_returns_mic_label_when_selected() {
        let list = vec![mic("a", "MacBook Pro Microphone", true)];
        let id = "a".to_string();
        assert_eq!(selected_label(&list, Some(&id)), "MacBook Pro Microphone");
    }

    #[test]
    fn format_subline_omits_zero_unknowns() {
        let mut m = mic("a", "Weird Mic", false);
        m.channels = 0;
        m.sample_rate_hz = 0;
        assert_eq!(format_subline(&m), "");
        m.is_default = true;
        assert_eq!(format_subline(&m), "default");
    }

    #[test]
    fn format_subline_full() {
        let m = mic("a", "Mic", true);
        assert_eq!(format_subline(&m), "48 kHz · stereo · default");
    }

    #[test]
    fn format_subline_mono_default_off() {
        let mut m = mic("a", "Mic", false);
        m.channels = 1;
        assert_eq!(format_subline(&m), "48 kHz · mono");
    }

    #[test]
    fn format_sample_rate_uses_kilohertz_when_multiple_of_1000() {
        assert_eq!(format_sample_rate(48_000), "48 kHz");
        assert_eq!(format_sample_rate(44_100), "44100 Hz");
        assert_eq!(format_sample_rate(16_000), "16 kHz");
    }
}
