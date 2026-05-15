//! `wisp-animation` — pure time→state animation primitives for
//! `wisp` and `wisp-chart`.
//!
//! # Headline architectural decision
//!
//! **`Animation` is a value; `Driver` owns the clock.** An
//! [`Animation`] is a *pure function from time to state*: ask it for
//! its [`duration`](Animation::duration), then [`sample`](Animation::sample)
//! it at any instant. The animation itself holds no playback state, no
//! `Instant`, no `Duration::ZERO` cursor. Two callers can sample the
//! same animation value at different times without contention.
//!
//! Time advances through a separate [`Driver`]. The driver owns the
//! elapsed clock, the playback rate, and the play/pause flag. It runs
//! in either [`DriverMode::Realtime`] (caller injects real-time `dt`
//! from a `winit` loop or `requestAnimationFrame`) or
//! [`DriverMode::Fixed`] (caller injects a fixed `dt` for
//! reproducible offline export — see `wisp-export-animated`).
//!
//! Both modes call the same `Animation::sample(t)`. That's the whole
//! determinism story: same animation, same `t`, same output, regardless
//! of platform, scheduler, or whether you're rendering live or to MP4.
//!
//! # Minimal example
//!
//! ```
//! use std::time::Duration;
//! use wisp_animation::{Animation, Driver, DriverMode};
//!
//! /// A 1-second linear ramp from 0.0 → 1.0.
//! struct UnitRamp;
//!
//! impl Animation for UnitRamp {
//!     type Output = f32;
//!     fn duration(&self) -> Duration { Duration::from_secs(1) }
//!     fn sample(&self, t: Duration) -> f32 {
//!         (t.as_secs_f32() / self.duration().as_secs_f32()).clamp(0.0, 1.0)
//!     }
//! }
//!
//! // Fixed-step driver: 60 fps, deterministic across runs.
//! let mut driver = Driver::fixed(Duration::from_secs_f32(1.0 / 60.0));
//! driver.play();
//! for _ in 0..30 {
//!     driver.tick(Duration::ZERO);
//! }
//! assert!((driver.sample(&UnitRamp) - 0.5).abs() < 0.01);
//! ```
//!
//! See the [wisp-animation book](https://eng-manager-xyz.github.io/Screen/wisp-animation/)
//! for a per-feature tour with live WebGPU demos.

use std::time::Duration;

pub mod animatable;
pub mod color_space;
pub mod composition;
pub mod ease;
pub mod flip;
pub mod lifecycle;
pub mod move_along_path;
pub mod path_morph;
pub mod repeat;
pub mod spring;
pub mod stagger;
pub mod target;
pub mod theme;
pub mod track;
pub mod tween;
pub mod typewriter;

pub use animatable::Animatable;
pub use color_space::{ColorSpace, ColorTween};
pub use composition::{Delay, Parallel, Sequence};
pub use ease::Ease;
pub use flip::{Flip, FlipState, NodeFlipTween};
pub use lifecycle::{AnimEvent, AnimId, AnimationLifecycleExt, EventReader, WithCallbacks};
pub use move_along_path::{MoveAlongPath, PathPose};
pub use path_morph::{DrawIn, PathMorph};
pub use repeat::{AnimationRepeatExt, Repeat, RepeatCount, RepeatStrategy};
pub use spring::Spring;
pub use stagger::{Stagger, StaggerFrom};
pub use target::{NodeProperty, Property, Target};
pub use theme::AnimTheme;
pub use track::{Curve, Key, Track};
pub use tween::Tween;
pub use typewriter::TypeWriter;

/// A pure time→state animation.
///
/// Implement this trait on any value that can produce an output for a
/// given moment in its own timeline. The animation knows its own
/// [`duration`](Animation::duration) but does NOT know wall-clock
/// time — that's the [`Driver`]'s job.
///
/// Implementations should be *side-effect-free* and *cheap* — they may
/// be sampled many times per frame for snapshot tests, scrubbing, or
/// FLIP-style state captures. Don't allocate inside `sample`.
pub trait Animation {
    /// The value produced at each sample point.
    type Output;

    /// Total duration of the animation. Implementations that loop
    /// forever should report [`Duration::MAX`].
    fn duration(&self) -> Duration;

    /// Evaluate the animation at time `t` from its own start. `t` will
    /// be clamped to `[0, duration]` by the caller (typically a
    /// [`Driver`]); implementations should still handle the boundary
    /// values gracefully (`t = 0`, `t = duration`).
    fn sample(&self, t: Duration) -> Self::Output;
}

