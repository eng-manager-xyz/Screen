//! `<CameraPicker />` — live-data dropdown selector for the Recorder
//! surface (M-CAM.4 / AUT-258 + M-REC.1 / AUT-260).
//!
//! Combines the M-CAM.2 IPC helpers with the storybook
//! `CaptureSourceRow` + `DevicePickerMenu` shapes (M-UI.8 / AUT-128)
//! to provide:
//!
//! - On mount: invoke `list_cameras` once to seed the picker.
//! - On open: re-fetch so newly-plugged USB cameras appear.
//! - On select: invoke `start_preview(camera_id)` + persist the
//!   selection to `LocalStorage` so re-opens land on the same camera.
//! - Permission-denied state mirrors M-CAM.3's `RecorderPreviewState`
//!   so the picker UI matches the canvas overlay.
//!
//! ```admonish note title="Storybook component reuse"
//! The visual elements (`CaptureSourceRow`, `DevicePickerMenu`)
//! live in `ui-storybook` and are pure presentational — they take
//! view-models, not state. This module wires the storybook
//! components to live IPC data via Leptos `Resource` + signals.
//! ```

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::camera_ipc::{self, CameraPermission, CameraView};

/// `LocalStorage` key for the "last used" camera id. Only referenced
/// from the wasm32 `read_last_used` / `write_last_used` helpers, so
/// cfg-gated to match — otherwise the native-target build (which CI's
/// `just lint` exercises) trips the `dead_code` lint.
#[cfg(target_arch = "wasm32")]
const LAST_USED_KEY: &str = "screen.camera.last_used_id";

