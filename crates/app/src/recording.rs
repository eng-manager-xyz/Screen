//! M-RECORD.0 — `RecordingSession` state machine + shared monotonic
//! clock for the coordinated recording lifecycle (M-RECORD-EXPORT).
//!
//! This module is **pure Rust** — no Tauri types, no I/O. It defines:
//!
//! - [`SessionState`] — `Idle → Starting → Running → Stopping → Idle`.
//!   Mirrors the per-channel `MicLifecycle` / `ScreenLifecycle` shape
//!   so the M-RECORD.2 LED renderer can reuse the same colour map.
//! - [`StreamKind`] — which of the four input streams (camera, screen,
//!   microphone, system audio) a [`StreamHealth`] refers to.
//! - [`StreamHealth`] — per-stream health snapshot (lifecycle +
//!   cumulative frame count + last-frame timestamp). Built fresh by
//!   M-RECORD.1's `recording_status` IPC every 500 ms.
//! - [`SessionStreams`] — which streams the user enabled for this
//!   session (boolean flags). Doesn't own the actual pipelines —
//!   those stay in their existing Tauri-managed `State<>` handles;
//!   the session just coordinates their lifecycles.
//! - [`RecordingSession`] — the orchestrator type itself. Wraps the
//!   four state pieces above into one immutable-once-started struct
//!   with a shared `started_at: Instant` clock used by the M-EXPORT
//!   encoder to compute per-frame PTS.
//!
//! ```admonish important title="What this commit ships vs. M-RECORD.1"
//! M-RECORD.0 lands the **types + state machine** only. The Tauri
//! `start_recording` / `stop_recording` / `recording_status` IPC and
//! the 500 ms event-push task that consumes this state live in
//! M-RECORD.1. Splitting the chunks keeps the state machine
//! unit-testable without Tauri's `AppHandle`.
//! ```

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use media::audio_mix::AudioMixer;
use media::encode::{GstreamerEncoder, VideoEncoder};
use serde::{Deserialize, Serialize};

/// Default audio mixer channel count (stereo). Matches the
/// `EncoderConfig` default + the SCK / mic worker output formats.
const DEFAULT_AUDIO_CHANNELS: u8 = 2;

/// Latest-frame-wins slot the capture pipelines write into and the
/// encoder feed thread reads from (M-PIX.0). `None` until the first
/// frame; the capture pipeline overwrites with each new frame; the
/// encoder reads (cloned) at render time.
pub type FrameSlot = Arc<Mutex<Option<Vec<u8>>>>;

/// Shared audio mixer (M-PIX.0) the mic worker + SCK audio
/// delegate push samples into, and the encoder feed thread drains
/// via `AudioMixer::pull()`.
pub type SharedAudioMixer = Arc<Mutex<AudioMixer>>;

/// Construct a fresh frame slot — `Arc<Mutex<None>>`.
#[must_use]
pub fn new_frame_slot() -> FrameSlot {
    Arc::new(Mutex::new(None))
}

/// Construct a fresh shared mixer with the default channel count.
#[must_use]
pub fn new_audio_mixer() -> SharedAudioMixer {
    Arc::new(Mutex::new(
        AudioMixer::new(DEFAULT_AUDIO_CHANNELS).expect("DEFAULT_AUDIO_CHANNELS > 0"),
    ))
}

/// Monotonically-increasing session id. Resets per process start;
/// the id only needs to be unique within a single app run so the
/// `recording-status` event consumer can ignore stale events from a
/// previous session.
static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

/// Master state of a [`RecordingSession`]. Mirrors the per-channel
/// `MicLifecycle` / `ScreenLifecycle` shape (and renders with the
/// same LED colour map in M-RECORD.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// No session active.
    #[default]
    Idle,
    /// `start_recording` invoked; per-channel pipelines being
    /// spawned. The session moves to `Running` once at least one
    /// enabled stream has reported its first frame.
    Starting,
    /// All enabled streams have produced at least one frame.
    Running,
    /// `stop_recording` invoked; per-channel pipelines being torn
    /// down. The session moves back to `Idle` after every enabled
    /// stream's lifecycle has reached its Idle state.
    Stopping,
}

impl SessionState {
    /// `Idle → Starting`; other states unchanged.
    #[must_use]
    pub fn try_start(self) -> Self {
        match self {
            Self::Idle => Self::Starting,
            other => other,
        }
    }

    /// `Starting → Running`; idempotent on `Running` (subsequent
    /// per-stream first-frame events don't re-trigger).
    #[must_use]
    pub fn mark_running(self) -> Self {
        match self {
            Self::Starting => Self::Running,
            other => other,
        }
    }

    /// `Starting | Running → Stopping`; `Idle | Stopping` unchanged.
    #[must_use]
    pub fn try_stop(self) -> Self {
        match self {
            Self::Running | Self::Starting => Self::Stopping,
            other => other,
        }
    }

    /// `Stopping → Idle`; other states unchanged.
    #[must_use]
    pub fn finish_stop(self) -> Self {
        match self {
            Self::Stopping => Self::Idle,
            other => other,
        }
    }
}

/// Which of the four input streams a [`StreamHealth`] describes.
/// Sent across the IPC seam so the Leptos `<RecorderControls />`
/// can colour the right LED.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamKind {
    /// Webcam capture (via gst `avfvideosrc` / `mfvideosrc` /
    /// `v4l2src`).
    Camera,
    /// Screen / window capture (via macOS `ScreenCaptureKit`).
    Screen,
    /// Microphone input (via gst `osxaudiosrc` / `wasapisrc` /
    /// `pulsesrc`).
    Microphone,
    /// System / per-application audio (via macOS SCK audio).
    SystemAudio,
}

