//! `<ScreenPicker />` — screen / window source picker for the
//! Recorder surface (M-SCK.4 / AUT-271).
//!
//! Mirror of [`crate::system_audio_picker`] for the visual capture
//! path. Master Start/Stop toggle drives the SCK screen-capture
//! session; the expandable dropdown shows attached displays and
//! visible windows so a future M-SCK.0.1 follow-up can switch
//! between them.
//!
//! ```admonish note title="V0 scope"
//! The dropdown lists sources but **selecting a non-default source
//! doesn't yet route**. M-SCK.0 always captures the primary display
//! per its `ScreenCaptureConfig`. Display/window picking is a
//! drop-in extension to `ScreenCaptureConfig` once
//! `M-SCK.0.1` lands. The picker UX exists today so the user can
//! see what's enumerable + the lifecycle works end-to-end.
//! ```

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::screen_ipc::{self, DisplaySourceView, ScreenSourcesResult, WindowSourceView};

/// `<ScreenPicker />` — master toggle + expandable display/window list.
#[component]
pub fn ScreenPicker() -> impl IntoView {
    let enabled = RwSignal::new(false);
    let expanded = RwSignal::new(false);
    let displays = RwSignal::new(Vec::<DisplaySourceView>::new());
    let windows = RwSignal::new(Vec::<WindowSourceView>::new());
    let error_message = RwSignal::new(Option::<String>::None);

    // Probe live status on mount so the picker reflects an
    // already-running session correctly (e.g., a tray-popover
    // re-open after Recorder was the active surface).
    spawn_local(async move {
        let active = screen_ipc::screen_capture_status().await;
        enabled.set(active);
    });

    let on_toggle_enabled = move |_| {
        let next = !enabled.get();
        enabled.set(next);
        spawn_local(async move {
            if next {
                match screen_ipc::start_screen_capture().await {
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
    };

    let on_toggle_expand = move |_| {
        let next = !expanded.get();
        expanded.set(next);
        if next {
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
            });
        }
    };

    view! {
        <div class="screen-picker">
            <div class="screen-picker-header">
                <button
                    type="button"
                    class="screen-picker-toggle"
                    role="switch"
                    aria-checked=move || enabled.get()
                    data-enabled=move || if enabled.get() { "true" } else { "false" }
                    on:click=on_toggle_enabled
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
                    />
                </div>
            </Show>
        </div>
    }
}

#[component]
fn ScreenPickerBody(
    displays: RwSignal<Vec<DisplaySourceView>>,
    windows: RwSignal<Vec<WindowSourceView>>,
    error_message: RwSignal<Option<String>>,
) -> impl IntoView {
    move || {
        match (error_message.get(), displays.get(), windows.get()) {
        (Some(msg), _, _) => view! {
            <div class="screen-picker-state-msg screen-picker-state-msg--error">
                <p>{"Couldn't list screen sources."}</p>
                <p class="screen-picker-state-help">{msg}</p>
                <p class="screen-picker-state-help">
                    {"Grant Screen Recording in System Settings → Privacy & Security, then quit and reopen the app."}
                </p>
            </div>
        }
        .into_any(),
        (None, d_list, w_list) if d_list.is_empty() && w_list.is_empty() => view! {
            <div class="screen-picker-state-msg">
                <p>{"No screen sources detected."}</p>
            </div>
        }
        .into_any(),
        (None, d_list, w_list) => view! {
            <div class="screen-picker-section-label">{"Displays"}</div>
            <ul class="screen-picker-list" role="none">
                {d_list
                    .into_iter()
                    .map(|d| view! {
                        <li class="screen-picker-row">
                            <span class="screen-picker-row-label">{d.label.clone()}</span>
                            <span class="screen-picker-row-sub">
                                {format!("{}×{}{}", d.width, d.height, if d.is_primary { " · primary" } else { "" })}
                            </span>
                        </li>
                    })
                    .collect_view()}
            </ul>
            <div class="screen-picker-section-label">{"Windows"}</div>
            <ul class="screen-picker-list" role="none">
                {w_list
                    .into_iter()
                    .map(|w| {
                        let title = if w.label.is_empty() { "(untitled)".to_string() } else { w.label.clone() };
                        view! {
                            <li class="screen-picker-row">
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
        .into_any(),
    }
    }
}
