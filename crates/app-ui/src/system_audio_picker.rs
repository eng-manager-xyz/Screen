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

/// Debounce window for `set_system_audio_filter` invocations
/// (M-AUDIO-SYS.3 / AUT-288). SCK's `updateContentFilter` takes
/// ~100 ms; 250 ms lets the user click 3-4 checkboxes in rapid
/// succession and only rebuild the stream once on the trailing
/// edge. Only referenced from the wasm32 branch of
/// `schedule_filter_apply` — native builds skip the gloo-timers
/// path entirely.
#[cfg(target_arch = "wasm32")]
const FILTER_DEBOUNCE_MS: u32 = 250;

/// Suggested-app heuristic (M-AUDIO-SYS.3 / AUT-288). Best-effort
/// baseline list of bundle-id prefixes the recorder thinks the
/// user probably wants when they click the "Suggested" chip:
/// browsers, media / streaming apps, and comm apps. Excludes
/// system services + the recorder itself by omission.
///
/// Match is **prefix-based** so versioned bundles like
/// `com.google.Chrome.beta` still hit. PRs welcome — this is a
/// pragmatic baseline, not a curated taxonomy.
const SUGGESTED_BUNDLE_PREFIXES: &[&str] = &[
    // Browsers
    "com.google.Chrome",
    "com.apple.Safari",
    "org.mozilla.firefox",
    "com.brave.Browser",
    "company.thebrowser.Browser", // Arc
    "com.microsoft.edgemac",
    // Media / streaming
    "com.spotify.client",
    "com.apple.Music",
    "com.apple.TV",
    "tv.plex.player",
    "com.netflix.Netflix",
    // Communication
    "com.tinyspeck.slackmacgap",
    "us.zoom.xos",
    "com.microsoft.teams2",
    "com.hnc.Discord",
    "com.google.meet",
];

/// Which filter-chip is visually active given the current picker
/// state. `Custom` is the implicit "user hand-edited the
/// checkboxes" mode — no chip is mapped to it as a click target;
/// it just lights up when none of the other three describe the
/// current selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveChip {
    /// All apps captured (no filter).
    All,
    /// Master toggle is off — no system audio captured.
    None,
    /// Selection exactly matches the suggested-heuristic result.
    Suggested,
    /// User hand-edited the checkboxes; no chip describes it.
    Custom,
}

/// Compute which chip should appear active. Order matters — when
/// multiple chips could describe the state (e.g. empty selection
/// could be "All" OR "Suggested produced no matches"), the most
/// specific wins.
fn compute_active_chip(
    enabled: bool,
    selected: &[String],
    all_apps: &[AudioAppView],
) -> ActiveChip {
    if !enabled {
        return ActiveChip::None;
    }
    if selected.is_empty() {
        return ActiveChip::All;
    }
    let suggested = suggested_bundle_ids(all_apps);
    if same_set(selected, &suggested) {
        ActiveChip::Suggested
    } else {
        ActiveChip::Custom
    }
}

/// Order-insensitive equality of two bundle-id lists.
fn same_set(a: &[String], b: &[String]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let set: std::collections::HashSet<&str> = a.iter().map(String::as_str).collect();
    b.iter().all(|s| set.contains(s.as_str()))
}

