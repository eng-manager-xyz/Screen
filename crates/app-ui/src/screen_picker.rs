//! `<ScreenPicker />` — screen / window source picker for the
//! Recorder surface (M-SCK.4 / AUT-271 + M-SCK.0.1 / AUT-291).
//!
//! Mirror of [`crate::system_audio_picker`] for the visual capture
//! path. Master Start/Stop toggle drives the SCK screen-capture
//! session; the expandable dropdown shows attached displays + visible
//! windows. **As of M-SCK.0.1 (M-RECORD-EXPORT)**, clicking a non-
//! primary row swaps the active `SCStream` to that source — picker
//! state is the authoritative selection and persists across launches
//! via `LocalStorage`.
//!
//! ```admonish important title="Last-used recovery"
//! Window IDs are NOT stable across launches (Apple's `CGWindowID`
//! is per-session). The picker reads `screen.screen_capture.last_source_id`
//! on mount; if the persisted id doesn't match any current display
//! or window (`list_screen_displays` + `list_screen_windows`), the
//! picker silently falls back to `PrimaryDisplay`. Documented gotcha
//! class in M-SCK.1 (AUT-268).
//! ```

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::screen_ipc::{self, DisplaySourceView, ScreenSourcesResult, WindowSourceView};

/// `LocalStorage` key for the persisted picker selection. Value is
/// `"display-<id>"` / `"window-<id>"` for a pinned source; absent
/// (or empty) means the picker falls back to `PrimaryDisplay` on
/// next launch. cfg-gated to wasm32 because `read_last_used` /
/// `write_last_used` are themselves wasm-only (native unit tests
/// don't touch `LocalStorage`); leaving it on native trips the
/// workspace `-D warnings` gate via `dead_code` (a recurring CI
/// failure mode, last seen in PR #49's CSS-debounce const).
#[cfg(target_arch = "wasm32")]
const LAST_USED_KEY: &str = "screen.screen_capture.last_source_id";

/// `<ScreenPicker />` — master toggle + expandable display/window list
/// + per-row click-to-route (M-SCK.0.1 / AUT-291).
#[component]
pub fn ScreenPicker() -> impl IntoView {
    let enabled = RwSignal::new(false);
    let expanded = RwSignal::new(false);
    let displays = RwSignal::new(Vec::<DisplaySourceView>::new());
    let windows = RwSignal::new(Vec::<WindowSourceView>::new());
    let error_message = RwSignal::new(Option::<String>::None);
    // None = PrimaryDisplay (M-SCK.0 default). Set from LocalStorage
    // on mount; replaced on row-click; cleared when the user toggles
    // off + back on without picking a row (defaults to primary).
    let active_source_id = RwSignal::new(read_last_used());
    // M-RECORD.3 — lock master + expand while a coordinated session
    // is active. Picker shows current state but disallows mutation.
    let recording_lock = RwSignal::new(false);
    crate::recording_ipc::install_recording_lock_listener(recording_lock);

    // Probe live status on mount so the picker reflects an
    // already-running session correctly.
    spawn_local(async move {
        let active = screen_ipc::screen_capture_status().await;
        enabled.set(active);
    });

    let on_toggle_enabled = make_on_toggle_enabled(enabled, active_source_id, error_message);
    let on_toggle_expand =
        make_on_toggle_expand(expanded, displays, windows, error_message, active_source_id);
    let on_pick_source = make_on_pick(enabled, active_source_id, error_message);

    view! {
        <div class="screen-picker">
            <div class="screen-picker-header">
                <button
                    type="button"
                    class="screen-picker-toggle"
                    role="switch"
                    aria-checked=move || enabled.get()
                    data-enabled=move || if enabled.get() { "true" } else { "false" }
                    prop:disabled=move || recording_lock.get()
                    title=move || if recording_lock.get() { "Recording in progress — stop the recording to toggle screen capture" } else { "" }
                    on:click=move |evt| {
                        if recording_lock.get() { return; }
                        on_toggle_enabled(evt);
                    }
                >
                    <span class="screen-picker-icon" aria-hidden="true">"🖥"</span>
                    <span class="screen-picker-label">"Screen"</span>
                    <span class="screen-picker-state">
                        {move || if enabled.get() { "On" } else { "Off" }}
                    </span>
                </button>
                <button
                    type="button"
                    class="screen-picker-expand"
                    aria-haspopup="listbox"
                    aria-expanded=move || expanded.get()
                    on:click=on_toggle_expand
                >
                    <span class="screen-picker-summary">"Sources"</span>
                    <span class="screen-picker-chevron" aria-hidden="true">"▾"</span>
                </button>
            </div>
            <Show when=move || expanded.get() fallback=|| view! { <></> }>
                <div class="screen-picker-menu" role="listbox">
                    <ScreenPickerBody
                        displays=displays
                        windows=windows
                        error_message=error_message
                        active_source_id=active_source_id
                        on_pick=on_pick_source
                    />
                </div>
            </Show>
        </div>
    }
}

