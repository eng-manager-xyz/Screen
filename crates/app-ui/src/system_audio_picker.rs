//! `<SystemAudioPicker />` — live-data picker for the speaker /
//! per-app audio capture in the Recorder surface (M-AUDIO-SYS.2 /
//! AUT-279 / AUT-282).
//!
//! Mirrors [`crate::mic_picker`] for the speaker path with two key
//! differences:
//!
//! - **Master on/off toggle** (`enabled` signal) gates everything.
//!   The mic picker has no master toggle because a mic device is
//!   always selectable; system audio is opt-in and the toggle is
//!   the cleanest UX.
//! - **Multi-select with bundle-id persistence**. Each app row has a
//!   checkbox; the selected bundle ids round-trip through
//!   `LocalStorage` so a Spotify selection survives across launches.
//!
//! ```admonish note title="What's deferred from the ticket spec"
//! The full ticket described filter chips (All / None / Suggested /
//! Custom) and a suggested-app heuristic. v0 ships the underlying
//! `AudioAppFilter` machinery + a simple multi-select grid; the
//! filter-chip + suggested heuristic UX is a follow-up
//! (M-AUDIO-SYS.2.1). The contract this commit ships is enough to
//! verify the SCK per-app filtering path end-to-end.
//! ```

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::system_audio_ipc::{self, AudioAppFilterView, AudioAppView, ListAudioAppsResult};

/// `LocalStorage` key for the persisted multi-select bundle-id set.
#[cfg(target_arch = "wasm32")]
const SELECTED_KEY: &str = "screen.system_audio.selected_bundle_ids";

/// `LocalStorage` key for the master enabled flag.
#[cfg(target_arch = "wasm32")]
const ENABLED_KEY: &str = "screen.system_audio.enabled";

