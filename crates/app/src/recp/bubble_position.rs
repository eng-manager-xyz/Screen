//! M-BUBBLE.3 / AUT-276 — webcam-bubble position math + corner snap.
//!
//! Pure-Rust helpers: no Tauri, no I/O, no async. The Tauri-side
//! caller in `commands.rs` invokes [`default_position`] when the
//! bubble first opens without a saved position, [`is_on_any_monitor`]
//! to validate a restored position is still visible after a display
//! unplug, and [`snap_to_nearest_corner`] for the snap-on-drag UX
//! (wired in a follow-up — for v0 these helpers ship tested but the
//! `Moved` event wiring stays inert to avoid the set-position →
//! Moved → set-position loop without a debounce in place).

use serde::{Deserialize, Serialize};

use super::tray_positioning::MonitorBounds;

/// On-disk-serialisable bubble window position. Stored as logical
/// pixels (the units `tauri::WebviewWindow::set_position` accepts).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BubblePosition {
    /// Window's top-left x in screen coordinates.
    pub x: i32,
    /// Window's top-left y in screen coordinates.
    pub y: i32,
}

/// Which corner of a monitor a window is closest to. Returned by
/// [`snap_to_nearest_corner`] (via `Option<(i32, i32, Corner)>`) so the
/// caller can render a "snapped to top-right" visual hint distinct
/// from "free-floating."
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    /// Monitor's top-left corner.
    TopLeft,
    /// Monitor's top-right corner.
    TopRight,
    /// Monitor's bottom-left corner.
    BottomLeft,
    /// Monitor's bottom-right corner.
    BottomRight,
}

/// Default first-open position: bottom-right of the supplied monitor,
/// inset by `inset_px` from both edges so the bubble doesn't kiss the
/// dock / taskbar.
#[must_use]
pub fn default_position(
    window_width: i32,
    window_height: i32,
    monitor: MonitorBounds,
    inset_px: i32,
) -> BubblePosition {
    BubblePosition {
        x: monitor.x + monitor.width - window_width - inset_px,
        y: monitor.y + monitor.height - window_height - inset_px,
    }
}

/// `true` iff the bubble (top-left at `pos`, size `(w, h)`) is at
/// least partially visible on any of the supplied monitors. Used after
/// a display unplug to decide whether the persisted position is still
/// usable or should fall back to [`default_position`].
///
/// A position counts as visible if the window's rectangle intersects
/// the monitor's rectangle (any pixel overlap — not requiring full
/// containment, since the user may have intentionally placed the
/// bubble half-off-screen and we shouldn't reset their choice).
#[must_use]
pub fn is_on_any_monitor(
    pos: BubblePosition,
    window_width: i32,
    window_height: i32,
    monitors: &[MonitorBounds],
) -> bool {
    let win_left = pos.x;
    let win_top = pos.y;
    let win_right = pos.x + window_width;
    let win_bottom = pos.y + window_height;

    monitors.iter().any(|m| {
        let mon_left = m.x;
        let mon_top = m.y;
        let mon_right = m.x + m.width;
        let mon_bottom = m.y + m.height;

        // Standard axis-aligned rectangle intersection.
        win_left < mon_right && win_right > mon_left && win_top < mon_bottom && win_bottom > mon_top
    })
}

/// Internal helper type — pairs a corner identity with the screen
/// coordinates of two matching points (the window's corner and the
/// monitor's corner) used to compute the Manhattan distance between
/// them. Aliased so the array literal stays readable + clippy's
/// `type_complexity` lint is satisfied.
type CornerCandidate = (Corner, (i32, i32), (i32, i32));

