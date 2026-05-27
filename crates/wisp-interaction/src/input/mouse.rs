//! Mouse-button enum + accumulated motion delta.

use glam::Vec2;

/// Mouse button identifier. Mirrors winit's `MouseButton` exactly so
/// the WI.6 adapter translation is a trivial `match`.
///
/// Browser `MouseEvent.button` numeric values (0 = primary, 1 = aux,
/// 2 = secondary, 3 = back, 4 = forward) map onto `Left / Middle /
/// Right / Back / Forward` per the W3C UI Events spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Primary button (typically left).
    Left,
    /// Secondary button (typically right).
    Right,
    /// Auxiliary button (typically middle / scroll wheel click).
    Middle,
    /// "Back" navigation button (5-button mice).
    Back,
    /// "Forward" navigation button (5-button mice).
    Forward,
    /// Vendor-extension button. Numeric ID is opaque.
    Other(u16),
}

/// Accumulated mouse-motion delta in device pixels for the current
/// frame. Hosts call `add(delta)` per motion event during ingestion
/// and `clear()` once per frame after consumers have read.
///
/// Distinct from per-event `MouseMotionEvent`s on the raw stream
/// because most consumers want "how far did the mouse move this
/// frame" (one read per frame), not "every individual motion event"
/// (one read per event). Matches Bevy's
/// `bevy_input::AccumulatedMouseMotion` shape.
#[derive(Debug, Clone, Copy, Default)]
pub struct AccumulatedMouseMotion {
    /// Sum of motion deltas since last `clear()`.
    pub delta: Vec2,
}

impl AccumulatedMouseMotion {
    /// Construct with zero delta.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a per-event delta into the accumulator.
    pub fn add(&mut self, d: Vec2) {
        self.delta += d;
    }

    /// Reset to zero. Hosts call this once per frame after consumers
    /// have read.
    pub fn clear(&mut self) {
        self.delta = Vec2::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulated_motion_sums_then_clears() {
        let mut m = AccumulatedMouseMotion::new();
        m.add(Vec2::new(1.0, 2.0));
        m.add(Vec2::new(3.0, -1.0));
        assert!((m.delta - Vec2::new(4.0, 1.0)).length() < 1e-6);
        m.clear();
        assert!(m.delta.length() < 1e-6);
    }
}
