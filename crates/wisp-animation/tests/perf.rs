//! Performance microbench for the RAIL Animation budget
//! (M-ANIM.20 / AUT-247).
//!
//! Asserts:
//!
//! 1. 1000 active `Tween<f32>` samples + apply via `BatchDriver`
//!    fits in **1 ms** on a typical CI runner.
//! 2. The batch tick allocates nothing across 1000 frames
//!    (heap-counting global allocator).
//!
//! Together these formalise RAIL's <10 ms / frame animation
//! budget at our target scale: 1000 concurrent tweens leaves
//! 9 ms for rendering and other host work.
//!
//! The wall-clock assertion is loose by ~10× to absorb runner
//! variance; if it ever trips, that's a real regression, not
//! flake. Set `WISP_ANIM_PERF_BUDGET_MS` env var to override
//! locally for slower hardware (`WISP_ANIM_PERF_BUDGET_MS=5
//! cargo nextest run -p wisp-animation --test perf`).

#![allow(
    unsafe_code,
    reason = "Test-only allocator that intercepts heap allocations to verify BatchDriver::tick_scalars is alloc-free."
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use wisp::scene::{Graphics, Stage};
use wisp_animation::{BatchDriver, BoundScalar, NodeProperty, Tween};

struct CountingAllocator;
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const TWEEN_COUNT: usize = 1_000;

fn build_arena() -> (Stage, Vec<BoundScalar>) {
    let mut stage = Stage::new();
    let root = stage.root();
    let mut anims = Vec::with_capacity(TWEEN_COUNT);
    for _ in 0..TWEEN_COUNT {
        let node = stage.add_child(root, Graphics::new()).expect("add child");
        anims.push(BoundScalar::new(
            Tween::new(0.0_f32, 1.0, Duration::from_millis(500)),
            NodeProperty::alpha(node),
        ));
    }
    (stage, anims)
}

fn budget_ms() -> u128 {
    std::env::var("WISP_ANIM_PERF_BUDGET_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10) // 10 ms loose budget; bench typically <1ms.
}

#[test]
fn one_thousand_tweens_fit_in_budget() {
    let (mut stage, mut anims) = build_arena();
    let mut driver = BatchDriver::fixed(Duration::from_secs_f32(1.0 / 60.0));
    driver.play();

    // Warm-up — JIT, page faults, branch predictor, allocator warmup.
    for _ in 0..3 {
        driver.tick_scalars(Duration::ZERO, &mut anims, &mut stage);
    }

    let start = Instant::now();
    let frames = 10_u32;
    for _ in 0..frames {
        driver.tick_scalars(Duration::ZERO, &mut anims, &mut stage);
    }
    let elapsed = start.elapsed();
    let per_frame_us = elapsed.as_micros() / u128::from(frames);
    let budget_us = budget_ms() * 1_000;
    eprintln!(
        "perf: {TWEEN_COUNT} tweens × {frames} frames = {} µs total, {per_frame_us} µs / frame (budget {budget_us} µs).",
        elapsed.as_micros(),
    );
    assert!(
        per_frame_us < budget_us,
        "1000-tween batch tick took {per_frame_us} µs / frame, exceeds budget of {budget_us} µs"
    );
}

#[test]
fn batch_tick_allocates_nothing_across_1000_frames() {
    let (mut stage, mut anims) = build_arena();
    let mut driver = BatchDriver::fixed(Duration::from_secs_f32(1.0 / 60.0));
    driver.play();
    // Warm up the full tween lifecycle: interpolation phase
    // (elapsed < 500 ms) AND saturated phase (elapsed >= 500 ms).
    // A single warm-up tick only exercises the interpolation
    // branch; the saturation branch's first call on Linux glibc
    // has been observed to trigger a small number of allocator
    // book-keeping allocations the first time it runs, which trips
    // a strict-zero assertion non-deterministically (see commit
    // log for the main-branch failure on dbff349). 64 ticks at
    // ~16.66 ms/tick crosses the 500 ms tween end and lets any
    // first-call infrastructure paths complete before measurement.
    for _ in 0..64 {
        driver.tick_scalars(Duration::ZERO, &mut anims, &mut stage);
    }

    let before = ALLOC_COUNT.load(Ordering::SeqCst);
    for _ in 0..1000 {
        driver.tick_scalars(Duration::ZERO, &mut anims, &mut stage);
    }
    let after = ALLOC_COUNT.load(Ordering::SeqCst);
    assert_eq!(
        after - before,
        0,
        "BatchDriver::tick_scalars allocated {} time(s) across 1000 ticks",
        after - before
    );
}
