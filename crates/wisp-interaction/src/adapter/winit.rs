//! winit 0.30 adapter — translate `winit::event::WindowEvent` into
//! `wisp-interaction` vocabulary (`InputEvent`, `PointerLocation`,
//! `PointerId`, `WheelDelta`).
//!
//! Pure-function shape — pass in the winit event + supporting context
//! (modifiers state, last DPR, etc.), get back the equivalent
//! `wisp-interaction` payload. Tests cover the translation without
//! ever opening a winit window.
//!
//! Supported events (one-to-one):
//! - `WindowEvent::CursorMoved` → [`InputEvent::MouseMotion`] +
//!   `PointerLocation`.
//! - `WindowEvent::MouseInput` → [`InputEvent::MouseButton`].
//! - `WindowEvent::MouseWheel` → [`InputEvent::MouseWheel`] with
//!   `WheelDelta::Pixel` or `Line`.
//! - `WindowEvent::KeyboardInput` → [`InputEvent::Keyboard`] +
//!   [`KeyCode`] mapping (subset).
//! - `WindowEvent::Touch` → [`InputEvent::Touch`] +
//!   [`PointerId::Touch`].
//! - `WindowEvent::ModifiersChanged` → [`ModifierState`] update.
//! - `WindowEvent::Focused(false)` → [`InputEvent::FocusLost`]
//!   (the dispatcher should call `release_all` on any
//!   `ButtonInput<T>`).
//!
//! Not translated (out of scope or platform-specific):
//! - `WindowEvent::Ime` — text input belongs to the text editor, not
//!   the controller layer.
//! - `WindowEvent::TouchpadPressure` — exotic Apple-only.

use glam::Vec2;
use winit::event::{
    ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, TouchPhase as WinitTouchPhase,
};
use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};

use crate::input::{
    InputEvent, KeyCode, KeyboardEvent, ModifierState, MouseButton, MouseButtonEvent,
    MouseMotionEvent, MouseWheelEvent, TouchEvent, TouchPhase, WheelDelta,
};
use crate::pointer::{PointerId, PointerLocation};

/// Translate a winit mouse button to our cross-platform enum.
#[must_use]
pub fn translate_mouse_button(b: WinitMouseButton) -> MouseButton {
    match b {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Back => MouseButton::Back,
        WinitMouseButton::Forward => MouseButton::Forward,
        WinitMouseButton::Other(n) => MouseButton::Other(n),
    }
}

/// Translate a winit `MouseScrollDelta`.
///
/// Browsers + GTK report wheel as PIXELS (per the W3C `WheelEvent`
/// spec); winit normalises into `LineDelta` when the source is a
/// physical wheel and `PixelDelta` when it's a trackpad / touch
/// surface. We pass that distinction through unmodified so consumers
/// can scale accordingly.
#[must_use]
pub fn translate_scroll(delta: MouseScrollDelta) -> WheelDelta {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => WheelDelta::Line(Vec2::new(x, y)),
        MouseScrollDelta::PixelDelta(p) =>
        {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "PixelDelta in f64 always fits in f32 for one wheel event — see comment"
            )]
            WheelDelta::Pixel(Vec2::new(p.x as f32, p.y as f32))
        }
    }
}

/// Translate winit's element state into a press / release bool.
/// `true` = pressed, `false` = released.
#[must_use]
pub fn translate_state(state: ElementState) -> bool {
    matches!(state, ElementState::Pressed)
}

/// Translate the winit touch phase.
#[must_use]
pub fn translate_touch_phase(phase: WinitTouchPhase) -> TouchPhase {
    match phase {
        WinitTouchPhase::Started => TouchPhase::Started,
        WinitTouchPhase::Moved => TouchPhase::Moved,
        WinitTouchPhase::Ended => TouchPhase::Ended,
        WinitTouchPhase::Cancelled => TouchPhase::Cancelled,
    }
}

