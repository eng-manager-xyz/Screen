//! `<RecorderPage />` — the live recorder surface that composes the
//! `ui-storybook` presentational components into the design shown
//! in the Screen-Studio-style mock (workspace badge, capture-mode
//! tabs, display source card, camera / mic / system-audio rows with
//! inline expandable pickers, on-screen options summary, and the
//! recording-controls footer).
//!
//! The presentational components stay pure (they take view-models +
//! optional callbacks). This module owns the live state — `RwSignal`s
//! seeded from the `*_ipc` modules — and converts each tick of state
//! into the matching view-model.
//!
//! IPC contracts (`camera_ipc`, `mic_ipc`, `screen_ipc`,
//! `system_audio_ipc`, `recording_ipc`) are NOT touched. This is a
//! pure render-layer rewire so the existing TCC / permission flows
//! keep working exactly as before.

use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;

use ui_storybook::components::menus::{PopoverPlacement, PopoverSurface};
use ui_storybook::components::primitives::{IconTile, IconTileKind};
use ui_storybook::components::recorder::{
    AppIconView, AudioAppView as StoryAudioAppView, AudioFilter, CaptureModeTabs,
    CaptureSourceKind, CaptureSourceView, DeviceOptionView, DevicePickerState, DeviceThumb,
    DisplayPreviewView, DisplaySourceCard, DisplaySourceView, OnScreenOptionKind,
    OnScreenOptionView, PreviewWindowChip, RecordingControlsFooter, RecordingControlsView,
    StartRecordingState, SystemAudioView, format_auto_zoom_label, format_countdown_label,
    format_selection_count,
};
use ui_storybook::fixtures::recorder::CaptureMode;

use crate::camera_ipc::{self, CameraPermission, CameraView};
use crate::mic_ipc::{self, MicrophoneView};
use crate::recording_ipc::{
    self, RecordingConfigView, RecordingStatusViewIpc, SessionStreamsView, default_output_path,
    recording_status, start_recording, stop_recording,
};
use crate::screen_ipc::{self, DisplaySourceView as IpcDisplaySourceView, ScreenSourcesResult};
use crate::system_audio_ipc::{
    self, AudioAppFilterView, AudioAppView as IpcAudioAppView, ListAudioAppsResult,
};

/// Which expandable picker, if any, is currently open. Mutually
/// exclusive so opening one closes the others.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OpenPicker {
    /// Nothing expanded.
    #[default]
    None,
    /// Camera device picker open.
    Camera,
    /// Microphone device picker open.
    Microphone,
    /// System-audio app list open.
    SystemAudio,
    /// On-screen options popover open.
    OnScreen,
}