/// How a [`Driver`] consumes time.
///
/// The mode is captured at construction and never mutates — flipping
/// between real-time and fixed-step mid-playback would defeat the
/// reproducibility contract.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DriverMode {
    /// The host supplies a real-world `dt` each call to
    /// [`Driver::tick`] (e.g. wall-clock delta between `winit`
    /// `RedrawRequested` frames). This is the interactive mode.
    #[default]
    Realtime,
    /// The driver advances by a fixed step on every [`Driver::tick`],
    /// ignoring the supplied `dt`. The host's only job is to call
    /// `tick` once per frame. This is the deterministic export mode
    /// used by `wisp-export-animated`.
    Fixed {
        /// Per-tick step size. Picking `1.0 / fps` produces an
        /// integer frame count.
        dt: Duration,
    },
}

/// Playback clock for one or more [`Animation`]s.
///
/// The driver tracks elapsed time, playback rate, and the play/pause
/// flag. It does NOT own animations — callers hold them as plain
/// values and pass them to [`Driver::sample`] when they want output.
///
/// # Why a separate clock
///
/// Decoupling the clock from the animation has three concrete payoffs:
///
/// - **Determinism.** Two `DriverMode::Fixed` drivers seeded
///   identically advance identically — guaranteeing byte-identical
///   MP4 export across runs and platforms.
/// - **Composition.** Multiple `Animation` values can read the same
///   `Driver::elapsed` to stay in lockstep without sharing state.
/// - **Wasm friendliness.** The driver never reads `Instant::now()`.
///   The host injects `dt`. That makes the whole crate trivially
///   wasm32-clean.
#[derive(Clone, Debug)]
pub struct Driver {
    mode: DriverMode,
    elapsed: Duration,
    time_scale: f32,
    playing: bool,
}

impl Driver {
    /// Construct a real-time driver. Starts paused; call [`play`](Driver::play)
    /// to begin advancing.
    #[must_use]
    pub const fn realtime() -> Self {
        Self {
            mode: DriverMode::Realtime,
            elapsed: Duration::ZERO,
            time_scale: 1.0,
            playing: false,
        }
    }

    /// Construct a fixed-step driver. Every call to [`tick`](Driver::tick)
    /// advances by exactly `dt`, regardless of what the caller passes
    /// in. Starts paused.
    #[must_use]
    pub const fn fixed(dt: Duration) -> Self {
        Self {
            mode: DriverMode::Fixed { dt },
            elapsed: Duration::ZERO,
            time_scale: 1.0,
            playing: false,
        }
    }

    /// The driver's playback mode (set at construction; immutable).
    #[must_use]
    pub const fn mode(&self) -> DriverMode {
        self.mode
    }

    /// Total time the driver has accumulated, scaled by
    /// [`time_scale`](Driver::time_scale) and gated by
    /// [`is_playing`](Driver::is_playing).
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Begin advancing on subsequent [`tick`](Driver::tick) calls.
    pub const fn play(&mut self) {
        self.playing = true;
    }

    /// Halt advancement. [`tick`](Driver::tick) becomes a no-op until
    /// [`play`](Driver::play) is called again.
    pub const fn pause(&mut self) {
        self.playing = false;
    }

    /// Whether the driver is currently advancing on `tick`.
    #[must_use]
    pub const fn is_playing(&self) -> bool {
        self.playing
    }

    /// Set the playback rate. `1.0` is real-time; `2.0` doubles speed;
    /// `0.5` halves it. Negative scales are clamped to zero — reverse
    /// playback lives on a future ticket (M-ANIM.4).
    pub const fn set_time_scale(&mut self, scale: f32) {
        // Manually clamp without `f32::max` so this method can stay
        // `const`. NaN propagates as 0.0 to avoid poisoning elapsed.
        self.time_scale = if scale > 0.0 { scale } else { 0.0 };
    }

    /// Current playback rate.
    #[must_use]
    pub const fn time_scale(&self) -> f32 {
        self.time_scale
    }

    /// Jump to an absolute elapsed time. Useful for scrubbing or for
    /// state restore in tests. Does not change the play/pause flag.
    pub const fn seek(&mut self, t: Duration) {
        self.elapsed = t;
    }

