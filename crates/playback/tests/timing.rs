//! Player timing contract.
//!
//! These tests prove the state machine behaves under the wall-clock contract
//! the shell will rely on: paused → no advance, playing → frames pulled at
//! the source frame rate, end-of-stream → `Ended` state. Mock-stream-driven
//! so they need a wgpu device but no real codec / file I/O.

use std::time::Duration;

use decode::mock::MockVideoStream;
use playback::{PlayState, Player};
use pollster::block_on;
use wisp::application::{AppConfig, Application};

fn app() -> Application {
    block_on(Application::new(AppConfig::default())).expect("Application::new")
}

#[test]
fn paused_player_does_not_advance() {
    let app = app();
    let stream = MockVideoStream::scrolling_gradient(64, 36, 10);
    let mut player = Player::new(&app, Box::new(stream));
    assert_eq!(player.state(), PlayState::Paused);

    let pulled = player.tick(&app, Duration::from_secs(1));
    assert_eq!(pulled, 0);
    assert_eq!(player.elapsed(), Duration::ZERO);
    assert!(player.last_uploaded_index().is_none());
}

#[test]
fn first_tick_uploads_first_frame_immediately() {
    let app = app();
    let stream = MockVideoStream::scrolling_gradient(64, 36, 10);
    let mut player = Player::new(&app, Box::new(stream));
    player.play();

    // Even a tiny dt must catch the t=0 frame, which is due at t=0.
    let pulled = player.tick(&app, Duration::from_millis(1));
    assert_eq!(pulled, 1);
    assert_eq!(player.last_uploaded_index(), Some(0));
}

#[test]
fn one_second_at_30fps_pulls_30_frames() {
    let app = app();
    // 30 fps, 60 total — gives us a full second of pulls plus headroom.
    let stream = MockVideoStream::scrolling_gradient(64, 36, 60).with_frame_rate(30.0);
    let mut player = Player::new(&app, Box::new(stream));
    player.play();

    let mut total = 0u32;
    // Drive at 60Hz render rate (16.67ms ticks) for one second.
    let dt = Duration::from_micros(16_667);
    for _ in 0..60 {
        total += player.tick(&app, dt);
    }
    // Source is 30 fps, so one wallclock second should yield 30 frames
    // (give or take one because of rounding).
    assert!(
        (29..=31).contains(&total),
        "expected ~30 frames in 1s at 30 fps, got {total}"
    );
    assert_eq!(player.state(), PlayState::Playing);
}

#[test]
fn stream_end_transitions_to_ended() {
    let app = app();
    let stream = MockVideoStream::scrolling_gradient(8, 8, 3).with_frame_rate(30.0);
    let mut player = Player::new(&app, Box::new(stream));
    player.play();

    // 1s at 30 fps would want ~30 frames but stream only has 3.
    let _ = player.tick(&app, Duration::from_secs(1));
    assert_eq!(player.state(), PlayState::Ended);
    assert_eq!(player.last_uploaded_index(), Some(2));
}

#[test]
fn pause_freezes_state_and_texture() {
    let app = app();
    let stream = MockVideoStream::scrolling_gradient(8, 8, 30);
    let mut player = Player::new(&app, Box::new(stream));
    player.play();
    let _ = player.tick(&app, Duration::from_millis(100));
    let frozen_idx = player.last_uploaded_index();
    let frozen_elapsed = player.elapsed();

    player.pause();
    let _ = player.tick(&app, Duration::from_secs(5));
    assert_eq!(player.last_uploaded_index(), frozen_idx);
    assert_eq!(player.elapsed(), frozen_elapsed);
}

#[test]
fn duration_hint_reflects_stream_metadata() {
    let app = app();
    let stream = MockVideoStream::scrolling_gradient(8, 8, 60).with_frame_rate(30.0);
    let player = Player::new(&app, Box::new(stream));
    let dur = player.duration_hint().expect("hint");
    assert!((dur.as_secs_f64() - 2.0).abs() < 1e-9);
}
