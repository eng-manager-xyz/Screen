//! Zoom regions — cinematic punch-ins on the project timeline.
//!
//! A [`ZoomSegment`] is a value type: a window of project frames, a zoom
//! `amount`, a target ([`ZoomMode`]), and an easing curve. At the render
//! boundary (ED.16) each compiles to a keyframed `wisp` transform
//! (scale + translate) applied to the screen sprite. Keeping the model
//! `wisp`-free here means the zoom math is unit-testable without a GPU.

use serde::{Deserialize, Serialize};

use crate::segment::Frame;

/// Stable identifier for a [`ZoomSegment`] within a project, so edit
/// operations (move / remove) can address one without relying on its
/// index, which shifts as zooms are added and removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ZoomId(pub u32);

/// Easing curve for a zoom's in/out animation. A deliberately small,
/// serde-friendly subset of the full `wisp_animation::Ease` set (which
/// includes a non-serializable function-pointer variant). The renderer
/// maps these to `wisp_animation::Ease` at ED.13/ED.16.
///
/// The default, [`EditEase::InOutCubic`], is the "Easy Ease" equivalent —
/// smooth acceleration in and deceleration out, which covers the vast
/// majority of zoom feels before any manual curve editing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditEase {
    /// Constant rate — mechanical, rarely wanted for a zoom.
    Linear,
    /// Accelerate in.
    InCubic,
    /// Decelerate out.
    OutCubic,
    /// Smooth in and out — the "Easy Ease" default.
    #[default]
    InOutCubic,
    /// Gentle sinusoidal in and out.
    InOutSine,
}

/// Where a zoom targets.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ZoomMode {
    /// Target is chosen automatically (e.g. from cursor/click telemetry,
    /// ED.17) or follows the cursor while held.
    Auto,
    /// Target a fixed point, normalized to `[0, 1]` in the composed
    /// frame: `(0, 0)` is top-left, `(1, 1)` bottom-right.
    Manual {
        /// Horizontal target, `0.0..=1.0`.
        x: f32,
        /// Vertical target, `0.0..=1.0`.
        y: f32,
    },
}

impl Default for ZoomMode {
    fn default() -> Self {
        // Centre-targeted manual zoom is the most predictable default for
        // a freshly added region; Auto is opt-in (telemetry-driven).
        Self::Manual { x: 0.5, y: 0.5 }
    }
}

/// A cinematic punch-in over `[start, end)` project frames.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZoomSegment {
    /// Stable identifier within the project.
    pub id: ZoomId,
    /// First project frame the zoom affects (inclusive).
    pub start: Frame,
    /// One past the last project frame the zoom affects (exclusive).
    pub end: Frame,
    /// Zoom factor at the hold. `1.0` = no zoom, `1.6` = 1.6× punch-in.
    /// Always `>= 1.0` in practice (a zoom out is unusual for a screen
    /// recording); the renderer clamps.
    pub amount: f64,
    /// What the zoom targets.
    #[serde(default)]
    pub mode: ZoomMode,
    /// Easing of the in/out animation.
    #[serde(default)]
    pub ease: EditEase,
}

impl ZoomSegment {
    /// A manual, centre-targeted zoom of `amount` over `[start, end)`
    /// with the default Easy-Ease curve.
    #[must_use]
    pub fn manual(id: ZoomId, start: Frame, end: Frame, amount: f64) -> Self {
        Self {
            id,
            start,
            end,
            amount,
            mode: ZoomMode::default(),
            ease: EditEase::default(),
        }
    }

    /// Length of the zoom window in project frames (saturating at 0).
    #[must_use]
    pub fn len(self) -> Frame {
        self.end.saturating_sub(self.start)
    }

    /// Whether the zoom window is empty (`end <= start`).
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.end <= self.start
    }

    /// Whether `project_frame` falls within this zoom's window.
    #[must_use]
    pub fn contains(self, project_frame: Frame) -> bool {
        project_frame >= self.start && project_frame < self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ease_is_easy_ease() {
        assert_eq!(EditEase::default(), EditEase::InOutCubic);
    }

    #[test]
    fn default_mode_is_centre_manual() {
        assert_eq!(ZoomMode::default(), ZoomMode::Manual { x: 0.5, y: 0.5 });
    }

    #[test]
    fn manual_builder_sets_window_and_amount() {
        let z = ZoomSegment::manual(ZoomId(7), 30, 90, 1.6);
        assert_eq!(z.id, ZoomId(7));
        assert_eq!(z.len(), 60);
        assert!(!z.is_empty());
        assert!((z.amount - 1.6).abs() < 1e-9);
        assert!(z.contains(30));
        assert!(z.contains(89));
        assert!(!z.contains(90));
        assert!(!z.contains(29));
    }

    #[test]
    fn empty_window_reports_empty() {
        let z = ZoomSegment::manual(ZoomId(0), 50, 50, 2.0);
        assert!(z.is_empty());
        assert_eq!(z.len(), 0);
        assert!(!z.contains(50));
    }
}