/// Per-stream health snapshot for the `recording-status` event push.
/// Built fresh every 500 ms by M-RECORD.1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamHealth {
    /// Which input stream this snapshot describes.
    pub kind: StreamKind,
    /// One of `"Idle"` / `"Starting"` / `"Running"` / `"Stopping"`
    /// from the per-channel lifecycle enum. Kept as a string so
    /// this struct doesn't need to import all four per-channel
    /// enums.
    pub lifecycle: String,
    /// Cumulative frame / chunk count since the session started.
    pub frame_count: u64,
    /// Milliseconds since the last frame was observed. `None` if
    /// no frame has arrived yet (still in `Starting`). The LED
    /// colour ramp in M-RECORD.2 reads this directly:
    /// green &lt; 1000 ms, yellow &lt; 5000 ms, red otherwise.
    pub last_frame_ms_ago: Option<u64>,
}

/// Which streams the user enabled at session-start time. Doesn't
/// own the actual pipeline handles — those stay in their existing
/// Tauri-managed `State<>` wrappers; the session just remembers
/// which channels to start + stop together.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "Each bool maps to one of the four physical input channels (camera / screen / mic / system audio). They're inherently independent flags — a bitflag would be less readable across the IPC seam where Leptos consumes them as `{ camera: bool, screen: bool, ... }`."
)]
pub struct SessionStreams {
    /// `true` if the camera channel should participate in this
    /// session.
    pub camera: bool,
    /// `true` if the screen-capture channel should participate.
    pub screen: bool,
    /// `true` if the microphone channel should participate.
    pub microphone: bool,
    /// `true` if the system-audio channel should participate.
    pub system_audio: bool,
}

impl SessionStreams {
    /// `true` if at least one channel is enabled. M-RECORD.1's
    /// `start_recording` rejects sessions with no streams selected.
    #[must_use]
    pub fn any_enabled(self) -> bool {
        self.camera || self.screen || self.microphone || self.system_audio
    }

    /// Iterate over the enabled `StreamKind`s in canonical order
    /// (camera → screen → microphone → system audio). Used by
    /// M-RECORD.1's status assembler to walk the per-channel
    /// `State<>` handles in a deterministic order.
    pub fn enabled_kinds(self) -> impl Iterator<Item = StreamKind> {
        [
            (self.camera, StreamKind::Camera),
            (self.screen, StreamKind::Screen),
            (self.microphone, StreamKind::Microphone),
            (self.system_audio, StreamKind::SystemAudio),
        ]
        .into_iter()
        .filter_map(|(on, kind)| if on { Some(kind) } else { None })
    }
}

/// One coordinated recording session — the orchestrator owned by
/// `RecordingState` (M-RECORD.1) for the lifetime of one
/// start → stop cycle.
///
/// Construction is staged so the Tauri-side `start_recording`
/// command can do the heavy lifting (spawn per-channel pipelines,
/// roll back on per-stream failure) without mutating session state
/// mid-failure:
///
/// 1. `RecordingSession::starting(streams)` — allocates the session
///    id, captures `Instant::now()` as `started_at`, sets state to
///    `Starting`. No pipelines spawned yet.
/// 2. Caller spawns each enabled per-channel pipeline; on any
///    failure, calls `RecordingSession::abort()` and returns
///    `Err(...)`.
/// 3. Once all enabled streams have reported their first frame,
///    `RecordingSession::mark_running()` flips state to `Running`.
/// 4. On `stop_recording`, `RecordingSession::begin_stop()` flips
///    to `Stopping`; caller tears down per-channel pipelines.
/// 5. `RecordingSession::finish_stop()` flips back to `Idle`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordingSession {
    /// Unique-per-process session id. Stamped on every emitted
    /// `recording-status` event so a delayed event from a prior
    /// session can be filtered.
    pub id: u64,
    /// Shared monotonic clock — every per-frame PTS pushed into the
    /// M-EXPORT encoder is computed as `Instant::now() - started_at`.
    /// Captured ONCE here so all four streams share the same origin.
    pub started_at: Instant,
    /// Wall-clock start time (Unix epoch seconds). Complements the
    /// monotonic `started_at`; M-SAVE.1 carries it through to export
    /// so the `Screen-YYYY-MM-DD-HHMMSS.<ext>` filename reflects when
    /// the recording *started*, not when the user clicked Export.
    pub started_at_unix_secs: u64,
    /// Master lifecycle.
    pub state: SessionState,
    /// Which channels are part of this session.
    pub streams: SessionStreams,
}

impl RecordingSession {
    /// Begin a new session — allocates an id, captures the start
    /// time, sets state to `Starting`. Caller is responsible for
    /// spawning the enabled per-channel pipelines.
    #[must_use]
    pub fn starting(streams: SessionStreams) -> Self {
        Self {
            id: NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed),
            started_at: Instant::now(),
            started_at_unix_secs: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            state: SessionState::Starting,
            streams,
        }
    }

    /// Mark the session `Running`. Called by M-RECORD.1 once all
    /// enabled streams have produced their first frame. Idempotent.
    pub fn mark_running(&mut self) {
        self.state = self.state.mark_running();
    }

    /// Begin tearing down — `Starting | Running → Stopping`.
    /// Idempotent on `Stopping`; no-op on `Idle`.
    pub fn begin_stop(&mut self) {
        self.state = self.state.try_stop();
    }

    /// Finish teardown — `Stopping → Idle`. Idempotent on `Idle`;
    /// no-op on `Starting | Running` (the `begin_stop` step has to
    /// happen first).
    pub fn finish_stop(&mut self) {
        self.state = self.state.finish_stop();
    }

    /// Hard-abort the session — flip state straight to `Idle`
    /// regardless of where it was. Used by M-RECORD.1 to roll back
    /// when a per-channel start failed mid-Starting and the partial
    /// pipelines have been torn down.
    pub fn abort(&mut self) {
        self.state = SessionState::Idle;
    }

    /// Elapsed time since `started_at`. Used by M-RECORD.2's
    /// `mm:ss` display + by M-EXPORT's PTS math.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
}

