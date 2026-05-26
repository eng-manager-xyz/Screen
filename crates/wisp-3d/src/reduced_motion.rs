//! `reduced_motion` — `prefers-reduced-motion` detector (W3D.7 /
//! AUT-299).
//!
//! ## Why this lives in wisp-3d
//!
//! Time-driven shaders (palette ramps, the spinning pyramid) need
//! a hook to freeze animation when the user has set
//! `prefers-reduced-motion: reduce` at the OS / browser level.
//! `wisp-3d` itself doesn't own the clock — that's
//! `wisp-animation::Driver`'s job — but the OS-query bit is
//! 3D-renderer-shaped because the consumer is the 3D scene's tick
//! loop. Lives here so the engmanager.xyz 404 port has one import
//! site for both the shader pipeline and the reduced-motion check.
//!
//! ## Integration with `wisp-animation::Driver`
//!
//! ```text
//! let mut driver = wisp_animation::Driver::realtime();
//! if wisp_3d::reduced_motion::detect_via_media_query() {
//!     driver.pause(); // Frozen at t = 0 — no clock advance.
//! }
//! ```
//!
//! Snippet is `text` (not `rust`) so the doctest doesn't pull
//! `wisp-animation` into `wisp-3d`'s dependency graph. Consumers
//! that DO depend on both (e.g. `wisp-3d-web`) write this directly.
//!
//! ## Platform behaviour
//!
//! - **wasm32**: queries `window.matchMedia("(prefers-reduced-motion:
//!   reduce)").matches` via `web-sys`. Returns `false` on browsers
//!   that don't expose the media query (older Safari < 10.1) so we
//!   default to "animation enabled".
//! - **Native**: stub returns `false`. Native consumers can override
//!   via [`set_reduced_motion_override`] (used in tests).

#[cfg(target_arch = "wasm32")]
mod web {
    /// Live `prefers-reduced-motion: reduce` check via
    /// `window.matchMedia`. Returns `false` if the browser doesn't
    /// expose `matchMedia` (very old Safari).
    #[must_use]
    pub fn detect_via_media_query() -> bool {
        let Some(window) = web_sys::window() else {
            return false;
        };
        match window.match_media("(prefers-reduced-motion: reduce)") {
            Ok(Some(mql)) => mql.matches(),
            _ => false,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use web::detect_via_media_query;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::sync::atomic::{AtomicBool, Ordering};

    static OVERRIDE: AtomicBool = AtomicBool::new(false);

    /// Native stub. Returns the value set by
    /// [`super::set_reduced_motion_override`] (default `false`).
    /// Native consumers that genuinely want to honour the OS
    /// preference should wire it to whatever platform crate fits
    /// (`objc2` for `AppKit`, `windows-rs` for Windows, `GSettings`
    /// for GTK) and forward via the override.
    #[must_use]
    pub fn detect_via_media_query() -> bool {
        OVERRIDE.load(Ordering::Relaxed)
    }

    /// Set the value returned by [`detect_via_media_query`] on
    /// native builds. No-op on wasm32. Tests rely on this; native
    /// consumers can forward an OS-level preference here.
    pub fn set_reduced_motion_override(value: bool) {
        OVERRIDE.store(value, Ordering::Relaxed);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::{detect_via_media_query, set_reduced_motion_override};

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;

    #[test]
    fn native_default_is_false() {
        // Save / restore the override so test order doesn't matter.
        let prior = detect_via_media_query();
        set_reduced_motion_override(false);
        assert!(!detect_via_media_query());
        set_reduced_motion_override(prior);
    }

    #[test]
    fn native_override_round_trips() {
        let prior = detect_via_media_query();
        set_reduced_motion_override(true);
        assert!(detect_via_media_query());
        set_reduced_motion_override(false);
        assert!(!detect_via_media_query());
        set_reduced_motion_override(prior);
    }
}