/// `<CameraPicker />` — live device picker rendered above the
/// `<CameraPreview />` canvas in the Recorder surface.
#[component]
pub fn CameraPicker() -> impl IntoView {
    let cameras = RwSignal::new(Vec::<CameraView>::new());
    let selected_id = RwSignal::new(Option::<String>::None);
    let permission = RwSignal::new(CameraPermission::Granted);
    let open = RwSignal::new(false);
    // M-RECORD.3 — disable the trigger while a coordinated
    // recording session is active so the user can't swap cameras
    // mid-record (would invalidate the M-EXPORT encoder state).
    let recording_lock = RwSignal::new(false);
    crate::recording_ipc::install_recording_lock_listener(recording_lock);

    // Initial mount: probe permission, then enumerate.
    spawn_local(async move {
        let perm = camera_ipc::camera_permission_status().await;
        permission.set(perm);
        let list = camera_ipc::list_cameras().await;
        let default_id = resolve_default(&list);
        if let Some(id) = &default_id {
            // Auto-start the preview on first mount so the canvas
            // gets frames the moment the wisp pipeline lands.
            let id = id.clone();
            spawn_local(async move {
                camera_ipc::start_preview(id).await;
            });
        }
        selected_id.set(default_id);
        cameras.set(list);
    });

    let on_select = move |id: String| {
        // Persist + propagate.
        write_last_used(&id);
        let id_for_invoke = id.clone();
        spawn_local(async move {
            camera_ipc::start_preview(id_for_invoke).await;
        });
        selected_id.set(Some(id));
        open.set(false);
    };

    view! {
        <div class="camera-picker">
            <button
                class="camera-picker-trigger"
                aria-haspopup="listbox"
                aria-expanded=move || open.get()
                prop:disabled=move || recording_lock.get()
                title=move || if recording_lock.get() { "Recording in progress — stop the recording to change cameras" } else { "" }
                on:click=move |_| {
                    if recording_lock.get() { return; }
                    open.update(|o| *o = !*o);
                }
            >
                <span class="camera-picker-label">
                    {move || selected_label(&cameras.get(), selected_id.get().as_ref())}
                </span>
                <span class="camera-picker-chevron" aria-hidden="true">"▾"</span>
            </button>
            <Show when=move || open.get() fallback=|| view! { <></> }>
                <div class="camera-picker-menu" role="listbox">
                    <CameraPickerBody
                        cameras=cameras
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
fn CameraPickerBody(
    cameras: RwSignal<Vec<CameraView>>,
    selected_id: RwSignal<Option<String>>,
    permission: RwSignal<CameraPermission>,
    on_select: Callback<String>,
) -> impl IntoView {
    move || {
        match (permission.get(), cameras.get()) {
        (CameraPermission::Denied | CameraPermission::NotDetermined, _) => view! {
            <div class="camera-picker-state camera-picker-state--permission">
                <p>{"Camera access required."}</p>
                <p class="camera-picker-state-help">
                    {"Grant access in System Settings → Privacy & Security, then re-open this picker."}
                </p>
                <button
                    type="button"
                    class="camera-picker-state-button"
                    on:click=move |_| {
                        spawn_local(async move {
                            camera_ipc::open_settings_camera().await;
                        });
                    }
                >
                    {"Open System Settings → Camera"}
                </button>
            </div>
        }
        .into_any(),
        (CameraPermission::Granted, list) if list.is_empty() => view! {
            <div class="camera-picker-state camera-picker-state--empty">
                <p>{"No cameras detected."}</p>
                <p class="camera-picker-state-help">
                    {"Plug in or pair a camera, then re-open this picker."}
                </p>
            </div>
        }
        .into_any(),
        (CameraPermission::Granted, list) => view! {
            <ul class="camera-picker-list" role="none">
                {list
                    .into_iter()
                    .map(|cam| render_row(cam, selected_id.get(), on_select))
                    .collect_view()}
            </ul>
        }
        .into_any(),
    }
    }
}

fn render_row(
    cam: CameraView,
    selected: Option<String>,
    on_select: Callback<String>,
) -> impl IntoView {
    let is_selected = selected.as_deref() == Some(cam.id.as_str());
    let mut class = String::from("camera-picker-row");
    if is_selected {
        class.push_str(" camera-picker-row-selected");
    }
    let id_for_click = cam.id.clone();
    let id_aria = cam.id.clone();
    let label = cam.label.clone();
    view! {
        <li>
            <button
                class=class
                role="option"
                aria-selected=is_selected
                data-camera-id=id_aria
                on:click=move |_| on_select.run(id_for_click.clone())
            >
                <span class="camera-picker-row-label">{label}</span>
                {is_selected.then(|| view! {
                    <span class="camera-picker-row-check" aria-hidden="true">"✓"</span>
                })}
            </button>
        </li>
    }
}

fn selected_label(cameras: &[CameraView], selected_id: Option<&String>) -> String {
    let Some(id) = selected_id else {
        return "Select camera".into();
    };
    cameras
        .iter()
        .find(|c| c.id == *id)
        .map_or_else(|| "Select camera".to_string(), |c| c.label.clone())
}

/// Resolve which camera should be selected on initial mount:
/// 1. Persisted "last used" id (if present + still in the device list).
/// 2. The `is_default = true` device.
/// 3. The first device.
/// 4. `None` if no devices.
fn resolve_default(cameras: &[CameraView]) -> Option<String> {
    if cameras.is_empty() {
        return None;
    }
    if let Some(last) = read_last_used()
        && cameras.iter().any(|c| c.id == last)
    {
        return Some(last);
    }
    cameras
        .iter()
        .find(|c| c.is_default)
        .map(|c| c.id.clone())
        .or_else(|| cameras.first().map(|c| c.id.clone()))
}

#[cfg(target_arch = "wasm32")]
fn read_last_used() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok().flatten()?;
    storage.get_item(LAST_USED_KEY).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
fn read_last_used() -> Option<String> {
    // Native unit-test path: web_sys::window() doesn't exist in
    // native nextest. Returning None lets the resolve_default tests
    // exercise the fallback branches without touching `LocalStorage`.
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
fn write_last_used(_id: &str) {
    // Native: no-op. LocalStorage is only meaningful in the browser
    // CSR target.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cam(id: &str, label: &str, is_default: bool) -> CameraView {
        CameraView {
            id: id.into(),
            label: label.into(),
            is_default,
        }
    }

    #[test]
    fn resolve_default_returns_none_for_empty() {
        assert_eq!(resolve_default(&[]), None);
    }

    #[test]
    fn resolve_default_picks_is_default_when_no_persisted() {
        let list = vec![
            cam("a", "Cam A", false),
            cam("b", "Cam B", true),
            cam("c", "Cam C", false),
        ];
        // Note: in a unit-test environment LocalStorage doesn't
        // exist, so read_last_used returns None and we fall through
        // to the is_default branch.
        assert_eq!(resolve_default(&list), Some("b".into()));
    }

    #[test]
    fn resolve_default_falls_back_to_first_when_none_default() {
        let list = vec![cam("a", "Cam A", false), cam("b", "Cam B", false)];
        assert_eq!(resolve_default(&list), Some("a".into()));
    }

    #[test]
    fn selected_label_returns_placeholder_when_unselected() {
        assert_eq!(selected_label(&[], None), "Select camera");
    }

    #[test]
    fn selected_label_returns_camera_label_when_selected() {
        let list = vec![cam("a", "FaceTime HD", true)];
        let id = "a".to_string();
        assert_eq!(selected_label(&list, Some(&id)), "FaceTime HD");
    }
}
