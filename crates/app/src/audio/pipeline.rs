//! M-MIC.1 / AUT-278 — microphone-capture worker thread.
//!
//! Owns a dedicated OS thread that runs the `gst-launch-1.0
//! autoaudiosrc` capture subprocess and pulls PCM chunks into Rust.
//! Mirror of [`crate::preview::pipeline`] for the audio path —
//! same Drop-safety, cancel-flag-+-join lifecycle, idempotent
//! `mark_running` transition.
//!
//! ```admonish important title="What this commit ships"
//! Just the **gst-into-Rust** layer for microphone PCM:
//!
//! 1. `start_mic_capture(mic_id)` spawns a [`MicCapturePipeline`].
//! 2. Worker opens
//!    [`media::gstreamer_audio::GstreamerAudioCapture::from_microphone`],
//!    triggering the macOS `NSMicrophoneUsageDescription` prompt on
//!    first run.
//! 3. Worker loops `next_chunk()` and advances the
//!    [`MicLifecycle`](crate::audio::MicLifecycle) — `Starting →
//!    Running` on first successful chunk.
//! 4. `stop_mic_capture` drops the worker; `Drop` flips the cancel
//!    flag and joins the thread. The gst child is killed by
//!    `GstreamerAudioCapture`'s own `Drop` impl (per CLAUDE.md
//!    "Drop-kill the child" pattern).
//!
//! NOT yet shipped:
//!
//! * **Per-device selection** — `autoaudiosrc` always opens the OS
//!   default; the `mic_id` parameter is plumbed and logged but not
//!   yet used to pick a specific input (deferred to a follow-up;
//!   pattern is `osxaudiosrc device-uid=…` on macOS,
//!   `pulsesrc device=…` on Linux).
//! * **RMS event emission to Leptos** — the chunks are pulled and
//!   dropped. M-MIC.2 wires the `audio-levels` Tauri event when it
//!   needs the meter.
//! * **Encode path** — M-RECORD multiplexes mic PCM into the
//!   final encoded stream.
//! ```
//!
//! Thread-affinity contract — `GstreamerAudioCapture` owns a
//! `std::process::Child` + a `ChildStdout` reader. Both are `Send`,
//! so the worker thread can own the stream exclusively. No `Rc` /
//! `RefCell` anywhere in the type, safe to move into a spawned
//! thread.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use media::audio::AudioFormat;
use media::gstreamer_audio::GstreamerAudioCapture;

use super::{MicCaptureState, MicError};

/// Native sample rate the worker requests from gst. `audioresample`
/// converts on the input side if the device doesn't natively support
/// it. 48 kHz matches the recorder's encoder target + the
/// `M-MIC.0` device-enumeration default for `sample_rate_hz == 0`.
pub const MIC_SAMPLE_RATE: u32 = 48_000;

/// Native channel count the worker requests. 2 = stereo —
/// `audioconvert` upmixes mono inputs and downmixes higher-count
/// inputs cleanly. Matches the `M-MIC.0` device-enumeration default
/// for `channels == 0`.
pub const MIC_CHANNELS: u8 = 2;

/// Frames per `next_chunk` call. 4800 frames @ 48 kHz = 100 ms of
/// audio per chunk — a comfortable balance between IPC overhead
/// (one chunk-pull per 100 ms is cheap) and lifecycle responsiveness
/// (the `Starting → Running` transition fires within 100 ms of the
/// first PCM frame leaving gst).
pub const MIC_CHUNK_FRAMES: u64 = 4_800;

// Compile-time invariants for the mic-pipeline constants. These fire
// at compile time (zero runtime cost) and fail the build if a future
// edit drifts the values, instead of just failing a test — same
// pattern as `crate::preview::pipeline`'s `const _: () =
// assert!(..)` guards.
const _: () = assert!(
    MIC_SAMPLE_RATE == 48_000,
    "MIC_SAMPLE_RATE must be 48000 — the encoder + downstream resampler assume 48 kHz"
);
const _: () = assert!(
    MIC_CHANNELS == 2,
    "MIC_CHANNELS must be 2 — audioconvert handles mono/multi inputs but the chunk \
     layout downstream assumes interleaved stereo"
);
const _: () = assert!(
    MIC_CHUNK_FRAMES == 4_800,
    "MIC_CHUNK_FRAMES must equal 100 ms @ MIC_SAMPLE_RATE for the lifecycle responsiveness target"
);