/// Tauri-managed wrapper around the optional active session. Held
/// in `tauri::State` so the `start_recording` / `stop_recording` /
/// `recording_status` IPC commands + the 500 ms event-push task
/// share one source of truth. Mirror of `MicCaptureState` /
/// `ScreenCaptureState`.
///
/// Two slots:
/// - `session: Mutex<Option<RecordingSession>>` — the immutable
///   snapshot (id, `started_at`, state, streams).
/// - `encoder: Mutex<Option<EncoderHandle>>` — the live encoder +
///   test-pattern feed thread. Separated from `session` so the
///   500 ms `recording-status` event push can take a cheap clone
///   of the session snapshot without holding the encoder mutex.
pub struct RecordingState {
    /// Immutable per-session snapshot.
    pub session: Mutex<Option<RecordingSession>>,
    /// Active encoder + its background feed thread. M-EXPORT.3.
    pub encoder: Mutex<Option<EncoderHandle>>,
    /// Latest BGRA frame from the camera capture worker (M-PIX.0).
    /// Written by `crates/app/src/preview/pipeline.rs::run_pipeline`;
    /// consumed by M-PIX.5's compose thread.
    pub camera_frame_slot: FrameSlot,
    /// Latest BGRA frame from the SCK screen-capture delegate
    /// (M-PIX.0). Written by
    /// `crates/media/src/sck_video.rs::ScreenOutputHandler`;
    /// consumed by M-PIX.5's compose thread.
    pub screen_frame_slot: FrameSlot,
    /// Shared two-source mic + system-audio mixer (M-PIX.0). Mic
    /// worker calls `push_mic`; SCK audio delegate calls
    /// `push_sys_audio`; encoder feed thread calls `pull()`.
    pub audio_mixer: SharedAudioMixer,
    /// A finished recording sitting in scratch, awaiting the user's
    /// format choice in the Save panel (M-SAVE.1). Set by
    /// `stop_recording`; cleared by `export_recording` /
    /// `discard_recording`. `Some` here is the "awaiting export"
    /// signal the Save panel keys off — there is no `SessionState`
    /// variant for it (keeps the state-machine matches + LED colour
    /// map untouched).
    pub pending_export: Mutex<Option<PendingExport>>,
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            session: Mutex::new(None),
            encoder: Mutex::new(None),
            camera_frame_slot: new_frame_slot(),
            screen_frame_slot: new_frame_slot(),
            audio_mixer: new_audio_mixer(),
            pending_export: Mutex::new(None),
        }
    }
}

impl RecordingState {
    /// `true` if a session is currently held.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    }

    /// Snapshot of the active session, if any. Cloned so callers
    /// don't hold the mutex across the rest of their work.
    #[must_use]
    pub fn snapshot(&self) -> Option<RecordingSession> {
        self.session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Install a fresh encoder handle (M-EXPORT.3). Replaces any
    /// prior handle without finalising — caller is responsible for
    /// having finalised the previous session first.
    pub fn install_encoder(&self, handle: EncoderHandle) {
        let mut guard = self
            .encoder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = Some(handle);
    }

    /// Take the encoder handle out for finalisation. Returns `None`
    /// when no session was active.
    #[must_use]
    pub fn take_encoder(&self) -> Option<EncoderHandle> {
        self.encoder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }

    /// Stash a finished recording awaiting export (M-SAVE.1).
    pub fn set_pending_export(&self, pending: PendingExport) {
        let mut guard = self
            .pending_export
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = Some(pending);
    }

    /// Take the pending export out (for `export_recording` /
    /// `discard_recording`). Returns `None` when nothing is awaiting.
    #[must_use]
    pub fn take_pending_export(&self) -> Option<PendingExport> {
        self.pending_export
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }

    /// `true` while a finished recording is awaiting export. Used by
    /// `start_recording` to refuse starting a new session on top of
    /// an un-exported one (which would orphan its scratch file).
    #[must_use]
    pub fn has_pending_export(&self) -> bool {
        self.pending_export
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    }

    /// IPC snapshot of the pending export, if any (M-SAVE.1). Cloned
    /// so the caller doesn't hold the mutex.
    #[must_use]
    pub fn pending_export_view(&self) -> Option<PendingExportView> {
        self.pending_export
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(PendingExport::view)
    }
}

/// Live encoder + its background feed-thread cancel flag. Owned by
/// [`RecordingState::encoder`] for the duration of a session.
///
/// The M-EXPORT.3 v0 ships with a **test-pattern feed thread** —
/// pushes a solid-colour BGRA frame at 30 fps so the encoder
/// produces a real (trivial) `.mp4` file the user can verify the
/// orchestration end-to-end with. Real per-channel pixel forwarding
/// (extending camera/screen/SCK pipelines to push frames) is the
/// `M-EXPORT.3.1` follow-up.
pub struct EncoderHandle {
    /// Cooperative cancel flag for the feed thread.
    pub cancel: Arc<AtomicBool>,
    /// The encoder itself. Wrapped in `Mutex<Option<...>>` so the
    /// feed thread can push from its lock; `finalize` calls `take()`
    /// + invokes `Box::finalize` after the thread joins.
    pub encoder: Arc<Mutex<Option<Box<dyn VideoEncoder>>>>,
    /// Output path the encoder was configured to write to. Mirrored
    /// here so `stop_recording` can include it in `RecordingSummary`
    /// without re-querying.
    pub output_path: std::path::PathBuf,
    /// Background thread feeding the encoder. `Some` while active,
    /// `None` after `take_for_finalize()` removes it.
    pub feed_thread: Option<std::thread::JoinHandle<()>>,
}

impl std::fmt::Debug for EncoderHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncoderHandle")
            .field("output_path", &self.output_path)
            .field("feed_thread_alive", &self.feed_thread.is_some())
            .finish_non_exhaustive()
    }
}

