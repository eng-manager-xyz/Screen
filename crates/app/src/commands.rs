//! Tauri `#[command]` wrappers around [`PlayerSession`].
//!
//! Every command is a one-liner — the heavy lifting is in
//! [`super::player_session`]. Splitting them out keeps the IPC surface
//! easy to audit (one file, four functions) and isolates the Tauri
//! framework dep from the testable `PlayerSession`.

#![allow(
    clippy::needless_pass_by_value,
    reason = "tauri::command requires State<T> by value (the macro's signature inspection rejects &State<T>)"
)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{Manager, PhysicalPosition, State};

use crate::audio::{MicCaptureHandle, MicCapturePipeline, MicCaptureState, MicError, MicLifecycle};
use crate::player_session::{PlayerSession, PlayerStatus};
use crate::preview::{
    CameraError, CameraPipeline, CameraPipelineHandle, DiagnosticsSnapshot, PreviewDiagnostics,
    PreviewLifecycle, PreviewState,
};
use crate::recording::{
    RecordingConfig, RecordingSession, RecordingState, RecordingStatusView, RecordingSummary,
    SessionState, SessionStreams, StreamHealth, StreamKind,
};
use crate::recp::bubble_position::{BubblePosition, default_position, is_on_any_monitor};
use crate::recp::settings_deep_link::{SettingsPane, open_command};
use crate::recp::tray_positioning::{MonitorBounds, pick_monitor, position_window_top_right};
#[cfg(target_os = "macos")]
use crate::screen_capture::ScreenCaptureState;
#[cfg(target_os = "macos")]
use crate::system_audio::SystemAudioCaptureState;
use crate::tray::bubble_toggle::{BubbleAction, BubbleVisibility};
use crate::tray::toggle::{Action, TrayPopoverState};

/// Tauri-managed wrapper around the tray-popover toggle state machine
/// (M-TRAY.0 / AUT-249). Held in `tauri::State` so the click handler in
/// `main.rs` and the `tray_toggle_popover` command share one source of
/// truth. `Mutex` rather than `parking_lot::Mutex` to avoid adding a new
/// workspace dep just for the tray; contention is non-existent (only
/// the click handler ever touches it).
#[derive(Default)]
pub struct TrayState(pub Mutex<TrayPopoverState>);

/// Tauri-managed state for the webcam-bubble window (M-BUBBLE.0 /
/// AUT-273 + M-BUBBLE.3 / AUT-276). Tracks both the visibility state
/// machine and the in-memory last-known position so position
/// persistence survives hide/show cycles.
///
/// `last_position = None` means "no remembered position — first-show
/// will compute a sensible default." Once set (either by loading from
/// disk on first show, by `WindowEvent::Moved` mid-session, or by
/// `set_last_position` for tests), the value is the source of truth
/// for the next show.
#[derive(Default)]
pub struct BubbleState {
    visibility: Mutex<BubbleVisibility>,
    last_position: Mutex<Option<BubblePosition>>,
}

impl BubbleState {
    /// Snapshot the current remembered position (or `None` if unset).
    /// Used by the `WindowEvent::Moved` handler in `main.rs` to keep
    /// the in-memory cache fresh during drags.
    #[must_use]
    pub fn last_position(&self) -> Option<BubblePosition> {
        *self
            .last_position
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Replace the remembered position. Cheap (one mutex acquire + an
    /// `i32` pair copy) so safe to call on every `WindowEvent::Moved`.
    pub fn set_last_position(&self, pos: BubblePosition) {
        *self
            .last_position
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(pos);
    }
}

/// Default inset (px) from the monitor edge for the bubble's
/// first-open position. Matches typical macOS-overlay convention.
const BUBBLE_DEFAULT_INSET_PX: i32 = 16;

/// Default bubble dimensions used when the live window can't be
/// queried (shouldn't happen — `tauri.conf.json` declares 200×200 —
/// but defensive so the show path never blocks on a query failure).
const BUBBLE_FALLBACK_W: i32 = 260;
const BUBBLE_FALLBACK_H: i32 = 320;

/// Webcam-bubble toggle command (M-BUBBLE.0 / AUT-273).
///
/// Resolves the bound bubble window by its `tauri.conf.json` label
/// (`webcam-bubble`), advances the state machine, then performs the
/// corresponding `show()`/`hide()` on the window. Returns `()` to
/// match the existing tray-toggle command shape — failures log via
/// `tracing::warn!` so the click is never user-facing silent.
///
/// Persistence (M-BUBBLE.3 / AUT-276): on Show, restore the last
/// remembered position (in-memory or, if first show of the session,
/// loaded from `bubble-position.txt` in the app's config dir). On
/// Hide, snapshot the window's current position into the in-memory
/// state and persist to disk so a future app launch reopens at the
/// same spot.
///
/// Notably does NOT call `set_focus()` on show — the bubble is a
/// peripheral overlay and shouldn't steal focus from the `AppShell`.
#[tauri::command]
pub fn toggle_webcam_bubble(app: tauri::AppHandle, state: State<'_, BubbleState>) {
    let Some(window) = app.get_webview_window("webcam-bubble") else {
        tracing::warn!("webcam-bubble window not found; tauri.conf.json may be missing it");
        return;
    };
    let action = {
        let mut guard = state
            .visibility
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.on_click()
    };
    apply_bubble_action(&app, &state, &window, action);
}

/// Explicit setter for the webcam bubble visibility. ISS-05 — the
/// recorder's `camera_enabled` `RwSignal` defaults to `true` while
/// `BubbleVisibility::default()` is `Hidden`, so the always-flip
/// [`toggle_webcam_bubble`] path was one click out of phase from
/// every page mount. The setter aligns the bubble to the caller's
/// source of truth instead, and no-ops when already in the requested
/// state — safe to spam from a reactive subscription.
#[tauri::command]
pub fn set_webcam_bubble_visibility(
    visible: bool,
    app: tauri::AppHandle,
    state: State<'_, BubbleState>,
) {
    let Some(window) = app.get_webview_window("webcam-bubble") else {
        tracing::warn!("webcam-bubble window not found; tauri.conf.json may be missing it");
        return;
    };
    let action = {
        let mut guard = state
            .visibility
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.set(visible)
    };
    if let Some(action) = action {
        apply_bubble_action(&app, &state, &window, action);
    }
}

/// Execute a [`BubbleAction`] against the bubble window. Shared by
/// the toggle + setter command paths so the position-cache + persist
/// behaviour stays identical regardless of which command was called.
fn apply_bubble_action(
    app: &tauri::AppHandle,
    state: &BubbleState,
    window: &tauri::WebviewWindow,
    action: BubbleAction,
) {
    match action {
        BubbleAction::Show => {
            restore_bubble_position(app, state, window);
            if let Err(err) = window.show() {
                tracing::warn!(?err, "failed to show webcam-bubble window");
            }
        }
        BubbleAction::Hide => {
            snapshot_and_persist_bubble_position(app, state, window);
            if let Err(err) = window.hide() {
                tracing::warn!(?err, "failed to hide webcam-bubble window");
            }
        }
    }
}

/// Look up the bubble window's last-known position (in-memory first;
/// then disk; then `default_position` on the primary monitor) and
/// apply it via `set_position` BEFORE `show()` so the window doesn't
/// flicker through a stale OS-default location.
fn restore_bubble_position(
    app: &tauri::AppHandle,
    state: &BubbleState,
    window: &tauri::WebviewWindow,
) {
    // 1. Hot path: in-memory state set by a previous Hide / Moved.
    if let Some(pos) = state.last_position() {
        apply_position(window, pos);
        return;
    }
    // 2. Cold path: try disk load. If found, hydrate the in-memory
    //    cache so future shows hit the hot path.
    if let Some(pos) = load_bubble_position(app) {
        let monitors = collect_monitor_bounds(app);
        let (w, h) = window_dims(window);
        if is_on_any_monitor(pos, w, h, &monitors) {
            state.set_last_position(pos);
            apply_position(window, pos);
            return;
        }
        tracing::info!(
            ?pos,
            "saved bubble position is off-screen (display unplugged?); falling back to default"
        );
    }
    // 3. Fallback: compute default for the primary monitor.
    if let Some(pos) = compute_default_position(app, window) {
        state.set_last_position(pos);
        apply_position(window, pos);
    }
}

/// Read the window's current outer position, store it in the
/// in-memory state, and persist to disk. Called on Hide so a
/// subsequent show (this session OR a later launch) restores the
/// user's chosen position.
fn snapshot_and_persist_bubble_position(
    app: &tauri::AppHandle,
    state: &BubbleState,
    window: &tauri::WebviewWindow,
) {
    let Ok(physical) = window.outer_position() else {
        tracing::warn!("could not read webcam-bubble position; persistence skipped this cycle");
        return;
    };
    let pos = BubblePosition {
        x: physical.x,
        y: physical.y,
    };
    state.set_last_position(pos);
    if let Err(err) = save_bubble_position(app, pos) {
        tracing::warn!(?err, "failed to persist webcam-bubble position to disk");
    }
}

/// Apply a position to the bubble window using a `PhysicalPosition`
/// (the same coordinate system `outer_position()` returns + the same
/// coordinate system `MonitorBounds` is in, per
/// `crate::recp::tray_positioning`).
fn apply_position(window: &tauri::WebviewWindow, pos: BubblePosition) {
    if let Err(err) = window.set_position(PhysicalPosition::new(pos.x, pos.y)) {
        tracing::warn!(?err, "set_position on webcam-bubble failed");
    }
}

/// Build a `MonitorBounds` vec from `app.available_monitors()`.
/// Empty on failure — callers must handle that case.
fn collect_monitor_bounds(app: &tauri::AppHandle) -> Vec<MonitorBounds> {
    let Ok(monitors) = app.available_monitors() else {
        return Vec::new();
    };
    monitors
        .iter()
        .map(|m| MonitorBounds {
            x: m.position().x,
            y: m.position().y,
            width: i32::try_from(m.size().width).unwrap_or(i32::MAX),
            height: i32::try_from(m.size().height).unwrap_or(i32::MAX),
        })
        .collect()
}

/// Resolve the window's physical inner-size into integer width/height,
/// falling back to the `tauri.conf.json` declared 200×200 if the live
/// query fails.
fn window_dims(window: &tauri::WebviewWindow) -> (i32, i32) {
    window
        .inner_size()
        .map_or((BUBBLE_FALLBACK_W, BUBBLE_FALLBACK_H), |size| {
            (
                i32::try_from(size.width).unwrap_or(BUBBLE_FALLBACK_W),
                i32::try_from(size.height).unwrap_or(BUBBLE_FALLBACK_H),
            )
        })
}

/// First-launch default: bottom-right of the primary monitor with a
/// 16 px inset. Returns `None` only when the OS reports zero monitors
/// — defensive; in practice `available_monitors()` always yields ≥1
/// when a webview is up.
fn compute_default_position(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
) -> Option<BubblePosition> {
    let monitors = collect_monitor_bounds(app);
    let primary = monitors.first().copied()?;
    let (w, h) = window_dims(window);
    Some(default_position(w, h, primary, BUBBLE_DEFAULT_INSET_PX))
}

/// Persisted-position file path: `<app-config-dir>/bubble-position.txt`.
/// The format is `"{x},{y}\n"` — two integers + a comma + a newline.
/// We deliberately avoid `serde_json` (no new workspace dep) and
/// avoid TOML (overkill for two integers); the file is human-readable
/// + trivially repairable + small enough to parse by hand.
fn bubble_position_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    Some(dir.join("bubble-position.txt"))
}

/// Persist `pos` to disk. Creates the app-config dir if it doesn't
/// exist yet (first-ever app launch).
fn save_bubble_position(app: &tauri::AppHandle, pos: BubblePosition) -> std::io::Result<()> {
    let path = bubble_position_path(app).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "app config dir unavailable")
    })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, encode_position(pos))
}

