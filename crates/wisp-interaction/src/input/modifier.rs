//! `ModifierState` — packed-bool struct for shift / ctrl / alt /
//! super (= command on macOS, windows-key elsewhere).
//!
//! Reported alongside pointer events + keyboard events so the
//! consumer doesn't need to query `ButtonInput<KeyCode>` for "was
//! shift held when this click fired" — the event payload itself
//! carries the snapshot.

/// Bitfield-style snapshot of modifier-key state at the moment an
/// event was dispatched.
///
/// Defaults to all-false. Adapters fill it from per-event modifier
/// flags (winit's `ModifiersState` / browser's `event.shiftKey` etc).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "Four modifier keys, four bools — the field-per-modifier shape matches winit's ModifiersState and the DOM event modifier flags. Bitfields would obscure that 1:1 correspondence with no measurable win."
)]
pub struct ModifierState {
    /// Either Shift key held.
    pub shift: bool,
    /// Either Control key held.
    pub ctrl: bool,
    /// Either Alt / Option key held.
    pub alt: bool,
    /// Either Super key held (Command on macOS, Windows key on Win).
    pub super_key: bool,
}

impl ModifierState {
    /// Construct with every modifier flag false.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// `true` iff no modifiers held.
    #[must_use]
    pub fn is_empty(self) -> bool {
        !(self.shift || self.ctrl || self.alt || self.super_key)
    }

    /// On macOS the "platform" modifier is Super (Command);
    /// elsewhere it's Control. Apps that want to treat ⌘+C and Ctrl+C
    /// as the same shortcut should use this accessor.
    #[must_use]
    pub fn platform(self) -> bool {
        if cfg!(target_os = "macos") {
            self.super_key
        } else {
            self.ctrl
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        assert!(ModifierState::default().is_empty());
    }

    #[test]
    fn shift_set_makes_non_empty() {
        let m = ModifierState {
            shift: true,
            ..Default::default()
        };
        assert!(!m.is_empty());
        assert!(m.shift);
    }

    #[test]
    fn platform_modifier_is_super_on_macos_ctrl_elsewhere() {
        let cmd = ModifierState {
            super_key: true,
            ..Default::default()
        };
        let ctrl = ModifierState {
            ctrl: true,
            ..Default::default()
        };
        // Whichever platform we're on, exactly one should report true.
        if cfg!(target_os = "macos") {
            assert!(cmd.platform());
            assert!(!ctrl.platform());
        } else {
            assert!(ctrl.platform());
            assert!(!cmd.platform());
        }
    }
}