impl EncoderHandle {
    /// Start the encoder + a real-capture feed thread that pulls
    /// composed frames from [`crate::recording_compose::RecordingCompose`]
    /// and mixed audio from [`SharedAudioMixer`] (M-PIX.6).
    ///
    /// The compose pump is constructed *inside* the feed thread —
    /// wisp's `Application` (wgpu Device + Queue) isn't `Send`
    /// across the worker boundary in all configurations, and
    /// constructing it on the thread that uses it avoids the
    /// question entirely.
    ///
    /// # Errors
    ///
    /// Returns the underlying encode error if the encoder can't
    /// be constructed (parent dir missing, scratch file open
    /// failure). Wisp init errors at compose-pump construction
    /// happen on the feed thread; logged but don't fail the
    /// handle creation.
    pub fn start_with_real_capture(
        encoder_config: media::encode::EncoderConfig,
        camera_slot: FrameSlot,
        screen_slot: FrameSlot,
        audio_mixer: SharedAudioMixer,
        screen_dims: wisp::recording::StreamDimensions,
        cam_dims: wisp::recording::StreamDimensions,
    ) -> Result<Self, media::encode::EncodeError> {
        crate::recording_paths::ensure_parent_dir(&encoder_config.output_path)
            .map_err(media::encode::EncodeError::Io)?;

        let width = encoder_config.width;
        let height = encoder_config.height;
        let framerate = encoder_config.framerate;
        let output_path = encoder_config.output_path.clone();

        let inner = GstreamerEncoder::new(encoder_config)?;
        let encoder: Arc<Mutex<Option<Box<dyn VideoEncoder>>>> =
            Arc::new(Mutex::new(Some(Box::new(inner))));
        let cancel = Arc::new(AtomicBool::new(false));

        let cancel_thread = Arc::clone(&cancel);
        let encoder_thread = Arc::clone(&encoder);
        let feed_thread = std::thread::Builder::new()
            .name(format!("recording-encoder-real-{width}x{height}"))
            .spawn(move || {
                feed_real_capture(
                    encoder_thread,
                    cancel_thread,
                    camera_slot,
                    screen_slot,
                    audio_mixer,
                    width,
                    height,
                    framerate,
                    screen_dims,
                    cam_dims,
                );
            })
            .map_err(|err| media::encode::EncodeError::Spawn {
                source: err,
                path: std::env::var("PATH").unwrap_or_else(|_| "<unset>".into()),
            })?;

        Ok(Self {
            cancel,
            encoder,
            output_path,
            feed_thread: Some(feed_thread),
        })
    }

    /// Start an encoder + spawn the test-pattern feed thread.
    /// Returns a handle whose `finalize_now()` produces the final
    /// container path.
    ///
    /// `palette_seed` controls the test-pattern colour (different
    /// per session id so multiple recordings produce visually
    /// distinct previews).
    pub fn start_with_test_pattern(
        encoder_config: media::encode::EncoderConfig,
        palette_seed: u64,
    ) -> Result<Self, media::encode::EncodeError> {
        // Ensure the parent dir of the output exists (M-EXPORT.4
        // helper). `EncoderConfig.output_path` is a file path.
        crate::recording_paths::ensure_parent_dir(&encoder_config.output_path)
            .map_err(media::encode::EncodeError::Io)?;

        let width = encoder_config.width;
        let height = encoder_config.height;
        let framerate = encoder_config.framerate;
        let channels = encoder_config.channels;
        let sample_rate = encoder_config.sample_rate;
        let output_path = encoder_config.output_path.clone();

        let inner = GstreamerEncoder::new(encoder_config)?;
        let encoder: Arc<Mutex<Option<Box<dyn VideoEncoder>>>> =
            Arc::new(Mutex::new(Some(Box::new(inner))));
        let cancel = Arc::new(AtomicBool::new(false));

        let cancel_thread = Arc::clone(&cancel);
        let encoder_thread = Arc::clone(&encoder);
        let feed_thread = std::thread::Builder::new()
            .name(format!("recording-encoder-feed-{palette_seed}"))
            .spawn(move || {
                feed_test_pattern(
                    encoder_thread,
                    cancel_thread,
                    width,
                    height,
                    framerate,
                    channels,
                    sample_rate,
                    palette_seed,
                );
            })
            .map_err(|err| media::encode::EncodeError::Spawn {
                source: err,
                path: std::env::var("PATH").unwrap_or_else(|_| "<unset>".into()),
            })?;

        Ok(Self {
            cancel,
            encoder,
            output_path,
            feed_thread: Some(feed_thread),
        })
    }

    /// Stop the feed thread and finalize the encoder. Returns the
    /// output path on success.
    ///
    /// # Errors
    ///
    /// Returns the underlying [`media::encode::EncodeError`] if the
    /// gst-launch subprocess fails.
    pub fn finalize_now(mut self) -> Result<std::path::PathBuf, media::encode::EncodeError> {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(handle) = self.feed_thread.take() {
            let _ = handle.join();
        }
        let encoder = self
            .encoder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        match encoder {
            Some(boxed) => boxed.finalize(),
            None => Ok(self.output_path),
        }
    }
}