/// `<SystemAudioPicker />` — master toggle + expandable per-app
/// checklist.
#[allow(
    clippy::too_many_lines,
    reason = "Leptos #[component] body is signals + closures + view!; splitting at the natural seam (each signal's click handler) would only hurt readability without reducing complexity."
)]
#[component]
pub fn SystemAudioPicker() -> impl IntoView {
    let enabled = RwSignal::new(read_enabled());
    let expanded = RwSignal::new(false);
    let apps = RwSignal::new(Vec::<AudioAppView>::new());
    let error_message = RwSignal::new(Option::<String>::None);
    let selected_ids = RwSignal::new(read_selected_ids());
    // M-AUDIO.METER / AUT-287 — master-stream RMS from the SCK
    // delegate. Per-app meters are deferred to M-AUDIO.METER.1.
    let level = RwSignal::new(0.0_f32);

    system_audio_ipc::subscribe_system_audio_level(move |l| level.set(l));

    // Master toggle: clicking starts/stops the SCK session.
    let on_toggle_enabled = move |_| {
        let next = !enabled.get();
        enabled.set(next);
        write_enabled(next);
        let selected_now = selected_ids.get();
        spawn_local(async move {
            if next {
                match system_audio_ipc::start_system_audio_capture().await {
                    Ok(()) => {
                        error_message.set(None);
                        apply_filter_from_selection(&selected_now).await;
                    }
                    Err(err) => {
                        // Revert master toggle since start failed.
                        enabled.set(false);
                        write_enabled(false);
                        error_message.set(Some(err));
                    }
                }
            } else {
                system_audio_ipc::stop_system_audio_capture().await;
            }
        });
    };

    // Open the expander → re-fetch apps so newly-launched ones appear.
    let on_toggle_expand = move |_| {
        let next = !expanded.get();
        expanded.set(next);
        if next {
            spawn_local(async move {
                match system_audio_ipc::list_audio_apps().await {
                    ListAudioAppsResult::Ok(list) => {
                        error_message.set(None);
                        apps.set(list);
                    }
                    ListAudioAppsResult::Err(msg) => {
                        error_message.set(Some(msg));
                        apps.set(Vec::new());
                    }
                }
            });
        }
    };

    let on_toggle_app = move |bundle_id: String| {
        selected_ids.update(|ids| {
            if let Some(pos) = ids.iter().position(|id| id == &bundle_id) {
                ids.remove(pos);
            } else {
                ids.push(bundle_id);
            }
        });
        let selected_now = selected_ids.get();
        write_selected_ids(&selected_now);
        if enabled.get() {
            spawn_local(async move {
                apply_filter_from_selection(&selected_now).await;
            });
        }
    };

    view! {
        <div class="system-audio-picker">
            <div class="system-audio-picker-header">
                <button
                    type="button"
                    class="system-audio-picker-toggle"
                    role="switch"
                    aria-checked=move || enabled.get()
                    data-enabled=move || if enabled.get() { "true" } else { "false" }
                    on:click=on_toggle_enabled
                >
                    <span class="system-audio-picker-icon" aria-hidden="true">"🔈"</span>
                    <span class="system-audio-picker-label">"System audio"</span>
                    <span class="system-audio-picker-state">
                        {move || if enabled.get() { "On" } else { "Off" }}
                    </span>
                </button>
                <button
                    type="button"
                    class="system-audio-picker-expand"
                    aria-haspopup="listbox"
                    aria-expanded=move || expanded.get()
                    on:click=on_toggle_expand
                >
                    <span class="system-audio-picker-summary">
                        {move || summary_label(selected_ids.get().len())}
                    </span>
                    <span class="system-audio-picker-chevron" aria-hidden="true">"▾"</span>
                </button>
            </div>
            <div class="audio-meter" aria-label="System audio output level">
                <div
                    class="audio-meter-bar"
                    style:width=move || format!("{:.1}%", (level.get() * 100.0).clamp(0.0, 100.0))
                ></div>
            </div>
            <Show when=move || expanded.get() fallback=|| view! { <></> }>
                <div class="system-audio-picker-menu" role="listbox">
                    <SystemAudioBody
                        apps=apps
                        selected_ids=selected_ids
                        error_message=error_message
                        on_toggle_app=Callback::new(on_toggle_app)
                    />
                </div>
            </Show>
        </div>
    }
}

#[component]
fn SystemAudioBody(
    apps: RwSignal<Vec<AudioAppView>>,
    selected_ids: RwSignal<Vec<String>>,
    error_message: RwSignal<Option<String>>,
    on_toggle_app: Callback<String>,
) -> impl IntoView {
    move || {
        match (error_message.get(), apps.get()) {
        (Some(msg), _) => view! {
            <div class="system-audio-picker-state-msg system-audio-picker-state-msg--error">
                <p>{"Couldn't list apps."}</p>
                <p class="system-audio-picker-state-help">{msg}</p>
                <p class="system-audio-picker-state-help">
                    {"Grant Screen Recording in System Settings → Privacy & Security, then quit and reopen the app."}
                </p>
                <button
                    type="button"
                    class="system-audio-picker-state-button"
                    on:click=move |_| {
                        spawn_local(async move {
                            system_audio_ipc::open_settings_screen_recording().await;
                        });
                    }
                >
                    {"Open System Settings → Screen Recording"}
                </button>
            </div>
        }
        .into_any(),
        (None, list) if list.is_empty() => view! {
            <div class="system-audio-picker-state-msg">
                <p>{"No running apps detected."}</p>
            </div>
        }
        .into_any(),
        (None, list) => view! {
            <ul class="system-audio-picker-list" role="none">
                {list
                    .into_iter()
                    .map(|app| render_app_row(app, selected_ids.get(), on_toggle_app))
                    .collect_view()}
            </ul>
        }
        .into_any(),
    }
    }
}

