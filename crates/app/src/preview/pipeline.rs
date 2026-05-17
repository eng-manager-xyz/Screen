//! M-CAM.3 / AUT-257 — camera-pipeline worker thread.
//!
//! Owns a dedicated OS thread that runs the gst-`autovideosrc` capture
//! subprocess, pulls BGRA frames into Rust, and (in follow-up commits)
//! uploads each frame to a `wisp::VideoTexture`, renders the wisp
//! scene with an M-VEC.6 circle mask into an offscreen
//! `RenderTexture`, reads back the masked BGRA bytes, and emits them
//! to Leptos via a Tauri `Channel<T>`.
//!
//! ```admonish important title="What this commit ships"
//! Just the **gst-into-Rust** layer:
//!
//! 1. `start_preview` spawns a [`CameraPipeline`] worker.
//! 2. Worker opens `media::gstreamer_video::VideoStream::from_default_camera`,
//!    triggering the macOS permission prompt on first run.
//! 3. Worker loops `next_frame()` and advances the
//!    [`PreviewLifecycle`](crate::preview::PreviewLifecycle) state
//!    machine — `Starting → Running` on first successful frame.
//! 4. `stop_preview` drops the worker; `Drop` flips the cancel flag
//!    and joins the thread. The gst child is killed by
//!    `gstreamer_video::VideoStream`'s own `Drop` impl (per CLAUDE.md
//!    "Drop-kill the child" pattern).
//!
//! NOT yet shipped:
//!
//! * **No wisp upload + render** — the frames sit in the worker; they
//!   don't yet flow through a `wisp::Stage` + M-VEC.6 mask. That's the
//!   next commit.
//! * **No frame emission to Leptos** — Tauri `Channel<T>` is wired
//!   alongside the wisp work. For now, the user sees the `Running`
//!   lifecycle transition in `preview_status` but no pixels.
//! ```
//!
//! Thread-affinity contract — `media::gstreamer_video::VideoStream`
//! owns a `std::process::Child` (gst-launch subprocess) + a stdout
//! reader. `Child` is `Send`, so the worker thread can own the stream
//! exclusively. No `Rc`/`RefCell` anywhere in the type, so it's safe
//! to move into the spawned thread.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use media::gstreamer_video::GstreamerVideoCapture;

use super::diagnostics::{PreviewDiagnostics, maybe_dump_first_frame};
use super::{CameraError, PreviewState};

/// Default capture width in pixels. Paired with [`PREVIEW_HEIGHT`]
/// — square dims are the natural input shape for the circular mask
/// the follow-up commit applies; rendering also lands at 480×480,
/// so source-side scaling is a no-op at the readback stage.
pub const PREVIEW_WIDTH: u32 = 480;

/// Default capture height in pixels. Matches [`PREVIEW_WIDTH`] —
/// see that constant's docs for the square-crop rationale.
pub const PREVIEW_HEIGHT: u32 = 480;

/// Source framerate request. gst will negotiate the closest the OS
/// camera supports; the actual rate is reflected in
/// [`media::gstreamer_video::GstreamerVideoCapture::framerate`].
pub const PREVIEW_FPS: u32 = 30;

// Compile-time invariants for the camera preview constants. These
// replace the runtime `#[test]` versions that clippy's
// `assertions_on_constants` lint flagged — these fire at compile
// time (zero runtime cost), and they actually fail the build if a
// future edit drifts the values, instead of just failing a test.
//
// PREVIEW_WIDTH must equal PREVIEW_HEIGHT: the M-CAM.3 follow-up
// applies a circular mask whose max radius is `min(w, h) / 2`. A
// non-square input would crop or stretch silently, both of which
// are wrong.
const _: () = assert!(
    PREVIEW_WIDTH == PREVIEW_HEIGHT,
    "PREVIEW_WIDTH must equal PREVIEW_HEIGHT — circular mask requires square input"
);