/// Real-capture feed loop (M-PIX.6) — pulls composed frames from
/// [`crate::recording_compose::RecordingCompose`] + mixed audio
/// from the shared [`SharedAudioMixer`] until cancel fires.
///
/// Built inside the worker thread so wisp's wgpu Application
/// doesn't cross thread boundaries. If the compose pump fails to
/// init (no GPU adapter, etc.) the thread logs + exits cleanly —
/// the encoder still finalizes whatever audio happened to arrive.
#[allow(
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    reason = "arg list is the necessary capture-state surface; Arcs are taken by value because this fn IS the thread body."
)]
fn feed_real_capture(
    encoder: Arc<Mutex<Option<Box<dyn VideoEncoder>>>>,
    cancel: Arc<AtomicBool>,
    camera_slot: FrameSlot,
    screen_slot: FrameSlot,
    audio_mixer: SharedAudioMixer,
    width: u32,
    height: u32,
    framerate: u32,
    screen_dims: wisp::recording::StreamDimensions,
    cam_dims: wisp::recording::StreamDimensions,
) {
    let mut compose =
        match crate::recording_compose::RecordingCompose::new(width, height, screen_dims, cam_dims)
        {
            Ok(c) => c,
            Err(err) => {
                tracing::error!(
                    ?err,
                    "feed_real_capture: RecordingCompose::new failed; exiting"
                );
                return;
            }
        };

    let frame_interval = Duration::from_micros(1_000_000_u64 / u64::from(framerate.max(1)));
    let started_at = Instant::now();
    let mut next_pts_frames: u64 = 0;
    let mut frames_pushed: u64 = 0;
    let mut audio_chunks_pushed: u64 = 0;

    while !cancel.load(Ordering::Relaxed) {
        let pts =
            Duration::from_micros(next_pts_frames * (1_000_000_u64 / u64::from(framerate.max(1))));
        // Compose this tick. None means no real content has arrived
        // yet — skip the push so we don't pollute the .mp4 with
        // clear-colour frames before the first capture lands.
        let composed = compose.compose_frame(&camera_slot, &screen_slot);
        if let Some(frame) = composed {
            let mut guard = encoder
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Some(ref mut enc) = *guard else { break };
            if let Err(err) = enc.push_video_frame(&frame.bytes, pts) {
                tracing::warn!(?err, "feed_real_capture: push_video_frame failed");
                break;
            }
            frames_pushed = frames_pushed.saturating_add(1);
        }

        // Pull any mixed audio for this tick.
        let audio_samples = {
            let mut mixer_guard = audio_mixer
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            mixer_guard.pull()
        };
        if !audio_samples.is_empty() {
            let mut guard = encoder
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(ref mut enc) = *guard {
                if let Err(err) = enc.push_audio_chunk(&audio_samples, pts) {
                    tracing::warn!(
                        ?err,
                        "feed_real_capture: push_audio_chunk failed (continuing)"
                    );
                } else {
                    audio_chunks_pushed = audio_chunks_pushed.saturating_add(1);
                }
            }
        }

        next_pts_frames = next_pts_frames.saturating_add(1);
        let multiplier = u32::try_from(next_pts_frames).unwrap_or(u32::MAX);
        let target = started_at + frame_interval * multiplier;
        if let Some(sleep) = target.checked_duration_since(Instant::now()) {
            std::thread::sleep(sleep);
        }
    }
    tracing::info!(
        frames = frames_pushed,
        audio_chunks_pushed,
        "feed_real_capture: feed thread exiting"
    );
}

/// Test-pattern feed loop running on a dedicated thread. Pushes a
/// solid colour BGRA frame at the encoder's framerate (and a chunk
/// of silence at the audio sample rate) until cancel fires.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    reason = "test-pattern math: u64 seed → u8 channel via wrapping arithmetic; arg list is the necessary EncoderConfig surface for this self-contained feeder; Arc<...> args are taken by value because this fn IS the thread body — the caller move-spawns and the Arcs are dropped when the closure returns."
)]
fn feed_test_pattern(
    encoder: Arc<Mutex<Option<Box<dyn VideoEncoder>>>>,
    cancel: Arc<AtomicBool>,
    width: u32,
    height: u32,
    framerate: u32,
    channels: u8,
    sample_rate: u32,
    palette_seed: u64,
) {
    let frame_interval = Duration::from_micros(1_000_000_u64 / u64::from(framerate.max(1)));
    let bytes_per_frame = (width as usize) * (height as usize) * 4;
    let mut frame = vec![0u8; bytes_per_frame];
    // Solid colour from the palette seed (different per session).
    let b = (palette_seed.wrapping_mul(73) & 0xff) as u8;
    let g = (palette_seed.wrapping_mul(151) & 0xff) as u8;
    let r = (palette_seed.wrapping_mul(229) & 0xff) as u8;
    for px in frame.chunks_exact_mut(4) {
        px[0] = b;
        px[1] = g;
        px[2] = r;
        px[3] = 255;
    }
    // Silence chunk sized for one frame interval at the configured
    // audio caps.
    let samples_per_frame =
        (u64::from(sample_rate) / u64::from(framerate.max(1))) as usize * channels as usize;
    let silence = vec![0.0_f32; samples_per_frame];

    let started_at = Instant::now();
    let mut next_pts_frames: u64 = 0;
    while !cancel.load(Ordering::Relaxed) {
        let pts =
            Duration::from_micros(next_pts_frames * (1_000_000_u64 / u64::from(framerate.max(1))));
        {
            let mut guard = encoder
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(ref mut enc) = *guard {
                if let Err(err) = enc.push_video_frame(&frame, pts) {
                    tracing::warn!(?err, "feed_test_pattern: push_video_frame failed");
                    break;
                }
                if let Err(err) = enc.push_audio_chunk(&silence, pts) {
                    tracing::trace!(
                        ?err,
                        "feed_test_pattern: push_audio_chunk failed (continuing)"
                    );
                }
            } else {
                break;
            }
        }
        next_pts_frames = next_pts_frames.saturating_add(1);
        // Pace at framerate without drifting too badly under load.
        let target = started_at + frame_interval * (next_pts_frames as u32);
        if let Some(sleep) = target.checked_duration_since(Instant::now()) {
            std::thread::sleep(sleep);
        }
    }
    tracing::info!(
        frames = next_pts_frames,
        "feed_test_pattern: feed thread exiting"
    );
}

// ---- IPC view types (M-RECORD.1) ---------------------------------

/// `start_recording` argument. Carries which streams to enable +
/// the per-channel picker selections + the output target. M-EXPORT.4
/// extends `output_path` / `format` semantics; here they're carried
/// through unchanged so the IPC seam doesn't need to break later.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordingConfig {
    /// Which physical channels to coordinate.
    pub streams: SessionStreams,
    /// Camera picker selection — empty = OS default (M-CAM.4).
    #[serde(default)]
    pub camera_id: String,
    /// Microphone picker selection — empty = OS default (M-MIC.3).
    #[serde(default)]
    pub microphone_id: String,
    /// Screen-source picker selection — `None` / empty = primary
    /// display (M-SCK.0.1).
    #[serde(default)]
    pub screen_source_id: Option<String>,
    /// Output file path. `None` means "use the M-EXPORT.4 default
    /// location"; M-RECORD.1 just carries the string through.
    #[serde(default)]
    pub output_path: Option<String>,
    /// Output container/codec format slug (e.g. `"mp4-h264"`,
    /// `"webm-vp9"`). `None` means "use the default" — M-EXPORT.1
    /// owns the slug → `OutputFormat` mapping.
    #[serde(default)]
    pub format: Option<String>,
}