/// Load `BubblePosition` from disk; returns `None` on missing file,
/// I/O error, or malformed contents.
fn load_bubble_position(app: &tauri::AppHandle) -> Option<BubblePosition> {
    let path = bubble_position_path(app)?;
    let raw = std::fs::read_to_string(&path).ok()?;
    decode_position(&raw)
}

/// Persistence file-format version prefix. Bumping this string causes
/// `decode_position` to reject any file written by an earlier version,
/// which falls through to `compute_default_position` and re-applies the
/// current default-corner rule (M-BUBBLE.3 originally shipped
/// bottom-right; the design pass moved the default to bottom-left, and
/// stale `v1` files were keeping the bubble in the old corner).
const BUBBLE_POSITION_FORMAT_VERSION: &str = "v2";

/// Format helper extracted for unit testing.
#[must_use]
fn encode_position(pos: BubblePosition) -> String {
    format!("{}:{},{}\n", BUBBLE_POSITION_FORMAT_VERSION, pos.x, pos.y)
}

/// Parse helper extracted for unit testing. Requires the
/// `BUBBLE_POSITION_FORMAT_VERSION` prefix so old-format files get
/// rejected (returns `None`), letting the caller fall through to
/// `compute_default_position` with the current default-corner rule.
#[must_use]
fn decode_position(raw: &str) -> Option<BubblePosition> {
    let trimmed = raw.trim();
    let body = trimmed.strip_prefix(&format!("{BUBBLE_POSITION_FORMAT_VERSION}:"))?;
    let (x, y) = body.split_once(',')?;
    Some(BubblePosition {
        x: x.trim().parse().ok()?,
        y: y.trim().parse().ok()?,
    })
}

/// Update the bubble window's in-memory position cache. Called from
/// `main.rs`'s `on_window_event` handler whenever the user drags the
/// bubble. Persistence happens on Hide (not on every Moved) to avoid
/// hammering the disk during a drag — per-frame `Moved` events on
/// macOS would otherwise cause thousands of writes per drag.
pub fn update_bubble_position_from_event(state: &BubbleState, physical_x: i32, physical_y: i32) {
    state.set_last_position(BubblePosition {
        x: physical_x,
        y: physical_y,
    });
}

/// Toggle whether the webcam-bubble window passes mouse events
/// through to whatever's underneath (M-BUBBLE.1 v0 / AUT-274).
///
/// When `enabled = true`, the entire bubble window is mouse-event
/// transparent — clicks and hovers reach the window below. Useful
/// when recording the bubble overlaying slides / a browser, so the
/// user can interact with the underlying app without the bubble
/// catching the click. To disable (let the user drag the bubble
/// again), the `AppShell`'s "Click-through bubble" button is the
/// out-of-band trigger; the bubble itself can't receive the click
/// while passthrough is on (chicken-and-egg).
///
/// Implementation is the macOS-blessed
/// `NSWindow.setIgnoresMouseEvents:` path exposed through Tauri 2's
/// `WebviewWindow::set_ignore_cursor_events`. The same call works on
/// Windows (`WS_EX_TRANSPARENT`) and Linux (compositor-dependent).
/// **Whole-window** — clicks on the visible bubble circle ALSO pass
/// through when enabled. Per-pixel hit-testing (only the circle
/// intercepts, corners pass through) needs an `NSView` subclass via
/// `objc2` and is deferred to a v1 follow-up under the same ticket.
#[tauri::command]
pub fn set_bubble_clickthrough(app: tauri::AppHandle, enabled: bool) {
    let Some(window) = app.get_webview_window("webcam-bubble") else {
        tracing::warn!("webcam-bubble window not found; clickthrough toggle no-op");
        return;
    };
    if let Err(err) = window.set_ignore_cursor_events(enabled) {
        tracing::warn!(
            ?err,
            enabled,
            "set_ignore_cursor_events on webcam-bubble failed"
        );
    }
}

/// `true` if `path` looks like the file `save_bubble_position` would
/// produce. Used in the persistence integration test (which writes a
/// canned file and verifies `load_bubble_position` reads it back).
#[doc(hidden)]
#[must_use]
pub fn __debug_is_bubble_position_path(path: &Path) -> bool {
    path.file_name().is_some_and(|n| n == "bubble-position.txt")
}

/// Tray-popover toggle command (M-TRAY.0 / AUT-249).
///
/// Resolves the bound popover window by its `tauri.conf.json` label
/// (`tray-popover`), advances the state machine, then performs the
/// corresponding `show()`/`hide()` on the window. Returning `()` rather
/// than `Result` matches the existing player_* commands' shape — failure
/// paths log via `tracing::warn!` so the click is never user-facing
/// silent.
#[tauri::command]
pub fn tray_toggle_popover(app: tauri::AppHandle, state: State<'_, TrayState>) {
    toggle_tray_popover(&app, &state);
}

/// Pure function variant of [`tray_toggle_popover`] — not a Tauri
/// command. Calls [`toggle_tray_popover_at`] with no click position so
/// the window opens at its previous position (or the OS-default
/// position on first show). Used by the IPC bus and the no-position
/// fallback for synthetic clicks in tests.
pub fn toggle_tray_popover(app: &tauri::AppHandle, state: &TrayState) {
    toggle_tray_popover_at(app, state, None);
}

/// Like [`toggle_tray_popover`] but anchors the popover under
/// `click_position` (M-RECP.1 / AUT-262 wiring). When the state
/// machine resolves to `Action::Show` AND `click_position` is set,
/// we look up the monitor the click happened on, compute the
/// below-click anchor, and `set_position` BEFORE showing the window.
/// Without the explicit `set_position` Tauri restores the last-known
/// position (or the OS default), which is the source of the "popover
/// doesn't follow the tray icon" bug.
pub fn toggle_tray_popover_at(
    app: &tauri::AppHandle,
    state: &TrayState,
    click_position: Option<(i32, i32)>,
) {
    let Some(window) = app.get_webview_window("tray-popover") else {
        tracing::warn!("tray-popover window not found; tauri.conf.json may be missing it");
        return;
    };
    let action = {
        let mut guard = state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.on_click()
    };
    match action {
        Action::Show => {
            if let Some((click_x, click_y)) = click_position {
                anchor_window_to_click(app, &window, click_x, click_y);
            }
            if let Err(err) = window.show() {
                tracing::warn!(?err, "failed to show tray-popover window");
                return;
            }
            if let Err(err) = window.set_focus() {
                tracing::warn!(?err, "failed to focus tray-popover window");
            }
            // Debug builds: surface the webview console so we can
            // diagnose blank-page / wasm-panic regressions without a
            // bundled-app context-menu DevTools toggle.
            #[cfg(debug_assertions)]
            window.open_devtools();
        }
        Action::Hide => {
            if let Err(err) = window.hide() {
                tracing::warn!(?err, "failed to hide tray-popover window");
            }
        }
    }
}

/// Pick the right monitor for `(click_x, click_y)` and place the
/// `tray-popover` window's top-left below the click. Logs and bails
/// out without setting a position when monitors can't be queried —
/// the window will still `show()` at its last-known position so the
/// user doesn't lose access to the recorder.
fn anchor_window_to_click(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    click_x: i32,
    click_y: i32,
) {
    let monitors = match app.available_monitors() {
        Ok(list) => list,
        Err(err) => {
            tracing::warn!(
                ?err,
                "available_monitors failed; popover stays at last position"
            );
            return;
        }
    };
    let bounds: Vec<MonitorBounds> = monitors
        .iter()
        .map(|m| MonitorBounds {
            x: m.position().x,
            y: m.position().y,
            width: i32::try_from(m.size().width).unwrap_or(i32::MAX),
            height: i32::try_from(m.size().height).unwrap_or(i32::MAX),
        })
        .collect();
    // `inner_size()` gives the size of the webview content rect. We
    // only need the width — the top-right anchor is independent of
    // window height. Falling through on lookup failure uses the
    // conf-declared width as a crude fallback so we still get a
    // sensible anchor.
    let window_w = window
        .inner_size()
        .map_or(1200, |size| i32::try_from(size.width).unwrap_or(1200));
    let Some((target_x, target_y)) = compute_popover_anchor(click_x, click_y, window_w, &bounds)
    else {
        tracing::warn!("no monitors reported; popover stays at last position");
        return;
    };
    // Monitor bounds, window inner_size, and the tray click position
    // are all in PHYSICAL pixels (`PhysicalPosition` / `PhysicalSize`
    // from Tauri 2). The previous `LogicalPosition::new` here applied
    // the value as logical pixels, so on a 2× Retina display the
    // popover landed at twice the intended position and the right
    // edge fell off the screen whenever the user clicked the tray
    // icon near the menubar's right side. Match the coordinate space
    // the geometry was computed in — same fix the bubble window's
    // `apply_position` already uses.
    if let Err(err) = window.set_position(PhysicalPosition::new(target_x, target_y)) {
        tracing::warn!(?err, "set_position on tray-popover failed");
    }
}

/// Pure compute step shared by [`anchor_window_to_click`] (runtime)
/// and the unit tests (no Tauri). Returns the popover's target
/// top-left position (anchored top-right of the picked monitor) in
/// screen coordinates, or `None` if the monitor list is empty.
///
/// Splitting this out exists so the click → monitor-pick →
/// top-right-anchor pipeline is verifiable without spinning up a
/// Tauri mock app — see the unit tests below.
fn compute_popover_anchor(
    click_x: i32,
    click_y: i32,
    window_w: i32,
    monitors: &[MonitorBounds],
) -> Option<(i32, i32)> {
    let monitor = pick_monitor(click_x, click_y, monitors)?;
    Some(position_window_top_right(window_w, monitor))
}

/// Open a video file and start it paused at frame 0.
#[tauri::command]
pub fn player_open(state: State<'_, PlayerSession>, path: String) -> Result<PlayerStatus, String> {
    state.open(&PathBuf::from(path))
}

/// Resume playback. No-op when nothing is loaded.
#[tauri::command]
pub fn player_play(state: State<'_, PlayerSession>) {
    state.play();
}

/// Pause playback. No-op when nothing is loaded.
#[tauri::command]
pub fn player_pause(state: State<'_, PlayerSession>) {
    state.pause();
}

/// Snapshot the current status. The shell normally subscribes to the
/// pushed `player-status` events instead of polling, but this command
/// is useful on initial mount to seed the UI before the first event.
#[tauri::command]
#[must_use]
pub fn player_status(state: State<'_, PlayerSession>) -> PlayerStatus {
    state.status()
}

/// View-model shape for the camera-list IPC command (M-CAM.2 /
/// AUT-256). Mirrors `media::CameraDevice` but lives in
/// `crates/app/` so the IPC schema is owned by the shell crate
/// rather than the media crate.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraView {
    /// Stable device id.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// First in the enumeration order.
    pub is_default: bool,
}

impl From<media::CameraDevice> for CameraView {
    fn from(value: media::CameraDevice) -> Self {
        Self {
            id: value.id,
            label: value.label,
            is_default: value.is_default,
        }
    }
}

/// Camera permission probe (M-CAM.2 / AUT-256). Stub-returns
/// `Granted` on every platform today; full macOS implementation via
/// `AVCaptureDevice.authorizationStatus(for:)` is M-RECP.0 territory.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum CameraPermission {
    /// User has granted camera access.
    Granted,
    /// macOS has not yet asked the user — the next capture call
    /// will trigger the OS-level prompt.
    NotDetermined,
    /// User has explicitly denied access.
    Denied,
}