fn render_app_row(
    app: AudioAppView,
    selected: Vec<String>,
    on_toggle_app: Callback<String>,
) -> impl IntoView {
    let is_selected = selected.iter().any(|id| id == &app.bundle_id);
    let mut class = String::from("system-audio-picker-row");
    if is_selected {
        class.push_str(" system-audio-picker-row-selected");
    }
    let bundle_for_click = app.bundle_id.clone();
    let bundle_for_attr = app.bundle_id.clone();
    let bundle_for_caption = app.bundle_id.clone();
    let name = app.display_name.clone();
    view! {
        <li>
            <button
                type="button"
                class=class
                role="option"
                aria-selected=is_selected
                data-bundle-id=bundle_for_attr
                on:click=move |_| on_toggle_app.run(bundle_for_click.clone())
            >
                <span class="system-audio-picker-row-icon" aria-hidden="true">
                    {if app.icon_png_bytes.is_empty() { "·" } else { "■" }}
                </span>
                <span class="system-audio-picker-row-label">{name}</span>
                <span class="system-audio-picker-row-bundle">{bundle_for_caption}</span>
                {is_selected.then(|| view! {
                    <span class="system-audio-picker-row-check" aria-hidden="true">"✓"</span>
                })}
            </button>
        </li>
    }
}

/// Build a filter from the current selection and push it to the
/// backend. Empty selection → `AllAudio` (capture everything);
/// non-empty → `OnlyApps` (capture just those).
async fn apply_filter_from_selection(selected_ids: &[String]) {
    let filter = if selected_ids.is_empty() {
        AudioAppFilterView::AllAudio
    } else {
        AudioAppFilterView::OnlyApps(selected_ids.to_vec())
    };
    let _ = system_audio_ipc::set_system_audio_filter(filter).await;
}

fn summary_label(count: usize) -> String {
    match count {
        0 => "All apps".into(),
        1 => "1 app".into(),
        n => format!("{n} apps"),
    }
}

#[cfg(target_arch = "wasm32")]
fn read_enabled() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return false;
    };
    matches!(
        storage.get_item(ENABLED_KEY).ok().flatten().as_deref(),
        Some("true")
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn read_enabled() -> bool {
    false
}

#[cfg(target_arch = "wasm32")]
fn write_enabled(value: bool) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return;
    };
    let _ = storage.set_item(ENABLED_KEY, if value { "true" } else { "false" });
}

#[cfg(not(target_arch = "wasm32"))]
fn write_enabled(_value: bool) {}

#[cfg(target_arch = "wasm32")]
fn read_selected_ids() -> Vec<String> {
    let Some(window) = web_sys::window() else {
        return Vec::new();
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return Vec::new();
    };
    storage
        .get_item(SELECTED_KEY)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn read_selected_ids() -> Vec<String> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
fn write_selected_ids(ids: &[String]) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return;
    };
    if let Ok(json) = serde_json::to_string(ids) {
        let _ = storage.set_item(SELECTED_KEY, &json);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_selected_ids(_ids: &[String]) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_label_zero_one_many() {
        assert_eq!(summary_label(0), "All apps");
        assert_eq!(summary_label(1), "1 app");
        assert_eq!(summary_label(5), "5 apps");
    }

    #[test]
    fn empty_selection_yields_all_audio_filter() {
        // The picker treats empty selection as "capture everything" —
        // matches the master-toggle UX where flipping On without
        // picking any specific apps should yield system-wide capture.
        let selected: Vec<String> = Vec::new();
        let filter = if selected.is_empty() {
            AudioAppFilterView::AllAudio
        } else {
            AudioAppFilterView::OnlyApps(selected)
        };
        assert_eq!(filter, AudioAppFilterView::AllAudio);
    }

    #[test]
    fn non_empty_selection_yields_only_apps_filter() {
        let selected = vec!["com.spotify.client".to_string()];
        let filter = if selected.is_empty() {
            AudioAppFilterView::AllAudio
        } else {
            AudioAppFilterView::OnlyApps(selected.clone())
        };
        assert_eq!(filter, AudioAppFilterView::OnlyApps(selected));
    }
}
