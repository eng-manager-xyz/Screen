//! Tauri-side player session — the shared state behind the IPC commands.
//!
//! Holds a long-lived [`Application`] (built once at app boot) and an
//! `Option<Inner>` carrying the live [`Player`] for the currently-open
//! video. `open` builds a fresh `Inner`; `play`/`pause`/`tick` mutate it.
//!
//! All public methods take `&self` and lock internally. The Tauri tick
//! thread and the IPC command handlers all share a single `PlayerSession`
//! through `tauri::State<PlayerSession>`.

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use decode::VideoStream;
use decode::gstreamer_pipe::GstreamerPipeStream;
use playback::{PlayState, Player};
use serde::{Deserialize, Serialize};
use wisp::application::{AppConfig, Application};

/// IPC-stable enum for the player's lifecycle. Mirrored on the frontend.
///
/// Distinct from [`playback::PlayState`] in that it carries an explicit
/// `Empty` variant for "no file loaded" — that's a Tauri-shell concern,
/// not a player-state-machine concern.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    /// No file is loaded. `open` transitions to `Paused`.
    Empty,
    /// A file is loaded; the player is not advancing.
    Paused,
    /// A file is loaded; `tick` advances `elapsed` and uploads frames.
    Playing,
    /// The stream reached EOF. `tick` is a no-op.
    Ended,
}

/// IPC payload emitted on the `player-status` event and returned from the
/// `player_open` / `player_status` commands.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerStatus {
    /// Lifecycle state — see [`SessionState`].
    pub state: SessionState,
    /// Wallclock elapsed since `play` was first called for this file, in
    /// milliseconds. Zero when paused at start; frozen across pause.
    pub elapsed_ms: u64,
    /// Total duration in milliseconds, when the decoder reports it.
    /// `None` for live streams or formats without a duration header.
    pub duration_ms: Option<u64>,
    /// Native frame width (pixels). `0` when no file is open.
    pub width: u32,
    /// Native frame height (pixels). `0` when no file is open.
    pub height: u32,
    /// Source frame rate (frames per second). `0.0` when no file is open.
    pub fps: f32,
    /// Total frame count when the decoder reports it. `None` for live
    /// streams or formats without a frame-count header.
    pub frame_count: Option<u64>,
}

impl PlayerStatus {
    fn empty() -> Self {
        Self {
            state: SessionState::Empty,
            elapsed_ms: 0,
            duration_ms: None,
            width: 0,
            height: 0,
            fps: 0.0,
            frame_count: None,
        }
    }
}

struct Inner {
    player: Player,
    last_tick: Instant,
    width: u32,
    height: u32,
    fps: f32,
    frame_count_hint: Option<u64>,
}

/// Tauri shell's player state. Shared via `tauri::State<PlayerSession>`.
pub struct PlayerSession {
    app: Application,
    inner: Mutex<Option<Inner>>,
}

impl PlayerSession {
    /// Boot the session — builds a [`wisp::application::Application`]
    /// up-front so subsequent `open` calls don't pay device-init latency.
    ///
    /// # Panics
    ///
    /// Panics if no compatible GPU adapter is available. The recorder
    /// can't function without one, so failing fast is preferable to
    /// surfacing a deferred error through the IPC layer.
    #[must_use]
    pub fn new() -> Self {
        let app = pollster::block_on(Application::new(AppConfig::default()))
            .expect("wisp::Application::new — no compatible adapter");
        Self {
            app,
            inner: Mutex::new(None),
        }
    }

    /// Open a video file. Replaces any currently-loaded session.
    ///
    /// Returns the fresh [`PlayerStatus`] (the new session begins in
    /// [`SessionState::Paused`]).
    ///
    /// # Errors
    ///
    /// Returns an error string if the file can't be opened (missing,
    /// unreadable, unsupported codec). The error is surfaced to the
    /// webview so the UI can show a toast.
    pub fn open(&self, path: &Path) -> Result<PlayerStatus, String> {
        let stream = GstreamerPipeStream::open(path).map_err(|e| e.to_string())?;
        let width = stream.width();
        let height = stream.height();
        let fps = stream.frame_rate();
        let frame_count_hint = stream.frame_count_hint();
        let player = Player::new(&self.app, Box::new(stream));

        let mut guard = self.inner.lock().expect("PlayerSession poisoned");
        *guard = Some(Inner {
            player,
            last_tick: Instant::now(),
            width,
            height,
            fps,
            frame_count_hint,
        });
        Ok(status_of(guard.as_ref()))
    }

    /// Transition to [`SessionState::Playing`]. No-op when empty or
    /// already playing.
    pub fn play(&self) {
        let mut guard = self.inner.lock().expect("PlayerSession poisoned");
        if let Some(inner) = guard.as_mut() {
            inner.player.play();
            inner.last_tick = Instant::now();
        }
    }

    /// Transition to [`SessionState::Paused`]. No-op when empty.
    pub fn pause(&self) {
        let mut guard = self.inner.lock().expect("PlayerSession poisoned");
        if let Some(inner) = guard.as_mut() {
            inner.player.pause();
        }
    }

    /// Drive the player by the wallclock delta since the last `tick`.
    ///
    /// Returns the number of frames the player uploaded. `0` when the
    /// session is empty, paused, ended, or no new frame was due.
    pub fn tick(&self) -> u32 {
        let mut guard = self.inner.lock().expect("PlayerSession poisoned");
        let Some(inner) = guard.as_mut() else {
            return 0;
        };
        let now = Instant::now();
        let dt = now.duration_since(inner.last_tick);
        inner.last_tick = now;
        inner.player.tick(&self.app, dt)
    }

    /// Snapshot of the current status. Cheap — no decoding work.
    #[must_use]
    pub fn status(&self) -> PlayerStatus {
        let guard = self.inner.lock().expect("PlayerSession poisoned");
        status_of(guard.as_ref())
    }
}

impl Default for PlayerSession {
    fn default() -> Self {
        Self::new()
    }
}

fn status_of(inner: Option<&Inner>) -> PlayerStatus {
    let Some(inner) = inner else {
        return PlayerStatus::empty();
    };
    let state = match inner.player.state() {
        PlayState::Paused => SessionState::Paused,
        PlayState::Playing => SessionState::Playing,
        PlayState::Ended => SessionState::Ended,
    };
    PlayerStatus {
        state,
        elapsed_ms: ms_from_duration(inner.player.elapsed()),
        duration_ms: inner.player.duration_hint().map(ms_from_duration),
        width: inner.width,
        height: inner.height,
        fps: inner.fps,
        frame_count: inner.frame_count_hint,
    }
}

fn ms_from_duration(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}