/// The full Recorder surface — exact match to the design mock.
#[component]
#[allow(
    clippy::too_many_lines,
    reason = "Top-level Leptos composition for the recorder surface; splitting helpers would lose the view! macro's single-tree expansion that keeps reactivity intact."
)]
pub fn RecorderPage() -> impl IntoView {
    // -------- live state ------------------------------------------------
    let cameras = RwSignal::new(Vec::<CameraView>::new());
    let camera_selected = RwSignal::new(Option::<String>::None);
    let camera_enabled = RwSignal::new(true);
    let camera_permission = RwSignal::new(CameraPermission::Granted);

    let mics = RwSignal::new(Vec::<MicrophoneView>::new());
    let mic_selected = RwSignal::new(Option::<String>::None);
    let mic_enabled = RwSignal::new(false);
    let mic_permission = RwSignal::new(CameraPermission::Granted);
    let mic_level = RwSignal::new(0.0_f32);

    let displays = RwSignal::new(Vec::<IpcDisplaySourceView>::new());
    let display_selected = RwSignal::new(Option::<String>::None);
    let display_err = RwSignal::new(Option::<String>::None);

    let audio_apps = RwSignal::new(Vec::<IpcAudioAppView>::new());
    let audio_app_selected = RwSignal::new(Vec::<String>::new());
    let system_audio_enabled = RwSignal::new(false);
    let audio_filter = RwSignal::new(AudioFilter::All);
    let audio_app_err = RwSignal::new(Option::<String>::None);

    let on_screen_opts = RwSignal::new(default_on_screen_options());
    let auto_zoom = RwSignal::new(2.0_f32);
    let countdown_seconds = RwSignal::new(3_u8);

    let capture_mode = RwSignal::new(CaptureMode::Screen);
    let open_picker = RwSignal::new(OpenPicker::None);

    let status = RwSignal::new(RecordingStatusViewIpc::idle());
    let error_msg = RwSignal::new(Option::<String>::None);

    // -------- subscriptions -------------------------------------------
    mic_ipc::subscribe_mic_level(move |lvl| mic_level.set(lvl));
    install_status_listener(status);

    spawn_local(async move {
        let view = recording_status().await;
        status.set(view);
    });
    refresh_cameras(cameras, camera_selected, camera_permission);
    refresh_mics(mics, mic_selected, mic_permission);
    refresh_displays(displays, display_selected, display_err);
    refresh_audio_apps(audio_apps, audio_app_err);

    // -------- view-models ---------------------------------------------
    let camera_view = move || -> CaptureSourceView {
        let label = selected_camera_label(&cameras.get(), camera_selected.get().as_deref());
        let subtitle = camera_subtitle(&cameras.get(), camera_selected.get().as_deref());
        CaptureSourceView {
            id: "camera".to_owned(),
            kind: CaptureSourceKind::Camera,
            title: label,
            subtitle,
            enabled: camera_enabled.get(),
            expanded: open_picker.get() == OpenPicker::Camera,
            favorite: false,
            level: None,
        }
    };

    let mic_view = move || -> CaptureSourceView {
        let label = selected_mic_label(&mics.get(), mic_selected.get().as_deref());
        let subtitle = mic_subtitle(&mics.get(), mic_selected.get().as_deref());
        CaptureSourceView {
            id: "microphone".to_owned(),
            kind: CaptureSourceKind::Microphone,
            title: label,
            subtitle,
            enabled: mic_enabled.get(),
            expanded: open_picker.get() == OpenPicker::Microphone,
            favorite: true,
            level: Some(mic_level.get().clamp(0.0, 1.0)),
        }
    };

    let display_card_view = move || -> DisplaySourceView {
        let list = displays.get();
        let selected = display_selected.get();
        let chosen = list.iter().find(|d| Some(&d.id) == selected.as_ref());
        let primary = list.iter().find(|d| d.is_primary);
        let active = chosen.or(primary).or_else(|| list.first());
        let name = active.map_or("No display detected".to_owned(), |d| d.label.clone());
        let (w, h) = active.map_or((1920, 1080), |d| (d.width, d.height));
        DisplaySourceView {
            id: active.map_or("display-none".to_owned(), |d| d.id.clone()),
            name,
            size_label: format_display_size(w, h),
            dimensions_label: format!("{w} × {h}"),
            is_favorite: true,
            is_selected: true,
            preview: DisplayPreviewView {
                aspect_ratio: aspect_ratio_for(w, h),
                overlay_label: Some(format!("{w} × {h}")),
                mock_windows: vec![PreviewWindowChip {
                    label: "Active window".to_owned(),
                    color: "rgba(200, 200, 200, 0.55)".to_owned(),
                    left_pct: 12,
                    top_pct: 14,
                    width_pct: 72,
                    height_pct: 60,
                }],
            },
        }
    };

    let system_audio_view = move || -> SystemAudioView {
        let apps = audio_apps.get();
        let selected = audio_app_selected.get();
        let total = apps.len();
        let icon_stack: Vec<AppIconView> = apps
            .iter()
            .filter(|a| selected.iter().any(|b| b == &a.bundle_id))
            .take(4)
            .map(|a| AppIconView {
                id: a.bundle_id.clone(),
                monogram: app_monogram(&a.display_name),
                color: app_color_for(&a.bundle_id),
            })
            .collect();
        SystemAudioView {
            enabled: system_audio_enabled.get(),
            expanded: open_picker.get() == OpenPicker::SystemAudio,
            selected_count: selected.len(),
            total_count: total,
            icon_stack,
        }
    };

    let on_screen_summary = move || -> String {
        let opts = on_screen_opts.get();
        let enabled: Vec<String> = opts
            .iter()
            .filter(|o| o.enabled)
            .map(|o| o.title.clone())
            .collect();
        if enabled.is_empty() {
            "Off".to_owned()
        } else {
            format!("{} on", enabled.len())
        }
    };

    let controls_view = move || -> RecordingControlsView {
        // Recording state — drive the page's CSS + footer button
        // variant. While Running / Stopping we render a custom Stop
        // button (see view! below); otherwise show the Ready button.
        let any_source = camera_enabled.get()
            || mic_enabled.get()
            || system_audio_enabled.get()
            || !displays.get().is_empty();
        let state = if any_source {
            StartRecordingState::Ready
        } else {
            StartRecordingState::Disabled
        };
        RecordingControlsView {
            auto_zoom_label: format_auto_zoom_label(Some(auto_zoom.get())),
            countdown_label: format_countdown_label(countdown_seconds.get()),
            shortcuts: vec!["⌘".to_owned(), "⇧".to_owned(), "2".to_owned()],
            start_state: state,
        }
    };
    let elapsed_label = move || {
        let total = status.get().elapsed_ms / 1000;
        let mm = total / 60;
        let ss = total % 60;
        format!("{mm:02}:{ss:02}")
    };
    let is_recording = move || status.get().is_recording();

    // -------- callbacks -----------------------------------------------
    let toggle_camera_picker = move || {
        let next = if open_picker.get() == OpenPicker::Camera {
            OpenPicker::None
        } else {
            OpenPicker::Camera
        };
        open_picker.set(next);
        if next == OpenPicker::Camera {
            refresh_cameras(cameras, camera_selected, camera_permission);
        }
    };
    let toggle_mic_picker = move || {
        let next = if open_picker.get() == OpenPicker::Microphone {
            OpenPicker::None
        } else {
            OpenPicker::Microphone
        };
        open_picker.set(next);
        if next == OpenPicker::Microphone {
            refresh_mics(mics, mic_selected, mic_permission);
        }
    };
    let toggle_audio_picker = move || {
        let next = if open_picker.get() == OpenPicker::SystemAudio {
            OpenPicker::None
        } else {
            OpenPicker::SystemAudio
        };
        open_picker.set(next);
        if next == OpenPicker::SystemAudio {
            refresh_audio_apps(audio_apps, audio_app_err);
        }
    };
    let toggle_on_screen = move || {
        let next = if open_picker.get() == OpenPicker::OnScreen {
            OpenPicker::None
        } else {
            OpenPicker::OnScreen
        };
        open_picker.set(next);
    };

    let on_camera_toggle = move |_: MouseEvent| {
        let next = !camera_enabled.get();
        camera_enabled.set(next);
        if next && camera_selected.get().is_none() {
            let list = cameras.get();
            if let Some(c) = list.first() {
                let id = c.id.clone();
                camera_selected.set(Some(id.clone()));
                spawn_local(async move {
                    camera_ipc::start_preview(id).await;
                });
            }
        }
        // Per the Screenplay-style design, the webcam preview lives in
        // a separate borderless Tauri window (bottom-left of the
        // primary monitor), not inside the recorder panel. Flipping
        // the camera toggle shows / hides that window. `toggle_*`
        // alternates Show ↔ Hide so we call it whenever the boolean
        // changes — IPC handles the state-machine consistency
        // (`BubbleState` on the Rust side).
        crate::bubble_ipc::toggle_webcam_bubble();
    };
    let on_mic_toggle = move |_: MouseEvent| {
        let next = !mic_enabled.get();
        mic_enabled.set(next);
        if next && mic_selected.get().is_none() {
            let list = mics.get();
            if let Some(m) = list.first() {
                let id = m.id.clone();
                mic_selected.set(Some(id.clone()));
                spawn_local(async move {
                    mic_ipc::start_mic_capture(id).await;
                });
            }
        }
        if !next {
            spawn_local(async move {
                mic_ipc::stop_mic_capture().await;
            });
        }
    };
    let on_system_audio_toggle = move |_: MouseEvent| {
        let next = !system_audio_enabled.get();
        system_audio_enabled.set(next);
        spawn_local(async move {
            if next {
                let _ = system_audio_ipc::start_system_audio_capture().await;
            } else {
                system_audio_ipc::stop_system_audio_capture().await;
            }
        });
    };

    let on_camera_select = move |id: String| {
        camera_selected.set(Some(id.clone()));
        open_picker.set(OpenPicker::None);
        spawn_local(async move {
            camera_ipc::start_preview(id).await;
        });
    };
    let on_mic_select = move |id: String| {
        mic_selected.set(Some(id.clone()));
        open_picker.set(OpenPicker::None);
        spawn_local(async move {
            mic_ipc::start_mic_capture(id).await;
        });
    };
    let on_audio_app_toggle = move |bundle_id: String| {
        let mut cur = audio_app_selected.get();
        if let Some(pos) = cur.iter().position(|b| b == &bundle_id) {
            cur.remove(pos);
        } else {
            cur.push(bundle_id);
        }
        audio_app_selected.set(cur.clone());
        spawn_local(async move {
            let filter = if cur.is_empty() {
                AudioAppFilterView::AllAudio
            } else {
                AudioAppFilterView::OnlyApps(cur)
            };
            let _ = system_audio_ipc::set_system_audio_filter(filter).await;
        });
    };
    let on_audio_filter = move |f: AudioFilter| {
        audio_filter.set(f);
        let apps = audio_apps.get();
        match f {
            AudioFilter::All => {
                let all: Vec<String> = apps.iter().map(|a| a.bundle_id.clone()).collect();
                audio_app_selected.set(all);
            }
            AudioFilter::None => {
                audio_app_selected.set(Vec::new());
            }
            AudioFilter::Suggested => {
                let suggested: Vec<String> = apps
                    .iter()
                    .filter(|a| is_suggested_app(&a.bundle_id))
                    .map(|a| a.bundle_id.clone())
                    .collect();
                audio_app_selected.set(suggested);
            }
        }
    };
    let on_screen_toggle = move |kind: OnScreenOptionKind| {
        let mut opts = on_screen_opts.get();
        for o in &mut opts {
            if o.id == kind {
                o.enabled = !o.enabled;
            }
        }
        on_screen_opts.set(opts);
    };

    let on_start = Callback::new(move |()| {
        if status.get().is_recording() {
            spawn_local(async move {
                match stop_recording().await {
                    Ok(_) => {
                        error_msg.set(None);
                        status.set(RecordingStatusViewIpc::idle());
                    }
                    Err(err) => error_msg.set(Some(err)),
                }
            });
            return;
        }
        let cam = camera_enabled.get();
        let mic = mic_enabled.get();
        let sys = system_audio_enabled.get();
        let cam_id = camera_selected.get();
        let mic_id = mic_selected.get();
        let screen_id = display_selected.get();
        spawn_local(async move {
            let path = default_output_path(Some("mp4-h264")).await;
            let config = RecordingConfigView {
                streams: SessionStreamsView {
                    camera: cam,
                    screen: true,
                    microphone: mic,
                    system_audio: sys,
                },
                camera_id: cam_id.unwrap_or_default(),
                microphone_id: mic_id.unwrap_or_default(),
                screen_source_id: screen_id,
                output_path: if path.is_empty() { None } else { Some(path) },
                format: Some("mp4-h264".to_owned()),
            };
            match start_recording(config).await {
                Ok(_) => error_msg.set(None),
                Err(err) => error_msg.set(Some(err)),
            }
        });
    });

    // -------- view ----------------------------------------------------
    view! {
        <section
            class="recorder-page"
            data-mode=move || capture_mode_slug(capture_mode.get())
            data-recording=move || if is_recording() { "true" } else { "false" }
        >
            <Show when=is_recording fallback=|| view! { <></> }>
                <div class="recorder-page-recording-pill" role="status" aria-live="polite">
                    <span class="recorder-page-recording-dot" aria-hidden="true"></span>
                    <span class="recorder-page-recording-label">"RECORDING"</span>
                    <span class="recorder-page-recording-elapsed">
                        {move || elapsed_label()}
                    </span>
                </div>
            </Show>
            <header class="recorder-page-header">
                <button
                    class="recorder-page-workspace"
                    type="button"
                    aria-label="Switch workspace"
                >
                    <IconTile kind=IconTileKind::Workspace>"N"</IconTile>
                </button>
                <CaptureModeTabs selected=capture_mode.get() />
            </header>

            <div class="recorder-page-body">
                <section class="recorder-page-display">
                    {move || view! { <DisplaySourceCard view=display_card_view() /> }}
                    <Show when=move || display_err.get().is_some() fallback=|| view! { <></> }>
                        <DisplayError msg=display_err />
                    </Show>
                </section>

                <section class="recorder-page-sources">
                    <LiveSourceRow
                        view=Signal::derive(camera_view)
                        on_chevron_click=Callback::new(move |()| toggle_camera_picker())
                        on_toggle=Callback::new(move |()| on_camera_toggle(MouseEvent::new("click").unwrap()))
                    />
                    <Show when=move || open_picker.get() == OpenPicker::Camera fallback=|| view! { <></> }>
                        <div class="recorder-page-picker recorder-page-picker--camera">
                            <LiveDevicePicker
                                kind=CaptureSourceKind::Camera
                                state=Signal::derive(move || device_state_for(camera_permission.get(), cameras.get().is_empty()))
                                devices=Signal::derive(move || camera_device_options(cameras.get(), camera_selected.get()))
                                on_select=Callback::new(on_camera_select)
                                on_grant_permission=Callback::new(move |()| {
                                    spawn_local(async move {
                                        let _ = camera_ipc::request_all_permissions().await;
                                    });
                                })
                            />
                        </div>
                    </Show>
                    // Per the design, the live webcam canvas lives in
                    // the floating webcam-bubble window, not inline
                    // inside the recorder panel. on_camera_toggle
                    // shows / hides that window via bubble_ipc.

                    <LiveSourceRow
                        view=Signal::derive(mic_view)
                        on_chevron_click=Callback::new(move |()| toggle_mic_picker())
                        on_toggle=Callback::new(move |()| on_mic_toggle(MouseEvent::new("click").unwrap()))
                    />
                    <Show when=move || open_picker.get() == OpenPicker::Microphone fallback=|| view! { <></> }>
                        <div class="recorder-page-picker recorder-page-picker--mic">
                            <LiveDevicePicker
                                kind=CaptureSourceKind::Microphone
                                state=Signal::derive(move || device_state_for(mic_permission.get(), mics.get().is_empty()))
                                devices=Signal::derive(move || mic_device_options(mics.get(), mic_selected.get()))
                                on_select=Callback::new(on_mic_select)
                                on_grant_permission=Callback::new(move |()| {
                                    spawn_local(async move {
                                        let _ = camera_ipc::request_all_permissions().await;
                                    });
                                })
                            />
                        </div>
                    </Show>
                </section>

                <section class="recorder-page-audio">
                    <LiveSystemAudioRow
                        view=Signal::derive(system_audio_view)
                        on_chevron_click=Callback::new(move |()| toggle_audio_picker())
                        on_toggle=Callback::new(move |()| on_system_audio_toggle(MouseEvent::new("click").unwrap()))
                    />
                    <Show when=move || open_picker.get() == OpenPicker::SystemAudio fallback=|| view! { <></> }>
                        <div class="recorder-page-applist">
                            <LiveAudioAppList
                                apps=Signal::derive(move || audio_app_view_list(
                                    audio_apps.get(),
                                    audio_app_selected.get(),
                                ))
                                active_filter=Signal::derive(move || audio_filter.get())
                                on_filter=Callback::new(on_audio_filter)
                                on_app_toggle=Callback::new(on_audio_app_toggle)
                            />
                            <Show when=move || audio_app_err.get().is_some() fallback=|| view! { <></> }>
                                <p class="recorder-page-audio-error">
                                    {move || audio_app_err.get().unwrap_or_default()}
                                </p>
                            </Show>
                        </div>
                    </Show>
                </section>

                <section class="recorder-page-on-screen">
                    <button
                        class="recorder-page-on-screen-row"
                        type="button"
                        aria-haspopup="dialog"
                        aria-expanded=move || open_picker.get() == OpenPicker::OnScreen
                        on:click=move |_| toggle_on_screen()
                    >
                        <IconTile kind=IconTileKind::Action>"✦"</IconTile>
                        <span class="recorder-page-on-screen-text">
                            <span class="recorder-page-on-screen-title">"On-screen"</span>
                            <span class="recorder-page-on-screen-summary">
                                {move || on_screen_summary()}
                            </span>
                        </span>
                        <span class="recorder-page-chevron" aria-hidden="true">"▸"</span>
                    </button>
                    <Show when=move || open_picker.get() == OpenPicker::OnScreen fallback=|| view! { <></> }>
                        <div class="recorder-page-on-screen-popover">
                            <LiveOnScreenPopover
                                opts=Signal::derive(move || on_screen_opts.get())
                                on_toggle=Callback::new(on_screen_toggle)
                            />
                        </div>
                    </Show>
                </section>
            </div>

            <PopoverSurface
                placement=PopoverPlacement::BottomLeft
                width_px=380_u16
            >
                <Show
                    when=is_recording
                    fallback=move || view! {
                        <RecordingControlsFooter
                            view=controls_view()
                            on_start=on_start
                        />
                    }
                >
                    <div class="recording-controls-footer recording-controls-footer-stop" data-state="recording">
                        <div class="recording-controls-selects">
                            <span class="recorder-page-recording-elapsed">
                                {move || elapsed_label()}
                            </span>
                        </div>
                        <div class="recording-controls-action">
                            <button
                                type="button"
                                class="start-recording-btn start-recording-stop"
                                aria-label="Stop recording"
                                on:click=move |_| on_start.run(())
                            >
                                <span class="start-recording-glyph" aria-hidden="true">"■"</span>
                                <span class="start-recording-label">"Stop recording"</span>
                            </button>
                        </div>
                    </div>
                </Show>
            </PopoverSurface>

            <Show when=move || error_msg.get().is_some() fallback=|| view! { <></> }>
                <div class="recorder-page-error" role="alert">
                    {move || error_msg.get().unwrap_or_default()}
                </div>
            </Show>
        </section>
    }
}

