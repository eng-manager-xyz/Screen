//! M-RECP.3 / AUT-264 — Display-keep-awake RAII guard.
//!
//! `KeepAwakeGuard` holds an OS-level assertion that prevents the
//! display from dimming / sleeping while the preview is active. On
//! drop the assertion releases automatically (RAII). Production
//! macOS implementation via `IOPMAssertion`, Windows via
//! `SetThreadExecutionState`, Linux via `org.freedesktop.ScreenSaver`
//! D-Bus inhibit — all **deferred** to a follow-up commit that
//! needs hardware verification.
//!
//! Today's implementation: a counter-only stub so the state-machine
//! tests verify the RAII contract on every OS. The
//! `PreviewSession::new_with_keep_awake` integration lives in
//! `crates/app/src/preview.rs` once the OS calls land.

use std::sync::atomic::{AtomicU32, Ordering};

/// Process-wide active-assertion count. Real OS assertions don't
/// stack but our stub does so the tests can assert on RAII drop.
static ACTIVE_COUNT: AtomicU32 = AtomicU32::new(0);

/// Snapshot of active keep-awake assertions across the process.
/// Useful for assertions in tests + for the `cleanup_smoke` test in
/// M-RECP.4 (no zombie assertions after app quit).
#[must_use]
pub fn active_assertions() -> u32 {
    ACTIVE_COUNT.load(Ordering::SeqCst)
}

/// RAII guard that holds the keep-awake assertion while alive.
///
/// Construct via [`KeepAwakeGuard::new`]; drop to release.
#[derive(Debug)]
pub struct KeepAwakeGuard {
    /// `true` once the OS assertion has been claimed. `false` after
    /// `release`. Drop-of-released is a no-op so `release` can be
    /// called explicitly.
    active: bool,
}

impl KeepAwakeGuard {
    /// Acquire a new keep-awake assertion. The real OS impl will fail
    /// if the assertion can't be created (rare); the stub today
    /// always succeeds.
    pub fn new() -> Self {
        ACTIVE_COUNT.fetch_add(1, Ordering::SeqCst);
        // M-RECP.3 OS calls land here:
        //   macOS: IOPMAssertionCreateWithName(kIOPMAssertionTypeNoDisplaySleep, ...)
        //   windows-rs: SetThreadExecutionState(ES_DISPLAY_REQUIRED | ES_CONTINUOUS)
        //   linux: ScreenSaver::Inhibit via zbus.
        Self { active: true }
    }

    /// Release the assertion explicitly (before Drop). Idempotent —
    /// repeated calls are safe.
    pub fn release(&mut self) {
        if self.active {
            ACTIVE_COUNT.fetch_sub(1, Ordering::SeqCst);
            self.active = false;
        }
    }
}

impl Default for KeepAwakeGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for KeepAwakeGuard {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialise these tests since they touch a process-wide static.
    static SERIAL: Mutex<()> = Mutex::new(());

    #[test]
    fn new_increments_count() {
        let _guard = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let baseline = active_assertions();
        let _g = KeepAwakeGuard::new();
        assert_eq!(active_assertions(), baseline + 1);
    }

    #[test]
    fn drop_decrements_count() {
        let _guard = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let baseline = active_assertions();
        {
            let _g = KeepAwakeGuard::new();
            assert_eq!(active_assertions(), baseline + 1);
        }
        assert_eq!(active_assertions(), baseline);
    }

    #[test]
    fn explicit_release_is_idempotent() {
        let _guard = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let baseline = active_assertions();
        let mut g = KeepAwakeGuard::new();
        assert_eq!(active_assertions(), baseline + 1);
        g.release();
        assert_eq!(active_assertions(), baseline);
        g.release(); // no-op
        assert_eq!(active_assertions(), baseline);
        // Drop after explicit release does NOT double-decrement.
        drop(g);
        assert_eq!(active_assertions(), baseline);
    }
}
