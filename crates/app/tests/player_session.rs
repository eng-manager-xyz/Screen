//! `PlayerSession` lifecycle tests — exercises the whole IPC backend
//! without going through Tauri's `State` wrapper.
//!
//! Each test pays the cost of booting a wisp `Application` (~200 ms on
//! Apple Silicon). Cases are grouped by lifecycle path to keep the total
//! runtime tight.

use std::path::Path;
use std::thread;
use std::time::Duration;

use screen_app::player_session::{PlayerSession, SessionState};

const FIXTURE: &str = "../decode/tests/fixtures/sample.mp4";

#[test]
fn empty_session_reports_empty() {
    let session = PlayerSession::new();
    let status = session.status();
    assert_eq!(status.state, SessionState::Empty);
    assert_eq!(status.elapsed_ms, 0);
    assert_eq!(status.duration_ms, None);
    assert_eq!(status.width, 0);
    assert_eq!(status.height, 0);
    assert!(
        status.fps.abs() < f32::EPSILON,
        "fps should be ~0, got {}",
        status.fps
    );
    assert_eq!(status.frame_count, None);
}

#[test]
fn open_transitions_to_paused_with_metadata() {
    assert!(Path::new(FIXTURE).exists(), "fixture missing: {FIXTURE}");
    let session = PlayerSession::new();
    let status = session.open(Path::new(FIXTURE)).expect("open fixture");
    assert_eq!(status.state, SessionState::Paused);
    assert_eq!(status.elapsed_ms, 0);
    // The M-DEC.1 fixture is 480×270 @ 30 fps.
    assert_eq!(status.width, 480);
    assert_eq!(status.height, 270);
    assert!(
        (status.fps - 30.0).abs() < 0.5,
        "expected ~30 fps, got {}",
        status.fps
    );
}

#[test]
fn play_pause_lifecycle() {
    let session = PlayerSession::new();
    session.open(Path::new(FIXTURE)).expect("open fixture");

    session.play();
    assert_eq!(session.status().state, SessionState::Playing);

    session.pause();
    assert_eq!(session.status().state, SessionState::Paused);

    session.play();
    assert_eq!(session.status().state, SessionState::Playing);
}

#[test]
fn tick_advances_elapsed_when_playing() {
    let session = PlayerSession::new();
    session.open(Path::new(FIXTURE)).expect("open fixture");
    session.play();

    thread::sleep(Duration::from_millis(80));
    let _frames = session.tick();
    let status = session.status();
    assert_eq!(status.state, SessionState::Playing);
    assert!(
        status.elapsed_ms >= 50,
        "expected elapsed >= 50ms after a 80ms sleep + tick, got {}",
        status.elapsed_ms
    );
}

#[test]
fn tick_is_noop_when_empty() {
    // Critical guard: the tick thread runs unconditionally, so a no-load
    // tick must never panic or affect anything observable.
    let session = PlayerSession::new();
    let frames = session.tick();
    assert_eq!(frames, 0);
    assert_eq!(session.status().state, SessionState::Empty);
}

#[test]
fn open_with_invalid_path_errors() {
    let session = PlayerSession::new();
    let err = session
        .open(Path::new("nonexistent-video-file-xyz123.mp4"))
        .expect_err("open should fail");
    assert!(!err.is_empty(), "error message should not be empty");
    // After a failed open, status remains Empty.
    assert_eq!(session.status().state, SessionState::Empty);
}
