//! M-RECP.5 / AUT-266 — Hot-swap crossfade state machine.
//!
//! Models a 150-ms cross-fade between two gst → wisp pipelines while
//! the user swaps cameras. The state machine is pure — the GPU /
//! wisp / gst lifecycle wiring lives in the M-CAM.3 follow-up.

use std::time::Duration;

/// Default crossfade duration.
pub const CROSSFADE_DURATION: Duration = Duration::from_millis(150);

/// Cross-fade lifecycle state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrossfadeState {
    /// No swap in progress.
    #[default]
    Steady,
    /// New camera's pipeline is ramping in over the current one. The
    /// `progress` value `[0, 1]` is the new camera's alpha.
    InProgress {
        /// 0–255 alpha for the incoming camera (255 = fully visible).
        progress: u8,
    },
    /// Crossfade completed — drop old pipeline + sprite slot.
    Settling,
}

impl CrossfadeState {
    /// Begin a crossfade. If already in progress, the current target
    /// is replaced with the new target (rapid third-camera click
    /// case from the ticket spec).
    #[must_use]
    pub fn begin(self) -> Self {
        Self::InProgress { progress: 0 }
    }

    /// Advance the crossfade by `elapsed` since the previous tick.
    /// Returns `Settling` when progress reaches 1.0.
    #[must_use]
    pub fn tick(self, elapsed: Duration) -> Self {
        match self {
            Self::InProgress { progress } => {
                let frac = elapsed.as_secs_f64() / CROSSFADE_DURATION.as_secs_f64();
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "progress is bounded [0, 255]; cast can't overflow after clamp"
                )]
                let delta = (frac * 255.0).round() as i32;
                let next = i32::from(progress).saturating_add(delta);
                if next >= 255 {
                    Self::Settling
                } else {
                    #[allow(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "next is bounded < 255 from the branch above"
                    )]
                    Self::InProgress {
                        progress: next as u8,
                    }
                }
            }
            // Cancelled / steady → no-op.
            other => other,
        }
    }

    /// Mark settling complete — return to steady state.
    #[must_use]
    pub fn settled(self) -> Self {
        match self {
            Self::Settling => Self::Steady,
            other => other,
        }
    }

    /// `true` once the crossfade has reached its target alpha.
    #[must_use]
    pub fn is_settling(self) -> bool {
        matches!(self, Self::Settling)
    }

    /// Current alpha for the incoming camera (0 in Steady, 255 in
    /// Settling). Caller passes this into the wisp scene as the
    /// secondary sprite's alpha multiplier.
    #[must_use]
    pub fn incoming_alpha(self) -> u8 {
        match self {
            Self::Steady => 0,
            Self::InProgress { progress } => progress,
            Self::Settling => 255,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_steady() {
        assert_eq!(CrossfadeState::default(), CrossfadeState::Steady);
    }

    #[test]
    fn begin_starts_at_zero() {
        let s = CrossfadeState::default().begin();
        assert_eq!(s, CrossfadeState::InProgress { progress: 0 });
        assert_eq!(s.incoming_alpha(), 0);
    }

    #[test]
    fn tick_advances_proportionally() {
        let s = CrossfadeState::default().begin();
        // Half the duration → ~128 alpha.
        let s = s.tick(CROSSFADE_DURATION / 2);
        if let CrossfadeState::InProgress { progress } = s {
            assert!((120..=135).contains(&progress), "progress = {progress}");
        } else {
            panic!("expected InProgress, got {s:?}");
        }
    }

    #[test]
    fn full_duration_reaches_settling() {
        let s = CrossfadeState::default().begin();
        let s = s.tick(CROSSFADE_DURATION);
        assert!(s.is_settling());
        assert_eq!(s.incoming_alpha(), 255);
    }

    #[test]
    fn settled_returns_to_steady() {
        let s = CrossfadeState::Settling.settled();
        assert_eq!(s, CrossfadeState::Steady);
    }

    #[test]
    fn third_camera_click_mid_crossfade_resets_progress() {
        let s = CrossfadeState::InProgress { progress: 128 }.begin();
        // Re-begin during in-progress resets to 0 (camera target
        // changed).
        assert_eq!(s, CrossfadeState::InProgress { progress: 0 });
    }
}