/// Enumerate attached cameras (M-CAM.2 / AUT-256).
///
/// Wraps `media::list_cameras()` and converts each `CameraDevice`
/// to a `CameraView`. Returns an empty `Vec` (not an error) if
/// `gst-device-monitor-1.0` isn't on `PATH` or no cameras are
/// attached — Leptos consumers should runtime-skip in that case.
#[tauri::command]
#[must_use]
pub fn list_cameras() -> Vec<CameraView> {
    media::list_cameras()
        .into_iter()
        .map(CameraView::from)
        .collect()
}

/// Probe the OS for camera permission (M-CAM.2 / AUT-256 +
/// M-RECP.7 / AUT-285).
///
/// macOS: real `AVCaptureDevice.authorizationStatusForMediaType:`
/// call via `objc2-av-foundation`. Returns `Granted` /
/// `NotDetermined` / `Denied` per the live TCC state.
///
/// Non-macOS: returns `Granted` (no TCC-equivalent the recorder
/// needs to probe for camera on Linux / Windows).
#[tauri::command]
#[must_use]
pub fn camera_permission_status() -> CameraPermission {
    #[cfg(target_os = "macos")]
    {
        av_authorization_status(AvMediaTypeKind::Video)
    }
    #[cfg(not(target_os = "macos"))]
    {
        CameraPermission::Granted
    }
}

// ---------------------------------------------------------------
// Settings deep-link commands (M-RECP.0 / AUT-261 — camera,
// M-RECP.6 / AUT-272 — screen recording, M-RECP.8 / AUT-286 — mic)
//
// Each wraps `settings_deep_link::open_command(pane)` and shells
// out via `std::process::Command`. Returns the underlying spawn
// error as a string so the Leptos picker can render it inline.
// macOS + Windows return real URLs; Linux is a no-op (the desktop
// environment determines the right command — no universal handle).
// ---------------------------------------------------------------

/// Shell out to open System Settings → Privacy & Security → Camera.
/// Falls back to a no-op on Linux (no universal Settings deep-link).
///
/// # Errors
///
/// Returns the OS spawn error as a string if `Command::spawn` fails
/// (e.g. `open` not on PATH on macOS — should never happen).
#[tauri::command]
pub fn open_settings_camera() -> Result<(), String> {
    open_settings_pane(SettingsPane::Camera)
}

/// Shell out to open System Settings → Privacy & Security →
/// Microphone. Linux no-op.
///
/// # Errors
///
/// Returns the OS spawn error as a string.
#[tauri::command]
pub fn open_settings_microphone() -> Result<(), String> {
    open_settings_pane(SettingsPane::Microphone)
}

/// Shell out to open System Settings → Privacy & Security →
/// Screen Recording. macOS only — Windows + Linux return a no-op
/// `Ok(())` because neither has a system-level Screen Recording
/// pane the recorder can deep-link to.
///
/// # Errors
///
/// Returns the OS spawn error as a string.
#[tauri::command]
pub fn open_settings_screen_recording() -> Result<(), String> {
    open_settings_pane(SettingsPane::ScreenRecording)
}

/// Shared shell-out helper. Resolves the OS-specific argv from
/// [`open_command`] and spawns it. Returns `Ok(())` even when no
/// deep-link is known for the pane on this OS (Linux, or Screen
/// Recording on Windows) — the caller treats "no error" as
/// "instruction displayed."
fn open_settings_pane(pane: SettingsPane) -> Result<(), String> {
    let Some(command_parts) = open_command(pane) else {
        tracing::info!(
            ?pane,
            "open_settings_pane: no deep-link known for this OS — no-op"
        );
        return Ok(());
    };
    let Some((program, rest)) = command_parts.split_first() else {
        return Err("open_command returned empty command".into());
    };
    std::process::Command::new(program)
        .args(rest)
        .spawn()
        .map_err(|err| format!("failed to open settings pane {pane:?}: {err}"))?;
    tracing::info!(?pane, ?command_parts, "open_settings_pane: spawned");
    Ok(())
}

/// Start the camera preview pipeline (M-CAM.2 / AUT-256).
///
/// Today: pure state-machine transition. M-CAM.3 (AUT-257) fills in
/// the actual gst → wisp → readback → frame-channel pipeline behind
/// this transition.
///
/// # Errors
///
/// Returns [`CameraError`] when the state machine refuses (already
/// running) or — once M-CAM.3 lands — when the gst pipeline fails
/// to produce frames.
#[tauri::command]
pub fn start_preview(
    app: tauri::AppHandle,
    state: State<'_, PreviewState>,
    pipeline_state: State<'_, CameraPipelineHandle>,
    camera_id: String,
) -> Result<(), CameraError> {
    // Advance lifecycle Idle → Starting. Re-entrant calls (already
    // Starting / Running / Stopping) are no-ops so the caller can
    // safely double-invoke.
    {
        let mut guard = state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let new_state = guard.try_start();
        if new_state == *guard {
            return Ok(());
        }
        *guard = new_state;
    }
    tracing::info!(
        camera_id = %camera_id,
        "preview Starting — spawning gst worker pinned to picked camera (M-CAM.4)"
    );
    // Spawn the M-CAM.3 worker, now M-CAM.4-routed: the camera_id
    // string is resolved to its OS-native gst source element inside
    // the worker via `media::camera::find_by_id`. The worker advances
    // Starting → Running on first frame; on gst spawn failure (or
    // CameraNotFound) it logs + resets the lifecycle to Idle so the
    // UI shows a recovery state.
    let pipeline = CameraPipeline::spawn(app, camera_id)?;
    pipeline_state.install(pipeline);
    Ok(())
}

/// Stop the camera preview pipeline (M-CAM.2 / AUT-256 + M-CAM.3 /
/// AUT-257).
///
/// Drops the [`CameraPipeline`] worker — which cancels the loop,
/// joins the thread, and (via `gstreamer_video::VideoStream`'s own
/// `Drop`) kills the gst-launch child. The worker thread itself
/// resets the lifecycle to `Idle` on its way out, but we also do it
/// here as a belt-and-braces guard in case the worker already exited
/// (gst failure path).
#[tauri::command]
pub fn stop_preview(
    state: State<'_, PreviewState>,
    pipeline_state: State<'_, CameraPipelineHandle>,
) {
    {
        let mut guard = state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = guard.try_stop();
    }
    // Drop the pipeline — Drop impl cancels + joins. This blocks
    // briefly (one gst frame interval, ~33ms at 30fps); acceptable
    // for a user-initiated stop.
    pipeline_state.shutdown();
    {
        let mut guard = state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = guard.finish_stop();
    }
    tracing::info!("preview Stopped");
}

/// Snapshot the current preview lifecycle (M-CAM.2 / AUT-256).
///
/// Useful for Leptos to seed its `RecorderPreviewState` enum on
/// first mount before the pushed frame events drive it.
#[tauri::command]
#[must_use]
pub fn preview_status(state: State<'_, PreviewState>) -> PreviewLifecycle {
    *state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Snapshot the camera-pipeline diagnostics (M-CAM.3 / AUT-257
/// diagnostic addition).
///
/// Returns total frames received, source dims, source fps × 100,
/// and the absolute path of the first-frame PNG dump (if one was
/// written this session). Leptos polls this every 500ms while the
/// Recorder surface is open to render a small overlay showing the
/// pipeline is alive — see the `<CameraDiagnostics />` component in
/// `crates/app-ui/src/camera_diagnostics.rs`.
///
/// Wait-free on the hot path (atomic loads only, no mutex on the
/// counters); the dump-path read takes a `Mutex<Option<PathBuf>>`
/// briefly but only when the snapshot is requested.
#[tauri::command]
#[must_use]
pub fn preview_diagnostics(state: State<'_, PreviewDiagnostics>) -> DiagnosticsSnapshot {
    state.snapshot()
}

// ---------------------------------------------------------------
// Microphone IPC surface (M-MIC.1 / AUT-278)
// ---------------------------------------------------------------

/// View-model shape for the microphone-list IPC command (M-MIC.1 /
/// AUT-278). Mirrors [`media::MicrophoneDevice`] but lives in
/// `crates/app/` so the IPC schema is owned by the shell crate.
/// Same shape contract as [`CameraView`] keeps the Leptos-side
/// picker code symmetrical between camera + mic.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MicrophoneView {
    /// Stable device id (`mic-…`).
    pub id: String,
    /// Human-readable label (`"MacBook Pro Microphone"` etc.).
    pub label: String,
    /// `true` for the OS-default mic.
    pub is_default: bool,
    /// Native channel count from the gst caps line (1 = mono, 2 =
    /// stereo). `0` means unknown — Leptos should default to 2.
    pub channels: u8,
    /// Native sample rate (typically 48000 / 44100). `0` means
    /// unknown — Leptos should default to 48000.
    pub sample_rate_hz: u32,
    /// Platform-native device identifier (M-MIC.3 / AUT-284).
    /// Round-tripped back through `start_mic_capture` so the worker
    /// can route it into the per-OS gst element (`osxaudiosrc
    /// device-uid=…` etc.). Empty when the underlying gst plugin
    /// didn't expose `unique-id` for this device — the worker
    /// falls back to `autoaudiosrc` in that case.
    pub native_id: String,
}

impl From<media::MicrophoneDevice> for MicrophoneView {
    fn from(value: media::MicrophoneDevice) -> Self {
        Self {
            id: value.id,
            label: value.label,
            is_default: value.is_default,
            channels: value.channels,
            sample_rate_hz: value.sample_rate_hz,
            native_id: value.native_id,
        }
    }
}

/// Enumerate attached microphones (M-MIC.1 / AUT-278).
///
/// Wraps [`media::list_microphones`]. Empty `Vec` (not an error)
/// when `gst-device-monitor-1.0` isn't on `PATH` or no mics are
/// attached — Leptos consumers should runtime-skip in that case.
#[tauri::command]
#[must_use]
pub fn list_microphones() -> Vec<MicrophoneView> {
    media::list_microphones()
        .into_iter()
        .map(MicrophoneView::from)
        .collect()
}

/// Probe the OS for microphone permission (M-MIC.2 / AUT-279 +
/// M-RECP.7 / AUT-285).
///
/// macOS: real `AVCaptureDevice.authorizationStatusForMediaType:`
/// call via `objc2-av-foundation`. Returns `Granted` /
/// `NotDetermined` / `Denied` per the live TCC state.
///
/// Non-macOS: returns `Granted`.
///
/// Reuses [`CameraPermission`] rather than introducing a separate
/// `MicrophonePermission` enum — the three states are
/// structurally identical and the picker components key off the
/// variant tags, not the type name.
#[tauri::command]
#[must_use]
pub fn microphone_permission_status() -> CameraPermission {
    #[cfg(target_os = "macos")]
    {
        av_authorization_status(AvMediaTypeKind::Audio)
    }
    #[cfg(not(target_os = "macos"))]
    {
        CameraPermission::Granted
    }
}

/// Discriminator for [`av_authorization_status`] — avoids leaking
/// `AVMediaType` (a Foundation type) into non-macOS callers.
#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug)]
enum AvMediaTypeKind {
    Video,
    Audio,
}

/// Shared macOS-only probe. Maps `AVAuthorizationStatus` to the
/// recorder's three-state [`CameraPermission`] enum. `Restricted`
/// (enterprise-managed) collapses into `Denied` since the user
/// can't grant it themselves. Future-proof: unknown variants fail
/// open as `Granted` to avoid bricking the picker on a future macOS
/// release.
#[cfg(target_os = "macos")]
#[allow(
    unsafe_code,
    reason = "AVFoundation FFI interop — every unsafe block has a SAFETY comment above it justifying soundness."
)]
fn av_authorization_status(kind: AvMediaTypeKind) -> CameraPermission {
    use objc2_av_foundation::{
        AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio, AVMediaTypeVideo,
    };
    // SAFETY: the `AVMediaType*` statics are Objective-C externals
    // marked `Option<&'static AVMediaType>`. They're populated by
    // AVFoundation's framework init, which runs before any Rust
    // code in a macOS process. Both should always be Some on a
    // healthy system — `.expect` documents the invariant.
    let media_type = match kind {
        AvMediaTypeKind::Video => unsafe { AVMediaTypeVideo }.expect("AVMediaTypeVideo present"),
        AvMediaTypeKind::Audio => unsafe { AVMediaTypeAudio }.expect("AVMediaTypeAudio present"),
    };
    // SAFETY: `authorizationStatusForMediaType:` is a class method
    // (no instance state) and documented thread-safe. The only
    // failure mode is being passed a media-type other than Video /
    // Audio, which throws an NSInvalidArgumentException — we only
    // ever pass those two constants above.
    let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) };
    match status {
        AVAuthorizationStatus::Authorized => CameraPermission::Granted,
        AVAuthorizationStatus::NotDetermined => CameraPermission::NotDetermined,
        AVAuthorizationStatus::Denied | AVAuthorizationStatus::Restricted => {
            CameraPermission::Denied
        }
        _ => {
            // Future variant — fail-open so the picker stays usable.
            // Logged so a future macOS release surprise is diagnosable.
            tracing::warn!(
                ?kind,
                ?status,
                "av_authorization_status: unknown variant; defaulting to Granted"
            );
            CameraPermission::Granted
        }
    }
}

