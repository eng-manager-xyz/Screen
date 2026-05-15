//! No-alloc invariants for `Driver::tick` and `Animation::sample`.
//!
//! Lives in its own integration-test binary so nothing else
//! shares the heap-counting global allocator — which means the
//! counter delta we measure is only this test's own work, not
//! parallel test-runner traffic.

#![allow(
    unsafe_code,
    reason = "Test-only allocator that intercepts heap allocations to verify per-frame work is alloc-free."
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use wisp_animation::{Driver, LinearRamp};

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

#[test]
fn tick_allocates_nothing() {
    let mut d = Driver::fixed(Duration::from_secs_f32(1.0 / 60.0));
    d.play();
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
        let _ = wisp_animation::Animation::sample(&anim, d.elapsed());
    }
    let after = ALLOC_COUNT.load(Ordering::SeqCst);
    assert_eq!(after - before, 0);
}
