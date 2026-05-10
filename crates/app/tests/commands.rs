//! Tier-1 IPC harness tests — exercise the registered Tauri commands the
//! way the Leptos frontend does, in-process via `tauri::test::mock_builder`.
//!
//! What this catches that direct `PlayerSession` tests don't:
//!
//! - Misregistration in `tauri::generate_handler!` (typo → silently
//!   missing command at runtime, no compile error).
//! - serde shape drift on the wire: e.g. renaming a `PlayerStatus`
//!   field would still compile but break the frontend deserializer.
//! - `State<PlayerSession>` plumbing — forgetting `.manage(...)` would
//!   make every command fail at runtime, not compile-time.
//!
//! Cost: each `mock_builder().build(...)` boots a real wisp Application
//! (~200 ms on Apple Silicon) — same as the `player_session` tests, so
//! we share session boot across cases by building the app once per test.

use decode::gstreamer_pipe::gstreamer_available;
use serde_json::json;
use tauri::WebviewUrl;
use tauri::WebviewWindowBuilder;
use tauri::http::HeaderMap;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets,
};
use tauri::webview::InvokeRequest;

use screen_app::commands;
use screen_app::player_session::{PlayerSession, PlayerStatus, SessionState};

/// Skip-or-run check for tests that hit `player_open`. CI sometimes
/// can't find the apt-installed gstreamer binaries from cargo nextest's
/// test processes; skipping gracefully matches the decode integration
/// test pattern.
macro_rules! gst_required {
    () => {
        if !gstreamer_available() {
            eprintln!(
                "gst-launch-1.0/gst-discoverer-1.0 not on PATH — skipping. \
                 PATH={}",
                std::env::var("PATH").unwrap_or_else(|_| "<unset>".into())
            );
            return;
        }
    };
}

const FIXTURE: &str = "../decode/tests/fixtures/sample.mp4";

fn build_app() -> tauri::App<MockRuntime> {
    mock_builder()
        .manage(PlayerSession::new())
        .invoke_handler(tauri::generate_handler![
            commands::player_open,
            commands::player_play,
            commands::player_pause,
            commands::player_status,
        ])
        .build(mock_context(noop_assets()))
        .expect("build mock app")
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

fn main_webview(app: &tauri::App<MockRuntime>) -> tauri::WebviewWindow<MockRuntime> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .build()
        .expect("build mock webview")
}

#[test]
fn player_status_returns_empty_initially() {
    let app = build_app();
    let webview = main_webview(&app);
    let value = invoke(&webview, "player_status", InvokeBody::default());
    let status: PlayerStatus = serde_json::from_value(value).unwrap();
    assert_eq!(status.state, SessionState::Empty);
    assert_eq!(status.elapsed_ms, 0);
}

#[test]
fn player_open_then_play_then_pause_via_ipc() {
    gst_required!();
    let app = build_app();
    let webview = main_webview(&app);

    // open — note: command takes a `path: String` arg, so the body is
    // `{"path": "..."}` exactly as the JS bridge sends it.
    let value = invoke(
        &webview,
        "player_open",
        InvokeBody::Json(json!({ "path": FIXTURE })),
    );
    let status: PlayerStatus = serde_json::from_value(value).unwrap();
    assert_eq!(status.state, SessionState::Paused);
    assert_eq!(status.width, 480);
    assert_eq!(status.height, 270);

    // play — no return value, just a state mutation.
    let _ = invoke(&webview, "player_play", InvokeBody::default());
    let value = invoke(&webview, "player_status", InvokeBody::default());
    let status: PlayerStatus = serde_json::from_value(value).unwrap();
    assert_eq!(status.state, SessionState::Playing);

    // pause — flips back.
    let _ = invoke(&webview, "player_pause", InvokeBody::default());
    let value = invoke(&webview, "player_status", InvokeBody::default());
    let status: PlayerStatus = serde_json::from_value(value).unwrap();
    assert_eq!(status.state, SessionState::Paused);
}

#[test]
fn serde_shape_uses_lowercase_session_state() {
    // Regression guard for the `#[serde(rename_all = "lowercase")]` on
    // SessionState. The frontend mirror (crates/app-ui/src/player_ipc.rs)
    // expects exactly these strings — if the rename attribute is dropped,
    // the wire format becomes `"Empty"`/`"Paused"`/etc. and the frontend
    // listener silently fails.
    let app = build_app();
    let webview = main_webview(&app);
    let value = invoke(&webview, "player_status", InvokeBody::default());
    let state_str = value
        .get("state")
        .and_then(serde_json::Value::as_str)
        .expect("state should be a JSON string");
    assert_eq!(state_str, "empty", "expected lowercase variant tag");
}

#[test]
fn player_open_with_invalid_path_returns_error() {
    gst_required!();
    let app = build_app();
    let webview = main_webview(&app);

    let url = "tauri://localhost".parse().unwrap();
    let request = InvokeRequest {
        cmd: "player_open".into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url,
        body: InvokeBody::Json(json!({ "path": "nonexistent-xyz.mp4" })),
        headers: HeaderMap::default(),
        invoke_key: INVOKE_KEY.to_string(),
    };
    let result = get_ipc_response(&webview, request);
    assert!(result.is_err(), "expected an error for missing file");
}