/// Proactively request macOS TCC permissions for all four protected
/// resources (M-PIX.9 of M-RECORD-EXPORT-REAL-PIXELS). Fires the
/// OS-level prompts that register `com.screen.app` in the TCC
/// database — without this, pickers enumerate empty on first launch
/// because no entry exists yet.
///
/// Returns the status of each resource after the user responds
/// (`Authorized` / `Denied` / `NotDetermined`). Blocks for up to
/// ~30 seconds while the user clicks; returns the current status
/// if the user dismisses without choosing.
///
/// Order matters: camera first (sync, quickest to dismiss), then
/// microphone, then screen-recording via SCK (which fires its own
/// prompt the first time `SCShareableContent.current` is called).
#[tauri::command]
#[allow(
    clippy::unused_async,
    reason = "Tauri commands must be async to keep a uniform signature across platforms; the macOS branch awaits spawn_blocking, the stub branch returns synchronously."
)]
pub async fn request_all_permissions() -> RequestPermissionsResult {
    #[cfg(target_os = "macos")]
    {
        // All three prompts run on a Tauri-provided worker thread
        // (each blocks for up to 30 s waiting on user input). Doing
        // them sequentially is fine — user can only click one
        // dialog at a time.
        tauri::async_runtime::spawn_blocking(|| {
            let camera = request_av_access_blocking(AvMediaTypeKind::Video);
            let microphone = request_av_access_blocking(AvMediaTypeKind::Audio);
            let screen_recording = request_screen_recording_access_blocking();
            RequestPermissionsResult {
                camera,
                microphone,
                screen_recording,
            }
        })
        .await
        .unwrap_or(RequestPermissionsResult {
            camera: CameraPermission::NotDetermined,
            microphone: CameraPermission::NotDetermined,
            screen_recording: CameraPermission::NotDetermined,
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        RequestPermissionsResult {
            camera: CameraPermission::Granted,
            microphone: CameraPermission::Granted,
            screen_recording: CameraPermission::Granted,
        }
    }
}

/// IPC view for the M-PIX.9 batch-request result. Each field is the
/// post-prompt status the OS reported.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RequestPermissionsResult {
    /// Camera access status after the prompt.
    pub camera: CameraPermission,
    /// Microphone access status after the prompt.
    pub microphone: CameraPermission,
    /// Screen Recording access status after the prompt.
    pub screen_recording: CameraPermission,
}

/// Wrapper around `AVCaptureDevice.requestAccessForMediaType:
/// completionHandler:`. Triggers the macOS prompt (registers the
/// bundle id in TCC), waits up to 30 seconds for the user's
/// response, returns the final status.
///
/// Blocks the calling thread on the channel `recv_timeout` — called
/// from inside `tauri::async_runtime::spawn_blocking` in the
/// `request_all_permissions` outer command so the Tauri runtime
/// stays unblocked.
#[cfg(target_os = "macos")]
#[allow(
    unsafe_code,
    reason = "AVFoundation FFI interop — every unsafe block has a SAFETY justification."
)]
fn request_av_access_blocking(kind: AvMediaTypeKind) -> CameraPermission {
    use objc2_av_foundation::{AVCaptureDevice, AVMediaTypeAudio, AVMediaTypeVideo};
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::mpsc::channel;

    // SAFETY: framework-init populated externals — see
    // av_authorization_status for the matching SAFETY comment.
    let media_type = match kind {
        AvMediaTypeKind::Video => unsafe { AVMediaTypeVideo }.expect("AVMediaTypeVideo present"),
        AvMediaTypeKind::Audio => unsafe { AVMediaTypeAudio }.expect("AVMediaTypeAudio present"),
    };

    let (tx, rx) = channel::<bool>();
    let tx_arc: Arc<Mutex<Option<std::sync::mpsc::Sender<bool>>>> = Arc::new(Mutex::new(Some(tx)));
    let tx_for_block = Arc::clone(&tx_arc);
    let block = block2::RcBlock::new(move |granted: objc2::runtime::Bool| {
        if let Some(sender) = tx_for_block
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        {
            let _ = sender.send(granted.as_bool());
        }
    });
    // SAFETY: requestAccess is documented + the completion block
    // signature matches `(BOOL) -> void`.
    unsafe {
        AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &block);
    }
    let _ = rx.recv_timeout(std::time::Duration::from_secs(30));
    av_authorization_status(kind)
}

/// Query the platform Screen Recording grant without touching SCK.
///
/// This uses CoreGraphics' screen-capture TCC preflight API, which is
/// the cheap permission check Apple exposes for this privacy class.
#[tauri::command]
#[must_use]
pub fn screen_recording_permission_status() -> CameraPermission {
    #[cfg(target_os = "macos")]
    {
        screen_recording_permission_status_macos()
    }
    #[cfg(not(target_os = "macos"))]
    {
        CameraPermission::Granted
    }
}

/// Proactively trigger the Screen Recording TCC request.
///
/// This is intentionally separate from `screen_recording_permission_status`:
/// status is a side-effect-free preflight, while this function is what
/// causes macOS to show the Screen & System Audio Recording consent sheet
/// and add the current app identity to the Settings list.
#[tauri::command]
#[allow(
    clippy::unused_async,
    reason = "Tauri commands must be async to keep a uniform signature across platforms; the macOS branch awaits spawn_blocking, the stub branch returns synchronously."
)]
pub async fn request_screen_recording_permission() -> CameraPermission {
    #[cfg(target_os = "macos")]
    {
        tauri::async_runtime::spawn_blocking(request_screen_recording_access_blocking)
            .await
            .unwrap_or(CameraPermission::NotDetermined)
    }
    #[cfg(not(target_os = "macos"))]
    {
        CameraPermission::Granted
    }
}

/// Trigger the Screen Recording TCC flow with CoreGraphics rather than
/// using `SCShareableContent` as an accidental permission probe.
///
/// SCK enumeration can fail for reasons other than missing TCC. Keeping
/// the permission request on `CGRequestScreenCaptureAccess` lets the UI
/// distinguish "permission not active for this app identity" from "SCK
/// source enumeration failed after permission was granted".
#[cfg(target_os = "macos")]
fn request_screen_recording_access_blocking() -> CameraPermission {
    use objc2_core_graphics::{CGPreflightScreenCaptureAccess, CGRequestScreenCaptureAccess};

    if CGPreflightScreenCaptureAccess() {
        return CameraPermission::Granted;
    }
    let _ = CGRequestScreenCaptureAccess();
    screen_recording_permission_status_macos()
}

#[cfg(target_os = "macos")]
fn screen_recording_permission_status_macos() -> CameraPermission {
    use objc2_core_graphics::CGPreflightScreenCaptureAccess;

    if CGPreflightScreenCaptureAccess() {
        CameraPermission::Granted
    } else {
        CameraPermission::Denied
    }
}

#[cfg(target_os = "macos")]
fn ensure_screen_recording_access() -> Result<(), String> {
    match screen_recording_permission_status_macos() {
        CameraPermission::Granted => Ok(()),
        CameraPermission::Denied | CameraPermission::NotDetermined => {
            let requested = request_screen_recording_access_blocking();
            if matches!(requested, CameraPermission::Granted) {
                return Ok(());
            }
            Err(
                "Screen Recording permission is not active for this app identity. Enable screen-app.app in System Settings → Privacy & Security → Screen & System Audio Recording, then quit and reopen the app without rebuilding. If this persists on macOS 15+, rebuild with SCREEN_CODESIGN_IDENTITY set to an Apple Development or Developer ID signing identity; ad-hoc signatures cannot reliably satisfy ScreenCapture TCC.".into(),
            )
        }
    }
}

/// Start the microphone capture worker (M-MIC.1 / AUT-278).
///
/// Advances [`MicLifecycle`] Idle → Starting and spawns a
/// [`MicCapturePipeline`]. Re-entrant calls while a session is
/// running cleanly tear down the previous worker (the handle's
/// `install` swap drops the old `Pipeline`, which drops the gst
/// child) before starting the new one.
///
/// # Errors
///
/// Returns [`MicError::GstFailed`] when the worker thread can't
/// spawn (effectively never happens). gst-side failures (no mic
/// attached, permission denied, etc.) are reported via the
/// `mic_status` snapshot returning to `Idle` after the worker
/// thread's error path runs.
#[tauri::command]
pub fn start_mic_capture(
    app: tauri::AppHandle,
    state: State<'_, MicCaptureState>,
    pipeline_state: State<'_, MicCaptureHandle>,
    mic_id: String,
) -> Result<(), MicError> {
    // M-MIC.3 / AUT-284 — resolve the FNV-1a mic_id to the
    // platform-native device identifier (osxaudiosrc device-uid /
    // pulsesrc device / wasapisrc device) by re-enumerating.
    //
    // Three cases:
    // 1. Empty mic_id → caller wants OS default. Pass empty native_id
    //    through; `from_microphone` routes to `autoaudiosrc`.
    // 2. Non-empty mic_id present in the live enumeration → use its
    //    native_id (which may itself be empty if the device didn't
    //    expose `unique-id`; that's a legit fall to autoaudiosrc and
    //    we log it).
    // 3. Non-empty mic_id NOT present in the live enumeration →
    //    stale picker state. Return Err(NotFound) so the UI
    //    re-enumerates instead of silently recording the wrong mic.
    //    M-RECORD-EXPORT tightening — was silently falling through.
    let native_id = if mic_id.is_empty() {
        String::new()
    } else if let Some(device) = media::microphone::find_by_id(&mic_id) {
        if device.native_id.is_empty() {
            tracing::warn!(
                mic_id = %mic_id,
                label = %device.label,
                "start_mic_capture: device enumerated but exposed no `unique-id`; \
                 falling back to autoaudiosrc (OS default) — picker selection will NOT pin"
            );
        }
        device.native_id
    } else {
        tracing::warn!(
            mic_id = %mic_id,
            "start_mic_capture: mic_id not present in live enumeration (stale picker?)"
        );
        return Err(MicError::NotFound(mic_id));
    };

    // Re-entrant calls: if a session is already up, tear it down
    // first so the new mic-id wins. Mirrors the M-CAM.2/.3
    // start_preview re-entrance contract — except the camera path
    // doesn't yet handle re-entrance (its docs say "the caller is
    // expected to first stop the existing session"). Here we do the
    // teardown ourselves so the picker UX (M-MIC.2) doesn't need to
    // sequence stop_mic_capture + start_mic_capture for every swap.
    let was_active = pipeline_state.is_active();
    if was_active {
        tracing::info!(
            mic_id = %mic_id,
            "start_mic_capture: tearing down previous session for re-entrant start"
        );
        pipeline_state.shutdown();
        // Force the lifecycle through Stopping → Idle so the
        // try_start below sees Idle.
        let mut guard = state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = guard.try_stop().finish_stop();
    }

    {
        let mut guard = state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let new_state = guard.try_start();
        if new_state == *guard {
            return Ok(());
        }
        *guard = new_state;
    }
    tracing::info!(
        mic_id = %mic_id,
        native_id = %native_id,
        "mic-capture Starting — spawning gst worker (preview, mixer-detached)"
    );
    // Preview path: `mixer = None`. The worker computes RMS for the
    // level meter but does NOT forward samples to the shared
    // AudioMixer — otherwise preview audio would accumulate during
    // device picking and contaminate the next recording.
    let pipeline = MicCapturePipeline::spawn(app, mic_id, native_id, None)?;
    pipeline_state.install(pipeline);
    Ok(())
}

