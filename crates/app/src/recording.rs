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

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

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
        for s in [SessionState::Starting, SessionState::Running, SessionState::Stopping] {
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
        assert_eq!(SessionState::Stopping.mark_running(), SessionState::Stopping);
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
            SessionStreams { camera: true, ..Default::default() },
            SessionStreams { screen: true, ..Default::default() },
            SessionStreams { microphone: true, ..Default::default() },
            SessionStreams { system_audio: true, ..Default::default() },
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
