//! Test suite for the `Animation` trait + `Driver`.
//!
//! Covers the determinism + state-mutation invariants from
//! M-ANIM.0 / AUT-227. The no-alloc invariants moved out to
//! `tests/no_alloc.rs` so they own their own test binary's
//! heap-counting global allocator without interference from
//! sibling tests.

use std::time::Duration;

use crate::{Animation, Driver, DriverMode, LinearRamp};

// ---------------------------------------------------------------------
// Determinism — two fixed-step drivers produce equal sample streams.
// ---------------------------------------------------------------------

#[test]
fn fixed_driver_is_deterministic_across_1000_frames() {
    let dt = Duration::from_secs_f32(1.0 / 60.0);
    let mut left = Driver::fixed(dt);
    let mut right = Driver::fixed(dt);
    left.play();
    right.play();

    let anim = LinearRamp::new(0.0, 100.0, Duration::from_secs(10));

    let mut left_samples = Vec::with_capacity(1000);
    let mut right_samples = Vec::with_capacity(1000);
    for _ in 0..1000 {
        left.tick(Duration::from_secs(999));
        right.tick(Duration::from_millis(1));
        left_samples.push(left.sample(&anim));
        right_samples.push(right.sample(&anim));
    }
    assert_eq!(left_samples, right_samples);
}

#[test]
fn fixed_driver_ignores_callers_dt() {
    let dt = Duration::from_secs_f32(1.0 / 60.0);
    let mut d = Driver::fixed(dt);
    d.play();
    let before = d.elapsed();
    d.tick(Duration::from_secs(100));
    assert_eq!(d.elapsed(), before + dt);
}

#[test]
fn realtime_driver_uses_callers_dt() {
    let mut d = Driver::realtime();
    d.play();
    let step = Duration::from_millis(33);
    d.tick(step);
    d.tick(step);
    assert_eq!(d.elapsed(), step * 2);
}

// ---------------------------------------------------------------------
// Pause / seek / time_scale round-trip.
// ---------------------------------------------------------------------

#[test]
fn pause_freezes_elapsed() {
    let mut d = Driver::fixed(Duration::from_millis(16));
    d.play();
    d.tick(Duration::ZERO);
    let frozen_at = d.elapsed();
    d.pause();
    d.tick(Duration::ZERO);
    d.tick(Duration::ZERO);
    d.tick(Duration::ZERO);
    assert_eq!(d.elapsed(), frozen_at);
}

#[test]
fn seek_jumps_clock_without_changing_playing_flag() {
    let mut d = Driver::fixed(Duration::from_millis(16));
    assert!(!d.is_playing());
    d.seek(Duration::from_secs(5));
    assert_eq!(d.elapsed(), Duration::from_secs(5));
    assert!(!d.is_playing());

    d.play();
    assert!(d.is_playing());
    d.seek(Duration::ZERO);
    assert_eq!(d.elapsed(), Duration::ZERO);
    assert!(d.is_playing());
}

#[test]
fn time_scale_doubles_step() {
    let dt = Duration::from_millis(100);
    let mut fast = Driver::fixed(dt);
    let mut slow = Driver::fixed(dt);
    fast.set_time_scale(2.0);
    slow.set_time_scale(0.5);
    fast.play();
    slow.play();
    for _ in 0..10 {
        fast.tick(Duration::ZERO);
        slow.tick(Duration::ZERO);
    }
    assert_eq!(fast.elapsed(), Duration::from_secs(2));
    assert_eq!(slow.elapsed(), Duration::from_millis(500));
}

#[test]
fn negative_time_scale_clamps_to_zero() {
    let mut d = Driver::fixed(Duration::from_millis(16));
    d.set_time_scale(-1.5);
    assert!(d.time_scale().abs() < f32::EPSILON);
    d.play();
    d.tick(Duration::ZERO);
    assert_eq!(d.elapsed(), Duration::ZERO);
}

#[test]
fn sample_clamps_past_end() {
    let anim = LinearRamp::new(0.0, 1.0, Duration::from_secs(1));
    let mut d = Driver::fixed(Duration::from_secs(2));
    d.play();
    d.tick(Duration::ZERO);
    assert!((d.sample(&anim) - 1.0).abs() < f32::EPSILON);
    assert!((d.progress(&anim) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn progress_returns_one_on_zero_duration() {
    let zero = LinearRamp::new(0.0, 1.0, Duration::ZERO);
    let d = Driver::fixed(Duration::from_millis(16));
    assert!((d.progress(&zero) - 1.0).abs() < f32::EPSILON);
}

// ---------------------------------------------------------------------
// LinearRamp invariants.
// ---------------------------------------------------------------------

#[test]
fn linear_ramp_hits_endpoints() {
    let r = LinearRamp::new(10.0, 20.0, Duration::from_secs(1));
    assert!((r.sample(Duration::ZERO) - 10.0).abs() < f32::EPSILON);
    assert!((r.sample(Duration::from_secs(1)) - 20.0).abs() < f32::EPSILON);
}

#[test]
fn linear_ramp_midpoint() {
    let r = LinearRamp::new(0.0, 1.0, Duration::from_secs(1));
    assert!((r.sample(Duration::from_millis(500)) - 0.5).abs() < 1e-6);
}

#[test]
fn linear_ramp_clamps_outside_window() {
    let r = LinearRamp::new(0.0, 1.0, Duration::from_secs(1));
    assert!((r.sample(Duration::from_secs(5)) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn driver_mode_default_is_realtime() {
    assert_eq!(DriverMode::default(), DriverMode::Realtime);
}