/// Stop the microphone capture worker (M-MIC.1 / AUT-278).
///
/// Drops the [`MicCapturePipeline`] (which cancels the loop, joins
/// the thread, and — via `GstreamerAudioCapture`'s own `Drop` —
/// kills + reaps the gst-launch child). The worker thread resets
/// the lifecycle to `Idle` on its way out; we also do it here as a
/// belt-and-braces guard in case the worker had already exited via
/// a gst failure path.
#[tauri::command]
pub fn stop_mic_capture(
    state: State<'_, MicCaptureState>,
    pipeline_state: State<'_, MicCaptureHandle>,
) {
    {
        let mut guard = state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = guard.try_stop();
    }
    // Drop the pipeline — Drop cancels + joins. Blocks briefly
    // (one chunk interval, ~100 ms at our 4800-frame chunks);
    // acceptable for a user-initiated stop.
    pipeline_state.shutdown();
    {
        let mut guard = state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = guard.finish_stop();
    }
    tracing::info!("mic-capture Stopped");
}

/// Snapshot the current mic-capture lifecycle (M-MIC.1 / AUT-278).
///
/// Useful for Leptos to seed UI state on first mount before the
/// (future) push-event mic-level stream drives it.
#[tauri::command]
#[must_use]
pub fn mic_status(state: State<'_, MicCaptureState>) -> MicLifecycle {
    *state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

// ---------------------------------------------------------------
// System-audio IPC surface (M-AUDIO-SYS.2 / AUT-282)
// ---------------------------------------------------------------

/// View-model for the per-app picker (M-AUDIO-SYS.2 / AUT-282).
/// Mirrors [`media::sck_audio::AudioApp`] but lives on the shell
/// crate so the IPC schema is owned here, not in `media`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioAppView {
    /// Process identifier observed at enumeration time. The Leptos
    /// side persists `bundle_id` for cross-restart durability, not
    /// `pid`.
    pub pid: u32,
    /// Bundle identifier (e.g. `"com.spotify.client"`).
    pub bundle_id: String,
    /// Human-readable display name (`"Spotify"`).
    pub display_name: String,
    /// 32×32 PNG icon bytes. Empty in v0; populated in M-AUDIO-SYS.1.1.
    pub icon_png_bytes: Vec<u8>,
}

#[cfg(target_os = "macos")]
impl From<media::sck_audio::AudioApp> for AudioAppView {
    fn from(value: media::sck_audio::AudioApp) -> Self {
        Self {
            pid: value.pid,
            bundle_id: value.bundle_id,
            display_name: value.display_name,
            icon_png_bytes: value.icon_png_bytes,
        }
    }
}

/// IPC-facing view of `media::sck_audio::AudioAppFilter`. Matches
/// the underlying enum 1-to-1 but lives in the shell crate so the
/// serde shape is owned here.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AudioAppFilterView {
    /// Capture every app's audio.
    AllAudio,
    /// Capture audio from only these apps (by bundle id).
    OnlyApps(Vec<String>),
    /// Capture audio from every app except these.
    ExcludeApps(Vec<String>),
}

#[cfg(target_os = "macos")]
impl From<AudioAppFilterView> for media::sck_audio::AudioAppFilter {
    fn from(value: AudioAppFilterView) -> Self {
        match value {
            AudioAppFilterView::AllAudio => Self::AllAudio,
            AudioAppFilterView::OnlyApps(ids) => Self::OnlyApps(ids),
            AudioAppFilterView::ExcludeApps(ids) => Self::ExcludeApps(ids),
        }
    }
}

/// Enumerate every running app SCK can see (M-AUDIO-SYS.2 / AUT-282).
///
/// Returns an empty Vec on non-macOS targets (system audio is
/// macOS-only); the Leptos picker treats empty as "no apps available"
/// and renders the empty state.
///
/// # Errors
///
/// Returns the underlying SCK error (TCC permission denied,
/// enumeration failed, etc.) as a string so the Leptos picker
/// can show it inline.
#[tauri::command]
pub fn list_audio_apps() -> Result<Vec<AudioAppView>, String> {
    #[cfg(target_os = "macos")]
    {
        ensure_screen_recording_access()?;
        media::sck_audio::list_audio_apps()
            .map(|apps| apps.into_iter().map(AudioAppView::from).collect())
            .map_err(|err| err.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(Vec::new())
    }
}

/// Start the system-audio capture session (M-AUDIO-SYS.2 / AUT-282).
///
/// Triggers the macOS Screen Recording permission prompt on first
/// run. On subsequent runs the session starts cleanly.
///
/// # Errors
///
/// Returns the underlying SCK error message as a string.
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn start_system_audio_capture(
    app: tauri::AppHandle,
    state: State<'_, SystemAudioCaptureState>,
) -> Result<(), String> {
    ensure_screen_recording_access()?;
    state
        .start(&app, media::sck_audio::SystemAudioConfig::default())
        .map_err(|err| err.to_string())
}

/// Non-macOS stub for `start_system_audio_capture`. Returns a
/// "not supported" error so the Leptos picker can show the user
/// they're on the wrong platform.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn start_system_audio_capture() -> Result<(), String> {
    Err("system audio capture requires macOS 13.0+".into())
}

/// Stop the active system-audio session, if any (M-AUDIO-SYS.2).
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn stop_system_audio_capture(state: State<'_, SystemAudioCaptureState>) {
    state.stop();
}

/// Non-macOS stub for `stop_system_audio_capture`. No-op since no
/// session can have been started on this platform.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn stop_system_audio_capture() {}

/// Apply a per-app filter to the active system-audio session
/// (M-AUDIO-SYS.2 / AUT-282).
///
/// The picker should call `start_system_audio_capture` first; if no
/// session is active this command returns an error.
///
/// # Errors
///
/// Returns the underlying SCK error message as a string.
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn set_system_audio_filter(
    state: State<'_, SystemAudioCaptureState>,
    filter: AudioAppFilterView,
) -> Result<(), String> {
    let internal: media::sck_audio::AudioAppFilter = filter.into();
    state.set_filter(&internal).map_err(|err| err.to_string())
}

/// Non-macOS stub for `set_system_audio_filter`. Returns the same
/// "not supported" error as the start command so the Leptos picker
/// can surface a consistent message on every platform.
///
/// # Errors
///
/// Always returns `"system audio capture requires macOS 13.0+"`.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn set_system_audio_filter(_filter: AudioAppFilterView) -> Result<(), String> {
    Err("system audio capture requires macOS 13.0+".into())
}

/// Whether a system-audio session is currently active
/// (M-AUDIO-SYS.2 / AUT-282). Drives the picker's master toggle
/// display.
#[cfg(target_os = "macos")]
#[tauri::command]
#[must_use]
pub fn system_audio_status(state: State<'_, SystemAudioCaptureState>) -> bool {
    state.is_active()
}

/// Non-macOS stub for `system_audio_status`. Always returns `false`
/// since no session can have been started on this platform.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
#[must_use]
pub fn system_audio_status() -> bool {
    false
}

// ---------------------------------------------------------------
// Screen-capture IPC surface (M-SCK.1 / AUT-268 + M-SCK.2 / AUT-269,
// lifecycle-only — frame channel deferred per the PR scope).
// ---------------------------------------------------------------

/// View-model for a display source (M-SCK.1 / AUT-268). Mirrors
/// `media::screen::DisplaySource` but lives in the shell crate so
/// the IPC schema is owned here.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DisplaySourceView {
    /// Stable id (`display-<displayID>`).
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Width in points.
    pub width: u32,
    /// Height in points.
    pub height: u32,
    /// `true` for the first display in the enumeration.
    pub is_primary: bool,
}

#[cfg(target_os = "macos")]
impl From<media::screen::DisplaySource> for DisplaySourceView {
    fn from(value: media::screen::DisplaySource) -> Self {
        Self {
            id: value.id,
            label: value.label,
            width: value.width,
            height: value.height,
            is_primary: value.is_primary,
        }
    }
}

/// View-model for a window source (M-SCK.1 / AUT-268).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowSourceView {
    /// Stable id for the current session (`window-<windowID>`).
    pub id: String,
    /// Window title (or empty).
    pub label: String,
    /// Width in points.
    pub width: u32,
    /// Height in points.
    pub height: u32,
    /// Owning app bundle id.
    pub bundle_id: String,
    /// Owning app display name.
    pub display_name: String,
}

#[cfg(target_os = "macos")]
impl From<media::screen::WindowSource> for WindowSourceView {
    fn from(value: media::screen::WindowSource) -> Self {
        Self {
            id: value.id,
            label: value.label,
            width: value.width,
            height: value.height,
            bundle_id: value.bundle_id,
            display_name: value.display_name,
        }
    }
}

