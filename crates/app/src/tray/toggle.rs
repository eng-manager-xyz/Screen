//! Tray-popover visibility state machine (M-TRAY.0 / AUT-249).
//!
//! Pure-Rust state machine — no Tauri types, no async, no I/O — so the
//! transition logic is verifiable on every OS (including Windows
//! where Tauri 2's `mock_builder` won't link at test-time per
//! CLAUDE.md "Tauri 2 specifics").
//!
//! M-TRAY.3 will rename this to `MainWindowVisibility` when the
//! tray-popover window is reshaped into the full app `main` window.
//! The transition contract — toggle → show-or-hide — carries over
//! unchanged.

/// Whether the tray-popover window is currently shown to the user.
///
/// The state lives in `tauri::Manager`-managed storage; this enum is
/// the source of truth for the click handler. `Default` is `Hidden`
/// (matching `tauri.conf.json`'s `visible: false`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrayPopoverState {
    /// Popover is hidden; next click shows it.
    #[default]
    Hidden,
    /// Popover is visible; next click hides it.
    Visible,
}

/// The action the click handler should perform after observing a
/// tray click.
///
/// Returning an action enum (rather than mutating Tauri windows
/// directly from inside the state machine) keeps this module free of
/// Tauri types — the caller in `main.rs` does the actual `show()` /
/// `hide()` call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Caller should `window.show()` + `window.set_focus()`.
    Show,
    /// Caller should `window.hide()`.
    Hide,
}

impl TrayPopoverState {
    /// Advance the state machine by one click; return the action the
    /// caller must perform.
    pub fn on_click(&mut self) -> Action {
        match *self {
            Self::Hidden => {
                *self = Self::Visible;
                Action::Show
            }
            Self::Visible => {
                *self = Self::Hidden;
                Action::Hide
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_hidden() {
        assert_eq!(TrayPopoverState::default(), TrayPopoverState::Hidden);
    }

    #[test]
    fn hidden_click_yields_show_and_becomes_visible() {
        let mut s = TrayPopoverState::Hidden;
        let action = s.on_click();
        assert_eq!(action, Action::Show);
        assert_eq!(s, TrayPopoverState::Visible);
    }

    #[test]
    fn visible_click_yields_hide_and_becomes_hidden() {
        let mut s = TrayPopoverState::Visible;
        let action = s.on_click();
        assert_eq!(action, Action::Hide);
        assert_eq!(s, TrayPopoverState::Hidden);
    }

    #[test]
    fn ten_alternating_clicks_round_trip() {
        // Acceptance criterion from AUT-249: "Toggle is stable across
        // at least 10 alternating clicks (no zombie windows, no
        // double-spawn)." The state machine half of that asserts that
        // the parity of click count drives state.
        let mut s = TrayPopoverState::Hidden;
        for i in 0..10 {
            let action = s.on_click();
            if i % 2 == 0 {
                assert_eq!(action, Action::Show);
                assert_eq!(s, TrayPopoverState::Visible);
            } else {
                assert_eq!(action, Action::Hide);
                assert_eq!(s, TrayPopoverState::Hidden);
            }
        }
        // After 10 (even) clicks the state should be back to Hidden.
        assert_eq!(s, TrayPopoverState::Hidden);
    }
}
