//! Tier-1 IPC harness for the M-MIC.1 / AUT-278 mic commands.
//!
//! Exercises `list_microphones` + `mic_status` the way the Leptos
//! frontend does, in-process via `tauri::test::mock_builder`. The
//! actual gst worker is NOT spawned by these tests — `start_mic_capture`
//! would fire the macOS `NSMicrophoneUsageDescription` prompt and
//! block the test runner waiting for a click. Worker-spawn coverage
//! is exercised manually per the M-MIC.1 acceptance criteria
//! (`cargo run -p screen-app` → tray → Recorder).
//!
//! # Why this file is skipped on Windows
//!
//! Same reason as `commands.rs` — `mock_builder` transitively links
//! `WebView2Loader.dll` and the preinstalled loader on the Windows
//! GitHub runner can't satisfy our pinned Tauri 2's expected exports
//! (`STATUS_ENTRYPOINT_NOT_FOUND`). macOS + Ubuntu exercise this.

#![cfg(not(target_os = "windows"))]

use tauri::WebviewUrl;
use tauri::WebviewWindowBuilder;
use tauri::http::HeaderMap;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets,
};
use tauri::webview::InvokeRequest;

use screen_app::audio::{MicCaptureHandle, MicCaptureState, MicLifecycle};
use screen_app::commands;

fn build_app() -> tauri::App<MockRuntime> {
    mock_builder()
        .manage(MicCaptureState::default())
        .manage(MicCaptureHandle::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_microphones,
            commands::mic_status,
        ])
        .build(mock_context(noop_assets()))
        .expect("build mock app")
}

fn main_webview(app: &tauri::App<MockRuntime>) -> tauri::WebviewWindow<MockRuntime> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .build()
        .expect("build mock webview")
}

fn invoke(
    webview: &tauri::WebviewWindow<MockRuntime>,
    cmd: &str,
    body: InvokeBody,
) -> serde_json::Value {
    let url = if cfg!(any(windows, target_os = "android")) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    }
    .parse()
    .unwrap();
    let request = InvokeRequest {
        cmd: cmd.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url,
        body,
        headers: HeaderMap::default(),
        invoke_key: INVOKE_KEY.to_string(),
    };
    let response = get_ipc_response(webview, request).unwrap_or_else(|err| {
        panic!("command {cmd} returned an error: {err}");
    });
    response.deserialize::<serde_json::Value>().unwrap()
}

#[test]
fn mic_status_returns_idle_initially() {
    let app = build_app();
    let webview = main_webview(&app);
    let value = invoke(&webview, "mic_status", InvokeBody::default());
    let status: MicLifecycle = serde_json::from_value(value).unwrap();
    assert_eq!(status, MicLifecycle::Idle);
}

#[test]
fn list_microphones_returns_array_shape() {
    // Even without a mic attached, list_microphones must return a
    // JSON array (possibly empty) — never an error. The contract
    // matches list_cameras for symmetry.
    let app = build_app();
    let webview = main_webview(&app);
    let value = invoke(&webview, "list_microphones", InvokeBody::default());
    assert!(value.is_array(), "expected array, got {value:?}");
}

#[test]
fn microphone_view_serde_shape_is_camel_aligned_with_camera_view() {
    // Regression guard: if a field is renamed / added / removed, the
    // shape changes and the Leptos `MicrophoneOptionView` decoder
    // breaks silently. We assert each expected field is present in
    // each row when the host has at least one mic.
    let app = build_app();
    let webview = main_webview(&app);
    let value = invoke(&webview, "list_microphones", InvokeBody::default());
    let arr = value.as_array().expect("array shape");
    let Some(first) = arr.first() else {
        eprintln!("no mics attached — skipping per-row shape assertion");
        return;
    };
    for key in ["id", "label", "is_default", "channels", "sample_rate_hz"] {
        assert!(
            first.get(key).is_some(),
            "MicrophoneView row missing field `{key}` in shape {first:?}"
        );
    }
}

// NOTE: start_mic_capture / stop_mic_capture take `tauri::AppHandle`
// as a runtime-generic parameter that Tauri 2's `generate_handler!`
// macro can't express against `MockRuntime` in the test harness
// (the same constraint applies to start_preview / stop_preview in
// `commands.rs` — they're deliberately omitted there). The state
// machine + handle + pipeline-spawn paths are exercised by the
// pure-Rust unit tests in `screen_app::audio::*::tests` and by
// hand per the M-MIC.1 acceptance criteria. The IPC tests above
// guarantee `list_microphones` + `mic_status` plumbing, which is
// the surface the Leptos picker (M-MIC.2) consumes for everything
// except the actual start/stop click.