/// Snapshot of the active session for the `recording_status` IPC
/// + the 500 ms `recording-status` event push.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordingStatusView {
    /// `None` when no session is active.
    pub session_id: Option<u64>,
    /// Master lifecycle. `Idle` when no session is active.
    pub state: SessionState,
    /// Elapsed time in milliseconds since `started_at`. `0` when
    /// no session is active.
    pub elapsed_ms: u64,
    /// One entry per enabled stream.
    pub streams: Vec<StreamHealth>,
}

impl RecordingStatusView {
    /// Empty / no-session snapshot. Returned by `recording_status`
    /// when nothing is recording.
    #[must_use]
    pub fn idle() -> Self {
        Self {
            session_id: None,
            state: SessionState::Idle,
            elapsed_ms: 0,
            streams: Vec::new(),
        }
    }
}

/// Result of a successful `stop_recording`. Distinct from
/// `RecordingStatusView` because it's a final summary (no live
/// state) — fields the UI uses to show the post-record toast +
/// "Reveal in Finder" button (M-EXPORT.4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordingSummary {
    /// The session id that just stopped.
    pub session_id: u64,
    /// Total session duration in milliseconds (`started_at` →
    /// `stop_recording`).
    pub elapsed_ms: u64,
    /// Final per-stream tally.
    pub streams: Vec<StreamHealth>,
    /// Always `None` since M-SAVE.1 — stop defers the save, so
    /// `pending_export` carries the handoff instead.
    pub output_path: Option<String>,
    /// Set when the stopped recording is sitting in scratch awaiting
    /// the user's format choice (M-SAVE.1). The Save panel keys off
    /// this to appear; `None` only on the legacy/no-encoder path.
    #[serde(default)]
    pub pending_export: Option<PendingExportView>,
}

/// A finished recording sitting in the scratch directory, awaiting
/// the user's format choice in the Save panel (M-SAVE.1). Held in
/// [`RecordingState::pending_export`] between `stop_recording` and
/// `export_recording` / `discard_recording`. **Pure Rust** — no
/// Tauri / serde; the IPC-facing mirror is [`PendingExportView`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingExport {
    /// Absolute path to the finalized scratch file. Always MP4 / H.264
    /// — the canonical intermediate; the Save panel's format choice
    /// decides whether export moves it (MP4) or transcodes it (`WebM`).
    pub scratch_path: std::path::PathBuf,
    /// Recording duration in ms, frozen at stop.
    pub duration_ms: u64,
    /// Wall-clock start time (Unix seconds) — drives the exported
    /// `Screen-YYYY-MM-DD-HHMMSS.<ext>` filename.
    pub started_at_unix_secs: u64,
}

impl PendingExport {
    /// Project to the IPC mirror — duration + the suggested base
    /// filename (no extension), dropping the internal scratch path.
    #[must_use]
    pub fn view(&self) -> PendingExportView {
        PendingExportView {
            duration_ms: self.duration_ms,
            suggested_basename: crate::recording_paths::default_basename(self.started_at_unix_secs),
        }
    }
}

