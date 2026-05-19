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
use tauri::Manager;

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

/// Frames per `next_chunk` call. 2400 frames @ 48 kHz = 50 ms of
/// audio per chunk — gives the M-AUDIO.METER / AUT-287 audio-level
/// meter ~20 Hz update cadence (one chunk → one RMS sample → one
/// `mic-level` event). Previously 4800 (100 ms / 10 Hz); reduced for
/// meter smoothness without measurable IPC overhead.
pub const MIC_CHUNK_FRAMES: u64 = 2_400;

/// EMA smoothing factor for the mic level meter (M-AUDIO.METER /
/// AUT-287). Higher = more reactive to transients; lower = smoother.
/// 0.3 balances "responds visibly when you speak" against
/// "doesn't flicker on micro-pauses."
pub const MIC_LEVEL_EMA_ALPHA: f32 = 0.3;

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
    MIC_CHUNK_FRAMES == 2_400,
    "MIC_CHUNK_FRAMES must equal 50 ms @ MIC_SAMPLE_RATE for the M-AUDIO.METER 20 Hz update cadence"
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

    // M-AUDIO.METER / AUT-287 — running EMA-smoothed RMS so the
    // Leptos meter doesn't flicker on transients. Emitted to the
    // webview via the `mic-level` Tauri event on every chunk
    // (~20 Hz at MIC_CHUNK_FRAMES = 50 ms).
    let mut smoothed_level: f32 = 0.0;

    // M-PIX.3 — forward chunks into the shared AudioMixer so the
    // encoder feed thread (M-PIX.5/6) sees mic samples on its
    // pull. Defensive Option — None preserves the legacy
    // meter-only behaviour for tests + standalone preview.
    let mixer = app
        .try_state::<crate::recording::RecordingState>()
        .map(|s| crate::recording::SharedAudioMixer::clone(&s.audio_mixer));

    while !cancel.load(Ordering::Relaxed) {
        match capture.next_chunk(MIC_CHUNK_FRAMES) {
            Ok(chunk) => {
                advance_to_running(app);
                let raw_rms = chunk.rms();
                smoothed_level =
                    MIC_LEVEL_EMA_ALPHA * raw_rms + (1.0 - MIC_LEVEL_EMA_ALPHA) * smoothed_level;
                emit_mic_level(app, smoothed_level);
                // M-PIX.3 — feed the mixer. The mic worker emits
                // stereo F32LE matching the mixer's default
                // channel count; alignment is enforced by the
                // mixer.
                if let Some(ref mixer_arc) = mixer {
                    let mut mixer_guard = mixer_arc
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if let Err(err) = mixer_guard.push_mic(chunk.samples()) {
                        // Misalignment shouldn't happen — chunk is
                        // already validated — but warn instead of
                        // panic so a stray sample-count anomaly
                        // doesn't kill the worker.
                        tracing::warn!(?err, "AudioMixer::push_mic rejected chunk");
                    }
                }
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

/// Push the smoothed mic level to the webview via the `mic-level`
/// Tauri event (M-AUDIO.METER / AUT-287). Failures are swallowed +
/// `tracing::trace!`'d — at 20 Hz a missed emit is invisible, and
/// surfacing the error to the worker loop would break audio
/// capture for cosmetic event-bus issues.
fn emit_mic_level(app: &tauri::AppHandle, level: f32) {
    use tauri::Emitter;
    if let Err(err) = app.emit("mic-level", level) {
        tracing::trace!(?err, "emit mic-level failed");
    }
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