/// If the bubble's top-left is within `snap_radius_px` of any
/// monitor's nearest corner (measured corner-to-corner of the window
/// vs monitor), snap to that corner's exact position and return the
/// snapped `(BubblePosition, Corner)`. Otherwise `None`.
///
/// "Nearest corner of the monitor for this window position" means:
///
/// * Top-left  → window's top-left  near monitor's top-left
/// * Top-right → window's top-right near monitor's top-right
/// * Bottom-* → analogous
///
/// So the comparison is between matching corners of the window and
/// the monitor. Snapping rewrites the window's top-left so the
/// matched corners align exactly (with optional inset — see
/// `inset_px`).
#[must_use]
pub fn snap_to_nearest_corner(
    pos: BubblePosition,
    window_width: i32,
    window_height: i32,
    monitor: MonitorBounds,
    snap_radius_px: i32,
    inset_px: i32,
) -> Option<(BubblePosition, Corner)> {
    let win_top_left = (pos.x, pos.y);
    let win_top_right = (pos.x + window_width, pos.y);
    let win_bot_left = (pos.x, pos.y + window_height);
    let win_bot_right = (pos.x + window_width, pos.y + window_height);

    let mon_top_left = (monitor.x, monitor.y);
    let mon_top_right = (monitor.x + monitor.width, monitor.y);
    let mon_bot_left = (monitor.x, monitor.y + monitor.height);
    let mon_bot_right = (monitor.x + monitor.width, monitor.y + monitor.height);

    let candidates: [CornerCandidate; 4] = [
        (Corner::TopLeft, win_top_left, mon_top_left),
        (Corner::TopRight, win_top_right, mon_top_right),
        (Corner::BottomLeft, win_bot_left, mon_bot_left),
        (Corner::BottomRight, win_bot_right, mon_bot_right),
    ];

    let (corner, _win, _mon, distance) = candidates
        .iter()
        .map(|(c, win, mon)| {
            let dx = win.0 - mon.0;
            let dy = win.1 - mon.1;
            // Manhattan distance keeps the math integer-only and is a
            // good-enough proxy for "user dragged near this corner."
            let dist = dx.abs() + dy.abs();
            (*c, *win, *mon, dist)
        })
        .min_by_key(|(_, _, _, d)| *d)?;

    if distance > snap_radius_px {
        return None;
    }

    let snapped = match corner {
        Corner::TopLeft => BubblePosition {
            x: monitor.x + inset_px,
            y: monitor.y + inset_px,
        },
        Corner::TopRight => BubblePosition {
            x: monitor.x + monitor.width - window_width - inset_px,
            y: monitor.y + inset_px,
        },
        Corner::BottomLeft => BubblePosition {
            x: monitor.x + inset_px,
            y: monitor.y + monitor.height - window_height - inset_px,
        },
        Corner::BottomRight => BubblePosition {
            x: monitor.x + monitor.width - window_width - inset_px,
            y: monitor.y + monitor.height - window_height - inset_px,
        },
    };

    Some((snapped, corner))
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
    fn default_position_lands_bottom_right_with_inset() {
        let m = mon(0, 0, 1920, 1080);
        // 200×200 window, 16px inset → top-left at (1920-200-16, 1080-200-16).
        let pos = default_position(200, 200, m, 16);
        assert_eq!(pos, BubblePosition { x: 1704, y: 864 });
    }

    #[test]
    fn default_position_respects_monitor_offset() {
        // Secondary monitor at x=1920.
        let m = mon(1920, 0, 1920, 1080);
        let pos = default_position(200, 200, m, 16);
        assert_eq!(pos.x, 1920 + 1920 - 200 - 16);
        assert_eq!(pos.y, 864);
    }

    #[test]
    fn is_on_any_monitor_true_for_position_fully_inside() {
        let m = vec![mon(0, 0, 1920, 1080)];
        assert!(is_on_any_monitor(
            BubblePosition { x: 100, y: 100 },
            200,
            200,
            &m
        ));
    }

    #[test]
    fn is_on_any_monitor_true_for_partial_overlap() {
        let m = vec![mon(0, 0, 1920, 1080)];
        // Half off-screen left.
        assert!(is_on_any_monitor(
            BubblePosition { x: -100, y: 100 },
            200,
            200,
            &m
        ));
    }

    #[test]
    fn is_on_any_monitor_false_for_fully_off_screen() {
        let m = vec![mon(0, 0, 1920, 1080)];
        assert!(!is_on_any_monitor(
            BubblePosition { x: 2000, y: 100 },
            200,
            200,
            &m
        ));
    }

    #[test]
    fn is_on_any_monitor_handles_unplugged_secondary() {
        // Saved position assumed a secondary monitor that's now gone.
        let m = vec![mon(0, 0, 1920, 1080)];
        let saved = BubblePosition { x: 2500, y: 500 }; // was on the now-gone secondary
        assert!(!is_on_any_monitor(saved, 200, 200, &m));
    }

    #[test]
    fn snap_to_nearest_corner_snaps_to_bottom_right_when_near() {
        let m = mon(0, 0, 1920, 1080);
        // Window at (1700, 850): bottom-right is (1900, 1050) — close
        // to monitor's bottom-right (1920, 1080). Manhattan distance
        // = 20 + 30 = 50, well within radius 50.
        let result =
            snap_to_nearest_corner(BubblePosition { x: 1700, y: 850 }, 200, 200, m, 50, 16);
        let (snapped, corner) = result.expect("should snap");
        assert_eq!(corner, Corner::BottomRight);
        // Bottom-right snap: x = 1920 - 200 - 16 = 1704, y = 1080 - 200 - 16 = 864.
        assert_eq!(snapped, BubblePosition { x: 1704, y: 864 });
    }

    #[test]
    fn snap_to_nearest_corner_snaps_to_top_left_when_near() {
        let m = mon(0, 0, 1920, 1080);
        // Window at (8, 12) — both corners within snap radius.
        let result = snap_to_nearest_corner(BubblePosition { x: 8, y: 12 }, 200, 200, m, 50, 16);
        let (snapped, corner) = result.expect("should snap");
        assert_eq!(corner, Corner::TopLeft);
        assert_eq!(snapped, BubblePosition { x: 16, y: 16 });
    }

    #[test]
    fn snap_to_nearest_corner_returns_none_when_far_from_all_corners() {
        let m = mon(0, 0, 1920, 1080);
        // Dead-centre of a 1920×1080 monitor — far from every corner.
        let result = snap_to_nearest_corner(BubblePosition { x: 860, y: 440 }, 200, 200, m, 50, 16);
        assert!(result.is_none());
    }

    #[test]
    fn snap_chooses_nearest_corner_when_two_in_range() {
        let m = mon(0, 0, 1920, 1080);
        // Position closer to bottom-right than top-right.
        let result =
            snap_to_nearest_corner(BubblePosition { x: 1700, y: 800 }, 200, 200, m, 200, 16);
        let (_, corner) = result.expect("should snap");
        assert_eq!(corner, Corner::BottomRight);
    }

    #[test]
    fn snap_respects_monitor_offset() {
        // Secondary monitor at x=1920.
        let m = mon(1920, 0, 1920, 1080);
        // Window placed near secondary's top-left.
        let result = snap_to_nearest_corner(BubblePosition { x: 1928, y: 8 }, 200, 200, m, 50, 16);
        let (snapped, corner) = result.expect("should snap");
        assert_eq!(corner, Corner::TopLeft);
        assert_eq!(snapped, BubblePosition { x: 1936, y: 16 });
    }
}