/// Enumerate every display SCK can see (M-SCK.1 / AUT-268).
/// Returns empty Vec on non-macOS targets.
///
/// # Errors
///
/// Returns the SCK error as a string when SCK refuses (TCC denied,
/// enumeration failed).
#[tauri::command]
pub fn list_screen_displays() -> Result<Vec<DisplaySourceView>, String> {
    #[cfg(target_os = "macos")]
    {
        ensure_screen_recording_access()?;
        media::screen::list_displays()
            .map(|v| v.into_iter().map(DisplaySourceView::from).collect())
            .map_err(|err| err.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(Vec::new())
    }
}

/// Enumerate every visible window SCK can see (M-SCK.1 / AUT-268).
///
/// # Errors
///
/// Returns the SCK error as a string.
#[tauri::command]
pub fn list_screen_windows() -> Result<Vec<WindowSourceView>, String> {
    #[cfg(target_os = "macos")]
    {
        ensure_screen_recording_access()?;
        media::screen::list_windows()
            .map(|v| v.into_iter().map(WindowSourceView::from).collect())
            .map_err(|err| err.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(Vec::new())
    }
}

/// Start the screen-capture session targeting the picker-selected
/// source (M-SCK.2 / AUT-269 + M-SCK.0.1 / AUT-291). `source_id` is
/// `Some("display-<id>")` / `Some("window-<id>")` / `None` (primary
/// display). Defaults to 1920×1080 @ 30 fps with cursor shown.
/// Triggers the macOS Screen Recording TCC prompt on first run.
///
/// Re-entrant: passing a fresh `source_id` to a live session tears
/// down the existing `SCStream` and starts a new one (the picker UX
/// for swapping mid-record is a single click; M-SCK.0.1's
/// `updateContentFilter` swap-in-place is a future optimization).
///
/// # Errors
///
/// Returns the SCK error as a string. Malformed `source_id` (wrong
/// prefix / non-numeric tail) surfaces as
/// `"malformed display source id ..."` / `"malformed window source
/// id ..."`. Unknown source id (display unplugged / window closed
/// between enumeration and start) surfaces as `"<kind> id ... not
/// present"`.
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn start_screen_capture(
    state: State<'_, ScreenCaptureState>,
    source_id: Option<String>,
) -> Result<(), String> {
    use media::sck_video::{ScreenCaptureConfig, ScreenCaptureSource};
    ensure_screen_recording_access()?;
    let source = match source_id.as_deref() {
        None | Some("") => ScreenCaptureSource::PrimaryDisplay,
        Some(id) if id.starts_with("display-") => ScreenCaptureSource::Display(id.to_string()),
        Some(id) if id.starts_with("window-") => ScreenCaptureSource::Window(id.to_string()),
        Some(other) => {
            return Err(format!(
                "unknown source_id prefix `{other}` (expected `display-…` or `window-…`)"
            ));
        }
    };
    state
        .start(ScreenCaptureConfig::for_source(source))
        .map_err(|err| err.to_string())
}

/// Non-macOS stub for `start_screen_capture`. Returns the
/// requires-macOS-13.0 error so the Leptos picker surfaces a
/// consistent message across platforms. Signature matches the macOS
/// variant so the IPC schema stays uniform.
///
/// # Errors
///
/// Always returns `"screen capture requires macOS 13.0+"`.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn start_screen_capture(_source_id: Option<String>) -> Result<(), String> {
    Err("screen capture requires macOS 13.0+".into())
}

/// Stop the active screen-capture session, if any.
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn stop_screen_capture(state: State<'_, ScreenCaptureState>) {
    state.stop();
}

/// Non-macOS stub for `stop_screen_capture`. No-op.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn stop_screen_capture() {}

/// `true` when a screen-capture session is currently running
/// (M-SCK.2 / AUT-269). The Leptos picker reads this on mount + on
/// every chevron-toggle to seed UI state.
#[cfg(target_os = "macos")]
#[tauri::command]
#[must_use]
pub fn screen_capture_status(state: State<'_, ScreenCaptureState>) -> bool {
    state.is_active()
}

/// Non-macOS stub for `screen_capture_status`. Always `false`.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
#[must_use]
pub fn screen_capture_status() -> bool {
    false
}

/// Cumulative frame counter for the active session
/// (M-SCK.2 / AUT-269). Returns `0` when no session is active. Used
/// by the Leptos diagnostic overlay + future frame-rate monitor.
#[cfg(target_os = "macos")]
#[tauri::command]
#[must_use]
pub fn screen_capture_frame_count(state: State<'_, ScreenCaptureState>) -> u64 {
    state.frames_received()
}

/// Non-macOS stub. Always 0.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
#[must_use]
pub fn screen_capture_frame_count() -> u64 {
    0
}

// ---- M-RECORD.1 — coordinated recording IPC --------------------------

/// Start a coordinated recording session (M-RECORD.1 of M-RECORD-EXPORT).
///
/// Spawns each enabled per-channel pipeline (camera / screen /
/// microphone / system audio) inside one [`RecordingSession`]
/// orchestrator. Picker selections (`camera_id`, `microphone_id`,
/// `screen_source_id`) are threaded through to the existing per-
/// channel start paths so M-CAM.4 / M-MIC.3 / M-SCK.0.1 routing
/// applies inside a session too.
///
/// **Rollback discipline:** if any one stream fails to start, the
/// session aborts — the streams that already started are stopped
/// best-effort and the function returns `Err`.
///
/// **Lifecycle:** session enters `Starting`. The per-channel
/// pipelines transition `Starting → Running` independently as each
/// produces its first frame; the `recording-status` event push
/// (M-RECORD.1 follow-up commit) will roll those up into the master
/// `Running` transition.
///
/// # Errors
///
/// - `"a recording session is already active"` — re-entrant call
///   without a prior `stop_recording`. Idempotent-by-design rather
///   than implicit-replace because mid-session changes invalidate
///   the M-EXPORT encoder state.
/// - `"no streams enabled — pick at least one input"` — caller
///   passed `SessionStreams { camera: false, ... }`.
/// - `"screen + system audio capture require macOS 13.0+"` —
///   non-macOS caller enabled either of those channels.
/// - Underlying per-channel start error, prefixed with the channel
///   name (e.g. `"camera start failed: ..."`).
#[tauri::command]
#[allow(
    clippy::too_many_lines,
    reason = "Top-level orchestrator: input validation + per-channel start (camera, screen, mic, sys-audio) + encoder spin-up + session persist. Splitting per-channel helpers would just push the line count one level down while obscuring the rollback contract — every per-channel start must roll back the prior ones on failure, which is most natural as a flat sequence here."
)]
pub fn start_recording(
    app: tauri::AppHandle,
    recording_state: State<'_, RecordingState>,
    preview_state: State<'_, PreviewState>,
    camera_handle: State<'_, CameraPipelineHandle>,
    mic_state: State<'_, MicCaptureState>,
    mic_handle: State<'_, MicCaptureHandle>,
    config: RecordingConfig,
) -> Result<u64, String> {
    if recording_state.is_active() {
        return Err("a recording session is already active".into());
    }
    if !config.streams.any_enabled() {
        return Err("no streams enabled — pick at least one input".into());
    }
    #[cfg(not(target_os = "macos"))]
    {
        if config.streams.screen || config.streams.system_audio {
            return Err("screen + system audio capture require macOS 13.0+".into());
        }
    }
    #[cfg(target_os = "macos")]
    {
        if config.streams.screen || config.streams.system_audio {
            ensure_screen_recording_access()?;
        }
    }

    let session = RecordingSession::starting(config.streams);
    let session_id = session.id;
    tracing::info!(
        session_id,
        camera = config.streams.camera,
        screen = config.streams.screen,
        microphone = config.streams.microphone,
        system_audio = config.streams.system_audio,
        "start_recording: spawning per-channel pipelines"
    );

    let mut started: Vec<StreamKind> = Vec::new();

    // Camera — re-uses the M-CAM.4 routing from start_preview.
    if config.streams.camera {
        if let Err(err) = start_camera_for_session(
            &app,
            &preview_state,
            &camera_handle,
            config.camera_id.clone(),
        ) {
            rollback_started(&app, &started);
            return Err(format!("camera start failed: {err}"));
        }
        started.push(StreamKind::Camera);
    }

    // Microphone — re-uses the M-MIC.3 native_id resolution.
    if config.streams.microphone {
        let mixer = crate::recording::SharedAudioMixer::clone(&recording_state.audio_mixer);
        if let Err(err) = start_mic_for_session(
            &app,
            &mic_state,
            &mic_handle,
            config.microphone_id.clone(),
            mixer,
        ) {
            rollback_started(&app, &started);
            return Err(format!("microphone start failed: {err}"));
        }
        started.push(StreamKind::Microphone);
    }

    // Screen (macOS-only) — re-uses M-SCK.0.1 source routing.
    #[cfg(target_os = "macos")]
    if config.streams.screen {
        if let Err(err) = start_screen_for_session(&app, config.screen_source_id.as_deref()) {
            rollback_started(&app, &started);
            return Err(format!("screen start failed: {err}"));
        }
        started.push(StreamKind::Screen);
    }

    // System audio (macOS-only).
    #[cfg(target_os = "macos")]
    if config.streams.system_audio {
        if let Err(err) = start_sys_audio_for_session(&app) {
            rollback_started(&app, &started);
            return Err(format!("system audio start failed: {err}"));
        }
        started.push(StreamKind::SystemAudio);
    }

    // M-EXPORT.3 + M-PIX.6 — spin up the encoder. Two feed-thread
    // variants:
    //
    // - If any video channel (camera or screen) is enabled, use
    //   the M-PIX.6 real-capture feed: pulls composed frames from
    //   the wisp render pump + mixed audio from the AudioMixer.
    // - Otherwise (audio-only or no-channels-enabled debug),
    //   fall back to the M-EXPORT.3 test-pattern feed so the
    //   encoder still produces a valid container.
    //
    // Output path + format come from the caller (M-EXPORT.4
    // defaults applied UI-side); failure here rolls back per-
    // channel streams too.
    let encoder_config = build_encoder_config_for_session(&config)?;
    let session_id_for_palette = session.id;
    let has_video = config.streams.camera || config.streams.screen;
    let handle_result = if has_video {
        use wisp::recording::StreamDimensions;
        // Scene dims match what the capture sides emit. On
        // non-macOS the SCK constants don't exist; the
        // SCK-derived screen dims default to 1920×1080 verbatim
        // (matching `media::sck_video::DEFAULT_WIDTH/HEIGHT`)
        // since neither the screen nor system-audio channel
        // actually runs there yet (rolled back at top of fn).
        #[cfg(target_os = "macos")]
        let (sck_w, sck_h) = (
            media::sck_video::DEFAULT_WIDTH,
            media::sck_video::DEFAULT_HEIGHT,
        );
        #[cfg(not(target_os = "macos"))]
        let (sck_w, sck_h) = (1920_u32, 1080_u32);
        let screen_dims = StreamDimensions::new(sck_w, sck_h);
        let cam_dims = StreamDimensions::new(
            crate::preview::pipeline::PREVIEW_WIDTH,
            crate::preview::pipeline::PREVIEW_HEIGHT,
        );
        let camera_slot = crate::recording::FrameSlot::clone(&recording_state.camera_frame_slot);
        let screen_slot = crate::recording::FrameSlot::clone(&recording_state.screen_frame_slot);
        let mixer = crate::recording::SharedAudioMixer::clone(&recording_state.audio_mixer);
        crate::recording::EncoderHandle::start_with_real_capture(
            encoder_config,
            camera_slot,
            screen_slot,
            mixer,
            screen_dims,
            cam_dims,
        )
    } else {
        crate::recording::EncoderHandle::start_with_test_pattern(
            encoder_config,
            session_id_for_palette,
        )
    };
    match handle_result {
        Ok(handle) => {
            tracing::info!(
                session_id,
                output_path = %handle.output_path.display(),
                feed_kind = if has_video { "real-capture" } else { "test-pattern" },
                "start_recording: encoder started"
            );
            recording_state.install_encoder(handle);
        }
        Err(err) => {
            rollback_started(&app, &started);
            return Err(format!("encoder start failed: {err}"));
        }
    }

    {
        let mut guard = recording_state
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = Some(session);
    }
    spawn_status_emitter(app.clone(), session_id);
    Ok(session_id)
}

/// Map a [`RecordingConfig`] (M-RECORD.1 IPC) to an
/// [`EncoderConfig`] (M-EXPORT.1). Resolves the format slug and the
/// output path; falls back to defaults for missing fields. Returns
/// `Result` for forward-compat (M-EXPORT.3.1 will add path-extension
/// validation that can fail).
#[allow(
    clippy::unnecessary_wraps,
    reason = "Forward-compat: M-EXPORT.3.1 adds validation that returns Err on extension mismatch; keeping Result now avoids a churn-only signature change later."
)]
fn build_encoder_config_for_session(
    config: &RecordingConfig,
) -> Result<media::encode::EncoderConfig, String> {
    use media::encode::OutputFormat;
    let format = config
        .format
        .as_deref()
        .and_then(OutputFormat::from_slug)
        .unwrap_or_default();
    let output_path = config.output_path.clone().map_or_else(
        || {
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            crate::recording_paths::default_output_path(now_secs, format)
        },
        std::path::PathBuf::from,
    );
    Ok(media::encode::EncoderConfig::for_output(
        output_path,
        format,
    ))
}