    /// Advance the clock and return the new [`elapsed`](Driver::elapsed).
    ///
    /// - In [`DriverMode::Realtime`] mode, the elapsed time grows by
    ///   `dt * time_scale` (rounded down to nanosecond resolution).
    /// - In [`DriverMode::Fixed`] mode, the elapsed time grows by the
    ///   stored `dt` (multiplied by `time_scale` for slow-mo / fast
    ///   playback of recorded animations) — the caller's `dt` is
    ///   ignored.
    ///
    /// When [`is_playing`](Driver::is_playing) is `false`, this is a
    /// no-op and returns the unchanged elapsed time.
    pub fn tick(&mut self, dt: Duration) -> Duration {
        if !self.playing {
            return self.elapsed;
        }
        let raw_step = match self.mode {
            DriverMode::Realtime => dt,
            DriverMode::Fixed { dt: step } => step,
        };
        self.elapsed = self
            .elapsed
            .saturating_add(scale_duration(raw_step, self.time_scale));
        self.elapsed
    }

    /// Sample an animation at the driver's current elapsed time.
    /// `t` is clamped to `[0, anim.duration()]` automatically.
    pub fn sample<A: Animation>(&self, anim: &A) -> A::Output {
        let dur = anim.duration();
        let t = if self.elapsed > dur {
            dur
        } else {
            self.elapsed
        };
        anim.sample(t)
    }

    /// Normalised progress through the given animation in `[0.0, 1.0]`.
    /// Returns `1.0` if the animation has zero duration to avoid
    /// dividing by zero.
    #[must_use]
    pub fn progress<A: Animation>(&self, anim: &A) -> f32 {
        let dur = anim.duration();
        if dur.is_zero() {
            return 1.0;
        }
        let t = if self.elapsed > dur {
            dur
        } else {
            self.elapsed
        };
        #[allow(
            clippy::cast_possible_truncation,
            reason = "progress is in [0, 1]; f64→f32 truncation is the desired narrowing"
        )]
        let p = (t.as_secs_f64() / dur.as_secs_f64()) as f32;
        p
    }
}

/// Apply a non-negative scale to a `Duration` via saturating
/// arithmetic. Extracted so the cast-attribute soup doesn't leak
/// into `Driver::tick`.
fn scale_duration(d: Duration, scale: f32) -> Duration {
    if scale <= 0.0 || !scale.is_finite() {
        return Duration::ZERO;
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "duration nanos < 2^63 in practice; f64 mantissa is sufficient for UI-grade scaling"
    )]
    let scaled = d.as_nanos() as f64 * f64::from(scale);
    #[allow(
        clippy::cast_precision_loss,
        reason = "guard threshold only; precision loss near 2^64 doesn't affect the correctness of the < comparison"
    )]
    let u64_max_f64 = u64::MAX as f64;
    if !scaled.is_finite() || scaled >= u64_max_f64 {
        return Duration::from_nanos(u64::MAX);
    }
    if scaled <= 0.0 {
        return Duration::ZERO;
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "scaled is in [0, u64::MAX as f64) and finite — guarded above"
    )]
    let nanos = scaled as u64;
    Duration::from_nanos(nanos)
}

impl Default for Driver {
    fn default() -> Self {
        Self::realtime()
    }
}

// ---------------------------------------------------------------------
// Test-only sentinel animations
//
// Used by the lib's own test suite and by `examples/tween_pulse` to
// prove the Driver works without depending on `Tween` (which lands
// in AUT-229 / M-ANIM.2). A hand-rolled `Animation` impl is enough
// to demonstrate the trait + Driver contract.
// ---------------------------------------------------------------------

/// A linear ramp from `from` to `to` over `duration`. Used by examples
/// and tests before the full `Tween<V>` type (M-ANIM.2) lands. Public
/// so the WASM demo in `wisp-chart-web` can drive a chart's rotation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearRamp {
    /// Starting value of the ramp.
    pub from: f32,
    /// Ending value at `t == duration`.
    pub to: f32,
    /// Total time from `from` to `to`.
    pub duration: Duration,
}

impl LinearRamp {
    /// Construct a ramp.
    #[must_use]
    pub const fn new(from: f32, to: f32, duration: Duration) -> Self {
        Self { from, to, duration }
    }
}

impl Animation for LinearRamp {
    type Output = f32;

    fn duration(&self) -> Duration {
        self.duration
    }

    fn sample(&self, t: Duration) -> f32 {
        if self.duration.is_zero() {
            return self.to;
        }
        #[allow(
            clippy::cast_possible_truncation,
            reason = "progress is in [0, 1]; f64→f32 truncation is the desired narrowing"
        )]
        let raw = (t.as_secs_f64() / self.duration.as_secs_f64()) as f32;
        let progress = raw.clamp(0.0, 1.0);
        self.from + (self.to - self.from) * progress
    }
}

#[cfg(test)]
mod tests;
