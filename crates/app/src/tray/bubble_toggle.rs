//! Webcam-bubble visibility state machine (M-BUBBLE.0 / AUT-273).
//!
//! Mirrors the [`super::toggle::TrayPopoverState`] shape — a pure-Rust
//! `Hidden ↔ Visible` state machine that returns an action enum so the
//! caller in `commands.rs` is responsible for the Tauri `show()` /
//! `hide()` calls. Splitting state from I/O keeps the transition logic
//! cross-OS-testable without a Tauri runtime (CLAUDE.md "Tauri 2
//! `mock_builder` aborts at list-time on Windows").

/// Whether the `webcam-bubble` window is currently shown to the user.
///
/// The state lives in `tauri::Manager`-managed storage; this enum is
/// the source of truth for the "Show webcam bubble" toggle button in
/// the Recorder surface. `Default` is `Hidden` (matching
/// `tauri.conf.json`'s `visible: false`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BubbleVisibility {
    /// Bubble window is hidden; next toggle shows it.
    #[default]
    Hidden,
    /// Bubble window is visible; next toggle hides it.
    Visible,
}

/// The action the toggle handler should perform after observing a
/// user click on "Show webcam bubble." Returning an enum (rather than
/// mutating Tauri windows directly here) keeps this module free of
/// Tauri types so the unit tests don't need a runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BubbleAction {
    /// Caller should `window.show()` (and not `set_focus()` — the
    /// bubble is meant to float as a peripheral, not steal focus from
    /// the `AppShell`).
    Show,
    /// Caller should `window.hide()`.
    Hide,
}

impl BubbleVisibility {
    /// Advance the state machine by one click; return the action the
    /// caller must perform.
    pub fn on_click(&mut self) -> BubbleAction {
        match *self {
            Self::Hidden => {
                *self = Self::Visible;
                BubbleAction::Show
            }
            Self::Visible => {
                *self = Self::Hidden;
                BubbleAction::Hide
            }
        }
    }

    /// Align the state to `visible`, returning the action the caller
    /// should perform — or `None` when already in the requested state.
    ///
    /// Distinct from [`Self::on_click`] (which always flips). Used by
    /// callers that own their own source of truth for the desired
    /// visibility (e.g. the recorder's `camera_enabled` signal) and
    /// need lockstep alignment without depending on the state
    /// machine's prior position. ISS-05.
    pub fn set(&mut self, visible: bool) -> Option<BubbleAction> {
        match (*self, visible) {
            (Self::Hidden, true) => {
                *self = Self::Visible;
                Some(BubbleAction::Show)
            }
            (Self::Visible, false) => {
                *self = Self::Hidden;
                Some(BubbleAction::Hide)
            }
            (Self::Hidden, false) | (Self::Visible, true) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_hidden() {
        assert_eq!(BubbleVisibility::default(), BubbleVisibility::Hidden);
    }

    #[test]
    fn hidden_click_yields_show_and_becomes_visible() {
        let mut s = BubbleVisibility::Hidden;
        let action = s.on_click();
        assert_eq!(action, BubbleAction::Show);
        assert_eq!(s, BubbleVisibility::Visible);
    }

    #[test]
    fn visible_click_yields_hide_and_becomes_hidden() {
        let mut s = BubbleVisibility::Visible;
        let action = s.on_click();
        assert_eq!(action, BubbleAction::Hide);
        assert_eq!(s, BubbleVisibility::Hidden);
    }

    #[test]
    fn ten_alternating_clicks_round_trip() {
        let mut s = BubbleVisibility::Hidden;
        for i in 0..10 {
            let action = s.on_click();
            if i % 2 == 0 {
                assert_eq!(action, BubbleAction::Show);
                assert_eq!(s, BubbleVisibility::Visible);
            } else {
                assert_eq!(action, BubbleAction::Hide);
                assert_eq!(s, BubbleVisibility::Hidden);
            }
        }
        assert_eq!(s, BubbleVisibility::Hidden);
    }

    #[test]
    fn set_true_from_hidden_yields_show() {
        let mut s = BubbleVisibility::Hidden;
        assert_eq!(s.set(true), Some(BubbleAction::Show));
        assert_eq!(s, BubbleVisibility::Visible);
    }

    #[test]
    fn set_false_from_visible_yields_hide() {
        let mut s = BubbleVisibility::Visible;
        assert_eq!(s.set(false), Some(BubbleAction::Hide));
        assert_eq!(s, BubbleVisibility::Hidden);
    }

    #[test]
    fn set_to_current_state_is_a_noop() {
        let mut hidden = BubbleVisibility::Hidden;
        assert_eq!(hidden.set(false), None);
        let mut visible = BubbleVisibility::Visible;
        assert_eq!(visible.set(true), None);
    }
}