/// Translate winit modifier state.
#[must_use]
pub fn translate_modifiers(m: winit::keyboard::ModifiersState) -> ModifierState {
    ModifierState {
        shift: m.contains(winit::keyboard::ModifiersState::SHIFT),
        ctrl: m.contains(winit::keyboard::ModifiersState::CONTROL),
        alt: m.contains(winit::keyboard::ModifiersState::ALT),
        super_key: m.contains(winit::keyboard::ModifiersState::SUPER),
    }
}

/// Translate winit's physical key code. Returns `None` for any key
/// not in our enum (which is OK — host adapter can filter unmapped
/// keys silently).
#[must_use]
pub fn translate_key_code(key: PhysicalKey) -> Option<KeyCode> {
    let code = match key {
        PhysicalKey::Code(c) => c,
        PhysicalKey::Unidentified(_) => return None,
    };
    Some(match code {
        WinitKeyCode::KeyA => KeyCode::KeyA,
        WinitKeyCode::KeyB => KeyCode::KeyB,
        WinitKeyCode::KeyC => KeyCode::KeyC,
        WinitKeyCode::KeyD => KeyCode::KeyD,
        WinitKeyCode::KeyE => KeyCode::KeyE,
        WinitKeyCode::KeyF => KeyCode::KeyF,
        WinitKeyCode::KeyG => KeyCode::KeyG,
        WinitKeyCode::KeyH => KeyCode::KeyH,
        WinitKeyCode::KeyI => KeyCode::KeyI,
        WinitKeyCode::KeyJ => KeyCode::KeyJ,
        WinitKeyCode::KeyK => KeyCode::KeyK,
        WinitKeyCode::KeyL => KeyCode::KeyL,
        WinitKeyCode::KeyM => KeyCode::KeyM,
        WinitKeyCode::KeyN => KeyCode::KeyN,
        WinitKeyCode::KeyO => KeyCode::KeyO,
        WinitKeyCode::KeyP => KeyCode::KeyP,
        WinitKeyCode::KeyQ => KeyCode::KeyQ,
        WinitKeyCode::KeyR => KeyCode::KeyR,
        WinitKeyCode::KeyS => KeyCode::KeyS,
        WinitKeyCode::KeyT => KeyCode::KeyT,
        WinitKeyCode::KeyU => KeyCode::KeyU,
        WinitKeyCode::KeyV => KeyCode::KeyV,
        WinitKeyCode::KeyW => KeyCode::KeyW,
        WinitKeyCode::KeyX => KeyCode::KeyX,
        WinitKeyCode::KeyY => KeyCode::KeyY,
        WinitKeyCode::KeyZ => KeyCode::KeyZ,
        WinitKeyCode::Digit0 => KeyCode::Digit0,
        WinitKeyCode::Digit1 => KeyCode::Digit1,
        WinitKeyCode::Digit2 => KeyCode::Digit2,
        WinitKeyCode::Digit3 => KeyCode::Digit3,
        WinitKeyCode::Digit4 => KeyCode::Digit4,
        WinitKeyCode::Digit5 => KeyCode::Digit5,
        WinitKeyCode::Digit6 => KeyCode::Digit6,
        WinitKeyCode::Digit7 => KeyCode::Digit7,
        WinitKeyCode::Digit8 => KeyCode::Digit8,
        WinitKeyCode::Digit9 => KeyCode::Digit9,
        WinitKeyCode::F1 => KeyCode::F1,
        WinitKeyCode::F2 => KeyCode::F2,
        WinitKeyCode::F3 => KeyCode::F3,
        WinitKeyCode::F4 => KeyCode::F4,
        WinitKeyCode::F5 => KeyCode::F5,
        WinitKeyCode::F6 => KeyCode::F6,
        WinitKeyCode::F7 => KeyCode::F7,
        WinitKeyCode::F8 => KeyCode::F8,
        WinitKeyCode::F9 => KeyCode::F9,
        WinitKeyCode::F10 => KeyCode::F10,
        WinitKeyCode::F11 => KeyCode::F11,
        WinitKeyCode::F12 => KeyCode::F12,
        WinitKeyCode::ArrowUp => KeyCode::ArrowUp,
        WinitKeyCode::ArrowDown => KeyCode::ArrowDown,
        WinitKeyCode::ArrowLeft => KeyCode::ArrowLeft,
        WinitKeyCode::ArrowRight => KeyCode::ArrowRight,
        WinitKeyCode::Enter => KeyCode::Enter,
        WinitKeyCode::Space => KeyCode::Space,
        WinitKeyCode::Escape => KeyCode::Escape,
        WinitKeyCode::Tab => KeyCode::Tab,
        WinitKeyCode::Backspace => KeyCode::Backspace,
        WinitKeyCode::Delete => KeyCode::Delete,
        WinitKeyCode::Insert => KeyCode::Insert,
        WinitKeyCode::Home => KeyCode::Home,
        WinitKeyCode::End => KeyCode::End,
        WinitKeyCode::PageUp => KeyCode::PageUp,
        WinitKeyCode::PageDown => KeyCode::PageDown,
        WinitKeyCode::ShiftLeft => KeyCode::ShiftLeft,
        WinitKeyCode::ShiftRight => KeyCode::ShiftRight,
        WinitKeyCode::ControlLeft => KeyCode::ControlLeft,
        WinitKeyCode::ControlRight => KeyCode::ControlRight,
        WinitKeyCode::AltLeft => KeyCode::AltLeft,
        WinitKeyCode::AltRight => KeyCode::AltRight,
        WinitKeyCode::SuperLeft => KeyCode::SuperLeft,
        WinitKeyCode::SuperRight => KeyCode::SuperRight,
        WinitKeyCode::CapsLock => KeyCode::CapsLock,
        WinitKeyCode::Backquote => KeyCode::Backquote,
        WinitKeyCode::Minus => KeyCode::Minus,
        WinitKeyCode::Equal => KeyCode::Equal,
        WinitKeyCode::BracketLeft => KeyCode::BracketLeft,
        WinitKeyCode::BracketRight => KeyCode::BracketRight,
        WinitKeyCode::Backslash => KeyCode::Backslash,
        WinitKeyCode::Semicolon => KeyCode::Semicolon,
        WinitKeyCode::Quote => KeyCode::Quote,
        WinitKeyCode::Comma => KeyCode::Comma,
        WinitKeyCode::Period => KeyCode::Period,
        WinitKeyCode::Slash => KeyCode::Slash,
        _ => return None,
    })
}