// ---------------------------------------------------------------------
// helper components (small render-only utilities for the page above)
// ---------------------------------------------------------------------

#[component]
fn DisplayError(msg: RwSignal<Option<String>>) -> impl IntoView {
    view! {
        <p class="recorder-page-display-error" role="alert">
            {move || msg.get().unwrap_or_default()}
        </p>
    }
}

/// Live, click-wired equivalent of `ui_storybook::components::recorder::CaptureSourceRow`.
///
/// Uses the same CSS class names as the storybook component so it
/// renders identically; the difference is that every interactive
/// surface (chevron button, toggle button) is a real `<button>` with
/// its own `on:click` handler. The previous transparent-overlay
/// approach didn't work because the overlays spanned the full row and
/// the toggle's stopPropagation didn't beat sibling DOM order.
#[component]
fn LiveSourceRow(
    view: Signal<CaptureSourceView>,
    on_chevron_click: Callback<()>,
    on_toggle: Callback<()>,
) -> impl IntoView {
    view! {
        {move || {
            let v = view.get();
            let mut class = String::from("capture-source-row");
            if v.expanded {
                class.push_str(" capture-source-row-expanded");
            }
            if !v.enabled {
                class.push_str(" capture-source-row-off");
            }
            let chevron_class = if v.expanded {
                "capture-source-chevron capture-source-chevron-open"
            } else {
                "capture-source-chevron"
            };
            let kind_attr = match v.kind {
                CaptureSourceKind::Camera => "camera",
                CaptureSourceKind::Microphone => "microphone",
            };
            let glyph = v.kind.glyph();
            let title = v.title.clone();
            let subtitle = v.subtitle.clone();
            let toggle_class = if v.enabled {
                "toggle-switch toggle-switch-checked"
            } else {
                "toggle-switch"
            };
            let toggle_aria = format!("Enable {}", v.kind.label());
            view! {
                <div class=class data-kind=kind_attr>
                    <span class="capture-source-leading">
                        <span class="icon-tile icon-tile-device" aria-hidden="true">{glyph}</span>
                    </span>
                    <span class="capture-source-text">
                        <span class="capture-source-title">{title}</span>
                        <span class="capture-source-subtitle">{subtitle}</span>
                    </span>
                    {v.kind == CaptureSourceKind::Microphone && v.level.is_some()}
                    {v.level.filter(|_| v.kind == CaptureSourceKind::Microphone).map(|l| view! {
                        <span class="capture-source-meter">
                            <span class="meter" data-level=format!("{:.2}", l)>
                                {(0_u8..10).map(|i| {
                                    let on = f32::from(i) / 10.0 < l;
                                    view! {
                                        <span class=if on { "meter-bar meter-bar-on" } else { "meter-bar" }></span>
                                    }
                                }).collect_view()}
                            </span>
                        </span>
                    })}
                    <button
                        type="button"
                        class=toggle_class
                        role="switch"
                        aria-checked=v.enabled
                        aria-label=toggle_aria
                        on:click=move |evt: MouseEvent| {
                            evt.stop_propagation();
                            on_toggle.run(());
                        }
                    >
                        <span class="toggle-switch-thumb" aria-hidden="true"></span>
                    </button>
                    <button
                        type="button"
                        class=chevron_class
                        aria-label="Expand device picker"
                        aria-expanded=v.expanded
                        on:click=move |evt: MouseEvent| {
                            evt.stop_propagation();
                            on_chevron_click.run(());
                        }
                    >
                        "▾"
                    </button>
                </div>
            }
        }}
    }
}

