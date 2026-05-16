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

/// Suggested position for the main window — below the tray click,
/// clamped so the window fits entirely within the monitor's bounds.
/// Returns top-left `(x, y)` in screen coordinates.
#[must_use]
pub fn position_window_below_click(
    click_x: i32,
    click_y: i32,
    window_width: i32,
    window_height: i32,
    monitor: MonitorBounds,
) -> (i32, i32) {
    // Centre horizontally on the click, ensure fully on-screen.
    let half_w = window_width / 2;
    let raw_x = click_x - half_w;
    let max_x = monitor.x + monitor.width - window_width;
    let x = raw_x.clamp(monitor.x, max_x.max(monitor.x));

    // Place below the menubar (assume ~24px on macOS).
    let raw_y = click_y + 4;
    let max_y = monitor.y + monitor.height - window_height;
    let y = raw_y.clamp(monitor.y, max_y.max(monitor.y));

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
    fn position_window_below_click_centres_horizontally() {
        let mon = mon(0, 0, 1920, 1080);
        let (x, _) = position_window_below_click(960, 24, 800, 600, mon);
        assert_eq!(x, 560);
    }

    #[test]
    fn position_window_below_click_clamps_right_edge() {
        let mon = mon(0, 0, 1920, 1080);
        let (x, _) = position_window_below_click(1900, 24, 800, 600, mon);
        // Click is near right edge; window should clamp to keep
        // fully on-screen.
        assert_eq!(x, 1120); // 1920 - 800
    }

    #[test]
    fn position_window_below_click_clamps_left_edge() {
        let mon = mon(0, 0, 1920, 1080);
        let (x, _) = position_window_below_click(10, 24, 800, 600, mon);
        assert_eq!(x, 0);
    }
}
