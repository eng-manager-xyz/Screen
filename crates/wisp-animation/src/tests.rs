//! Test suite for the `Animation` trait + `Driver`.
//!
//! Covers the three invariants from M-ANIM.0 / AUT-227:
//!
//! 1. **Determinism**: two `DriverMode::Fixed` drivers seeded
//!    identically produce equal samples for 1000 frames.
//! 2. **Pause/seek/scale round-trip**: explicit state mutations
//!    behave as advertised.
//! 3. **No-alloc on `tick`**: once a Driver is built, advancing it
//!    never allocates (validated by direct heap-counter check; see
//!    `tick_allocates_nothing`).

#![allow(
    unsafe_code,
    reason = "Test-only allocator that intercepts heap allocations to verify Driver::tick is alloc-free. The `unsafe impl GlobalAlloc` forwards to System verbatim."
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::{Animation, Driver, DriverMode, LinearRamp};

// ---------------------------------------------------------------------
// Allocation-counting global allocator
//
// Counts the number of `alloc` calls. Tests can take a baseline,
// run code, and assert the delta is zero. Implemented inline (no
// extra dep) — `Vec`s constructed in test setup happen *before*
// the baseline snapshot, so they don't affect the assertion.
// ---------------------------------------------------------------------

struct CountingAllocator;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        // SAFETY: forwarding to the system allocator with the
        // same layout the caller passed us.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: forwarding to the system allocator with the
        // same pointer and layout the caller passed us.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

// ---------------------------------------------------------------------
// Invariant 1: determinism
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
        // Realtime `dt` is *ignored* in Fixed mode; pass garbage
        // to prove the driver doesn't read it.
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
    d.tick(Duration::from_secs(100)); // outlandish — should be ignored
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
// Invariant 2: pause / seek / time_scale round-trip
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
    // 10 ticks × 100 ms × 2.0 = 2 s, × 0.5 = 500 ms.
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
    d.tick(Duration::ZERO); // elapsed = 2s — past anim.duration()
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
// Invariant 3: no-alloc on tick
// ---------------------------------------------------------------------

#[test]
fn tick_allocates_nothing() {
    let mut d = Driver::fixed(Duration::from_secs_f32(1.0 / 60.0));
    d.play();
    // Warm-up tick — ensures any lazy statics are initialised
    // before we take the baseline.
    d.tick(Duration::ZERO);

    let before = ALLOC_COUNT.load(Ordering::SeqCst);
    for _ in 0..1000 {
        d.tick(Duration::ZERO);
    }
    let after = ALLOC_COUNT.load(Ordering::SeqCst);
    assert_eq!(
        after - before,
        0,
        "Driver::tick allocated {} time(s) across 1000 ticks",
        after - before
    );
}

#[test]
fn sample_allocates_nothing() {
    let anim = LinearRamp::new(0.0, 100.0, Duration::from_secs(1));
    let mut d = Driver::fixed(Duration::from_secs_f32(1.0 / 60.0));
    d.play();
    d.tick(Duration::ZERO);

    let before = ALLOC_COUNT.load(Ordering::SeqCst);
    for _ in 0..1000 {
        let _ = d.sample(&anim);
        let _ = d.progress(&anim);
    }
    let after = ALLOC_COUNT.load(Ordering::SeqCst);
    assert_eq!(after - before, 0);
}

// ---------------------------------------------------------------------
// LinearRamp invariants (the only built-in animation in this ticket)
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
    // The Driver normally clamps, but `sample` itself must be safe
    // when called directly past the end.
    assert!((r.sample(Duration::from_secs(5)) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn driver_mode_default_is_realtime() {
    assert_eq!(DriverMode::default(), DriverMode::Realtime);
}