/// Live, click-wired system-audio row. Mirrors
/// `SystemAudioRow`'s HTML for visual fidelity but routes the toggle
/// + expand clicks to proper callbacks.
#[component]
fn LiveSystemAudioRow(
    view: Signal<SystemAudioView>,
    on_chevron_click: Callback<()>,
    on_toggle: Callback<()>,
) -> impl IntoView {
    view! {
        {move || {
            let v = view.get();
            let chevron_class = if v.expanded {
                "system-audio-chevron system-audio-chevron-open"
            } else {
                "system-audio-chevron"
            };
            let count_label = format_selection_count(v.selected_count, v.total_count);
            let icons = v.icon_stack.clone();
            let visible = icons.iter().take(3).cloned().collect::<Vec<_>>();
            let overflow = icons.len().saturating_sub(visible.len());
            let toggle_class = if v.enabled {
                "toggle-switch toggle-switch-checked"
            } else {
                "toggle-switch"
            };
            view! {
                <div class="system-audio-row">
                    <span class="system-audio-leading">
                        <span class="system-audio-icon-stack">
                            {visible.into_iter().map(|icon| {
                                let style = format!("background:{}", icon.color);
                                let monogram = icon.monogram.clone();
                                view! {
                                    <span class="system-audio-icon" style=style aria-hidden="true">{monogram}</span>
                                }
                            }).collect_view()}
                            {(overflow > 0).then(|| {
                                let label = format!("+{overflow}");
                                view! { <span class="system-audio-icon-overflow">{label}</span> }
                            })}
                        </span>
                    </span>
                    <span class="system-audio-text">
                        <span class="system-audio-title">"System audio"</span>
                        <span class="system-audio-subtitle">{count_label}</span>
                    </span>
                    <button
                        type="button"
                        class=toggle_class
                        role="switch"
                        aria-checked=v.enabled
                        aria-label="Enable system audio"
                        on:click=move |evt: MouseEvent| {
                            evt.stop_propagation();
                            on_toggle.run(());
                        }
                    >
                        <span class="toggle-switch-thumb" aria-hidden="true"></span>
                    </button>
                    <button
                        type="button"
                        class=chevron_class
                        aria-label="Expand system audio app list"
                        aria-expanded=v.expanded
                        on:click=move |evt: MouseEvent| {
                            evt.stop_propagation();
                            on_chevron_click.run(());
                        }
                    >
                        "▾"
                    </button>
                </div>
            }
        }}
    }
}