/// Closure factory for the master toggle. Extracted so
/// `ScreenPicker` stays under the `clippy::too_many_lines` cap.
fn make_on_toggle_enabled(
    enabled: RwSignal<bool>,
    active_source_id: RwSignal<Option<String>>,
    error_message: RwSignal<Option<String>>,
) -> impl Fn(leptos::ev::MouseEvent) + Copy + Send + Sync + 'static {
    move |_| {
        let next = !enabled.get();
        enabled.set(next);
        let source_for_start = active_source_id.get();
        spawn_local(async move {
            if next {
                match screen_ipc::start_screen_capture(source_for_start).await {
                    Ok(()) => error_message.set(None),
                    Err(err) => {
                        enabled.set(false);
                        error_message.set(Some(err));
                    }
                }
            } else {
                screen_ipc::stop_screen_capture().await;
            }
        });
    }
}

/// Closure factory for the expand-dropdown button.
fn make_on_toggle_expand(
    expanded: RwSignal<bool>,
    displays: RwSignal<Vec<DisplaySourceView>>,
    windows: RwSignal<Vec<WindowSourceView>>,
    error_message: RwSignal<Option<String>>,
    active_source_id: RwSignal<Option<String>>,
) -> impl Fn(leptos::ev::MouseEvent) + Copy + Send + Sync + 'static {
    move |_| {
        let next = !expanded.get();
        expanded.set(next);
        if !next {
            return;
        }
        spawn_local(async move {
            match screen_ipc::list_screen_displays().await {
                ScreenSourcesResult::Ok(list) => {
                    error_message.set(None);
                    displays.set(list);
                }
                ScreenSourcesResult::Err(msg) => {
                    error_message.set(Some(msg));
                    displays.set(Vec::new());
                }
            }
            match screen_ipc::list_screen_windows().await {
                ScreenSourcesResult::Ok(list) => windows.set(list),
                ScreenSourcesResult::Err(_) => windows.set(Vec::new()),
            }
            // After enumeration: stale persisted id → drop to
            // primary. "I picked Safari window last week" case.
            let displays_now = displays.get();
            let windows_now = windows.get();
            if let Some(id) = active_source_id.get()
                && !displays_now.iter().any(|d| d.id == id)
                && !windows_now.iter().any(|w| w.id == id)
            {
                active_source_id.set(None);
                write_last_used(None);
            }
        });
    }
}

/// Closure factory for per-row click. Persists the new source and,
/// if a session is live, tears down + restarts on the new source.
fn make_on_pick(
    enabled: RwSignal<bool>,
    active_source_id: RwSignal<Option<String>>,
    error_message: RwSignal<Option<String>>,
) -> impl Fn(Option<String>) + Copy + Send + Sync + 'static {
    move |source_id: Option<String>| {
        active_source_id.set(source_id.clone());
        write_last_used(source_id.as_deref());
        if enabled.get() {
            let source_clone = source_id.clone();
            spawn_local(async move {
                screen_ipc::stop_screen_capture().await;
                match screen_ipc::start_screen_capture(source_clone).await {
                    Ok(()) => error_message.set(None),
                    Err(err) => {
                        enabled.set(false);
                        error_message.set(Some(err));
                    }
                }
            });
        }
    }
}

