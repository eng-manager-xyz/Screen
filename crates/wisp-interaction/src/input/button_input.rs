//! `ButtonInput<T>` — Bevy-shaped three-set state machine for
//! anything pressable.
//!
//! Mirrors `bevy_input::ButtonInput<T>` exactly enough that anyone
//! familiar with Bevy's input layer can read this without remapping
//! vocabulary:
//!
//! - `pressed: HashSet<T>` — every button currently held
//! - `just_pressed: HashSet<T>` — buttons that went down THIS frame
//! - `just_released: HashSet<T>` — buttons that went up THIS frame
//!
//! `clear()` clears the `just_*` sets only — `pressed` survives so
//! "is W still held?" stays true across frames. Hosts call `clear()`
//! once per frame after consumers have read state.
//!
//! Pre-instantiated as [`KeyboardInput`] (`ButtonInput<KeyCode>`) and
//! [`MouseButtonInput`] (`ButtonInput<MouseButton>`).
//!
//! ## Reference
//!
//! - Bevy `crates/bevy_input/src/button_input.rs:12-60` is the
//!   architectural shape we're matching. The Rust impl is short
//!   enough that the cross-translation is mechanical; we deviate
//!   only by dropping the `#[derive(Resource)]` (no ECS).

use std::collections::HashSet;
use std::hash::Hash;

use crate::input::key_code::KeyCode;
use crate::input::mouse::MouseButton;

/// Generic three-set button state.
///
/// `T` is the button kind — `KeyCode`, `MouseButton`, or a custom
/// gamepad button enum a downstream crate adds.
#[derive(Debug, Clone, Default)]
pub struct ButtonInput<T: Copy + Eq + Hash> {
    pressed: HashSet<T>,
    just_pressed: HashSet<T>,
    just_released: HashSet<T>,
}

impl<T: Copy + Eq + Hash> ButtonInput<T> {
    /// Construct an empty input — no buttons pressed.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            just_released: HashSet::new(),
        }
    }

    /// Record a press. If the button is already in `pressed`,
    /// `just_pressed` is NOT re-added — auto-repeat is filtered out
    /// at this layer (callers that want auto-repeat read the raw
    /// `InputEvent` stream instead).
    pub fn press(&mut self, button: T) {
        if self.pressed.insert(button) {
            self.just_pressed.insert(button);
        }
    }

    /// Record a release. `just_released` is set, `pressed` cleared.
    /// If the button wasn't in `pressed` (out-of-order events from
    /// a focus-loss / OS quirk), `just_released` is still set — the
    /// invariant "release was observed" matters even if a matching
    /// press was missed.
    pub fn release(&mut self, button: T) {
        // Drop from pressed if present; either way, record the release.
        // The invariant "release was observed" matters even if no
        // matching press was seen (focus-loss + OS quirk).
        self.pressed.remove(&button);
        self.just_released.insert(button);
    }

    /// Release every pressed button. Called on focus-loss to avoid
    /// "stuck key" bugs where Tab-away leaves a button forever in
    /// `pressed`. Matches Bevy `keyboard_input_system`'s response to
    /// `KeyboardFocusLost`.
    pub fn release_all(&mut self) {
        // Move pressed → just_released wholesale.
        for button in self.pressed.drain() {
            self.just_released.insert(button);
        }
    }

    /// `true` if the button is currently held.
    #[must_use]
    pub fn pressed(&self, button: T) -> bool {
        self.pressed.contains(&button)
    }

    /// `true` if the button went down this frame.
    #[must_use]
    pub fn just_pressed(&self, button: T) -> bool {
        self.just_pressed.contains(&button)
    }

    /// `true` if the button went up this frame.
    #[must_use]
    pub fn just_released(&self, button: T) -> bool {
        self.just_released.contains(&button)
    }

    /// `true` if any of the buttons in the iter is currently held.
    pub fn any_pressed(&self, buttons: impl IntoIterator<Item = T>) -> bool {
        buttons.into_iter().any(|b| self.pressed(b))
    }

    /// `true` if any of the buttons in the iter went down this frame.
    pub fn any_just_pressed(&self, buttons: impl IntoIterator<Item = T>) -> bool {
        buttons.into_iter().any(|b| self.just_pressed(b))
    }

    /// Iterator over currently-pressed buttons (order undefined).
    pub fn get_pressed(&self) -> impl Iterator<Item = &T> {
        self.pressed.iter()
    }

    /// Iterator over buttons pressed this frame (order undefined).
    pub fn get_just_pressed(&self) -> impl Iterator<Item = &T> {
        self.just_pressed.iter()
    }

    /// Iterator over buttons released this frame (order undefined).
    pub fn get_just_released(&self) -> impl Iterator<Item = &T> {
        self.just_released.iter()
    }

    /// Clear the `just_pressed` + `just_released` sets. Called once
    /// per frame by the host AFTER consumers have read the per-frame
    /// state. `pressed` is preserved.
    pub fn clear(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }

    /// Reset every state. Useful for test fixtures.
    pub fn reset_all(&mut self) {
        self.pressed.clear();
        self.just_pressed.clear();
        self.just_released.clear();
    }
}