/// Stop the active recording session (M-RECORD.1).
///
/// Tears down each enabled per-channel pipeline in reverse start
/// order. The returned [`RecordingSummary`] carries the final
/// per-stream tally + the encoded file path (M-EXPORT.4 populates
/// the path; today it's `None`).
///
/// # Errors
///
/// - `"no recording session is active"` — caller invoked stop
///   without a matching start.
#[tauri::command]
pub fn stop_recording(
    app: tauri::AppHandle,
    recording_state: State<'_, RecordingState>,
    preview_state: State<'_, PreviewState>,
    camera_handle: State<'_, CameraPipelineHandle>,
    mic_state: State<'_, MicCaptureState>,
    mic_handle: State<'_, MicCaptureHandle>,
) -> Result<RecordingSummary, String> {
    let Some(mut session) = recording_state.snapshot() else {
        return Err("no recording session is active".into());
    };
    session.begin_stop();
    {
        let mut guard = recording_state
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = Some(session.clone());
    }

    let final_health = build_stream_health_snapshot(&app, session.streams, session.started_at);

    // Reverse start order so teardown mirrors construction.
    #[cfg(target_os = "macos")]
    if session.streams.system_audio {
        let _ = stop_sys_audio_for_session(&app);
    }
    #[cfg(target_os = "macos")]
    if session.streams.screen {
        let _ = stop_screen_for_session(&app);
    }
    if session.streams.microphone {
        stop_mic_for_session(&mic_state, &mic_handle);
    }
    if session.streams.camera {
        stop_camera_for_session(&preview_state, &camera_handle);
    }

    session.finish_stop();
    let elapsed_ms = u64::try_from(session.elapsed().as_millis()).unwrap_or(u64::MAX);

    // M-EXPORT.3 — finalize the encoder + generate the AVIF poster.
    let output_path = if let Some(handle) = recording_state.take_encoder() {
        let path = handle.output_path.clone();
        match handle.finalize_now() {
            Ok(final_path) => {
                tracing::info!(
                    session_id = session.id,
                    output = %final_path.display(),
                    "stop_recording: encoder finalized"
                );
                // M-EXPORT.5 — best-effort poster (silently skipped
                // when `avifenc` not installed; logged when it
                // genuinely fails).
                match media::encode::generate_poster(&final_path) {
                    Ok(Some(poster)) => {
                        tracing::info!(poster = %poster.display(), "stop_recording: poster ready");
                    }
                    Ok(None) => {
                        tracing::debug!("stop_recording: poster skipped (avifenc missing)");
                    }
                    Err(err) => {
                        tracing::warn!(?err, "stop_recording: poster generation failed");
                    }
                }
                Some(final_path.to_string_lossy().into_owned())
            }
            Err(err) => {
                tracing::error!(?err, output = %path.display(), "stop_recording: encoder finalize failed");
                Some(path.to_string_lossy().into_owned())
            }
        }
    } else {
        None
    };

    let summary = RecordingSummary {
        session_id: session.id,
        elapsed_ms,
        streams: final_health,
        output_path,
    };

    {
        let mut guard = recording_state
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = None;
    }
    tracing::info!(
        session_id = summary.session_id,
        elapsed_ms,
        "stop_recording: session torn down"
    );
    Ok(summary)
}

/// Live snapshot of the recording session for the picker LED ramp
/// + elapsed counter. Returns `RecordingStatusView::idle()` when no
/// session is active. Also published via the `recording-status`
/// event every 500 ms while a session is running.
#[tauri::command]
#[must_use]
pub fn recording_status(
    app: tauri::AppHandle,
    recording_state: State<'_, RecordingState>,
) -> RecordingStatusView {
    let Some(session) = recording_state.snapshot() else {
        return RecordingStatusView::idle();
    };
    let elapsed_ms = u64::try_from(session.elapsed().as_millis()).unwrap_or(u64::MAX);
    let streams = build_stream_health_snapshot(&app, session.streams, session.started_at);
    RecordingStatusView {
        session_id: Some(session.id),
        state: session.state,
        elapsed_ms,
        streams,
    }
}

// ---- M-RECORD.1 internal helpers ---------------------------------

/// Direct-call equivalent of `start_preview` — bypasses the
/// `#[tauri::command]` layer so the session orchestrator can
/// coordinate with the existing `PreviewState` lifecycle.
fn start_camera_for_session(
    app: &tauri::AppHandle,
    preview_state: &PreviewState,
    camera_handle: &CameraPipelineHandle,
    camera_id: String,
) -> Result<(), CameraError> {
    {
        let mut guard = preview_state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let new_state = guard.try_start();
        // Re-entrant attempt (already Starting/Running): treat as
        // success — the session is reusing the existing pipeline.
        if new_state == *guard {
            return Ok(());
        }
        *guard = new_state;
    }
    let pipeline = CameraPipeline::spawn(app.clone(), camera_id)?;
    camera_handle.install(pipeline);
    Ok(())
}

fn stop_camera_for_session(preview_state: &PreviewState, camera_handle: &CameraPipelineHandle) {
    {
        let mut guard = preview_state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = guard.try_stop();
    }
    camera_handle.shutdown();
    {
        let mut guard = preview_state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = guard.finish_stop();
    }
}

fn start_mic_for_session(
    app: &tauri::AppHandle,
    mic_state: &MicCaptureState,
    mic_handle: &MicCaptureHandle,
    mic_id: String,
    mixer: crate::recording::SharedAudioMixer,
) -> Result<(), MicError> {
    let native_id = if mic_id.is_empty() {
        String::new()
    } else if let Some(device) = media::microphone::find_by_id(&mic_id) {
        device.native_id
    } else {
        return Err(MicError::NotFound(mic_id));
    };

    // Tear down any prior session held by an out-of-band caller
    // (e.g. the picker's preview pipeline driving the level meter).
    if mic_handle.is_active() {
        mic_handle.shutdown();
        let mut guard = mic_state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = guard.try_stop().finish_stop();
    }
    {
        let mut guard = mic_state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let prev_state = *guard;
        let new_state = guard.try_start();
        if new_state == *guard {
            // State was already Starting/Running/Stopping — no new
            // pipeline is spawned. The recording session won't see
            // any mic samples until whatever owns the prior worker
            // tears it down. Surfaces as a silent-audio recording,
            // which is the bug class this warning exists to catch.
            tracing::warn!(
                ?prev_state,
                "start_mic_for_session: state desynced from handle; no pipeline spawned"
            );
            return Ok(());
        }
        *guard = new_state;
    }
    // Recording path: pass `Some(mixer)` so the worker forwards samples
    // into the shared AudioMixer for the encoder feed thread to pull.
    let pipeline = MicCapturePipeline::spawn(app.clone(), mic_id, native_id, Some(mixer))?;
    mic_handle.install(pipeline);
    Ok(())
}

fn stop_mic_for_session(mic_state: &MicCaptureState, mic_handle: &MicCaptureHandle) {
    {
        let mut guard = mic_state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = guard.try_stop();
    }
    mic_handle.shutdown();
    {
        let mut guard = mic_state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = guard.finish_stop();
    }
}

#[cfg(target_os = "macos")]
fn start_screen_for_session(app: &tauri::AppHandle, source_id: Option<&str>) -> Result<(), String> {
    use media::sck_video::{ScreenCaptureConfig, ScreenCaptureSource};
    let Some(state) = app.try_state::<ScreenCaptureState>() else {
        return Err("ScreenCaptureState not managed".into());
    };
    let source = match source_id {
        None | Some("") => ScreenCaptureSource::PrimaryDisplay,
        Some(id) if id.starts_with("display-") => ScreenCaptureSource::Display(id.to_string()),
        Some(id) if id.starts_with("window-") => ScreenCaptureSource::Window(id.to_string()),
        Some(other) => return Err(format!("unknown source_id prefix `{other}`")),
    };
    // M-PIX.2 — plumb the shared screen frame slot from
    // RecordingState into the SCK delegate so it writes BGRA bytes
    // there for the encoder feed thread.
    let frame_slot = app
        .try_state::<RecordingState>()
        .map(|s| crate::recording::FrameSlot::clone(&s.screen_frame_slot));
    state
        .start_with_frame_slot(ScreenCaptureConfig::for_source(source), frame_slot)
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn stop_screen_for_session(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(state) = app.try_state::<ScreenCaptureState>() else {
        return Err("ScreenCaptureState not managed".into());
    };
    state.stop();
    Ok(())
}

#[cfg(target_os = "macos")]
fn start_sys_audio_for_session(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(state) = app.try_state::<SystemAudioCaptureState>() else {
        return Err("SystemAudioCaptureState not managed".into());
    };
    // M-PIX.4 — plumb the shared AudioMixer from RecordingState so
    // the SCK delegate forwards system-audio F32 samples into it
    // for the encoder feed thread to pull.
    let mixer = app
        .try_state::<RecordingState>()
        .map(|s| crate::recording::SharedAudioMixer::clone(&s.audio_mixer));
    state
        .start_with_mixer(app, media::sck_audio::SystemAudioConfig::default(), mixer)
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn stop_sys_audio_for_session(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(state) = app.try_state::<SystemAudioCaptureState>() else {
        return Err("SystemAudioCaptureState not managed".into());
    };
    state.stop();
    Ok(())
}

/// Roll back partially-started channels after a per-channel start
/// failure mid-session. Best-effort — each stop swallows its own
/// errors since we're already on the error path.
fn rollback_started(app: &tauri::AppHandle, started: &[StreamKind]) {
    tracing::warn!(?started, "start_recording: rolling back partial start");
    for kind in started.iter().rev() {
        match kind {
            StreamKind::Camera => {
                if let (Some(preview), Some(handle)) = (
                    app.try_state::<PreviewState>(),
                    app.try_state::<CameraPipelineHandle>(),
                ) {
                    stop_camera_for_session(&preview, &handle);
                }
            }
            StreamKind::Microphone => {
                if let (Some(state), Some(handle)) = (
                    app.try_state::<MicCaptureState>(),
                    app.try_state::<MicCaptureHandle>(),
                ) {
                    stop_mic_for_session(&state, &handle);
                }
            }
            #[cfg(target_os = "macos")]
            StreamKind::Screen => {
                let _ = stop_screen_for_session(app);
            }
            #[cfg(target_os = "macos")]
            StreamKind::SystemAudio => {
                let _ = stop_sys_audio_for_session(app);
            }
            #[cfg(not(target_os = "macos"))]
            StreamKind::Screen | StreamKind::SystemAudio => {
                // Can never have been started — guarded out at top of
                // start_recording.
            }
        }
    }
}

/// Build the per-stream `StreamHealth` snapshot by querying each
/// enabled channel's existing State<> handle. Called by both
/// `recording_status` (live polling) and `stop_recording` (final
/// summary). `last_frame_ms_ago` is left `None` for now — the
/// per-channel handles don't yet expose a `last_frame_at` timestamp
/// (TODO M-RECORD-EXPORT follow-up; M-RECORD.2's LED ramp already
/// handles `None` as "no recent frame, render yellow/red based on
/// session age").
fn build_stream_health_snapshot(
    app: &tauri::AppHandle,
    streams: SessionStreams,
    _started_at: std::time::Instant,
) -> Vec<StreamHealth> {
    let mut out: Vec<StreamHealth> = Vec::new();
    for kind in streams.enabled_kinds() {
        let (lifecycle, frame_count) = match kind {
            StreamKind::Camera => {
                let life = app.try_state::<PreviewState>().map_or_else(
                    || "Idle".into(),
                    |s| {
                        format!(
                            "{:?}",
                            *s.0.lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner)
                        )
                    },
                );
                let count = app
                    .try_state::<PreviewDiagnostics>()
                    .map_or(0, |s| s.snapshot().frames_received);
                (life, count)
            }
            StreamKind::Microphone => {
                let life = app.try_state::<MicCaptureState>().map_or_else(
                    || "Idle".into(),
                    |s| {
                        format!(
                            "{:?}",
                            *s.0.lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner)
                        )
                    },
                );
                // No frame-counter exposed today; left 0.
                (life, 0)
            }
            #[cfg(target_os = "macos")]
            StreamKind::Screen => {
                let count = app
                    .try_state::<ScreenCaptureState>()
                    .map_or(0, |s| s.frames_received());
                let life = if app
                    .try_state::<ScreenCaptureState>()
                    .is_some_and(|s| s.is_active())
                {
                    "Running".into()
                } else {
                    "Idle".into()
                };
                (life, count)
            }
            #[cfg(not(target_os = "macos"))]
            StreamKind::Screen => ("Idle".into(), 0),
            #[cfg(target_os = "macos")]
            StreamKind::SystemAudio => {
                let active = app
                    .try_state::<SystemAudioCaptureState>()
                    .is_some_and(|s| s.is_active());
                (
                    if active {
                        "Running".into()
                    } else {
                        "Idle".into()
                    },
                    0,
                )
            }
            #[cfg(not(target_os = "macos"))]
            StreamKind::SystemAudio => ("Idle".into(), 0),
        };
        out.push(StreamHealth {
            kind,
            lifecycle,
            frame_count,
            last_frame_ms_ago: None,
        });
    }
    out
}

