//! M-RECP.1 / AUT-262 — Multi-display window positioning under the
//! tray click.
//!
//! Pure-Rust helper that picks the right monitor for a given click
//! position. The Tauri-side caller in `main.rs` invokes
//! [`pick_monitor`] inside the `on_tray_icon_event` handler before
//! showing the main window. OS hardware integration (querying real
//! monitor bounds, applying the clamp) is the deferred follow-up.

/// Axis-aligned rectangle in screen coordinates. Same shape as
/// Tauri's `tauri::PhysicalRect` but lives here as a `Copy`-friendly
/// pure-Rust struct so the picker is unit-testable without a Tauri
/// runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorBounds {
    /// Top-left x.
    pub x: i32,
    /// Top-left y.
    pub y: i32,
    /// Width in pixels.
    pub width: i32,
    /// Height in pixels.
    pub height: i32,
}

impl MonitorBounds {
    /// `true` if the point `(x, y)` falls inside this monitor.
    #[must_use]
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// Find the monitor whose bounds contain the click position. Returns
/// the first monitor as a fallback if no bounds contain the click
/// (defensive — shouldn't normally happen with valid monitor
/// geometry, but multi-display setups can have negative coords).
#[must_use]
pub fn pick_monitor(
    click_x: i32,
    click_y: i32,
    monitors: &[MonitorBounds],
) -> Option<MonitorBounds> {
    if monitors.is_empty() {
        return None;
    }
    monitors
        .iter()
        .find(|m| m.contains(click_x, click_y))
        .copied()
        .or_else(|| monitors.first().copied())
}

/// Top-left position that anchors the popover's **top-right corner**
/// to the monitor's top-right corner — flush against the screen edge,
/// matching the macOS Control-Center / Notification-Center
/// convention. The click position is only used by the caller to pick
/// which monitor to anchor on; within that monitor the popover always
/// lands top-right.
///
/// Returns top-left `(x, y)` in screen coordinates. macOS clamps the
/// window's titlebar below the menubar automatically when `y` falls
/// in the menubar region, so passing `monitor.y` directly is safe.
#[must_use]
pub fn position_window_top_right(window_width: i32, monitor: MonitorBounds) -> (i32, i32) {
    let x = monitor.x + (monitor.width - window_width).max(0);
    let y = monitor.y;
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mon(x: i32, y: i32, w: i32, h: i32) -> MonitorBounds {
        MonitorBounds {
            x,
            y,
            width: w,
            height: h,
        }
    }

    #[test]
    fn contains_includes_top_left_excludes_bottom_right() {
        let m = mon(0, 0, 100, 100);
        assert!(m.contains(0, 0));
        assert!(m.contains(50, 50));
        assert!(!m.contains(100, 50));
        assert!(!m.contains(50, 100));
    }

    #[test]
    fn pick_monitor_returns_none_for_empty_list() {
        assert!(pick_monitor(0, 0, &[]).is_none());
    }

    #[test]
    fn pick_monitor_finds_the_one_containing_click() {
        let mons = vec![mon(0, 0, 1920, 1080), mon(1920, 0, 1920, 1080)];
        assert_eq!(pick_monitor(2000, 100, &mons), Some(mons[1]));
        assert_eq!(pick_monitor(500, 100, &mons), Some(mons[0]));
    }

    #[test]
    fn pick_monitor_falls_back_to_first_if_no_match() {
        let mons = vec![mon(0, 0, 1920, 1080)];
        // Negative click way out of bounds — fallback to first.
        assert_eq!(pick_monitor(-100, -100, &mons), Some(mons[0]));
    }

    #[test]
    fn position_window_top_right_anchors_to_monitor_top_right() {
        let mon = mon(0, 0, 1920, 1080);
        // Window's top-right should sit at monitor's top-right
        // (1920, 0) → top-left at (1920 - 800, 0) = (1120, 0).
        assert_eq!(position_window_top_right(800, mon), (1120, 0));
    }

    #[test]
    fn position_window_top_right_respects_offset_monitor_origin() {
        // Secondary display sitting to the right of the primary.
        let mon = mon(1920, 0, 2560, 1440);
        // Top-right of monitor is at (1920 + 2560, 0) = (4480, 0).
        // Window top-left = (4480 - 600, 0) = (3880, 0).
        assert_eq!(position_window_top_right(600, mon), (3880, 0));
    }

    #[test]
    fn position_window_top_right_clamps_when_window_wider_than_monitor() {
        // Pathological: window is wider than the monitor. Don't push
        // the left edge into negative territory inside the monitor —
        // clamp left edge to monitor.x so the window starts at the
        // monitor's left edge (and overflows on the right, but the
        // user will see *some* of it instead of all of it being
        // pushed off the left side).
        let mon = mon(0, 0, 800, 600);
        assert_eq!(position_window_top_right(1000, mon), (0, 0));
    }
}
