//! `EditorSession` — backend playhead state + transport for the editor
//! (ED.7 / M-EDIT).
//!
//! The editor's clock is the native [`EditorPlayer`] (a pure frame-indexed
//! clock). It lives here, in the Tauri backend, alongside the future
//! preview window; the webview is a thin transport UI that sends
//! [`TransportAction`]s and renders the returned [`EditorStatusView`].
//!
//! Ticking follows `EditorPlayer`/`Driver`'s "host injects dt" model: while
//! playing, the UI sends [`TransportAction::Tick`] from its animation loop
//! and the clock advances by that `dt`. (When the native preview window
//! lands it drives the same tick + renders the frame at `current_frame`.)

#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects State<'_, T> into #[command] fns by value; it is borrowed, not moved"
)]

use std::sync::Mutex;
use std::time::Duration;

use playback::EditorPlayer;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Serializable snapshot of the editor playhead for the transport UI.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct EditorStatusView {
    /// Current playhead frame.
    pub current_frame: u64,
    /// Total project length in frames.
    pub duration_frames: u64,
    /// Whether the clock is advancing.
    pub playing: bool,
    /// Project frame rate.
    pub fps: u32,
    /// Playback rate multiplier.
    pub rate: f32,
    /// In-point (inclusive).
    pub in_frame: u64,
    /// Out-point (exclusive).
    pub out_frame: u64,
    /// Whether looping over the in/out range is enabled.
    pub looping: bool,
}

/// A transport command from the UI. Enum-dispatched through the single
/// `editor_transport` command so registration stays a one-liner.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransportAction {
    /// Start advancing.
    Play,
    /// Stop advancing.
    Pause,
    /// Toggle play/pause.
    TogglePlay,
    /// Advance the clock by `dt_ms` milliseconds (the UI's per-frame tick).
    Tick {
        /// Elapsed milliseconds since the last tick.
        dt_ms: u32,
    },
    /// Seek to an exact frame.
    Seek {
        /// Target frame.
        frame: u64,
    },
    /// Step `delta` frames (negative = back) and pause.
    Step {
        /// Frame delta.
        delta: i64,
    },
    /// Set the playback rate.
    SetRate {
        /// New rate multiplier.
        rate: f32,
    },
    /// Set the in/out points (order-independent).
    SetInOut {
        /// One bound.
        a: u64,
        /// The other bound.
        b: u64,
    },
    /// Clear the in/out points to the full project.
    ClearInOut,
    /// Enable/disable looping.
    SetLooping {
        /// Loop flag.
        looping: bool,
    },
    /// Update the project length after an edit changed it (ED.11 ripple /
    /// trim) so the clock's range tracks the new timeline.
    SetDuration {
        /// New total project length in frames.
        frames: u64,
    },
    /// No-op — just read the current status.
    Status,
}

/// The editor's playhead clock for one open clip.
pub struct EditorSession {
    player: EditorPlayer,
}

impl EditorSession {
    /// A session for a project of `duration_frames` at `fps`.
    #[must_use]
    pub fn new(fps: u32, duration_frames: u64) -> Self {
        Self {
            player: EditorPlayer::new(fps, duration_frames),
        }
    }

    /// Apply a transport action and return the resulting status.
    pub fn apply(&mut self, action: TransportAction) -> EditorStatusView {
        match action {
            TransportAction::Play => self.player.play(),
            TransportAction::Pause => self.player.pause(),
            TransportAction::TogglePlay => self.player.toggle_play(),
            TransportAction::Tick { dt_ms } => {
                self.player.tick(Duration::from_millis(u64::from(dt_ms)));
            }
            TransportAction::Seek { frame } => self.player.seek(frame),
            TransportAction::Step { delta } => self.player.step(delta),
            TransportAction::SetRate { rate } => self.player.set_rate(rate),
            TransportAction::SetInOut { a, b } => self.player.set_in_out(a, b),
            TransportAction::ClearInOut => self.player.clear_in_out(),
            TransportAction::SetLooping { looping } => self.player.set_looping(looping),
            TransportAction::SetDuration { frames } => self.player.set_duration(frames),
            TransportAction::Status => {}
        }
        self.status()
    }

    /// Current playhead status.
    #[must_use]
    pub fn status(&self) -> EditorStatusView {
        EditorStatusView {
            current_frame: self.player.current_frame(),
            duration_frames: self.player.duration_frames(),
            playing: self.player.is_playing(),
            fps: self.player.fps(),
            rate: self.player.rate(),
            in_frame: self.player.in_frame(),
            out_frame: self.player.out_frame(),
            looping: self.player.looping(),
        }
    }
}

/// Tauri-managed editor session state. `None` until a clip is opened.
#[derive(Default)]
pub struct EditorSessionState(pub Mutex<Option<EditorSession>>);

impl std::fmt::Debug for EditorSessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorSessionState").finish_non_exhaustive()
    }
}

/// Apply a transport action to the active editor session. Returns `None`
/// when no clip is open.
#[tauri::command]
pub fn editor_transport(
    action: TransportAction,
    state: State<'_, EditorSessionState>,
) -> Option<EditorStatusView> {
    let mut guard = state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let session = guard.as_mut()?;
    Some(session.apply(action))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> EditorSession {
        EditorSession::new(30, 900)
    }

    #[test]
    fn play_pause_reflected_in_status() {
        let mut s = session();
        assert!(!s.status().playing);
        assert!(s.apply(TransportAction::Play).playing);
        assert!(!s.apply(TransportAction::Pause).playing);
        assert!(s.apply(TransportAction::TogglePlay).playing);
    }

    #[test]
    fn tick_advances_while_playing() {
        let mut s = session();
        s.apply(TransportAction::Play);
        let st = s.apply(TransportAction::Tick { dt_ms: 1000 }); // 1s @ 30fps
        assert_eq!(st.current_frame, 30);
    }

    #[test]
    fn seek_and_step() {
        let mut s = session();
        assert_eq!(
            s.apply(TransportAction::Seek { frame: 100 }).current_frame,
            100
        );
        assert_eq!(
            s.apply(TransportAction::Step { delta: 1 }).current_frame,
            101
        );
        let st = s.apply(TransportAction::Step { delta: -5 });
        assert_eq!(st.current_frame, 96);
        assert!(!st.playing, "stepping pauses");
    }

    #[test]
    fn rate_and_in_out_and_loop_in_status() {
        let mut s = session();
        let st = s.apply(TransportAction::SetRate { rate: 2.0 });
        assert!((st.rate - 2.0).abs() < 1e-6);
        let st = s.apply(TransportAction::SetInOut { a: 200, b: 100 });
        assert_eq!((st.in_frame, st.out_frame), (100, 200));
        assert!(
            s.apply(TransportAction::SetLooping { looping: true })
                .looping
        );
        let st = s.apply(TransportAction::ClearInOut);
        assert_eq!((st.in_frame, st.out_frame), (0, 900));
    }

    #[test]
    fn status_action_is_a_pure_read() {
        let mut s = session();
        s.apply(TransportAction::Seek { frame: 42 });
        let a = s.apply(TransportAction::Status);
        let b = s.apply(TransportAction::Status);
        assert_eq!(a, b);
        assert_eq!(a.current_frame, 42);
    }
}