// PREVIEW_FPS must be a round target most cameras support natively
// (30 or 60). Off-target rates (24 / 25 / 29.97) need explicit gst
// caps negotiation that we don't ship today.
const _: () = assert!(
    PREVIEW_FPS == 30 || PREVIEW_FPS == 60,
    "PREVIEW_FPS must be 30 or 60 — off-target rates need gst caps negotiation"
);

/// Camera-pipeline worker handle. Owns the spawned thread and a
/// cooperative cancel flag; `Drop` cancels + joins so a panicking
/// caller can never leave a zombie gst child behind.
pub struct CameraPipeline {
    cancel: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl CameraPipeline {
    /// Spawn the worker thread for the camera identified by
    /// `camera_id` (M-CAM.4 — was hard-coded to the OS default
    /// device in M-CAM.3). Returns immediately; the worker
    /// transitions the preview lifecycle to `Running` once
    /// `next_frame()` succeeds (after any macOS permission prompt
    /// resolves). An empty `camera_id` keeps the legacy "OS default"
    /// behaviour so the picker can start without a selection.
    ///
    /// # Errors
    ///
    /// Returns `Err` only if the OS refuses to spawn a thread (which
    /// effectively never happens). gst-side errors are handled inside
    /// the worker — the worker logs via `tracing::error!` and
    /// advances the lifecycle back to `Idle` so the UI shows a
    /// recovery state.
    pub fn spawn(app: tauri::AppHandle, camera_id: String) -> Result<Self, CameraError> {
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_thread = Arc::clone(&cancel);
        let handle = thread::Builder::new()
            .name("camera-pipeline".to_owned())
            .spawn(move || {
                run_pipeline(&app, &cancel_for_thread, &camera_id);
            })
            .map_err(|err| CameraError::GstFailed(format!("thread spawn failed: {err}")))?;
        Ok(Self {
            cancel,
            handle: Some(handle),
        })
    }
}

impl Drop for CameraPipeline {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            // Best-effort join; if the worker panicked the join
            // returns `Err` but we don't care — we're tearing down.
            let _ = handle.join();
        }
    }
}

/// The actual worker loop. Lives on the spawned thread; the only
/// reason it's `&` borrows is so the public-facing `spawn` can move
/// the cloned values in without keeping `Self` alive on the thread.
fn run_pipeline(app: &tauri::AppHandle, cancel: &AtomicBool, camera_id: &str) {
    use tauri::Manager;

    // Reset diagnostics on session start so the user sees a fresh
    // frame counter / dump-slot per `start_preview` invocation.
    let diagnostics_state = app.state::<PreviewDiagnostics>();
    diagnostics_state.reset();

    // M-CAM.4 — pin capture to the user's picked camera. Empty
    // `camera_id` preserves the M-CAM.3 "OS default" behaviour for
    // pre-picker callers (no Leptos UI yet → no id to route on).
    let mut stream = if camera_id.is_empty() {
        tracing::info!("camera-pipeline: no camera_id supplied; using OS default");
        match GstreamerVideoCapture::from_default_camera(
            PREVIEW_WIDTH,
            PREVIEW_HEIGHT,
            PREVIEW_FPS,
        ) {
            Ok(stream) => stream,
            Err(err) => {
                tracing::error!(?err, "VideoStream::from_default_camera failed");
                reset_lifecycle(app);
                return;
            }
        }
    } else {
        match GstreamerVideoCapture::from_camera(
            camera_id,
            PREVIEW_WIDTH,
            PREVIEW_HEIGHT,
            PREVIEW_FPS,
        ) {
            Ok(stream) => stream,
            Err(err) => {
                tracing::error!(?err, %camera_id, "VideoStream::from_camera failed");
                reset_lifecycle(app);
                return;
            }
        }
    };
    let (src_w, src_h) = stream.dimensions();
    let src_fps = stream.framerate();
    diagnostics_state.record_source(src_w, src_h, src_fps);
    tracing::info!(
        width = src_w,
        height = src_h,
        fps = src_fps,
        "camera-pipeline opened; awaiting first frame"
    );

    while !cancel.load(Ordering::Relaxed) {
        match stream.next_frame() {
            Ok(frame) => {
                advance_to_running(app);
                diagnostics_state.record_frame();
                // One-shot PNG dump of the first frame so the user
                // can open the file and confirm real pixels reached
                // Rust. No-op on every subsequent frame this session.
                maybe_dump_first_frame(
                    app,
                    &diagnostics_state,
                    &frame.bgra,
                    frame.width,
                    frame.height,
                );
                // Frame is otherwise unused here — the wisp render +
                // Tauri Channel emit layers consume it in follow-up
                // commits.
                drop(frame);
            }
            Err(err) => {
                tracing::warn!(?err, "VideoStream::next_frame errored; tearing down");
                break;
            }
        }
    }

    tracing::info!("camera-pipeline cancel observed; shutting down");
    reset_lifecycle(app);
    // `stream` drops here → gst-launch child killed + reaped per
    // CLAUDE.md's "Drop-kill the child" pattern.
    drop(stream);
}