/// `ButtonInput<KeyCode>` — keyboard state.
pub type KeyboardInput = ButtonInput<KeyCode>;

/// `ButtonInput<MouseButton>` — mouse-button state.
pub type MouseButtonInput = ButtonInput<MouseButton>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressed_returns_true_after_press() {
        let mut b: ButtonInput<KeyCode> = ButtonInput::new();
        assert!(!b.pressed(KeyCode::KeyW));
        b.press(KeyCode::KeyW);
        assert!(b.pressed(KeyCode::KeyW));
        assert!(b.just_pressed(KeyCode::KeyW));
    }

    #[test]
    fn just_pressed_clears_after_frame_pressed_persists() {
        let mut b = KeyboardInput::new();
        b.press(KeyCode::KeyA);
        assert!(b.just_pressed(KeyCode::KeyA));
        b.clear();
        assert!(!b.just_pressed(KeyCode::KeyA), "just_pressed must reset");
        assert!(b.pressed(KeyCode::KeyA), "pressed must persist");
    }

    #[test]
    fn just_released_set_on_release_after_press() {
        let mut b = KeyboardInput::new();
        b.press(KeyCode::KeyS);
        b.clear();
        b.release(KeyCode::KeyS);
        assert!(!b.pressed(KeyCode::KeyS));
        assert!(b.just_released(KeyCode::KeyS));
        b.clear();
        assert!(!b.just_released(KeyCode::KeyS));
    }

    #[test]
    fn auto_repeat_is_filtered() {
        // Bevy convention: if a button is already pressed, repeat
        // press events DO NOT re-add to just_pressed. Callers wanting
        // auto-repeat read the raw event stream.
        let mut b = KeyboardInput::new();
        b.press(KeyCode::KeyD);
        b.clear();
        b.press(KeyCode::KeyD);
        assert!(
            !b.just_pressed(KeyCode::KeyD),
            "repeat press without intervening release must not re-trip just_pressed"
        );
        assert!(b.pressed(KeyCode::KeyD));
    }

    #[test]
    fn release_all_clears_all_pressed_into_just_released() {
        let mut b = MouseButtonInput::new();
        b.press(MouseButton::Left);
        b.press(MouseButton::Right);
        b.clear();
        b.release_all();
        assert!(!b.pressed(MouseButton::Left));
        assert!(!b.pressed(MouseButton::Right));
        assert!(b.just_released(MouseButton::Left));
        assert!(b.just_released(MouseButton::Right));
    }

    #[test]
    fn release_without_prior_press_still_sets_just_released() {
        // Out-of-order safety: focus-loss + spurious release. The
        // invariant "the release was observed" matters even if the
        // matching press was missed.
        let mut b = KeyboardInput::new();
        b.release(KeyCode::Space);
        assert!(b.just_released(KeyCode::Space));
        assert!(!b.pressed(KeyCode::Space));
    }

    #[test]
    fn any_pressed_helper() {
        let mut b = KeyboardInput::new();
        b.press(KeyCode::KeyW);
        assert!(b.any_pressed([KeyCode::KeyW, KeyCode::KeyA]));
        assert!(!b.any_pressed([KeyCode::KeyA, KeyCode::KeyS]));
    }
}