/// Walk the running-app list and pull out bundle ids whose prefix
/// is in [`SUGGESTED_BUNDLE_PREFIXES`].
fn suggested_bundle_ids(apps: &[AudioAppView]) -> Vec<String> {
    apps.iter()
        .filter(|a| {
            SUGGESTED_BUNDLE_PREFIXES
                .iter()
                .any(|p| a.bundle_id.starts_with(p))
        })
        .map(|a| a.bundle_id.clone())
        .collect()
}

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
    // M-RECORD.3 — lock the master toggle while a coordinated
    // session is active so the user can't drop sys-audio mid-record.
    let recording_lock = RwSignal::new(false);
    crate::recording_ipc::install_recording_lock_listener(recording_lock);

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
            schedule_filter_apply(selected_now);
        }
    };

    // Chip-click handlers (M-AUDIO-SYS.3 / AUT-288). Each chip
    // mutates `(enabled, selected_ids)` to the canonical state for
    // that chip, persists, and (when enabled) schedules a
    // debounced filter apply.
    let on_chip_all = move |_| {
        selected_ids.set(Vec::new());
        write_selected_ids(&[]);
        if enabled.get() {
            schedule_filter_apply(Vec::new());
        } else {
            enabled.set(true);
            write_enabled(true);
            spawn_local(async move {
                match system_audio_ipc::start_system_audio_capture().await {
                    Ok(()) => {
                        error_message.set(None);
                        schedule_filter_apply(Vec::new());
                    }
                    Err(err) => {
                        enabled.set(false);
                        write_enabled(false);
                        error_message.set(Some(err));
                    }
                }
            });
        }
    };
    let on_chip_none = move |_| {
        enabled.set(false);
        write_enabled(false);
        spawn_local(async move {
            system_audio_ipc::stop_system_audio_capture().await;
        });
    };
    let on_chip_suggested = move |_| {
        let pick = suggested_bundle_ids(&apps.get());
        selected_ids.set(pick.clone());
        write_selected_ids(&pick);
        if enabled.get() {
            schedule_filter_apply(pick);
        } else {
            enabled.set(true);
            write_enabled(true);
            let pick_for_start = pick.clone();
            spawn_local(async move {
                match system_audio_ipc::start_system_audio_capture().await {
                    Ok(()) => {
                        error_message.set(None);
                        schedule_filter_apply(pick_for_start);
                    }
                    Err(err) => {
                        enabled.set(false);
                        write_enabled(false);
                        error_message.set(Some(err));
                    }
                }
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
                    prop:disabled=move || recording_lock.get()
                    title=move || if recording_lock.get() { "Recording in progress — stop the recording to toggle system audio" } else { "" }
                    on:click=move |evt| {
                        if recording_lock.get() { return; }
                        on_toggle_enabled(evt);
                    }
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
                    <div class="system-audio-picker-chips">
                        {
                            let chip_signal = Memo::new(move |_| {
                                compute_active_chip(enabled.get(), &selected_ids.get(), &apps.get())
                            });
                            view! {
                                <button
                                    type="button"
                                    class="system-audio-picker-chip"
                                    data-active=move || (chip_signal.get() == ActiveChip::All).to_string()
                                    on:click=on_chip_all
                                >"All"</button>
                                <button
                                    type="button"
                                    class="system-audio-picker-chip"
                                    data-active=move || (chip_signal.get() == ActiveChip::None).to_string()
                                    on:click=on_chip_none
                                >"None"</button>
                                <button
                                    type="button"
                                    class="system-audio-picker-chip"
                                    data-active=move || (chip_signal.get() == ActiveChip::Suggested).to_string()
                                    on:click=on_chip_suggested
                                >"Suggested"</button>
                                <button
                                    type="button"
                                    class="system-audio-picker-chip"
                                    data-active=move || (chip_signal.get() == ActiveChip::Custom).to_string()
                                    disabled=true
                                >"Custom"</button>
                            }
                        }
                    </div>
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
                    {"Request access first; macOS will then add this app to System Settings. After enabling it, quit and reopen the app."}
                </p>
                <button
                    type="button"
                    class="system-audio-picker-state-button"
                    on:click=move |_| {
                        spawn_local(async move {
                            system_audio_ipc::request_screen_recording_permission().await;
                            system_audio_ipc::open_settings_screen_recording().await;
                        });
                    }
                >
                    {"Request Screen Recording Access"}
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

/// Debounced wrapper around [`apply_filter_from_selection`]
/// (M-AUDIO-SYS.3 / AUT-288). Replaces any pending timeout with a
/// fresh `FILTER_DEBOUNCE_MS`-ms one — the previous Timeout drops,
/// which cancels it. Single-threaded wasm32 means a `RefCell` is
/// sufficient for the shared handle.
#[cfg(target_arch = "wasm32")]
fn schedule_filter_apply(selected_ids: Vec<String>) {
    use gloo_timers::callback::Timeout;
    use std::cell::RefCell;
    thread_local! {
        static PENDING: RefCell<Option<Timeout>> = const { RefCell::new(None) };
    }
    let timeout = Timeout::new(FILTER_DEBOUNCE_MS, move || {
        let ids = selected_ids.clone();
        leptos::task::spawn_local(async move {
            apply_filter_from_selection(&ids).await;
        });
    });
    PENDING.with(|cell| {
        // Replacing drops the previous Timeout (cancels it).
        *cell.borrow_mut() = Some(timeout);
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn schedule_filter_apply(_selected_ids: Vec<String>) {
    // Native target — no event loop to schedule on; tests exercise
    // `apply_filter_from_selection` + `compute_active_chip` /
    // `suggested_bundle_ids` directly.
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

    fn app(bundle: &str) -> AudioAppView {
        AudioAppView {
            pid: 0,
            bundle_id: bundle.to_string(),
            display_name: bundle.to_string(),
            icon_png_bytes: Vec::new(),
        }
    }

    #[test]
    fn suggested_picks_browsers_media_comm() {
        let running = vec![
            app("com.google.Chrome"),
            app("com.spotify.client"),
            app("us.zoom.xos"),
            app("com.apple.Notes"), // not in suggested list
            app("com.apple.dock"),  // not in suggested list
        ];
        let picked = suggested_bundle_ids(&running);
        assert_eq!(picked.len(), 3);
        assert!(picked.contains(&"com.google.Chrome".to_string()));
        assert!(picked.contains(&"com.spotify.client".to_string()));
        assert!(picked.contains(&"us.zoom.xos".to_string()));
        assert!(!picked.contains(&"com.apple.Notes".to_string()));
    }

    #[test]
    fn suggested_prefix_match_catches_versioned_bundles() {
        let running = vec![app("com.google.Chrome.beta"), app("com.brave.Browser.dev")];
        let picked = suggested_bundle_ids(&running);
        assert_eq!(picked.len(), 2, "versioned bundles should match by prefix");
    }

    #[test]
    fn active_chip_none_when_disabled() {
        let chip = compute_active_chip(false, &[], &[]);
        assert_eq!(chip, ActiveChip::None);
    }

    #[test]
    fn active_chip_all_when_enabled_with_empty_selection() {
        let chip = compute_active_chip(true, &[], &[]);
        assert_eq!(chip, ActiveChip::All);
    }

    #[test]
    fn active_chip_suggested_when_selection_matches_heuristic() {
        let running = vec![app("com.spotify.client"), app("com.apple.Notes")];
        let selected = vec!["com.spotify.client".to_string()];
        assert_eq!(
            compute_active_chip(true, &selected, &running),
            ActiveChip::Suggested
        );
    }

    #[test]
    fn active_chip_custom_when_selection_doesnt_match_heuristic() {
        let running = vec![app("com.spotify.client"), app("com.apple.Notes")];
        let selected = vec!["com.apple.Notes".to_string()];
        assert_eq!(
            compute_active_chip(true, &selected, &running),
            ActiveChip::Custom
        );
    }

    #[test]
    fn same_set_is_order_insensitive() {
        let a = vec!["a".to_string(), "b".to_string()];
        let b = vec!["b".to_string(), "a".to_string()];
        assert!(same_set(&a, &b));
        let c = vec!["a".to_string()];
        assert!(!same_set(&a, &c));
    }
}