/// Spawn the 500 ms event-push thread. Loops emitting
/// `recording-status` until the session is gone from
/// `RecordingState`. Self-terminates on session end so callers don't
/// need to track the `JoinHandle`. Plain `std::thread` rather than a
/// tokio task — Tauri's `Emitter` is sync-friendly and avoids
/// adding a direct tokio dep (Tauri uses tokio internally but
/// doesn't re-export `tokio::time::interval`).
fn spawn_status_emitter(app: tauri::AppHandle, session_id: u64) {
    use tauri::Emitter;
    std::thread::Builder::new()
        .name(format!("recording-status-emitter-{session_id}"))
        .spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let Some(state) = app.try_state::<RecordingState>() else {
                    break;
                };
                let Some(session) = state.snapshot() else {
                    break;
                };
                if session.id != session_id {
                    // A new session started before this thread
                    // observed its predecessor's end. Newer thread
                    // takes over.
                    break;
                }
                let elapsed_ms = u64::try_from(session.elapsed().as_millis()).unwrap_or(u64::MAX);
                let view = RecordingStatusView {
                    session_id: Some(session.id),
                    state: session.state,
                    elapsed_ms,
                    streams: build_stream_health_snapshot(
                        &app,
                        session.streams,
                        session.started_at,
                    ),
                };
                // M-RECORD.1: fold per-stream Running observation up
                // to the master session — every enabled stream
                // non-Idle → advance Starting → Running.
                if session.state == SessionState::Starting
                    && !view.streams.is_empty()
                    && view.streams.iter().all(|h| h.lifecycle != "Idle")
                    && let Some(s) = app.try_state::<RecordingState>()
                {
                    let mut guard = s
                        .session
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if let Some(ref mut sess) = *guard {
                        sess.mark_running();
                    }
                }
                if let Err(err) = app.emit("recording-status", &view) {
                    tracing::trace!(?err, "emit recording-status failed");
                }
            }
            tracing::debug!(session_id, "status-emitter thread exiting");
        })
        .expect("recording-status-emitter thread spawn must succeed");
}

// ---- M-EXPORT.4 — file save + reveal IPC ----------------------------

/// Resolve the default output path for a recording starting now
/// with the given format slug. Returns the absolute path as a
/// string (the JS side feeds it back into `start_recording`'s
/// `output_path` if the user doesn't override).
///
/// `format_slug` is one of `"mp4-h264"`, `"mp4-h265"`, `"webm-vp9"`,
/// `"webm-av1"`. Unknown slugs fall back to the default
/// (`mp4-h264`).
#[tauri::command]
#[must_use]
pub fn default_recording_output_path(format_slug: Option<String>) -> String {
    use media::encode::OutputFormat;
    let format = format_slug
        .as_deref()
        .and_then(OutputFormat::from_slug)
        .unwrap_or_default();
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    crate::recording_paths::default_output_path(now_secs, format)
        .to_string_lossy()
        .into_owned()
}

/// Return the latest BGRA frame from the camera capture slot
/// (M-PIX.8). Used by `<CameraPreview />`'s 15fps poll to paint
/// the live webcam into the canvas. Returns raw bytes via
/// `tauri::ipc::Response` so the JS side receives an `ArrayBuffer`
/// directly (no JSON-array serialization overhead).
///
/// Empty `Response` when no frame is available yet — the JS side
/// skips painting on this tick.
///
/// Reads the same `CameraFrameSlot` the encoder feed thread reads
/// from. The capture worker writes latest-frame-wins, so both
/// consumers see the most recent frame; neither blocks the other
/// (the preview's `take()` clears the slot, but the next capture
/// tick re-fills within ~33 ms at 30 fps).
#[tauri::command]
#[must_use]
pub fn latest_camera_frame_bgra(
    recording_state: State<'_, RecordingState>,
) -> tauri::ipc::Response {
    let bytes = recording_state
        .camera_frame_slot
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
        .unwrap_or_default();
    tauri::ipc::Response::new(bytes)
}

/// Open the OS file manager focused on the given recording file
/// (M-EXPORT.4). macOS: `open -R`. Windows:
/// `explorer /select,`. Linux: `xdg-open <parent-dir>` (no portable
/// "select" verb).
///
/// # Errors
///
/// Returns the spawn error as a string when the file-manager binary
/// isn't on PATH.
#[tauri::command]
pub fn reveal_recording_in_file_manager(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    crate::recording_paths::reveal_in_file_manager(&p)
}

/// Test-only entry point for `WebDriver` e2e suites. Emits a
/// `file-dropped` event with the same shape as the real OS drag-drop
/// handler in `main.rs`. Gated on `debug_assertions` so it's stripped
/// from release builds; `main.rs` likewise registers it conditionally
/// in `generate_handler!`.
///
/// Why this exists: `WebDriver` clients can't synthesize OS-level
/// drag-drop events. Without this command, the e2e tests would have
/// to use platform-specific tools (`xdotool` on Linux, etc.) which are
/// fragile and don't help the rest of the test suite.
///
/// # Errors
///
/// Returns the underlying [`tauri::Error`] message string if the event
/// emit fails (no listeners is not an error — `Emitter::emit` returns
/// `Ok(())` regardless).
#[cfg(debug_assertions)]
#[tauri::command]
pub fn __test_drop_file(app: tauri::AppHandle, path: String) -> Result<(), String> {
    use tauri::Emitter;
    app.emit("file-dropped", path).map_err(|e| e.to_string())
}

/// Test-only entry point: synthesize a `DragDropEvent::Enter` for the
/// `WebDriver` e2e suite. Emits the same `file-drag-enter` event as the
/// real OS drag-enter handler. Debug-only, parallel to [`__test_drop_file`].
///
/// # Errors
///
/// Returns the underlying [`tauri::Error`] message if the event emit
/// fails.
#[cfg(debug_assertions)]
#[tauri::command]
pub fn __test_drag_enter(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    app.emit("file-drag-enter", ()).map_err(|e| e.to_string())
}

/// Test-only entry point: synthesize a `DragDropEvent::Leave`.
/// Pair with [`__test_drag_enter`].
///
/// # Errors
///
/// Returns the underlying [`tauri::Error`] message if the event emit
/// fails.
#[cfg(debug_assertions)]
#[tauri::command]
pub fn __test_drag_leave(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    app.emit("file-drag-leave", ()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mon(x: i32, y: i32, w: i32, h: i32) -> MonitorBounds {
        MonitorBounds {
            x,
            y,
            width: w,
            height: h,
        }
    }

    #[test]
    fn anchor_returns_none_when_no_monitors() {
        assert_eq!(compute_popover_anchor(500, 12, 800, &[]), None);
    }

    #[test]
    fn anchor_lands_at_monitor_top_right_regardless_of_click() {
        let monitors = vec![mon(0, 0, 1920, 1080)];
        // The click position influences monitor selection only; the
        // popover always lands flush with the monitor's top-right.
        // Click at the far right of the menubar:
        let (x, y) = compute_popover_anchor(1820, 12, 800, &monitors).expect("Some(_)");
        assert_eq!((x, y), (1120, 0));
        // Click near the left edge of the menubar — same anchor.
        let (x, y) = compute_popover_anchor(50, 12, 800, &monitors).expect("Some(_)");
        assert_eq!((x, y), (1120, 0));
    }

    #[test]
    fn anchor_picks_secondary_monitor_for_a_click_on_it() {
        // Two side-by-side 1920×1080 monitors. A click at x=3820 lives
        // in the second monitor; the popover anchors top-right of it.
        let monitors = vec![mon(0, 0, 1920, 1080), mon(1920, 0, 1920, 1080)];
        let (x, y) = compute_popover_anchor(3820, 12, 800, &monitors).expect("Some(_)");
        // Monitor 2 right edge = 1920 + 1920 = 3840; x = 3840 - 800 = 3040.
        assert_eq!((x, y), (3040, 0));
    }

    // ── M-BUBBLE.3 / AUT-276 — position persistence file format ──

    #[test]
    fn encode_position_uses_canonical_format() {
        let s = encode_position(BubblePosition { x: 100, y: 200 });
        assert_eq!(s, "v2:100,200\n");
    }

    #[test]
    fn encode_position_handles_negative_coords() {
        // Multi-monitor layouts often have negative coords (secondary
        // monitor to the left of the primary).
        let s = encode_position(BubblePosition { x: -500, y: 0 });
        assert_eq!(s, "v2:-500,0\n");
    }

    #[test]
    fn decode_position_round_trips_encoded_value() {
        let pos = BubblePosition { x: 1234, y: -56 };
        let encoded = encode_position(pos);
        assert_eq!(decode_position(&encoded), Some(pos));
    }

    #[test]
    fn decode_position_tolerates_missing_trailing_newline() {
        assert_eq!(
            decode_position("v2:42,99"),
            Some(BubblePosition { x: 42, y: 99 })
        );
    }

    #[test]
    fn decode_position_tolerates_whitespace_around_values() {
        assert_eq!(
            decode_position("v2: 7 , 11 \n"),
            Some(BubblePosition { x: 7, y: 11 })
        );
    }

    #[test]
    fn decode_position_rejects_missing_comma() {
        assert_eq!(decode_position("v2:123 456"), None);
    }

    #[test]
    fn decode_position_rejects_non_integer() {
        assert_eq!(decode_position("v2:3.14,2.71"), None);
        assert_eq!(decode_position("v2:abc,def"), None);
        assert_eq!(decode_position("v2:,100"), None);
    }

    #[test]
    fn decode_position_rejects_empty_input() {
        assert_eq!(decode_position(""), None);
        assert_eq!(decode_position("   \n"), None);
    }

    #[test]
    fn decode_position_rejects_v1_legacy_format() {
        // Pre-design-pass file format was bare `x,y\n`. Bumping the
        // prefix to `v2:` lets us migrate users off the old default
        // bottom-right corner without writing a per-user one-shot
        // migration: stale files just fail to parse and fall through
        // to `compute_default_position`.
        assert_eq!(decode_position("100,200\n"), None);
        assert_eq!(decode_position("-500,0"), None);
    }

    #[test]
    fn bubble_state_update_position_round_trips() {
        let state = BubbleState::default();
        assert_eq!(state.last_position(), None);
        state.set_last_position(BubblePosition { x: 50, y: 60 });
        assert_eq!(state.last_position(), Some(BubblePosition { x: 50, y: 60 }));
        // update_bubble_position_from_event is the public hook the
        // WindowEvent::Moved handler uses.
        update_bubble_position_from_event(&state, 99, -7);
        assert_eq!(state.last_position(), Some(BubblePosition { x: 99, y: -7 }));
    }
}