#[component]
fn ScreenPickerBody(
    displays: RwSignal<Vec<DisplaySourceView>>,
    windows: RwSignal<Vec<WindowSourceView>>,
    error_message: RwSignal<Option<String>>,
    active_source_id: RwSignal<Option<String>>,
    on_pick: impl Fn(Option<String>) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    move || {
        match (error_message.get(), displays.get(), windows.get()) {
            (Some(msg), _, _) => view! {
                <div class="screen-picker-state-msg screen-picker-state-msg--error">
                    <p>{"Couldn't list screen sources."}</p>
                    <p class="screen-picker-state-help">{msg}</p>
                    <p class="screen-picker-state-help">
                        {"Request access first; macOS will then add this app to System Settings. After enabling it, quit and reopen the app."}
                    </p>
                    <button
                        type="button"
                        class="screen-picker-state-button"
                        on:click=move |_| {
                            spawn_local(async move {
                                screen_ipc::request_screen_recording_permission().await;
                                screen_ipc::open_settings_screen_recording().await;
                            });
                        }
                    >
                        {"Request Screen Recording Access"}
                    </button>
                </div>
            }
            .into_any(),
            (None, d_list, w_list) if d_list.is_empty() && w_list.is_empty() => view! {
                <div class="screen-picker-state-msg">
                    <p>{"No screen sources detected."}</p>
                </div>
            }
            .into_any(),
            (None, d_list, w_list) => {
                let active = active_source_id.get();
                let primary_active = active.is_none();
                view! {
                    <div class="screen-picker-section-label">{"Displays"}</div>
                    <ul class="screen-picker-list" role="none">
                        // Primary-display sentinel row (M-SCK.0 default).
                        // Always present so the user can revert to it.
                        <li
                            class="screen-picker-row"
                            role="option"
                            aria-selected=move || primary_active
                            data-active=move || if primary_active { "true" } else { "false" }
                            on:click=move |_| on_pick(None)
                        >
                            <span class="screen-picker-row-check" aria-hidden="true">
                                {if primary_active { "✓" } else { " " }}
                            </span>
                            <span class="screen-picker-row-label">{"Primary display (auto)"}</span>
                            <span class="screen-picker-row-sub">{"First connected display"}</span>
                        </li>
                        {d_list
                            .into_iter()
                            .map(|d| {
                                let id = d.id.clone();
                                let is_active = active.as_deref() == Some(id.as_str());
                                view! {
                                    <li
                                        class="screen-picker-row"
                                        role="option"
                                        aria-selected=is_active
                                        data-active=if is_active { "true" } else { "false" }
                                        on:click=move |_| on_pick(Some(id.clone()))
                                    >
                                        <span class="screen-picker-row-check" aria-hidden="true">
                                            {if is_active { "✓" } else { " " }}
                                        </span>
                                        <span class="screen-picker-row-label">{d.label.clone()}</span>
                                        <span class="screen-picker-row-sub">
                                            {format!("{}×{}{}", d.width, d.height, if d.is_primary { " · primary" } else { "" })}
                                        </span>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                    <div class="screen-picker-section-label">{"Windows"}</div>
                    <ul class="screen-picker-list" role="none">
                        {w_list
                            .into_iter()
                            .map(|w| {
                                let id = w.id.clone();
                                let is_active = active.as_deref() == Some(id.as_str());
                                let title = if w.label.is_empty() { "(untitled)".to_string() } else { w.label.clone() };
                                view! {
                                    <li
                                        class="screen-picker-row"
                                        role="option"
                                        aria-selected=is_active
                                        data-active=if is_active { "true" } else { "false" }
                                        on:click=move |_| on_pick(Some(id.clone()))
                                    >
                                        <span class="screen-picker-row-check" aria-hidden="true">
                                            {if is_active { "✓" } else { " " }}
                                        </span>
                                        <span class="screen-picker-row-label">{title}</span>
                                        <span class="screen-picker-row-sub">
                                            {format!("{} · {}×{}", w.display_name, w.width, w.height)}
                                        </span>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                }
                .into_any()
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn read_last_used() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok().flatten()?;
    let raw = storage.get_item(LAST_USED_KEY).ok().flatten()?;
    if raw.is_empty() { None } else { Some(raw) }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_last_used() -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
fn write_last_used(id: Option<&str>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return;
    };
    match id {
        Some(value) => {
            let _ = storage.set_item(LAST_USED_KEY, value);
        }
        None => {
            let _ = storage.remove_item(LAST_USED_KEY);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_last_used(_id: Option<&str>) {}