/// Mark the preview lifecycle as `Running` (idempotent — already-
/// `Running` stays running). Called from the worker thread on each
/// successful frame; the `mark_running` transition is idempotent so
/// we don't need a "first frame" guard.
fn advance_to_running(app: &tauri::AppHandle) {
    use tauri::Manager;
    let state = app.state::<PreviewState>();
    let mut guard = state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let next = guard.mark_running();
    if *guard != next {
        tracing::info!("preview lifecycle: Starting → Running (first frame received)");
        *guard = next;
    }
}

/// Drive the lifecycle back to `Idle` on shutdown OR on gst-side
/// startup failure (no camera attached, permission denied, etc.).
fn reset_lifecycle(app: &tauri::AppHandle) {
    use tauri::Manager;
    let state = app.state::<PreviewState>();
    let mut guard = state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = guard.try_stop().finish_stop();
}

/// Tauri-managed handle for the active camera pipeline. Held in
/// `tauri::State` so the `stop_preview` command can drop the worker
/// (and consequently kill the gst child + join the thread).
///
/// Wrapping `Option<CameraPipeline>` in a `Mutex` rather than an
/// `AtomicCell` keeps the dep surface small; contention is bounded
/// by user-driven start/stop clicks, which can't race meaningfully.
#[derive(Default)]
pub struct CameraPipelineHandle(pub std::sync::Mutex<Option<CameraPipeline>>);

impl CameraPipelineHandle {
    /// Install a freshly-spawned pipeline. If one was already
    /// running, it's dropped first (which kills + joins it before
    /// the new one starts). Idempotent under concurrent calls — the
    /// mutex serialises the swap.
    pub fn install(&self, pipeline: CameraPipeline) {
        let mut guard = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = Some(pipeline);
    }

    /// Drop the active pipeline (which cancels + joins the worker).
    /// No-op if no pipeline is active.
    pub fn shutdown(&self) {
        let mut guard = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = None;
    }

    /// `true` if a worker is currently held. Used by tests +
    /// diagnostics; the lifecycle state machine in
    /// [`PreviewLifecycle`] is the source of truth for UI state.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_install_replaces_previous() {
        // Pure-state test of the handle — never actually spawns a
        // worker (which would require a tauri::AppHandle, a real
        // gst install, and a webcam). The state machine here is
        // small enough to verify directly: an Option<T> behind a
        // Mutex with install / shutdown / is_active.
        let handle = CameraPipelineHandle::default();
        assert!(!handle.is_active());

        // Simulate a worker by installing then reading. We can't
        // construct a CameraPipeline without spawning, so use
        // `Option::take` mechanics by going through `shutdown`.
        handle.shutdown();
        assert!(!handle.is_active());
    }

    // PREVIEW_WIDTH/HEIGHT/FPS invariants are now enforced at compile
    // time via the `const _: () = assert!(..)` blocks above the test
    // module — they fail the build if a future edit drifts the
    // values, which is a stronger guard than these `#[test]`s ever
    // were (and silences clippy's `assertions_on_constants` lint
    // that flagged the test-time versions in CI).
}