/// Build a [`PointerLocation`] from a winit cursor position + the
/// current modifier state.
#[must_use]
pub fn pointer_location(
    position: winit::dpi::PhysicalPosition<f64>,
    modifiers: ModifierState,
) -> PointerLocation {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "winit PhysicalPosition<f64> in pixels fits comfortably in f32 for any plausible display"
    )]
    PointerLocation {
        viewport: Vec2::new(position.x as f32, position.y as f32),
        modifiers,
    }
}

/// `winit::event::Touch::id` is `u64`; map directly into
/// [`PointerId::Touch`].
#[must_use]
pub fn pointer_id_from_touch(id: u64) -> PointerId {
    PointerId::Touch(id)
}

/// Build a synthetic [`InputEvent::Keyboard`] from the translated key
/// + state + repeat flag.
#[must_use]
pub fn key_event(
    key: KeyCode,
    pressed: bool,
    repeat: bool,
    modifiers: ModifierState,
) -> InputEvent {
    InputEvent::Keyboard(KeyboardEvent {
        key,
        pressed,
        repeat,
        modifiers,
    })
}

/// Build a synthetic [`InputEvent::MouseButton`].
#[must_use]
pub fn mouse_button_event(
    button: MouseButton,
    pressed: bool,
    modifiers: ModifierState,
) -> InputEvent {
    InputEvent::MouseButton(MouseButtonEvent {
        button,
        pressed,
        modifiers,
    })
}