/// IPC mirror of [`PendingExport`]. Carries only what the Save panel
/// needs — never the internal scratch path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingExportView {
    /// Recording duration in ms.
    pub duration_ms: u64,
    /// Suggested base filename (no extension), e.g.
    /// `"Screen-2026-05-25-123700"`. The panel appends the extension
    /// matching the chosen format.
    pub suggested_basename: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- SessionState transitions ----

    #[test]
    fn state_default_is_idle() {
        assert_eq!(SessionState::default(), SessionState::Idle);
    }

    #[test]
    fn state_full_round_trip() {
        let mut s = SessionState::default();
        s = s.try_start();
        assert_eq!(s, SessionState::Starting);
        s = s.mark_running();
        assert_eq!(s, SessionState::Running);
        s = s.try_stop();
        assert_eq!(s, SessionState::Stopping);
        s = s.finish_stop();
        assert_eq!(s, SessionState::Idle);
    }

    #[test]
    fn state_re_entrant_start_is_noop() {
        for s in [
            SessionState::Starting,
            SessionState::Running,
            SessionState::Stopping,
        ] {
            assert_eq!(s.try_start(), s);
        }
    }

    #[test]
    fn state_mark_running_only_advances_from_starting() {
        assert_eq!(SessionState::Idle.mark_running(), SessionState::Idle);
        assert_eq!(SessionState::Starting.mark_running(), SessionState::Running);
        // Idempotent on Running — subsequent per-stream first-frame
        // events don't re-trigger.
        assert_eq!(SessionState::Running.mark_running(), SessionState::Running);
        assert_eq!(
            SessionState::Stopping.mark_running(),
            SessionState::Stopping
        );
    }

    #[test]
    fn state_stop_only_advances_from_starting_or_running() {
        assert_eq!(SessionState::Idle.try_stop(), SessionState::Idle);
        assert_eq!(SessionState::Starting.try_stop(), SessionState::Stopping);
        assert_eq!(SessionState::Running.try_stop(), SessionState::Stopping);
        assert_eq!(SessionState::Stopping.try_stop(), SessionState::Stopping);
    }

    #[test]
    fn state_finish_stop_only_advances_from_stopping() {
        assert_eq!(SessionState::Idle.finish_stop(), SessionState::Idle);
        assert_eq!(SessionState::Starting.finish_stop(), SessionState::Starting);
        assert_eq!(SessionState::Running.finish_stop(), SessionState::Running);
        assert_eq!(SessionState::Stopping.finish_stop(), SessionState::Idle);
    }

    #[test]
    fn state_serde_round_trip() {
        for v in [
            SessionState::Idle,
            SessionState::Starting,
            SessionState::Running,
            SessionState::Stopping,
        ] {
            let json = serde_json::to_string(&v).unwrap();
            let back: SessionState = serde_json::from_str(&json).unwrap();
            assert_eq!(back, v);
        }
    }

    // ---- SessionStreams ----

    #[test]
    fn streams_default_is_all_off() {
        let s = SessionStreams::default();
        assert!(!s.any_enabled());
        assert_eq!(s.enabled_kinds().count(), 0);
    }

    #[test]
    fn streams_any_enabled_true_when_any_field_on() {
        for s in [
            SessionStreams {
                camera: true,
                ..Default::default()
            },
            SessionStreams {
                screen: true,
                ..Default::default()
            },
            SessionStreams {
                microphone: true,
                ..Default::default()
            },
            SessionStreams {
                system_audio: true,
                ..Default::default()
            },
        ] {
            assert!(s.any_enabled());
        }
    }

    #[test]
    fn streams_enabled_kinds_walks_in_canonical_order() {
        let all_on = SessionStreams {
            camera: true,
            screen: true,
            microphone: true,
            system_audio: true,
        };
        let kinds: Vec<_> = all_on.enabled_kinds().collect();
        assert_eq!(
            kinds,
            vec![
                StreamKind::Camera,
                StreamKind::Screen,
                StreamKind::Microphone,
                StreamKind::SystemAudio,
            ]
        );
    }

    #[test]
    fn streams_enabled_kinds_filters_off_channels() {
        let cam_only = SessionStreams {
            camera: true,
            ..Default::default()
        };
        let kinds: Vec<_> = cam_only.enabled_kinds().collect();
        assert_eq!(kinds, vec![StreamKind::Camera]);
    }

    // ---- RecordingSession orchestrator ----

    #[test]
    fn session_starts_with_unique_increasing_ids() {
        let a = RecordingSession::starting(SessionStreams::default());
        let b = RecordingSession::starting(SessionStreams::default());
        let c = RecordingSession::starting(SessionStreams::default());
        assert!(b.id > a.id);
        assert!(c.id > b.id);
    }

    #[test]
    fn session_starting_state_is_starting() {
        let s = RecordingSession::starting(SessionStreams::default());
        assert_eq!(s.state, SessionState::Starting);
    }

    #[test]
    fn session_full_lifecycle_round_trip() {
        let mut s = RecordingSession::starting(SessionStreams {
            camera: true,
            microphone: true,
            ..Default::default()
        });
        assert_eq!(s.state, SessionState::Starting);
        s.mark_running();
        assert_eq!(s.state, SessionState::Running);
        s.begin_stop();
        assert_eq!(s.state, SessionState::Stopping);
        s.finish_stop();
        assert_eq!(s.state, SessionState::Idle);
    }

    #[test]
    fn session_abort_flips_state_to_idle_from_any_state() {
        for start_state in [
            SessionState::Starting,
            SessionState::Running,
            SessionState::Stopping,
        ] {
            let mut s = RecordingSession::starting(SessionStreams::default());
            s.state = start_state;
            s.abort();
            assert_eq!(s.state, SessionState::Idle);
        }
    }

    #[test]
    fn session_elapsed_is_monotonically_nondecreasing() {
        let s = RecordingSession::starting(SessionStreams::default());
        let first = s.elapsed();
        // Tiny busy-wait so the second sample is strictly after the
        // first on every clock granularity we care about.
        for _ in 0..10_000 {
            std::hint::spin_loop();
        }
        let second = s.elapsed();
        assert!(second >= first);
    }

    // ---- StreamHealth + StreamKind ----

    #[test]
    fn stream_kind_serde_round_trip() {
        for k in [
            StreamKind::Camera,
            StreamKind::Screen,
            StreamKind::Microphone,
            StreamKind::SystemAudio,
        ] {
            let json = serde_json::to_string(&k).unwrap();
            let back: StreamKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, k);
        }
    }

    #[test]
    fn stream_health_serde_round_trip() {
        let h = StreamHealth {
            kind: StreamKind::Camera,
            lifecycle: "Running".to_string(),
            frame_count: 1234,
            last_frame_ms_ago: Some(42),
        };
        let json = serde_json::to_string(&h).unwrap();
        let back: StreamHealth = serde_json::from_str(&json).unwrap();
        assert_eq!(back, h);
    }

    // ---- RecordingState wrapper ----

    // ---- M-PIX.0 slot + mixer plumbing ----

    #[test]
    fn new_frame_slot_starts_none() {
        let slot = new_frame_slot();
        assert!(slot.lock().unwrap().is_none());
    }

    #[test]
    fn frame_slot_latest_wins_on_overwrite() {
        let slot = new_frame_slot();
        *slot.lock().unwrap() = Some(vec![1, 2, 3]);
        *slot.lock().unwrap() = Some(vec![4, 5, 6, 7]);
        let read = slot.lock().unwrap().clone();
        assert_eq!(read, Some(vec![4, 5, 6, 7]));
    }

    #[test]
    fn frame_slot_take_clears() {
        let slot = new_frame_slot();
        *slot.lock().unwrap() = Some(vec![1, 2, 3]);
        let taken = slot.lock().unwrap().take();
        assert_eq!(taken, Some(vec![1, 2, 3]));
        assert!(slot.lock().unwrap().is_none());
    }

    #[test]
    fn frame_slot_is_arc_cheap_to_clone() {
        let slot = new_frame_slot();
        let clone1 = Arc::clone(&slot);
        let clone2 = Arc::clone(&slot);
        *clone1.lock().unwrap() = Some(vec![0xff; 16]);
        assert_eq!(clone2.lock().unwrap().as_ref().unwrap().len(), 16);
        assert_eq!(slot.lock().unwrap().as_ref().unwrap().len(), 16);
    }

    #[test]
    fn new_audio_mixer_has_stereo_channels() {
        let mixer = new_audio_mixer();
        assert_eq!(mixer.lock().unwrap().channels(), DEFAULT_AUDIO_CHANNELS);
        assert_eq!(mixer.lock().unwrap().channels(), 2);
    }

    #[test]
    fn shared_audio_mixer_is_arc_cheap_to_clone() {
        let mixer = new_audio_mixer();
        let writer_clone = Arc::clone(&mixer);
        let reader_clone = Arc::clone(&mixer);
        writer_clone.lock().unwrap().push_mic(&[0.1, 0.2]).unwrap();
        assert_eq!(reader_clone.lock().unwrap().mic_queued(), 2);
    }

    #[test]
    fn recording_state_default_wires_slots_and_mixer() {
        let state = RecordingState::default();
        assert!(state.camera_frame_slot.lock().unwrap().is_none());
        assert!(state.screen_frame_slot.lock().unwrap().is_none());
        assert_eq!(state.audio_mixer.lock().unwrap().channels(), 2);
    }

    #[test]
    fn recording_state_starts_inactive() {
        let s = RecordingState::default();
        assert!(!s.is_active());
        assert!(s.snapshot().is_none());
    }

    #[test]
    fn recording_state_holds_then_releases_session() {
        let s = RecordingState::default();
        {
            let mut guard = s.session.lock().unwrap();
            *guard = Some(RecordingSession::starting(SessionStreams {
                camera: true,
                ..Default::default()
            }));
        }
        assert!(s.is_active());
        let snap = s.snapshot().unwrap();
        assert_eq!(snap.state, SessionState::Starting);
        assert!(snap.streams.camera);

        // Clear it
        s.session.lock().unwrap().take();
        assert!(!s.is_active());
    }

    // ---- RecordingStatusView / RecordingSummary / RecordingConfig ----

    #[test]
    fn status_view_idle_is_empty() {
        let view = RecordingStatusView::idle();
        assert!(view.session_id.is_none());
        assert_eq!(view.state, SessionState::Idle);
        assert_eq!(view.elapsed_ms, 0);
        assert!(view.streams.is_empty());
    }

    #[test]
    fn status_view_serde_round_trip() {
        let view = RecordingStatusView {
            session_id: Some(7),
            state: SessionState::Running,
            elapsed_ms: 12_345,
            streams: vec![StreamHealth {
                kind: StreamKind::Camera,
                lifecycle: "Running".into(),
                frame_count: 90,
                last_frame_ms_ago: Some(33),
            }],
        };
        let json = serde_json::to_string(&view).unwrap();
        let back: RecordingStatusView = serde_json::from_str(&json).unwrap();
        assert_eq!(back, view);
    }

    #[test]
    fn summary_serde_round_trip() {
        let summary = RecordingSummary {
            session_id: 42,
            elapsed_ms: 10_000,
            streams: vec![],
            output_path: Some("/tmp/screen.mp4".into()),
            pending_export: None,
        };
        let json = serde_json::to_string(&summary).unwrap();
        let back: RecordingSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back, summary);
    }

    // ---- M-SAVE.1 pending-export state ----

    fn sample_pending() -> PendingExport {
        PendingExport {
            scratch_path: std::path::PathBuf::from("/tmp/scratch-7.mp4"),
            duration_ms: 8_000,
            started_at_unix_secs: 1_763_402_400,
        }
    }

    #[test]
    fn pending_export_set_take_round_trip() {
        let state = RecordingState::default();
        assert!(!state.has_pending_export());
        assert!(state.take_pending_export().is_none());

        state.set_pending_export(sample_pending());
        assert!(state.has_pending_export());

        let taken = state.take_pending_export().expect("pending present");
        assert_eq!(taken, sample_pending());
        // take() clears it — a second take yields None.
        assert!(!state.has_pending_export());
        assert!(state.take_pending_export().is_none());
    }

    #[test]
    fn pending_export_view_drops_scratch_path_and_strips_extension() {
        let view = sample_pending().view();
        assert_eq!(view.duration_ms, 8_000);
        assert_eq!(view.suggested_basename, "Screen-2025-11-17-180000");
        // The view exposes no scratch path; the basename has no ext.
        assert!(
            std::path::Path::new(&view.suggested_basename)
                .extension()
                .is_none()
        );
    }

    #[test]
    fn pending_export_view_matches_state_accessor() {
        let state = RecordingState::default();
        assert!(state.pending_export_view().is_none());
        state.set_pending_export(sample_pending());
        assert_eq!(state.pending_export_view(), Some(sample_pending().view()));
    }

    #[test]
    fn pending_export_view_serde_round_trip() {
        let view = sample_pending().view();
        let json = serde_json::to_string(&view).unwrap();
        let back: PendingExportView = serde_json::from_str(&json).unwrap();
        assert_eq!(back, view);
    }

    #[test]
    fn config_serde_round_trip_with_all_optionals() {
        let cfg = RecordingConfig {
            streams: SessionStreams {
                camera: true,
                microphone: true,
                ..Default::default()
            },
            camera_id: "cam-feedface".into(),
            microphone_id: "mic-cafebabe".into(),
            screen_source_id: Some("display-1".into()),
            output_path: Some("/tmp/test.mp4".into()),
            format: Some("mp4-h264".into()),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: RecordingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cfg);
    }

    #[test]
    fn config_serde_back_compat_when_optionals_absent() {
        // Frontend may omit any/all of the optional fields; the
        // #[serde(default)] on each must keep deserialization clean.
        let legacy =
            r#"{"streams":{"camera":true,"screen":false,"microphone":false,"system_audio":false}}"#;
        let parsed: RecordingConfig = serde_json::from_str(legacy).unwrap();
        assert!(parsed.streams.camera);
        assert_eq!(parsed.camera_id, "");
        assert_eq!(parsed.microphone_id, "");
        assert!(parsed.screen_source_id.is_none());
        assert!(parsed.output_path.is_none());
        assert!(parsed.format.is_none());
    }

    #[test]
    fn stream_health_handles_none_last_frame() {
        // Still in Starting, no frame yet — None must round-trip.
        let h = StreamHealth {
            kind: StreamKind::SystemAudio,
            lifecycle: "Starting".to_string(),
            frame_count: 0,
            last_frame_ms_ago: None,
        };
        let json = serde_json::to_string(&h).unwrap();
        let back: StreamHealth = serde_json::from_str(&json).unwrap();
        assert_eq!(back, h);
    }
}