/// Mic-pipeline worker handle. Owns the spawned thread and a
/// cooperative cancel flag; `Drop` cancels + joins so a panicking
/// caller can never leave a zombie gst child behind.
pub struct MicCapturePipeline {
    cancel: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl MicCapturePipeline {
    /// Spawn the worker thread. Returns immediately; the worker
    /// transitions [`MicLifecycle`] to `Running` once
    /// `next_chunk()` first succeeds (after any macOS permission
    /// prompt resolves).
    ///
    /// # Errors
    ///
    /// Returns `Err` only if the OS refuses to spawn a thread
    /// (effectively never happens). gst-side errors are handled
    /// inside the worker — the worker logs via `tracing::error!`
    /// and resets the lifecycle to `Idle` so the UI shows a recovery
    /// state.
    pub fn spawn(
        app: tauri::AppHandle,
        mic_id: String,
        native_id: String,
    ) -> Result<Self, MicError> {
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_thread = Arc::clone(&cancel);
        let handle = thread::Builder::new()
            .name("mic-capture".to_owned())
            .spawn(move || {
                run_pipeline(&app, &mic_id, &native_id, &cancel_for_thread);
            })
            .map_err(|err| MicError::GstFailed(format!("thread spawn failed: {err}")))?;
        Ok(Self {
            cancel,
            handle: Some(handle),
        })
    }
}

impl Drop for MicCapturePipeline {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            // Best-effort join; if the worker panicked we don't
            // care — we're tearing down.
            let _ = handle.join();
        }
    }
}

/// The actual worker loop. `&` borrows let the public-facing
/// `spawn` move cloned values onto the thread without keeping
/// `Self` alive on the thread.
fn run_pipeline(app: &tauri::AppHandle, mic_id: &str, native_id: &str, cancel: &AtomicBool) {
    let format = AudioFormat::stereo_f32(MIC_SAMPLE_RATE);
    let mut capture = match GstreamerAudioCapture::from_microphone(mic_id, native_id, format) {
        Ok(cap) => cap,
        Err(err) => {
            tracing::error!(?err, mic_id, native_id, "from_microphone failed");
            reset_lifecycle(app);
            return;
        }
    };
    tracing::info!(
        mic_id,
        native_id,
        sample_rate = MIC_SAMPLE_RATE,
        channels = MIC_CHANNELS,
        "mic-capture opened; awaiting first chunk"
    );

    while !cancel.load(Ordering::Relaxed) {
        match capture.next_chunk(MIC_CHUNK_FRAMES) {
            Ok(chunk) => {
                advance_to_running(app);
                // Chunk is otherwise unused here — the M-MIC.2
                // `audio-levels` event emitter + the M-RECORD encode
                // path consume it in follow-up commits.
                drop(chunk);
            }
            Err(err) => {
                tracing::warn!(?err, "next_chunk errored; tearing down");
                break;
            }
        }
    }

    tracing::info!("mic-capture cancel observed; shutting down");
    reset_lifecycle(app);
    // `capture` drops here → gst-launch child killed + reaped per
    // CLAUDE.md's "Drop-kill the child" pattern.
    drop(capture);
}

/// Mark the mic lifecycle as `Running` (idempotent — already-
/// `Running` stays running). Called on each successful chunk; the
/// `mark_running` transition is idempotent so we don't need a
/// "first chunk" guard.
fn advance_to_running(app: &tauri::AppHandle) {
    use tauri::Manager;
    let state = app.state::<MicCaptureState>();
    let mut guard = state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let next = guard.mark_running();
    if *guard != next {
        tracing::info!("mic lifecycle: Starting → Running (first chunk received)");
        *guard = next;
    }
}

/// Drive the lifecycle back to `Idle` on shutdown OR on gst-side
/// startup failure (no mic attached, permission denied, etc.).
fn reset_lifecycle(app: &tauri::AppHandle) {
    use tauri::Manager;
    let state = app.state::<MicCaptureState>();
    let mut guard = state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = guard.try_stop().finish_stop();
}

/// Tauri-managed handle for the active mic pipeline. Mirror of
/// [`crate::preview::CameraPipelineHandle`]. Wrapping
/// `Option<MicCapturePipeline>` in a `Mutex` rather than an
/// `AtomicCell` keeps the dep surface small; contention is bounded
/// by user start/stop clicks, which can't race meaningfully.
#[derive(Default)]
pub struct MicCaptureHandle(pub std::sync::Mutex<Option<MicCapturePipeline>>);

impl MicCaptureHandle {
    /// Install a freshly-spawned pipeline. If one was already
    /// running, it's dropped first (which kills + joins the previous
    /// worker before the new one starts). Idempotent under
    /// concurrent calls — the mutex serialises the swap.
    pub fn install(&self, pipeline: MicCapturePipeline) {
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
    /// diagnostics; [`MicLifecycle`] is the source of truth for UI
    /// state.
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
    fn handle_starts_inactive_and_shutdown_is_noop() {
        // Pure-state test — never actually spawns a worker (which
        // would require a tauri::AppHandle, a real gst install, and
        // a mic). Same shape as the M-CAM.3 handle smoke.
        let handle = MicCaptureHandle::default();
        assert!(!handle.is_active());

        // Shutdown on an empty handle must not panic.
        handle.shutdown();
        assert!(!handle.is_active());
    }

    // MIC_SAMPLE_RATE / MIC_CHANNELS / MIC_CHUNK_FRAMES invariants
    // are enforced at compile time via the `const _: () =
    // assert!(..)` blocks above — they fail the build if a future
    // edit drifts the values, which is a stronger guard than
    // `#[test]`s ever were.
}