/// Build a synthetic [`InputEvent::MouseMotion`].
#[must_use]
pub fn mouse_motion_event(delta: Vec2) -> InputEvent {
    InputEvent::MouseMotion(MouseMotionEvent { delta })
}

/// Build a synthetic [`InputEvent::MouseWheel`].
#[must_use]
pub fn mouse_wheel_event(delta: WheelDelta, modifiers: ModifierState) -> InputEvent {
    InputEvent::MouseWheel(MouseWheelEvent { delta, modifiers })
}

/// Build a synthetic [`InputEvent::Touch`].
#[must_use]
pub fn touch_event(id: u64, phase: TouchPhase, position: Vec2) -> InputEvent {
    InputEvent::Touch(TouchEvent {
        id,
        position,
        phase,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_button_translation_covers_all_winit_variants() {
        assert_eq!(
            translate_mouse_button(WinitMouseButton::Left),
            MouseButton::Left
        );
        assert_eq!(
            translate_mouse_button(WinitMouseButton::Right),
            MouseButton::Right
        );
        assert_eq!(
            translate_mouse_button(WinitMouseButton::Middle),
            MouseButton::Middle
        );
        assert_eq!(
            translate_mouse_button(WinitMouseButton::Back),
            MouseButton::Back
        );
        assert_eq!(
            translate_mouse_button(WinitMouseButton::Forward),
            MouseButton::Forward
        );
        assert_eq!(
            translate_mouse_button(WinitMouseButton::Other(42)),
            MouseButton::Other(42)
        );
    }

    #[test]
    fn scroll_translation_preserves_pixel_vs_line() {
        let line = translate_scroll(MouseScrollDelta::LineDelta(0.0, 3.0));
        assert!(matches!(line, WheelDelta::Line(d) if (d.y - 3.0).abs() < 1e-6));

        let pixel = translate_scroll(MouseScrollDelta::PixelDelta(
            winit::dpi::PhysicalPosition::new(0.0, 120.0),
        ));
        assert!(matches!(pixel, WheelDelta::Pixel(d) if (d.y - 120.0).abs() < 1e-6));
    }

    #[test]
    fn key_translation_subset_returns_some_for_known_keys_and_none_for_unknown() {
        assert_eq!(
            translate_key_code(PhysicalKey::Code(WinitKeyCode::KeyW)),
            Some(KeyCode::KeyW)
        );
        assert_eq!(
            translate_key_code(PhysicalKey::Code(WinitKeyCode::ArrowUp)),
            Some(KeyCode::ArrowUp)
        );
        // Numpad keys are not mapped today — should return None.
        assert_eq!(
            translate_key_code(PhysicalKey::Code(WinitKeyCode::Numpad0)),
            None
        );
    }

    #[test]
    fn modifier_translation_round_trips() {
        let m = winit::keyboard::ModifiersState::SHIFT | winit::keyboard::ModifiersState::CONTROL;
        let out = translate_modifiers(m);
        assert!(out.shift);
        assert!(out.ctrl);
        assert!(!out.alt);
        assert!(!out.super_key);
    }

    #[test]
    fn touch_phase_translation_covers_all_variants() {
        assert_eq!(
            translate_touch_phase(WinitTouchPhase::Started),
            TouchPhase::Started
        );
        assert_eq!(
            translate_touch_phase(WinitTouchPhase::Moved),
            TouchPhase::Moved
        );
        assert_eq!(
            translate_touch_phase(WinitTouchPhase::Ended),
            TouchPhase::Ended
        );
        assert_eq!(
            translate_touch_phase(WinitTouchPhase::Cancelled),
            TouchPhase::Cancelled
        );
    }

    #[test]
    fn pointer_location_carries_modifiers() {
        let modifiers = ModifierState {
            shift: true,
            ctrl: false,
            alt: false,
            super_key: false,
        };
        let loc = pointer_location(winit::dpi::PhysicalPosition::new(100.0, 200.0), modifiers);
        assert_eq!(loc.viewport, Vec2::new(100.0, 200.0));
        assert!(loc.modifiers.shift);
    }
}