/// Live, click-wired device-picker (camera + mic). Each row is a real
/// `<button>` that fires `on_select(id)` when clicked.
#[component]
fn LiveDevicePicker(
    kind: CaptureSourceKind,
    state: Signal<DevicePickerState>,
    devices: Signal<Vec<DeviceOptionView>>,
    on_select: Callback<String>,
    on_grant_permission: Callback<()>,
) -> impl IntoView {
    view! {
        {move || {
            let body: AnyView = match state.get() {
                DevicePickerState::PermissionNeeded => view! {
                    <div class="device-picker-state">
                        <div class="device-picker-state-glyph" aria-hidden="true">"⚠"</div>
                        <div class="device-picker-state-title">
                            {match kind {
                                CaptureSourceKind::Camera => "Camera access required",
                                CaptureSourceKind::Microphone => "Microphone access required",
                            }}
                        </div>
                        <div class="device-picker-state-subtitle">
                            "Grant access in System Settings → Privacy & Security, then re-open this menu."
                        </div>
                        <button
                            type="button"
                            class="btn btn-default btn-sm"
                            style="margin-top:8px"
                            on:click=move |_| on_grant_permission.run(())
                        >"Grant access"</button>
                    </div>
                }.into_any(),
                DevicePickerState::Empty => view! {
                    <div class="device-picker-state">
                        <div class="device-picker-state-glyph" aria-hidden="true">"⚠"</div>
                        <div class="device-picker-state-title">"No devices detected"</div>
                        <div class="device-picker-state-subtitle">
                            "Plug in or pair a device, then re-open this menu."
                        </div>
                    </div>
                }.into_any(),
                DevicePickerState::Populated => view! {
                    <ul class="device-picker-rows" role="menu">
                        {devices.get().into_iter().map(|opt| {
                            let mut class = String::from("device-picker-row");
                            if opt.selected {
                                class.push_str(" device-picker-row-selected");
                            }
                            let id = opt.id.clone();
                            let id_for_aria = opt.id.clone();
                            let level_view = opt.level.map(|l| view! {
                                <span class="device-picker-row-meter">
                                    <span class="meter">
                                        {(0_u8..10).map(|i| {
                                            let on = f32::from(i) / 10.0 < l;
                                            view! {
                                                <span class=if on { "meter-bar meter-bar-on" } else { "meter-bar" }></span>
                                            }
                                        }).collect_view()}
                                    </span>
                                </span>
                            });
                            let thumb_view = opt.thumbnail.as_ref().map(|t| {
                                let style = format!("background:{}", t.background);
                                let glyph = t.glyph.clone();
                                view! {
                                    <span class="device-picker-thumb" style=style aria-hidden="true">
                                        <span class="device-picker-thumb-glyph">{glyph}</span>
                                    </span>
                                }
                            });
                            let badge_view = opt.badge.clone().map(|b| view! {
                                <span class="badge badge-plan">{b}</span>
                            });
                            view! {
                                <li class="device-picker-row-item" role="none">
                                    <button
                                        class=class
                                        role="menuitem"
                                        aria-pressed=opt.selected
                                        data-id=id_for_aria
                                        on:click=move |_| on_select.run(id.clone())
                                    >
                                        {thumb_view}
                                        <span class="device-picker-row-text">
                                            <span class="device-picker-row-name">{opt.name}</span>
                                            <span class="device-picker-row-detail">{opt.detail}</span>
                                        </span>
                                        {badge_view}
                                        {level_view}
                                        {opt.selected.then(|| view! {
                                            <span class="device-picker-row-check" aria-hidden="true">"✓"</span>
                                        })}
                                    </button>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any(),
            };
            view! {
                <div class="popover-surface popover-surface-bottom-left device-picker">
                    {body}
                </div>
            }
        }}
    }
}

/// Live, click-wired system-audio app list. All / None / Suggested
/// filter chips fire `on_filter`; clicking an app row fires
/// `on_app_toggle(bundle_id)`.
#[component]
fn LiveAudioAppList(
    apps: Signal<Vec<StoryAudioAppView>>,
    active_filter: Signal<AudioFilter>,
    on_filter: Callback<AudioFilter>,
    on_app_toggle: Callback<String>,
) -> impl IntoView {
    view! {
        {move || {
            let filter = active_filter.get();
            let chips: Vec<_> = [AudioFilter::All, AudioFilter::None, AudioFilter::Suggested]
                .iter()
                .map(|f| {
                    let f = *f;
                    let mut class = String::from("system-audio-filter");
                    if f == filter {
                        class.push_str(" system-audio-filter-active");
                    }
                    view! {
                        <button
                            type="button"
                            class=class
                            data-filter=f.slug()
                            aria-pressed=f == filter
                            on:click=move |_| on_filter.run(f)
                        >{f.label()}</button>
                    }
                })
                .collect();
            let rows: Vec<_> = apps.get().into_iter().map(|app| {
                let mut class = String::from("audio-app-row");
                if app.selected {
                    class.push_str(" audio-app-row-selected");
                }
                if app.live {
                    class.push_str(" audio-app-row-live");
                }
                let icon_style = format!("background:{}", app.icon.color);
                let monogram = app.icon.monogram.clone();
                let bundle = app.id.clone();
                let level_view = app.level.map(|l| view! {
                    <span class="audio-app-meter">
                        <span class="meter">
                            {(0_u8..8).map(|i| {
                                let on = f32::from(i) / 8.0 < l;
                                view! {
                                    <span class=if on { "meter-bar meter-bar-on" } else { "meter-bar" }></span>
                                }
                            }).collect_view()}
                        </span>
                    </span>
                });
                view! {
                    <li
                        class=class
                        role="option"
                        aria-selected=app.selected
                        on:click=move |_| on_app_toggle.run(bundle.clone())
                    >
                        <span class=if app.selected { "audio-app-check audio-app-check-on" } else { "audio-app-check" } aria-hidden="true">
                            {if app.selected { "✓" } else { "" }}
                        </span>
                        <span class="audio-app-icon" style=icon_style aria-hidden="true">{monogram}</span>
                        <span class="audio-app-text">
                            <span class="audio-app-name">{app.name}</span>
                            <span class="audio-app-context">{app.context}</span>
                        </span>
                        {app.suggested.then(|| view! {
                            <span class="badge badge-accent">"Suggested"</span>
                        })}
                        {app.live.then(|| view! {
                            <span class="audio-app-live" aria-label="Live audio">
                                <span class="audio-app-live-dot" aria-hidden="true"></span>
                                "LIVE"
                            </span>
                        })}
                        {level_view}
                    </li>
                }
            }).collect();
            view! {
                <div class="system-audio-applist">
                    <div class="system-audio-filters" role="toolbar" aria-label="Filter audio apps">
                        {chips}
                    </div>
                    <ul class="system-audio-rows" role="listbox" aria-label="Audio apps">
                        {rows}
                    </ul>
                </div>
            }
        }}
    }
}

/// Live on-screen options popover with click-wired toggles. Mirrors
/// the storybook `OnScreenOptionsPopover` HTML so CSS stays shared.
#[component]
fn LiveOnScreenPopover(
    opts: Signal<Vec<OnScreenOptionView>>,
    on_toggle: Callback<OnScreenOptionKind>,
) -> impl IntoView {
    view! {
        <div class="popover-surface popover-surface-bottom-left on-screen-options-popover">
            <div class="popover-surface-header">
                <div class="popover-surface-title">"On-screen"</div>
                <div class="popover-surface-description">"Choose what shows during recording."</div>
            </div>
            <ul class="on-screen-options" role="group" aria-label="On-screen options">
                {move || opts.get().into_iter().map(|o| {
                    let kind = o.id;
                    let mut class = String::from("on-screen-option-row");
                    if o.disabled {
                        class.push_str(" on-screen-option-row-disabled");
                    }
                    let toggle_class = if o.enabled {
                        "toggle-switch toggle-switch-checked"
                    } else {
                        "toggle-switch"
                    };
                    view! {
                        <li class=class data-option=match kind {
                            OnScreenOptionKind::CleanDesktop => "clean-desktop",
                            OnScreenOptionKind::ShowKeys => "show-keys",
                            OnScreenOptionKind::BlurSensitiveInfo => "blur-sensitive-info",
                        }>
                            <span class="on-screen-option-toggle">
                                <button
                                    type="button"
                                    class=toggle_class
                                    role="switch"
                                    aria-checked=o.enabled
                                    disabled=o.disabled
                                    on:click=move |_| on_toggle.run(kind)
                                >
                                    <span class="toggle-switch-thumb" aria-hidden="true"></span>
                                </button>
                            </span>
                            <span class="on-screen-option-text">
                                <span class="on-screen-option-title">{o.title}</span>
                                <span class="on-screen-option-description">{o.description}</span>
                            </span>
                        </li>
                    }
                }).collect_view()}
            </ul>
        </div>
    }
}

// ---------------------------------------------------------------------
// pure helpers (unit-testable, no Leptos)
// ---------------------------------------------------------------------

fn capture_mode_slug(m: CaptureMode) -> &'static str {
    match m {
        CaptureMode::Screen => "screen",
        CaptureMode::Window => "window",
        CaptureMode::Area => "area",
    }
}

fn default_on_screen_options() -> Vec<OnScreenOptionView> {
    vec![
        OnScreenOptionView {
            id: OnScreenOptionKind::CleanDesktop,
            title: "Clean desktop".to_owned(),
            description: "Hide icons + dock during recording.".to_owned(),
            enabled: true,
            disabled: false,
        },
        OnScreenOptionView {
            id: OnScreenOptionKind::ShowKeys,
            title: "Show keys".to_owned(),
            description: "Render keypress badges over the recording.".to_owned(),
            enabled: true,
            disabled: false,
        },
        OnScreenOptionView {
            id: OnScreenOptionKind::BlurSensitiveInfo,
            title: "Blur sensitive".to_owned(),
            description: "Auto-blur regions tagged as sensitive (coming soon).".to_owned(),
            enabled: false,
            disabled: true,
        },
    ]
}

fn selected_camera_label(list: &[CameraView], selected: Option<&str>) -> String {
    selected
        .and_then(|id| list.iter().find(|c| c.id == id))
        .or_else(|| list.first())
        .map_or_else(|| "No camera".to_owned(), |c| c.label.clone())
}

fn camera_subtitle(list: &[CameraView], selected: Option<&str>) -> String {
    let count = list.len();
    let active = selected
        .and_then(|id| list.iter().find(|c| c.id == id))
        .or_else(|| list.first());
    match (active, count) {
        (None, _) => "Plug in a camera to capture".to_owned(),
        (Some(c), _) if c.is_default => "Built-in · default".to_owned(),
        (Some(_), 1) => "USB · 1 device".to_owned(),
        (Some(_), n) => format!("USB · {n} devices"),
    }
}

fn selected_mic_label(list: &[MicrophoneView], selected: Option<&str>) -> String {
    selected
        .and_then(|id| list.iter().find(|m| m.id == id))
        .or_else(|| list.first())
        .map_or_else(|| "No microphone".to_owned(), |m| m.label.clone())
}

fn mic_subtitle(list: &[MicrophoneView], selected: Option<&str>) -> String {
    let active = selected
        .and_then(|id| list.iter().find(|m| m.id == id))
        .or_else(|| list.first());
    match active {
        None => "Plug in a microphone to capture".to_owned(),
        Some(m) if m.is_default => "Built-in · default".to_owned(),
        Some(_) => "External device".to_owned(),
    }
}

fn camera_device_options(list: Vec<CameraView>, selected: Option<String>) -> Vec<DeviceOptionView> {
    list.into_iter()
        .map(|c| DeviceOptionView {
            id: c.id.clone(),
            name: c.label.clone(),
            detail: if c.is_default {
                "Built-in".to_owned()
            } else {
                "External".to_owned()
            },
            badge: None,
            selected: selected.as_deref() == Some(c.id.as_str()),
            level: None,
            thumbnail: Some(DeviceThumb {
                background: "linear-gradient(135deg, #4338ca, #db2777)".to_owned(),
                glyph: monogram_for(&c.label),
            }),
        })
        .collect()
}

fn mic_device_options(
    list: Vec<MicrophoneView>,
    selected: Option<String>,
) -> Vec<DeviceOptionView> {
    list.into_iter()
        .map(|m| DeviceOptionView {
            id: m.id.clone(),
            name: m.label.clone(),
            detail: if m.is_default {
                "Built-in".to_owned()
            } else {
                "External".to_owned()
            },
            badge: None,
            selected: selected.as_deref() == Some(m.id.as_str()),
            level: Some(0.0),
            thumbnail: None,
        })
        .collect()
}

fn audio_app_view_list(
    list: Vec<IpcAudioAppView>,
    selected: Vec<String>,
) -> Vec<StoryAudioAppView> {
    list.into_iter()
        .map(|a| {
            let is_selected = selected.iter().any(|b| b == &a.bundle_id);
            let suggested = is_suggested_app(&a.bundle_id);
            let monogram = app_monogram(&a.display_name);
            let color = app_color_for(&a.bundle_id);
            StoryAudioAppView {
                id: a.bundle_id.clone(),
                name: a.display_name.clone(),
                context: if a.bundle_id.is_empty() {
                    "App".to_owned()
                } else {
                    a.bundle_id.clone()
                },
                selected: is_selected,
                suggested,
                live: is_selected && suggested,
                level: if is_selected { Some(0.4) } else { None },
                icon: AppIconView {
                    id: a.bundle_id.clone(),
                    monogram,
                    color,
                },
            }
        })
        .collect()
}

fn device_state_for(perm: CameraPermission, empty: bool) -> DevicePickerState {
    match (perm, empty) {
        (CameraPermission::Denied | CameraPermission::NotDetermined, _) => {
            DevicePickerState::PermissionNeeded
        }
        (CameraPermission::Granted, true) => DevicePickerState::Empty,
        (CameraPermission::Granted, false) => DevicePickerState::Populated,
    }
}

fn aspect_ratio_for(w: u32, h: u32) -> (u16, u16) {
    if w == 0 || h == 0 {
        return (16, 10);
    }
    let g = gcd(w, h);
    let nw = u16::try_from((w / g).min(u32::from(u16::MAX))).unwrap_or(u16::MAX);
    let nh = u16::try_from((h / g).min(u32::from(u16::MAX))).unwrap_or(u16::MAX);
    (nw, nh)
}

const fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn format_display_size(w: u32, h: u32) -> String {
    // Diagonal in inches assuming ~226 dpi (Retina). Best-effort label.
    let w = f64::from(w);
    let h = f64::from(h);
    let diag = w.hypot(h) / 226.0;
    format!("{diag:.0}\"")
}

fn monogram_for(label: &str) -> String {
    let mut out = String::new();
    for word in label.split_whitespace().take(2) {
        if let Some(c) = word.chars().next() {
            out.push(c.to_ascii_uppercase());
        }
    }
    if out.is_empty() {
        out.push('?');
    }
    out
}

fn app_monogram(name: &str) -> String {
    name.chars()
        .find(char::is_ascii_alphanumeric)
        .map_or_else(|| "·".to_owned(), |c| c.to_ascii_uppercase().to_string())
}

fn app_color_for(bundle_id: &str) -> String {
    // Stable per-bundle hue derived from a tiny FNV-1a hash.
    let mut h: u32 = 2_166_136_261;
    for b in bundle_id.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(16_777_619);
    }
    let hue = h % 360;
    format!("hsl({hue} 70% 45%)")
}

fn is_suggested_app(bundle_id: &str) -> bool {
    matches!(
        bundle_id,
        "com.spotify.client" | "com.google.Chrome" | "com.apple.Safari" | "com.microsoft.teams2"
    )
}

// ---------------------------------------------------------------------
// IPC refreshers (spawn_local, write into RwSignals)
// ---------------------------------------------------------------------

fn refresh_cameras(
    cameras: RwSignal<Vec<CameraView>>,
    selected: RwSignal<Option<String>>,
    permission: RwSignal<CameraPermission>,
) {
    spawn_local(async move {
        permission.set(camera_ipc::camera_permission_status().await);
        let list = camera_ipc::list_cameras().await;
        if selected.get().is_none()
            && let Some(first) = list.first()
        {
            let id = first.id.clone();
            selected.set(Some(id.clone()));
            spawn_local(async move {
                camera_ipc::start_preview(id).await;
            });
        }
        cameras.set(list);
    });
}

fn refresh_mics(
    mics: RwSignal<Vec<MicrophoneView>>,
    selected: RwSignal<Option<String>>,
    permission: RwSignal<CameraPermission>,
) {
    spawn_local(async move {
        permission.set(mic_ipc::microphone_permission_status().await);
        let list = mic_ipc::list_microphones().await;
        if selected.get().is_none()
            && let Some(first) = list.first()
        {
            selected.set(Some(first.id.clone()));
        }
        mics.set(list);
    });
}

fn refresh_displays(
    displays: RwSignal<Vec<IpcDisplaySourceView>>,
    selected: RwSignal<Option<String>>,
    err: RwSignal<Option<String>>,
) {
    spawn_local(async move {
        match screen_ipc::list_screen_displays().await {
            ScreenSourcesResult::Ok(list) => {
                if selected.get().is_none()
                    && let Some(primary) =
                        list.iter().find(|d| d.is_primary).or_else(|| list.first())
                {
                    selected.set(Some(primary.id.clone()));
                }
                displays.set(list);
                err.set(None);
            }
            ScreenSourcesResult::Err(msg) => err.set(Some(msg)),
        }
    });
}

fn refresh_audio_apps(apps: RwSignal<Vec<IpcAudioAppView>>, err: RwSignal<Option<String>>) {
    spawn_local(async move {
        match system_audio_ipc::list_audio_apps().await {
            ListAudioAppsResult::Ok(list) => {
                apps.set(list);
                err.set(None);
            }
            ListAudioAppsResult::Err(msg) => err.set(Some(msg)),
        }
    });
}

fn install_status_listener(status: RwSignal<RecordingStatusViewIpc>) {
    // Subscribe to the Tauri-side `recording-status` push event so the
    // page reactively reflects Running / Stopping / Idle. Without this
    // the Start↔Stop cycle would never flip — the button would stay at
    // "Start recording" and a second click would launch ANOTHER
    // recording instead of stopping the first.
    recording_ipc::install_recording_status_listener(status);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monogram_initials() {
        assert_eq!(monogram_for("Spotify"), "S");
        assert_eq!(monogram_for("FaceTime HD Camera"), "FH");
        assert_eq!(monogram_for("iPhone 15 Pro"), "I1");
        assert_eq!(monogram_for(""), "?");
    }

    #[test]
    fn aspect_ratio_reduces() {
        assert_eq!(aspect_ratio_for(3024, 1964), (756, 491));
        assert_eq!(aspect_ratio_for(1920, 1080), (16, 9));
        assert_eq!(aspect_ratio_for(0, 0), (16, 10));
    }

    #[test]
    fn capture_mode_slugs() {
        assert_eq!(capture_mode_slug(CaptureMode::Screen), "screen");
        assert_eq!(capture_mode_slug(CaptureMode::Window), "window");
        assert_eq!(capture_mode_slug(CaptureMode::Area), "area");
    }

    #[test]
    fn device_state_classification() {
        assert_eq!(
            device_state_for(CameraPermission::Denied, false),
            DevicePickerState::PermissionNeeded
        );
        assert_eq!(
            device_state_for(CameraPermission::Granted, true),
            DevicePickerState::Empty
        );
        assert_eq!(
            device_state_for(CameraPermission::Granted, false),
            DevicePickerState::Populated
        );
    }

    #[test]
    fn camera_subtitle_handles_states() {
        assert_eq!(camera_subtitle(&[], None), "Plug in a camera to capture");
        let cam = CameraView {
            id: "id".into(),
            label: "Cam".into(),
            is_default: true,
        };
        assert_eq!(camera_subtitle(&[cam], Some("id")), "Built-in · default");
    }

    #[test]
    fn default_on_screen_has_three() {
        let opts = default_on_screen_options();
        assert_eq!(opts.len(), 3);
    }

    #[test]
    fn suggested_includes_spotify() {
        assert!(is_suggested_app("com.spotify.client"));
        assert!(!is_suggested_app("com.example.unknown"));
    }
}
